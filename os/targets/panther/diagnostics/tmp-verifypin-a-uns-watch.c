/* ONE live: pin1==1 → VerifyPin A+AID (no CardPower). Watch unsolicited SIT
   on umts_ipc0 during/after. Logs type/id/len/error only — never PIN/IMSI/ICCID/AID.
   No cbd/rild, POWER_OFF, crash, EFS RW, invent 0x2f50. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/file.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <time.h>
#include <unistd.h>

#include "sit-sim-layout.h"

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
        puts("rfs large drained");
        return;
    }
    printf("rfs bytes=%zd cmd=%u\n", n, buf[0]);
    if (n == 12 && buf[0] == 0x07 && !buf[1] && !buf[2] && !buf[3] &&
        buf[4] == 0x04 && !buf[5] && !buf[6] && !buf[7] &&
        buf[8] == 0x03 && !buf[9] && !buf[10] && !buf[11]) {
        static const uint8_t reply[16] = {
            0x03, 0, 0, 0, 0x08, 0, 0, 0, 0, 0, 0, 0, 0x03, 0, 0, 0};
        if (write(fd, reply, sizeof reply) < 0)
            puts("rfs unprotect write fail");
        else
            puts("rfs unprotect reply");
        return;
    }
    puts("rfs ignored");
}

static int g_app = -1, g_pin = -1, g_remain = -1, g_aid_len = 0;
static uint8_t g_aid[16];
static unsigned g_uns_n = 0;

/* Classify: REQ match vs unsolicited. Never print payloads. */
static void note_frame(const char *phase, const uint8_t *b, int len, unsigned want_id,
                       int32_t want_tok, int *matched) {
    unsigned typ = b[0];
    unsigned fid = le16(b + 2);
    uint32_t ftok = len >= 10 ? le32(b + 6) : 0;
    unsigned err = len > 10 ? b[10] : 0xff;
    int is_match = (fid == want_id) && (want_tok < 0 || ftok == (uint32_t)want_tok);
    if (is_match) {
        printf("%s MATCH type=%u id=0x%04x tok=%u len=%d err=%u\n",
               phase, typ, fid, ftok, len, err);
        if (matched) *matched = 1;
    } else {
        g_uns_n++;
        printf("%s UNSOL type=%u id=0x%04x tok=%u len=%d err=%u\n",
               phase, typ, fid, ftok, len, err);
        /* Decode known status/reg inds without secrets. */
        if (fid == 0x0200 && len >= SIT_SIM_PIN1_REMAIN + 1 && !b[10]) {
            printf("%s UNSOL_SIM app=%u pin1=%u remain=%u card=%u apps=%u\n",
                   phase, b[SIT_SIM_APP_STATE], b[SIT_SIM_PIN1], b[SIT_SIM_PIN1_REMAIN],
                   b[SIT_SIM_CARD], b[SIT_SIM_APPS]);
        } else if ((fid == 0x0701 || fid == 0x0702) && len >= 16) {
            printf("%s UNSOL_REG id=0x%04x reg=%u tech=%u\n",
                   phase, fid, b[12], b[15]);
        }
    }
    if (fid == 0x0200 && len >= SIT_SIM_APP_TYPE + SIT_SIM_APP_STRIDE && !b[10] &&
        b[SIT_SIM_APPS]) {
        g_app = b[SIT_SIM_APP_STATE];
        g_pin = b[SIT_SIM_PIN1];
        g_remain = b[SIT_SIM_PIN1_REMAIN];
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
        printf("%s SIM app=%d pin1=%d remain=%d aid_len=%d\n",
               phase, g_app, g_pin, g_remain, g_aid_len);
    }
}

static int exchange_watch(int ipc, int rfs, const uint8_t *req, size_t req_len,
                          unsigned id, int32_t want_tok, int timeout_ms,
                          const char *phase, uint8_t *resp, int *resp_len) {
    ssize_t wr = write(ipc, req, req_len);
    if (wr != (ssize_t)req_len) {
        printf("%s write fail wr=%zd errno=%d\n", phase, wr, errno);
        return -1;
    }
    uint8_t buf[8192];
    size_t used = 0;
    int matched = 0;
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
            note_frame(phase, buf + off, len, id, want_tok, &matched);
            if (matched && resp && resp_len) {
                int keep = len > 512 ? 512 : len;
                memcpy(resp, buf + off, (size_t)keep);
                *resp_len = keep;
                /* keep reading a little for more unsolicited after match */
            }
            off += (size_t)len;
        }
        if (off) {
            memmove(buf, buf + off, used - off);
            used -= off;
        }
        if (matched && now_ms() + 800 >= end) break; /* allow brief post-match window */
        if (matched && timeout_ms > 2000) {
            /* shrink remaining wait to ~1.5s after match to catch inds */
            int64_t soft = now_ms() + 1500;
            if (soft < end) end = soft;
        }
    }
    if (!matched) {
        printf("%s timeout id=0x%x\n", phase, id);
        return 1;
    }
    return 0;
}

static void watch_idle(int ipc, int rfs, int ms, const char *phase) {
    int64_t end = now_ms() + ms;
    uint8_t buf[4096];
    size_t used = 0;
    while (now_ms() < end) {
        struct pollfd pfd[2] = {{ipc, POLLIN, 0}, {rfs, POLLIN, 0}};
        poll(pfd, 2, 200);
        if (pfd[1].revents & POLLIN) service_rfs(rfs);
        if (pfd[0].revents & POLLIN && used < sizeof buf) {
            ssize_t n = read(ipc, buf + used, sizeof buf - used);
            if (n > 0) used += (size_t)n;
        }
        size_t off = 0;
        while (off < used) {
            int len = frame_size(buf + off, used - off);
            if (len < 0) {
                used = 0;
                break;
            }
            if (!len) break;
            note_frame(phase, buf + off, len, 0xffff, -1, NULL);
            off += (size_t)len;
        }
        if (off) {
            memmove(buf, buf + off, used - off);
            used -= off;
        }
    }
}

static int query_sim(int ipc, int rfs, uint32_t tok, const char *phase) {
    uint8_t req[12] = {0}, resp[512];
    int n = 0;
    req[2] = 0x00;
    req[3] = 0x02;
    req[4] = 12;
    req[6] = (uint8_t)tok;
    return exchange_watch(ipc, rfs, req, 12, 0x0200, (int32_t)tok, 15000, phase, resp, &n);
}

static int query_reg(int ipc, int rfs, unsigned id, uint32_t tok, const char *phase) {
    uint8_t req[12] = {0}, resp[512];
    int n = 0;
    req[2] = (uint8_t)(id & 0xff);
    req[3] = (uint8_t)((id >> 8) & 0xff);
    req[4] = 12;
    req[6] = (uint8_t)tok;
    int rc = exchange_watch(ipc, rfs, req, 12, id, (int32_t)tok, 8000, phase, resp, &n);
    if (!rc && n >= 16)
        printf("%s reg_byte12=%u tech=%u\n", phase, resp[12], resp[15]);
    return rc;
}

static int verify_a(int ipc, int rfs, uint32_t tok) {
    uint8_t req[38], resp[512];
    int n = 0;
    memset(req, 0, sizeof req);
    req[2] = 0x01;
    req[3] = 0x02;
    req[4] = 38;
    req[6] = (uint8_t)tok;
    req[12] = 4;
    {
        uint8_t d = '0';
        for (int i = 0; i < 4; i++) req[13 + i] = d;
    }
    if (g_aid_len > 0 && g_aid_len <= 16) {
        req[21] = (uint8_t)g_aid_len;
        memcpy(req + 22, g_aid, (size_t)g_aid_len);
    }
    printf("VerifyPin A aid_len=%d (PIN/AID not printed) timeout_ms=90000\n", g_aid_len);
    int rc = exchange_watch(ipc, rfs, req, 38, 0x0201, -1, 90000, "verify", resp, &n);
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
    return (int)err;
}

static void print_rmnet(void) {
    for (int i = 0; i < 6; i++) {
        char path[64];
        snprintf(path, sizeof path, "/sys/class/net/rmnet%d/statistics/rx_bytes", i);
        FILE *f = fopen(path, "r");
        if (!f) continue;
        unsigned long long rx = 0, tx = 0;
        if (fscanf(f, "%llu", &rx) != 1) rx = 0;
        fclose(f);
        snprintf(path, sizeof path, "/sys/class/net/rmnet%d/statistics/tx_bytes", i);
        f = fopen(path, "r");
        if (f) {
            if (fscanf(f, "%llu", &tx) != 1) tx = 0;
            fclose(f);
        }
        printf("rmnet%d rx=%llu tx=%llu\n", i, rx, tx);
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

    puts("=== pre (no CardPower; pin1 must already be 1) ===");
    if (query_sim(ipc, rfs, 1, "pre")) puts("SIM timeout");
    if (g_remain >= 0 && g_remain <= 1) {
        puts("STOP remain<=1");
        return 2;
    }
    if (g_pin != 1) {
        printf("STOP pin1=%d not 1; no VerifyPin\n", g_pin);
        close(ipc);
        close(rfs);
        close(lock);
        return 3;
    }
    if (g_aid_len <= 0) {
        puts("STOP no AID from GET_STATUS; no VerifyPin");
        close(ipc);
        close(rfs);
        close(lock);
        return 4;
    }
    puts("CONFIRMED pin1=1 NOT_VERIFIED; proceeding VerifyPin A+AID");

    (void)query_reg(ipc, rfs, 0x0702, 2, "pre_voice");
    (void)query_reg(ipc, rfs, 0x0701, 3, "pre_data");

    int err = verify_a(ipc, rfs, 7);
    puts("=== post-verify idle watch 8s ===");
    watch_idle(ipc, rfs, 8000, "idle");

    puts("=== after ===");
    if (query_sim(ipc, rfs, 8, "after")) puts("SIM timeout");
    (void)query_reg(ipc, rfs, 0x0702, 9, "after_voice");
    (void)query_reg(ipc, rfs, 0x0701, 10, "after_data");
    print_rmnet();

    printf("RESULT app=%d pin1=%d remain=%d verify_err=%d unsol_frames=%u\n",
           g_app, g_pin, g_remain, err, g_uns_n);
    if (g_app == 1 || g_app == 4 || g_app == 5)
        puts("EDGE READY-ish app in {1,4,5} — caller may chase bearer");
    else
        puts("still not START_NETWORK set (app not in {1,4,5})");

    memset(g_aid, 0, sizeof g_aid);
    close(ipc);
    close(rfs);
    close(lock);
    return 0;
}
