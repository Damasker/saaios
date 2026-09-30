/* ONE CardPower DOWN(4)→UP(1) tray analog (BuildSetSimCardPower 0x024c).
   Proven opcode/states only. No VerifyPin, no Radio, no POWER_OFF/crash.
   Prints app/pin1/remain/card only — never PIN/AID/IMSI/ICCID. */
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

static unsigned le16(const uint8_t *p) { return p[0] | ((unsigned)p[1] << 8); }
static uint32_t le32(const uint8_t *p) { return le16(p) | ((uint32_t)le16(p + 2) << 16); }
static int64_t now_ms(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}
static int frame_size(const uint8_t *p, size_t n) {
    if (n < 6) return 0;
    if (p[0] > 2) return -1;
    unsigned min = p[0] == 2 ? 8 : 12;
    unsigned len = le16(p + 4);
    if (len < min || len > 65536) return -1;
    return n < len ? 0 : (int)len;
}
static int open_node(const char *node, const char *sysdev) {
    unsigned maj = 0, min = 0;
    FILE *f = fopen(sysdev, "r");
    if (!f || fscanf(f, "%u:%u", &maj, &min) != 2) {
        if (f) fclose(f);
        return -1;
    }
    fclose(f);
    int fd = open(node, O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    struct stat st;
    if (fd < 0 || fstat(fd, &st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev) != maj || minor(st.st_rdev) != min) {
        if (fd >= 0) close(fd);
        return -1;
    }
    return fd;
}
static void service_rfs(int fd) {
    uint8_t buf[256];
    ssize_t n = read(fd, buf, sizeof buf);
    if (n <= 0) return;
    if (n == (ssize_t)sizeof buf) {
        uint8_t dump[256];
        while (read(fd, dump, sizeof dump) > 0) {
        }
        return;
    }
    if (n == 12 && buf[0] == 0x07 && !buf[1] && !buf[2] && !buf[3] &&
        buf[4] == 0x04 && !buf[5] && !buf[6] && !buf[7] &&
        buf[8] == 0x03 && !buf[9] && !buf[10] && !buf[11]) {
        static const uint8_t reply[16] = {
            0x03, 0, 0, 0, 0x08, 0, 0, 0, 0, 0, 0, 0, 0x03, 0, 0, 0};
        (void)write(fd, reply, sizeof reply);
        return;
    }
}

static int g_app = -1, g_pin = -1, g_remain = -1, g_card = -1;

static void note_sim(const uint8_t *b, int len) {
    if (len < 75 || b[10]) return;
    g_card = b[12];
    g_app = b[17];
    g_pin = b[72];
    g_remain = b[74];
    printf("SIM card=%d app=%d pin1=%d remain=%d\n", g_card, g_app, g_pin, g_remain);
}

static int exchange_ex(int ipc, int rfs, const uint8_t *req, size_t req_len, unsigned id,
                       int32_t want_tok, int timeout_ms, uint8_t *resp, int *resp_len) {
    ssize_t wr = write(ipc, req, req_len);
    if (wr != (ssize_t)req_len) {
        printf("write fail wr=%zd errno=%d\n", wr, errno);
        return -1;
    }
    uint8_t buf[8192];
    size_t used = 0;
    unsigned seen = 0, seen_id = 0;
    int64_t end = now_ms() + timeout_ms;
    while (now_ms() < end) {
        struct pollfd pfd[2] = {{ipc, POLLIN, 0}, {rfs, POLLIN, 0}};
        if (poll(pfd, 2, 250) < 0) {
            if (errno == EINTR) continue;
            return -1;
        }
        if (pfd[1].revents & POLLIN) service_rfs(rfs);
        if (pfd[0].revents & POLLIN && used < sizeof buf) {
            ssize_t n = read(ipc, buf + used, sizeof buf - used);
            if (n > 0) used += (size_t)n;
        }
        size_t off = 0;
        while (off < used) {
            int len = frame_size(buf + off, used - off);
            if (len < 0) {
                puts("malformed frame");
                return -1;
            }
            if (!len) break;
            unsigned fid = le16(buf + off + 2);
            uint32_t ftok = le32(buf + off + 6);
            seen++;
            if (fid == 0x0200) note_sim(buf + off, len);
            if (fid == id && (want_tok < 0 || ftok == (uint32_t)want_tok)) {
                int keep = len > 512 ? 512 : len;
                memcpy(resp, buf + off, (size_t)keep);
                *resp_len = keep;
                return 0;
            }
            if (fid == id) seen_id++;
            off += (size_t)len;
        }
        if (off) {
            memmove(buf, buf + off, used - off);
            used -= off;
        }
    }
    printf("timeout id=0x%x seen_frames=%u seen_id=%u\n", id, seen, seen_id);
    return 1;
}

static int send_card(int ipc, int rfs, uint8_t state, uint32_t tok) {
    uint8_t req[13] = {0}, resp[512];
    int n = 0;
    req[2] = 0x4c;
    req[3] = 0x02;
    req[4] = 13;
    req[6] = (uint8_t)tok;
    req[12] = state;
    printf("card power state=%u\n", state);
    int rc = exchange_ex(ipc, rfs, req, 13, 0x024c, (int32_t)tok, 15000, resp, &n);
    if (rc) return rc;
    printf("card power length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    return 0;
}

static int query_sim(int ipc, int rfs, uint32_t tok) {
    uint8_t req[12] = {0}, resp[512];
    int n = 0;
    req[2] = 0x00;
    req[3] = 0x02;
    req[4] = 12;
    req[6] = (uint8_t)tok;
    return exchange_ex(ipc, rfs, req, 12, 0x0200, (int32_t)tok, 15000, resp, &n);
}

static void wait_drain(int ipc, int rfs, int ms) {
    int64_t end = now_ms() + ms;
    while (now_ms() < end) {
        struct pollfd pfd[2] = {{ipc, POLLIN, 0}, {rfs, POLLIN, 0}};
        poll(pfd, 2, 200);
        if (pfd[1].revents & POLLIN) service_rfs(rfs);
        if (pfd[0].revents & POLLIN) {
            uint8_t junk[1024];
            (void)read(ipc, junk, sizeof junk);
        }
    }
}

int main(void) {
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1 || strcmp(state, "ONLINE")) {
        puts("requires ONLINE");
        return 1;
    }
    if (f) fclose(f);
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        puts("lock busy");
        return 1;
    }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) {
        puts("open fail");
        return 1;
    }

    puts("=== cardpower-reseat pre ===");
    if (query_sim(ipc, rfs, 1)) puts("SIM timeout");
    int pre_app = g_app, pre_pin = g_pin, pre_card = g_card, pre_remain = g_remain;
    if (g_remain >= 0 && g_remain <= 1) {
        puts("STOP remain<=1");
        return 2;
    }

    /* Proven HAL1.6 cycle: POWER_DOWN(4) then POWER_UP(1). */
    if (send_card(ipc, rfs, 4, 2)) return 1;
    wait_drain(ipc, rfs, 3000);
    puts("=== after DOWN ===");
    if (query_sim(ipc, rfs, 3)) puts("SIM timeout");
    int mid_app = g_app, mid_pin = g_pin, mid_card = g_card;

    if (send_card(ipc, rfs, 1, 4)) return 1;
    wait_drain(ipc, rfs, 8000);
    puts("=== cardpower-reseat post ===");
    if (query_sim(ipc, rfs, 5)) puts("SIM timeout");

    printf("RESULT pre card=%d app=%d pin1=%d remain=%d\n", pre_card, pre_app, pre_pin,
           pre_remain);
    printf("RESULT mid card=%d app=%d pin1=%d\n", mid_card, mid_app, mid_pin);
    printf("RESULT post card=%d app=%d pin1=%d remain=%d\n", g_card, g_app, g_pin, g_remain);
    if (pre_card != g_card || (pre_card == 1 && mid_card == 0))
        puts("NOTE card_state changed (ABSENT/PRESENT edge possible)");
    if (pre_pin == 2 && g_pin == 1) puts("NOTE pin1 2->1 NOT_VERIFIED (VerifyPin window)");
    if (pre_app == 2 && g_app == 2 && pre_pin == g_pin && pre_card == g_card)
        puts("NOTE CardPower != physical reseat (no EDGE shape change)");

    close(ipc);
    close(rfs);
    close(lock);
    return 0;
}
