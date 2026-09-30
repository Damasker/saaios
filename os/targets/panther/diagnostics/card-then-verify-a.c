/* CardPower DOWN→UP → confirm pin1=1 → VerifyPin A (then B if allowed).
   RFS-aware, long wait, match 0x0201 any token. Never prints PIN/AID digits.
   Stop if remain<=1. */
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
    if (!f || fscanf(f, "%u:%u", &maj, &min) != 2) { if (f) fclose(f); return -1; }
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
        while (read(fd, dump, sizeof dump) > 0) {}
        puts("rfs large drained");
        return;
    }
    printf("rfs bytes=%zd cmd=%u\n", n, buf[0]);
    if (n == 12 && buf[0] == 0x07 && !buf[1] && !buf[2] && !buf[3] &&
        buf[4] == 0x04 && !buf[5] && !buf[6] && !buf[7] &&
        buf[8] == 0x03 && !buf[9] && !buf[10] && !buf[11]) {
        static const uint8_t reply[16] = {
            0x03,0,0,0, 0x08,0,0,0, 0,0,0,0, 0x03,0,0,0
        };
        (void)write(fd, reply, sizeof reply);
        puts("rfs unprotect reply");
        return;
    }
    if (n == 20 && buf[0] == 0x03 && le32(buf + 4) == 12) {
        puts("rfs op_status no reply");
        return;
    }
    if (n >= 24 && buf[0] == 0x06) {
        puts(le32(buf + 20) == 2 ? "rfs write op=2 refused" : "rfs io no reply");
        return;
    }
    puts("rfs ignored");
}

static int g_app = -1, g_pin = -1, g_remain = -1, g_aid_len = 0;
static uint8_t g_aid[16];

static void note_sim(const uint8_t *b, int len) {
    if (len < 75 || b[10]) return;
    g_app = b[17];
    g_pin = b[72];
    g_remain = b[74];
    g_aid_len = 0;
    memset(g_aid, 0, sizeof g_aid);
    if (len > 18) {
        unsigned n = b[18];
        if (n > 16) n = 16;
        if (19 + (int)n <= len) {
            g_aid_len = (int)n;
            if (n) memcpy(g_aid, b + 19, n);
        }
    }
    printf("SIM app=%d pin1=%d remain=%d aid_len=%d\n", g_app, g_pin, g_remain, g_aid_len);
}

/* Match id; if want_tok<0 accept any token for that id. */
static int exchange_ex(int ipc, int rfs, const uint8_t *req, size_t req_len,
                       unsigned id, int32_t want_tok, int timeout_ms,
                       uint8_t *resp, int *resp_len, int verbose) {
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
        if (poll(pfd, 2, 250) < 0) { if (errno == EINTR) continue; return -1; }
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
            if (fid == id) {
                seen_id++;
                if (want_tok < 0 || ftok == (uint32_t)want_tok) {
                    int keep = len > 512 ? 512 : len;
                    memcpy(resp, buf + off, (size_t)keep);
                    *resp_len = keep;
                    if (verbose)
                        printf("matched id=0x%x tok=%u len=%d\n", id, ftok, len);
                    return 0;
                }
                if (verbose)
                    printf("id=0x%x wrong_tok=%u (want %d) len=%d\n", id, ftok, want_tok, len);
            } else if (verbose && fid != 0x0200) {
                printf("ipc id=%u tok=%u length=%d\n", fid, ftok, len);
            }
            off += (size_t)len;
        }
        if (off) { memmove(buf, buf + off, used - off); used -= off; }
    }
    printf("timeout id=0x%x seen_frames=%u seen_id=%u\n", id, seen, seen_id);
    return 1;
}

static int send_card(int ipc, int rfs, uint8_t state, uint32_t tok) {
    uint8_t req[13] = {0}, resp[512];
    int n = 0;
    req[2] = 0x4c; req[3] = 0x02; req[4] = 13; req[6] = (uint8_t)tok; req[12] = state;
    printf("card power state=%u\n", state);
    int rc = exchange_ex(ipc, rfs, req, 13, 0x024c, (int32_t)tok, 15000, resp, &n, 1);
    if (rc) return rc;
    printf("card power length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    return 0;
}
static int query_sim(int ipc, int rfs, uint32_t tok) {
    uint8_t req[12] = {0}, resp[512];
    int n = 0;
    req[2] = 0x00; req[3] = 0x02; req[4] = 12; req[6] = (uint8_t)tok;
    return exchange_ex(ipc, rfs, req, 12, 0x0200, (int32_t)tok, 15000, resp, &n, 0);
}
static int fill_verify(uint8_t *req, uint32_t tok, int which) {
    if (which != 1 && which != 2) return -1;
    memset(req, 0, 38);
    req[2] = 0x01; req[3] = 0x02; req[4] = 38; req[6] = (uint8_t)tok;
    req[12] = 4;
    {
        uint8_t d = which == 1 ? '0' : '1';
        for (int i = 0; i < 4; i++) req[13 + i] = d;
    }
    if (g_aid_len > 0 && g_aid_len <= 16) {
        req[21] = (uint8_t)g_aid_len;
        memcpy(req + 22, g_aid, (size_t)g_aid_len);
    }
    return 38;
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
static int radio_on(int ipc, int rfs) {
    uint8_t req[18] = {0}, resp[512];
    int n = 0;
    req[2] = 0x00; req[3] = 0x08; req[4] = 18; req[6] = 40; req[12] = 2;
    puts("radio ON");
    return exchange_ex(ipc, rfs, req, 18, 0x0800, 40, 10000, resp, &n, 1);
}
static int verify_one(int ipc, int rfs, int which, uint32_t tok) {
    uint8_t req[38], resp[512];
    int n = 0;
    if (fill_verify(req, tok, which) != 38) return -1;
    printf("VerifyPin candidate=%c aid_len=%d timeout_ms=45000\n",
           which == 1 ? 'A' : 'B', g_aid_len);
    /* Match any token: CP sometimes echoes differently under load. */
    int rc = exchange_ex(ipc, rfs, req, 38, 0x0201, -1, 45000, resp, &n, 1);
    memset(req, 0, sizeof req);
    if (rc) {
        memset(resp, 0, sizeof resp);
        return rc;
    }
    unsigned err = n > 10 ? resp[10] : 0;
    unsigned remain = n >= 16 ? le32(resp + 12) : 0;
    printf("VerifyPin length=%d error_raw=%u remain_raw=%u\n", n, err, remain);
    if (n >= 16) g_remain = (int)remain;
    memset(resp, 0, sizeof resp);
    return (int)err; /* 0 ok, >0 protocol error, -1 write fail handled above */
}
static void chase(int ipc, int rfs) {
    uint8_t req[16], resp[512];
    int n = 0;
    memset(req, 0, sizeof req);
    req[2] = 0x0a; req[3] = 0x07; req[4] = 16; req[6] = 50; req[12] = 11;
    puts("set preferred LTE");
    (void)exchange_ex(ipc, rfs, req, 16, 0x070a, 50, 10000, resp, &n, 0);
    memset(req, 0, sizeof req);
    req[2] = 0x04; req[3] = 0x07; req[4] = 12; req[6] = 51;
    puts("set selection auto");
    (void)exchange_ex(ipc, rfs, req, 12, 0x0704, 51, 10000, resp, &n, 0);
    for (int i = 0; i < 15; i++) {
        memset(req, 0, sizeof req);
        req[2] = 0x01; req[3] = 0x07; req[4] = 12; req[6] = (uint8_t)(60 + i);
        n = 0;
        if (!exchange_ex(ipc, rfs, req, 12, 0x0701, 60 + i, 8000, resp, &n, 0) && n >= 16)
            printf("data_reg=%u tech=%u\n", resp[12], resp[15]);
        if (n >= 16 && resp[12] >= 1 && resp[12] <= 5) break;
        wait_drain(ipc, rfs, 2000);
    }
    for (int i = 0; i < 4; i++) {
        char path[64];
        snprintf(path, sizeof path, "/sys/class/net/rmnet%d", i);
        if (access(path, F_OK)) continue;
        char cmd[80];
        snprintf(cmd, sizeof cmd, "ip link set rmnet%d up 2>/dev/null", i);
        (void)system(cmd);
        snprintf(path, sizeof path, "/sys/class/net/rmnet%d/statistics/rx_bytes", i);
        FILE *f = fopen(path, "r");
        unsigned long long rx = 0;
        if (f) { if (fscanf(f, "%llu", &rx) != 1) rx = 0; fclose(f); }
        printf("rmnet%d rx=%llu\n", i, rx);
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
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("open fail"); return 1; }

    puts("=== pre ===");
    if (query_sim(ipc, rfs, 1)) puts("SIM timeout");
    if (g_remain >= 0 && g_remain <= 1) { puts("STOP remain<=1"); return 2; }

    if (send_card(ipc, rfs, 4, 2)) return 1;
    wait_drain(ipc, rfs, 3000);
    if (send_card(ipc, rfs, 1, 3)) return 1;
    wait_drain(ipc, rfs, 8000);
    (void)radio_on(ipc, rfs);
    wait_drain(ipc, rfs, 2000);

    puts("=== confirm pin1 after card ===");
    if (query_sim(ipc, rfs, 4)) puts("SIM timeout");
    if (g_remain >= 0 && g_remain <= 1) { puts("STOP remain<=1"); return 2; }
    if (g_pin != 1) {
        printf("pin1=%d not 1; no VerifyPin\n", g_pin);
        close(ipc); close(rfs); close(lock);
        return 3;
    }
    puts("CONFIRMED pin1=1 NOT_VERIFIED");

    int err_a = verify_one(ipc, rfs, 1, 7);
    wait_drain(ipc, rfs, 1000);
    puts("=== after A ===");
    if (query_sim(ipc, rfs, 8)) puts("SIM timeout");
    printf("RESULT_A app=%d pin1=%d remain=%d verify_err=%d\n",
           g_app, g_pin, g_remain, err_a);

    if (g_app == 5 || g_app == 6 || g_app == 7) {
        puts("app left PIN; chase");
        chase(ipc, rfs);
        close(ipc); close(rfs); close(lock);
        return 0;
    }
    if (g_remain >= 0 && g_remain <= 1) {
        puts("STOP remain<=1; no B");
        close(ipc); close(rfs); close(lock);
        return 2;
    }
    /* B only if still NOT_VERIFIED, remain>=2, and A was not wrong-PIN (3). */
    if (g_pin == 1 && g_remain >= 2 && err_a != 3 && err_a != 0) {
        puts("=== candidate B ===");
        int err_b = verify_one(ipc, rfs, 2, 8);
        wait_drain(ipc, rfs, 1000);
        puts("=== after B ===");
        if (query_sim(ipc, rfs, 9)) puts("SIM timeout");
        printf("RESULT_B app=%d pin1=%d remain=%d verify_err=%d\n",
               g_app, g_pin, g_remain, err_b);
        if (g_app == 5 || g_app == 6 || g_app == 7) {
            puts("app left PIN; chase");
            chase(ipc, rfs);
        }
    } else {
        puts("B skipped");
    }

    memset(g_aid, 0, sizeof g_aid);
    close(ipc); close(rfs); close(lock);
    return 0;
}
