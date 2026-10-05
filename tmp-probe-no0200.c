/* Live: never GET 0x0200 until end. Radio ON + LTE_ONLY + AllowData;
 * poll 0x0700/0x0701/rmnet ~75s; then one 0x0200.
 * Requires ONLINE. No PIN/PUK/EFS. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/file.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <time.h>
#include <unistd.h>
#include <signal.h>

static unsigned le16(const uint8_t *p) { return p[0] | ((unsigned)p[1] << 8); }
static uint32_t le32(const uint8_t *p) { return le16(p) | ((uint32_t)le16(p + 2) << 16); }
static void put_le16(uint8_t *p, unsigned v) { p[0] = (uint8_t)v; p[1] = (uint8_t)(v >> 8); }
static void put_le32(uint8_t *p, uint32_t v) {
    put_le16(p, (unsigned)v);
    put_le16(p + 2, (unsigned)(v >> 16));
}
static int64_t now_ms(void) {
    struct timespec t;
    if (clock_gettime(CLOCK_MONOTONIC, &t)) return -1;
    return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}
static int frame_size(const uint8_t *p, size_t n) {
    if (n < 6) return 0;
    if (p[0] > 2) return -1;
    unsigned min = p[0] == 2 ? 8 : 12;
    unsigned len = le16(p + 4);
    if (len < min) return -1;
    return n < len ? 0 : (int)len;
}

static int open_ipc(void) {
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f) return -1;
    if (fscanf(f, "%31s", state) != 1) { fclose(f); return -1; }
    fclose(f);
    if (strcmp(state, "ONLINE")) {
        printf("requires ONLINE got=%s\n", state);
        return -1;
    }
    unsigned maj = 0, min = 0;
    f = fopen("/sys/class/cpif/umts_ipc0/dev", "r");
    if (!f || fscanf(f, "%u:%u", &maj, &min) != 2) {
        if (f) fclose(f);
        return -1;
    }
    fclose(f);
    int fd = open("/dev/umts_ipc0", O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    struct stat st;
    if (fd < 0 || fstat(fd, &st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev) != maj || minor(st.st_rdev) != min) {
        if (fd >= 0) close(fd);
        return -1;
    }
    return fd;
}

/* Wait for response matching id+token; print compact line. Returns 0 ok, 2 err, 3 timeout. */
static int wait_rsp(int fd, unsigned id, uint32_t token, int64_t deadline_ms,
                    uint8_t *out, size_t out_cap, int *out_len) {
    uint8_t buffer[65536];
    size_t used = 0;
    unsigned frames = 0;
    while (now_ms() < deadline_ms && frames < 256) {
        struct pollfd pfd = {fd, POLLIN, 0};
        int ready = poll(&pfd, 1, 400);
        if (ready < 0) {
            if (errno == EINTR) continue;
            return 1;
        }
        if (!ready) continue;
        if (pfd.revents & (POLLERR | POLLHUP | POLLNVAL)) return 1;
        if (!(pfd.revents & POLLIN)) continue;
        ssize_t n = read(fd, buffer + used, sizeof(buffer) - used);
        if (n < 0 && (errno == EINTR || errno == EAGAIN)) continue;
        if (n <= 0) return 1;
        used += (size_t)n;
        while (used) {
            int len = frame_size(buffer, used);
            if (len < 0) {
                printf("malformed framing\n");
                return 1;
            }
            if (!len) break;
            frames++;
            if (buffer[0] == 1 && le16(buffer + 2) == id && le32(buffer + 6) == token) {
                if (out && out_cap) {
                    size_t c = (size_t)len < out_cap ? (size_t)len : out_cap;
                    memcpy(out, buffer, c);
                    if (out_len) *out_len = (int)c;
                }
                return buffer[10] ? 2 : 0;
            }
            used -= (size_t)len;
            memmove(buffer, buffer + len, used);
        }
        if (used == sizeof(buffer)) return 1;
    }
    printf("timeout id=0x%04x tok=%u frames=%u\n", id, token, frames);
    return 3;
}

static int send_raw(int fd, const uint8_t *req, size_t n) {
    ssize_t w = write(fd, req, n);
    return (w == (ssize_t)n) ? 0 : -1;
}

static void rmnet_snap(const char *tag) {
    printf("RMNET %s:", tag);
    for (int i = 0; i < 3; i++) {
        char path[80];
        unsigned long rx = 0, tx = 0;
        snprintf(path, sizeof(path), "/sys/class/net/rmnet%d/statistics/rx_bytes", i);
        FILE *f = fopen(path, "r");
        if (f) {
            if (fscanf(f, "%lu", &rx) != 1) rx = 0;
            fclose(f);
        }
        snprintf(path, sizeof(path), "/sys/class/net/rmnet%d/statistics/tx_bytes", i);
        f = fopen(path, "r");
        if (f) {
            if (fscanf(f, "%lu", &tx) != 1) tx = 0;
            fclose(f);
        }
        printf(" rmnet%d rx=%lu tx=%lu", i, rx, tx);
    }
    {
        char line[160] = "-";
        FILE *p = popen("ip -4 -o addr show 2>/dev/null | grep -E 'rmnet[0-9]' | head -1 || true", "r");
        if (p) {
            if (!fgets(line, sizeof(line), p)) strcpy(line, "-");
            pclose(p);
            char *nl = strchr(line, '\n');
            if (nl) *nl = 0;
        }
        printf(" ipv4_line=%s\n", line);
    }
}

int main(void) {
    alarm(120);
    setvbuf(stdout, NULL, _IONBF, 0);
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        puts("lock busy");
        return 1;
    }
    int fd = open_ipc();
    if (fd < 0) {
        close(lock);
        return 1;
    }
    printf("START no-0x0200 UP=%lld\n", (long long)now_ms());
    rmnet_snap("t0");

    uint32_t tok = 10;
    uint8_t rsp[256];
    int rlen = 0;
    int rc;

    /* 1) RadioPower ON: 0x0800 len18 word=2 */
    {
        uint8_t req[18] = {0};
        put_le16(req + 2, 0x0800);
        put_le16(req + 4, 18);
        put_le32(req + 6, tok);
        put_le32(req + 12, 2); /* ON */
        if (send_raw(fd, req, sizeof(req))) {
            puts("write RadioPower fail");
            return 1;
        }
        rc = wait_rsp(fd, 0x0800, tok, now_ms() + 8000, rsp, sizeof(rsp), &rlen);
        printf("RadioPowerON rc=%d len=%d err=%u\n", rc, rlen, rlen > 10 ? rsp[10] : 0xff);
        tok++;
    }

    /* 2) optional radio GET 0x0801 */
    {
        uint8_t req[12] = {0};
        put_le16(req + 2, 0x0801);
        put_le16(req + 4, 12);
        put_le32(req + 6, tok);
        if (send_raw(fd, req, sizeof(req))) {
            puts("write RadioGet fail");
            return 1;
        }
        rc = wait_rsp(fd, 0x0801, tok, now_ms() + 8000, rsp, sizeof(rsp), &rlen);
        uint32_t st = (rlen >= 16 && !rsp[10]) ? le32(rsp + 12) : 0xffffffffu;
        printf("RadioGet rc=%d len=%d err=%u state=%u\n", rc, rlen, rlen > 10 ? rsp[10] : 0xff, st);
        tok++;
    }

    /* 3) set preferred LTE_ONLY=11 via 0x070a len16 */
    {
        uint8_t req[16] = {0};
        put_le16(req + 2, 0x070a);
        put_le16(req + 4, 16);
        put_le32(req + 6, tok);
        put_le32(req + 12, 11);
        if (send_raw(fd, req, sizeof(req))) {
            puts("write Pref fail");
            return 1;
        }
        rc = wait_rsp(fd, 0x070a, tok, now_ms() + 8000, rsp, sizeof(rsp), &rlen);
        printf("PrefLTE rc=%d len=%d err=%u\n", rc, rlen, rlen > 10 ? rsp[10] : 0xff);
        tok++;
    }

    /* 4) AllowData 0x0710 len13 byte12=1 */
    {
        uint8_t req[13] = {0};
        put_le16(req + 2, 0x0710);
        put_le16(req + 4, 13);
        put_le32(req + 6, tok);
        req[12] = 1;
        if (send_raw(fd, req, sizeof(req))) {
            puts("write AllowData fail");
            return 1;
        }
        rc = wait_rsp(fd, 0x0710, tok, now_ms() + 8000, rsp, sizeof(rsp), &rlen);
        printf("AllowData rc=%d len=%d err=%u\n", rc, rlen, rlen > 10 ? rsp[10] : 0xff);
        tok++;
    }

    /* 5) auto selection 0x0704 (proven empty) */
    {
        uint8_t req[12] = {0};
        put_le16(req + 2, 0x0704);
        put_le16(req + 4, 12);
        put_le32(req + 6, tok);
        if (send_raw(fd, req, sizeof(req))) {
            puts("write AutoSel fail");
            return 1;
        }
        rc = wait_rsp(fd, 0x0704, tok, now_ms() + 8000, rsp, sizeof(rsp), &rlen);
        printf("AutoSel rc=%d len=%d err=%u\n", rc, rlen, rlen > 10 ? rsp[10] : 0xff);
        tok++;
    }

    /* 6) poll voice/data/rmnet ~75s — NEVER 0x0200 */
    int64_t end = now_ms() + 75000;
    int saw_reg = 0, saw_rmnet = 0;
    int round = 0;
    while (now_ms() < end) {
        round++;
        /* voice 0x0700 */
        {
            uint8_t req[12] = {0};
            put_le16(req + 2, 0x0700);
            put_le16(req + 4, 12);
            put_le32(req + 6, tok);
            if (send_raw(fd, req, sizeof(req))) return 1;
            rc = wait_rsp(fd, 0x0700, tok, now_ms() + 5000, rsp, sizeof(rsp), &rlen);
            unsigned reg = (rlen >= 16 && !rsp[10]) ? rsp[12] : 0xff;
            unsigned tech = (rlen >= 16 && !rsp[10]) ? rsp[15] : 0xff;
            printf("V%02d rc=%d reg=%u tech=%u\n", round, rc, reg, tech);
            if (reg == 1 || reg == 5) saw_reg = 1;
            tok++;
        }
        /* data 0x0701 */
        {
            uint8_t req[12] = {0};
            put_le16(req + 2, 0x0701);
            put_le16(req + 4, 12);
            put_le32(req + 6, tok);
            if (send_raw(fd, req, sizeof(req))) return 1;
            rc = wait_rsp(fd, 0x0701, tok, now_ms() + 5000, rsp, sizeof(rsp), &rlen);
            unsigned reg = (rlen >= 16 && !rsp[10]) ? rsp[12] : 0xff;
            unsigned rej = (rlen >= 16 && !rsp[10]) ? rsp[13] : 0xff;
            unsigned tech = (rlen >= 16 && !rsp[10]) ? rsp[15] : 0xff;
            printf("D%02d rc=%d reg=%u rej=%u tech=%u\n", round, rc, reg, rej, tech);
            if (reg == 1 || reg == 5) saw_reg = 1;
            tok++;
        }
        rmnet_snap(round == 1 ? "p1" : "pn");
        /* check rx */
        {
            FILE *f = fopen("/sys/class/net/rmnet0/statistics/rx_bytes", "r");
            unsigned long rx = 0;
            if (f) {
                fscanf(f, "%lu", &rx);
                fclose(f);
            }
            if (rx > 0) saw_rmnet = 1;
        }
        if (saw_reg || saw_rmnet) {
            printf("BEARER_HINT saw_reg=%d saw_rmnet=%d — continue chase\n", saw_reg, saw_rmnet);
        }
        sleep(5);
    }

    /* 7) ONE final 0x0200 */
    {
        uint8_t req[12] = {0};
        put_le16(req + 2, 0x0200);
        put_le16(req + 4, 12);
        put_le32(req + 6, tok);
        if (send_raw(fd, req, sizeof(req))) {
            puts("write SIM fail");
            return 1;
        }
        rc = wait_rsp(fd, 0x0200, tok, now_ms() + 8000, rsp, sizeof(rsp), &rlen);
        unsigned card = 0xff, pin = 0xff, apps = 0xff, app = 0xff, pin1 = 0xff;
        if (rlen >= 15 && !rsp[10]) {
            card = rsp[12];
            pin = rsp[13];
            apps = rsp[14];
        }
        /* app_state often at byte 17 on longer frames (prior live) */
        if (rlen >= 18 && !rsp[10]) app = rsp[17];
        if (rlen >= 20 && !rsp[10]) pin1 = rsp[19];
        printf("FINAL_SIM rc=%d len=%d err=%u card=%u upin=%u apps=%u app=%u pin1=%u\n",
               rc, rlen, rlen > 10 ? rsp[10] : 0xff, card, pin, apps, app, pin1);
        /* Do not dump payload hex — may include AID. */
    }
    rmnet_snap("end");
    printf("RESULT saw_reg=%d saw_rmnet=%d\n", saw_reg, saw_rmnet);
    close(fd);
    close(lock);
    return 0;
}
