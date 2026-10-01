/* Post-READY bearer chase: Radio ON, LTE preferred, selection auto,
   AllowData, SetupDataCall (APN file), poll reg/rmnet. No secrets logged.
   No CardPower/VerifyPin/cbd/rild/POWER_OFF/EFS. */
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
        return;
    }
    if (n == 12 && buf[0] == 0x07 && !buf[1] && !buf[2] && !buf[3] &&
        buf[4] == 0x04 && !buf[5] && !buf[6] && !buf[7] &&
        buf[8] == 0x03 && !buf[9] && !buf[10] && !buf[11]) {
        static const uint8_t reply[16] = {
            0x03, 0, 0, 0, 0x08, 0, 0, 0, 0, 0, 0, 0, 0x03, 0, 0, 0};
        if (write(fd, reply, sizeof reply) < 0)
            puts("rfs unprotect write fail");
        else
            puts("rfs unprotect reply");
    }
}

static int g_app = -1, g_pin = -1;

static void note(const char *tag, const uint8_t *b, int len, unsigned want) {
    unsigned typ = b[0], fid = le16(b + 2);
    uint32_t tok = len >= 10 ? le32(b + 6) : 0;
    unsigned err = len > 10 ? b[10] : 0xff;
    if (fid == want)
        printf("%s MATCH type=%u id=0x%04x tok=%u len=%d err=%u\n", tag, typ, fid, tok, len, err);
    else
        printf("%s UNSOL type=%u id=0x%04x tok=%u len=%d err=%u\n", tag, typ, fid, tok, len, err);
    if (fid == 0x0200 && len >= SIT_SIM_PIN1_REMAIN + 1 && !b[10] && b[SIT_SIM_APPS]) {
        g_app = b[SIT_SIM_APP_STATE];
        g_pin = b[SIT_SIM_PIN1];
        printf("%s SIM app=%d pin1=%d\n", tag, g_app, g_pin);
    }
    if ((fid == 0x0701 || fid == 0x0702) && len >= 16)
        printf("%s REG id=0x%04x reg=%u tech=%u\n", tag, fid, b[12], b[15]);
}

static int exchange(int ipc, int rfs, const uint8_t *req, size_t req_len, unsigned id,
                    int32_t want_tok, int timeout_ms, const char *tag, uint8_t *resp,
                    int *resp_len) {
    if (write(ipc, req, req_len) != (ssize_t)req_len) {
        printf("%s write fail\n", tag);
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
            if (len < 0) return -1;
            if (!len) break;
            unsigned fid = le16(buf + off + 2);
            uint32_t tok = le32(buf + off + 6);
            note(tag, buf + off, len, id);
            if (fid == id && (want_tok < 0 || tok == (uint32_t)want_tok)) {
                int keep = len > 512 ? 512 : len;
                if (resp && resp_len) {
                    memcpy(resp, buf + off, (size_t)keep);
                    *resp_len = keep;
                }
                matched = 1;
                int64_t soft = now_ms() + 800;
                if (soft < end) end = soft;
            }
            off += (size_t)len;
        }
        if (off) {
            memmove(buf, buf + off, used - off);
            used -= off;
        }
        if (matched && now_ms() + 100 >= end) break;
    }
    return matched ? 0 : 1;
}

static void print_rmnet(void) {
    for (int i = 0; i < 6; i++) {
        char path[80];
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
        char cmd[64];
        snprintf(cmd, sizeof cmd, "ip link set rmnet%d up 2>/dev/null", i);
        if (system(cmd) < 0) { /* ignore */ }
        snprintf(path, sizeof path, "/sys/class/net/rmnet%d", i);
        /* IPv4 presence only — never print address */
        snprintf(cmd, sizeof cmd, "ip -4 addr show rmnet%d 2>/dev/null | grep -q inet", i);
        int has = system(cmd) == 0;
        printf("rmnet%d rx=%llu tx=%llu ipv4=%s\n", i, rx, tx, has ? "yes" : "none");
        if (has) puts("BEARER_OK");
    }
}

/* SetupDataCall 0x0600 factory layout length 246 — reuse setup-data-call binary if present. */
static int run_setup_dc(void) {
    if (access("/data/saaios/bin/setup-data-call", X_OK) == 0 &&
        access("/data/saaios/etc/apn", R_OK) == 0) {
        puts("invoking setup-data-call --apn-file /data/saaios/etc/apn");
        return system("/data/saaios/bin/setup-data-call --apn-file /data/saaios/etc/apn");
    }
    puts("setup-data-call or APN missing");
    return 1;
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

    uint8_t req[32] = {0}, resp[512];
    int n = 0;

    puts("=== GET_APP ===");
    memset(req, 0, sizeof req);
    req[2] = 0x00;
    req[3] = 0x02;
    req[4] = 12;
    req[6] = 1;
    (void)exchange(ipc, rfs, req, 12, 0x0200, 1, 12000, "sim", resp, &n);
    if (g_app != 1 && g_app != 4 && g_app != 5) {
        printf("STOP app=%d not in {1,4,5}\n", g_app);
        close(ipc);
        close(rfs);
        close(lock);
        return 2;
    }
    printf("CONFIRMED READY-ish app=%d pin1=%d\n", g_app, g_pin);

    puts("=== skip radio ON (already RADIO_ON=10 on this path) ===");

    puts("=== preferred LTE 0x070a ===");
    memset(req, 0, sizeof req);
    req[2] = 0x0a;
    req[3] = 0x07;
    req[4] = 16;
    req[6] = 50;
    req[12] = 11;
    (void)exchange(ipc, rfs, req, 16, 0x070a, 50, 10000, "pref", resp, &n);

    puts("=== selection auto 0x0704 ===");
    memset(req, 0, sizeof req);
    req[2] = 0x04;
    req[3] = 0x07;
    req[4] = 12;
    req[6] = 51;
    (void)exchange(ipc, rfs, req, 12, 0x0704, 51, 10000, "auto", resp, &n);

    puts("=== AllowData 0x0710 ===");
    memset(req, 0, sizeof req);
    req[2] = 0x10;
    req[3] = 0x07;
    req[4] = 13;
    req[6] = 52;
    req[12] = 1;
    (void)exchange(ipc, rfs, req, 13, 0x0710, 52, 8000, "allow", resp, &n);

    puts("=== GetPs 0x0711 ===");
    memset(req, 0, sizeof req);
    req[2] = 0x11;
    req[3] = 0x07;
    req[4] = 12;
    req[6] = 53;
    (void)exchange(ipc, rfs, req, 12, 0x0711, 53, 8000, "ps", resp, &n);

    /* Release lock briefly so setup-data-call can take it */
    close(ipc);
    close(rfs);
    flock(lock, LOCK_UN);
    close(lock);
    (void)run_setup_dc();

    lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        puts("lock busy after setup");
        return 1;
    }
    ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) {
        puts("reopen fail");
        return 1;
    }

    for (int i = 0; i < 10; i++) {
        printf("=== poll %d ===\n", i);
        memset(req, 0, sizeof req);
        req[2] = 0x01;
        req[3] = 0x07;
        req[4] = 12;
        req[6] = (uint8_t)(60 + i);
        n = 0;
        (void)exchange(ipc, rfs, req, 12, 0x0701, 60 + i, 8000, "data", resp, &n);
        memset(req, 0, sizeof req);
        req[2] = 0x02;
        req[3] = 0x07;
        req[4] = 12;
        req[6] = (uint8_t)(80 + i);
        (void)exchange(ipc, rfs, req, 12, 0x0702, 80 + i, 8000, "voice", resp, &n);
        print_rmnet();
        memset(req, 0, sizeof req);
        req[2] = 0x00;
        req[3] = 0x02;
        req[4] = 12;
        req[6] = (uint8_t)(100 + i);
        (void)exchange(ipc, rfs, req, 12, 0x0200, 100 + i, 8000, "sim", resp, &n);
        if (n >= 16 && resp[12] >= 1 && resp[12] <= 5) {
            /* wrong — resp is last frame; check via printed REG */
        }
        sleep(2);
    }

    puts("=== FINAL ===");
    print_rmnet();
    printf("RESULT app=%d pin1=%d\n", g_app, g_pin);
    close(ipc);
    close(rfs);
    close(lock);
    return 0;
}
