/* Bounded post-READY network check for an isolated Pixel 7 diagnostic stack.
 * Opens only umts_ipc0. The separate RFS broker must remain the sole RFS reader.
 * No PIN, card power, APN, NV, rild/cbd, or radio-power request is sent here.
 */
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
#include "sit-network-layout.h"

enum { RX_CAP = 65536, REPLY_CAP = 256, TOTAL_MS = 120000, EXCHANGE_MS = 10000 };

static unsigned le16(const uint8_t *p) { return p[0] | ((unsigned)p[1] << 8); }
static uint32_t le32(const uint8_t *p) { return le16(p) | ((uint32_t)le16(p + 2) << 16); }
static void put32(uint8_t *p, uint32_t v) {
    p[0] = (uint8_t)v; p[1] = (uint8_t)(v >> 8);
    p[2] = (uint8_t)(v >> 16); p[3] = (uint8_t)(v >> 24);
}
static int64_t now_ms(void) {
    struct timespec t;
    if (clock_gettime(CLOCK_MONOTONIC, &t)) return -1;
    return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}

/* 0=incomplete, -1=invalid, positive=one complete SIT frame. */
static int frame_size(const uint8_t *p, size_t n) {
    if (n < 6) return 0;
    if (p[0] > 2) return -1;
    unsigned len = le16(p + 4);
    if (len < (p[0] == 2 ? 8U : 12U) || len > RX_CAP) return -1;
    return n < len ? 0 : (int)len;
}

static int response_ok(const uint8_t *p, size_t n, unsigned id) {
    return n >= 12 && p[0] == 1 && le16(p + 2) == id && p[10] == 0;
}
static int sim_ready(const uint8_t *p, size_t n) {
    if (!response_ok(p, n, 0x0200) || n < SIT_SIM_PIN1 + 1 ||
        p[SIT_SIM_CARD] != 1 || p[SIT_SIM_APPS] < 1 || p[SIT_SIM_APPS] > 4 ||
        n < 15U + SIT_SIM_APP_STRIDE * p[SIT_SIM_APPS]) return 0;
    return p[SIT_SIM_APP_STATE] == 5;
}
static int radio_on(const uint8_t *p, size_t n) {
    return response_ok(p, n, 0x0801) && n >= 16 && le32(p + 12) == 10;
}
/* CP2A sit-stream ProtocolNetSelModeAdapter reads the mode at byte 12. */
static int selection_mode(const uint8_t *p, size_t n) {
    if (!response_ok(p, n, 0x0703) || n < 13 || p[12] > 1) return -1;
    return p[12]; /* 0 auto: skip 0x0704; 1 manual: send it once. */
}

static void make_request(uint8_t *out, unsigned id, unsigned len, uint32_t token) {
    memset(out, 0, 16);
    out[2] = (uint8_t)id; out[3] = (uint8_t)(id >> 8);
    out[4] = (uint8_t)len; out[5] = (uint8_t)(len >> 8);
    put32(out + 6, token);
    if (id == 0x0710) out[12] = 1; /* factory BuildAllowData(1) */
}

static int online(void) {
    char state[32] = {0};
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    if (!f) return 0;
    int ok = fscanf(f, "%31s", state) == 1 && strcmp(state, "ONLINE") == 0;
    fclose(f);
    return ok;
}

static int open_verified_ipc0(void) {
    unsigned maj, min;
    FILE *f = fopen("/sys/class/cpif/umts_ipc0/dev", "r");
    if (!f) return -1;
    int ok = fscanf(f, "%u:%u", &maj, &min) == 2;
    fclose(f);
    if (!ok) return -1;
    int fd = open("/dev/umts_ipc0", O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    struct stat st;
    if (fd < 0) return -1;
    if (fstat(fd, &st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev) != maj || minor(st.st_rdev) != min) {
        close(fd);
        return -1;
    }
    return fd;
}

static int exchange(int fd, unsigned id, unsigned len, uint32_t token,
                    int64_t global_end, uint8_t reply[REPLY_CAP], size_t *reply_len) {
    uint8_t request[16];
    int64_t start = now_ms();
    if (start < 0 || start >= global_end) return -1;
    make_request(request, id, len, token);
    if (!online() || write(fd, request, len) != (ssize_t)len) return -1;
    int64_t end = start + EXCHANGE_MS;
    if (end > global_end) end = global_end;
    static uint8_t rx[RX_CAP];
    size_t used = 0;
    unsigned frames = 0;
    while (frames < 128) {
        int64_t now = now_ms();
        if (now < 0) return -1;
        int64_t remain = end - now;
        if (remain <= 0) break;
        struct pollfd pfd = { fd, POLLIN, 0 };
        int rc = poll(&pfd, 1, remain > 250 ? 250 : (int)remain);
        if (rc < 0 && errno == EINTR) continue;
        if (rc < 0 || (pfd.revents & (POLLERR | POLLHUP | POLLNVAL))) return -1;
        if (!(pfd.revents & POLLIN)) continue;
        if (used == sizeof rx) return -1;
        ssize_t n = read(fd, rx + used, sizeof rx - used);
        if (n < 0 && (errno == EINTR || errno == EAGAIN)) continue;
        if (n <= 0) return -1;
        used += (size_t)n;
        size_t off = 0;
        while (off < used) {
            int frame_len = frame_size(rx + off, used - off);
            if (frame_len < 0) return -1;
            if (!frame_len) break;
            const uint8_t *p = rx + off;
            frames++;
            if (p[0] == 1 && le16(p + 2) == id && le32(p + 6) == token) {
                size_t keep = (size_t)frame_len > REPLY_CAP ? REPLY_CAP : (size_t)frame_len;
                memcpy(reply, p, keep);
                *reply_len = keep;
                return 0;
            }
            off += (size_t)frame_len; /* Never log unsolicited private payloads. */
        }
        if (off) { memmove(rx, rx + off, used - off); used -= off; }
    }
    return 1; /* timeout, never retransmit */
}

static void print_rmnet(void) {
    for (int i = 0; i < 6; ++i) {
        char path[96];
        unsigned long long rx = 0, tx = 0;
        snprintf(path, sizeof path, "/sys/class/net/rmnet%d/statistics/rx_bytes", i);
        FILE *f = fopen(path, "r");
        if (!f) continue;
        int rx_ok = fscanf(f, "%llu", &rx) == 1;
        fclose(f);
        snprintf(path, sizeof path, "/sys/class/net/rmnet%d/statistics/tx_bytes", i);
        f = fopen(path, "r");
        if (!f) continue;
        int tx_ok = fscanf(f, "%llu", &tx) == 1;
        fclose(f);
        if (rx_ok && tx_ok) printf("rmnet%d rx=%llu tx=%llu\n", i, rx, tx);
    }
}

/* Read-only SIT snapshot: only scalar status and response presence are exposed.
 * In particular the 0x0702 operator payload (PLMN/names) is never copied to
 * stdout or interpreted as registration. */
static int snapshot_one(int fd, unsigned id, const char *label, uint32_t *token,
                        int64_t end) {
    uint8_t reply[REPLY_CAP];
    size_t n = 0;
    int rc = exchange(fd, id, 12, ++*token, end, reply, &n);
    if (rc) {
        printf("%s response=no status=%s\n", label, rc == 1 ? "timeout" : "io_error");
        return 1;
    }
    if (n < 12) { printf("%s response=yes payload=short\n", label); return 1; }
    printf("%s response=yes error_raw=%u", label, reply[10]);
    if (reply[10]) { putchar('\n'); return 1; }
    switch (id) {
    case 0x0200:
        if (n < 15) break;
        printf(" card_raw=%u apps=%u", reply[SIT_SIM_CARD], reply[SIT_SIM_APPS]);
        if (!reply[SIT_SIM_APPS]) { putchar('\n'); return 0; }
        if (n < SIT_SIM_PIN1 + 1 ||
            n < 15U + SIT_SIM_APP_STRIDE * reply[SIT_SIM_APPS]) break;
        printf(" app_raw=%u pin1_raw=%u", reply[SIT_SIM_APP_STATE], reply[SIT_SIM_PIN1]);
        putchar('\n'); return 0;
    case 0x0801:
        if (n < 16) break;
        printf(" radio_raw=%u", le32(reply + 12));
        putchar('\n'); return 0;
    case SIT_NET_VOICE_REG:
    case SIT_NET_DATA_REG:
        if (n < (id == SIT_NET_DATA_REG ? 16U : 14U)) break;
        printf(" registration_raw=%u reject_raw=%u",
               reply[SIT_NET_REG_STATE_OFFSET], reply[SIT_NET_REJECT_OFFSET]);
        if (id == SIT_NET_DATA_REG && n >= 16)
            printf(" tech_raw=%u", reply[SIT_NET_DATA_TECH_OFFSET]);
        putchar('\n'); return 0;
    case SIT_NET_SELECTION_MODE:
        if (n < 13) break;
        printf(" mode_raw=%u", reply[12]);
        putchar('\n'); return 0;
    case SIT_NET_PREFERRED_GET:
        if (n < 16) break;
        printf(" preferred_raw=%u", le32(reply + 12));
        putchar('\n'); return 0;
    case SIT_NET_OPERATOR:
    case 0x0900: /* Operator identifiers and signal payload are suppressed. */
        putchar('\n'); return 0;
    default:
        break;
    }
    puts(" payload=short");
    return 1;
}

static int snapshot(int fd, uint32_t token, int64_t end) {
    static const struct {
        unsigned id;
        const char *label;
    } reads[] = {
        {0x0200, "sim"}, {0x0801, "radio"},
        {SIT_NET_VOICE_REG, "voice"}, {SIT_NET_DATA_REG, "data"},
        {SIT_NET_OPERATOR, "operator"}, {SIT_NET_SELECTION_MODE, "selection"},
        {SIT_NET_PREFERRED_GET, "preferred"}, {0x0900, "signal"}
    };
    int failures = 0;
    for (size_t i = 0; i < sizeof reads / sizeof reads[0]; ++i)
        failures += snapshot_one(fd, reads[i].id, reads[i].label, &token, end);
    print_rmnet();
    return failures ? 1 : 0;
}

static int self_test(void) {
    uint8_t p[256] = {0}, req[16];
    p[0] = 1; p[2] = 0; p[3] = 2; p[4] = 78;
    p[SIT_SIM_CARD] = 1; p[SIT_SIM_APPS] = 1;
    p[SIT_SIM_APP_STATE] = 5; p[SIT_SIM_PIN1] = 2;
    if (frame_size(p, 77) != 0 || frame_size(p, 78) != 78 || !sim_ready(p, 78)) return 1;
    p[SIT_SIM_APP_STATE] = 2; if (sim_ready(p, 78)) return 1;
    p[SIT_SIM_APP_STATE] = 5; if (sim_ready(p, 77)) return 1;
    p[10] = 2; if (sim_ready(p, 78)) return 1;
    memset(p, 0, sizeof p); p[0] = 1; p[2] = 1; p[3] = 8; p[4] = 16;
    p[12] = 10; if (!radio_on(p, 16)) return 1;
    p[12] = 1; if (radio_on(p, 16)) return 1;
    memset(p, 0, sizeof p); p[0] = 1; p[2] = 3; p[3] = 7; p[4] = 13;
    if (selection_mode(p, 13) != 0) return 1;
    p[12] = 1; if (selection_mode(p, 13) != 1) return 1;
    p[12] = 2; if (selection_mode(p, 13) != -1) return 1;
    p[12] = 0; if (selection_mode(p, 12) != -1) return 1;
    if (!sit_net_is_registration(SIT_NET_VOICE_REG) ||
        !sit_net_is_registration(SIT_NET_DATA_REG) ||
        sit_net_is_registration(SIT_NET_OPERATOR)) return 1;
    make_request(req, 0x0704, 12, 0x78563412);
    if (req[0] != 0 || req[2] != 4 || req[3] != 7 || req[4] != 12 ||
        le32(req + 6) != 0x78563412 || req[12] != 0) return 1;
    make_request(req, 0x0710, 13, 3);
    if (req[2] != 0x10 || req[3] != 7 || req[4] != 13 || req[12] != 1) return 1;
    p[0] = 3; if (frame_size(p, 13) != -1) return 1;
    puts("PASS ready-network-once fixtures: gate, selection, network IDs, framing, signed requests");
    return 0;
}

int main(int argc, char **argv) {
    if (argc == 2 && strcmp(argv[1], "self-test") == 0) return self_test();
    int snapshot_mode = argc == 2 && strcmp(argv[1], "snapshot") == 0;
    if (argc != 2 || (!snapshot_mode && strcmp(argv[1], "run") != 0)) {
        fputs("usage: ready-network-once self-test|snapshot|run\n", stderr);
        return 64;
    }
    int lock = open("/run/saaios-sit-status.lock",
                    O_CREAT | O_RDWR | O_CLOEXEC | O_NOFOLLOW, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        fputs("ABORT common SIT lock busy\n", stderr);
        return 1;
    }
    if (!online()) { fputs("ABORT modem not ONLINE\n", stderr); close(lock); return 1; }
    int fd = open_verified_ipc0();
    if (fd < 0) { fputs("ABORT unverified umts_ipc0\n", stderr); close(lock); return 1; }
    int64_t start = now_ms();
    if (start < 0) { fputs("ABORT monotonic clock unavailable\n", stderr); close(fd); close(lock); return 1; }
    int64_t end = start + TOTAL_MS;
    uint32_t token = ((uint32_t)start << 8) ^ (uint32_t)getpid();
    if (!token) token = 1;
    uint8_t reply[REPLY_CAP]; size_t n = 0;
    int rc = 1;
    if (snapshot_mode) {
        rc = snapshot(fd, token, end);
        goto done;
    }
#define QUERY(ID, LEN) exchange(fd, ID, LEN, ++token, end, reply, &n)
    if (QUERY(0x0200, 12) || !sim_ready(reply, n)) {
        puts("ABORT fresh SIM response is not READY(5)"); goto done;
    }
    puts("preflight SIM READY(5)");
    if (QUERY(0x0801, 12) || !radio_on(reply, n)) {
        puts("ABORT fresh radio response is not ON(10)"); goto done;
    }
    puts("preflight radio ON(10)");
    if (QUERY(0x0703, 12)) { puts("ABORT selection-mode GET failed"); goto done; }
    int mode = selection_mode(reply, n);
    if (mode < 0) { puts("ABORT selection-mode response invalid"); goto done; }
    printf("selection_mode_raw=%d\n", mode);
    /* Re-check both guards immediately before either state-changing request. */
    if (QUERY(0x0200, 12) || !sim_ready(reply, n) ||
        QUERY(0x0801, 12) || !radio_on(reply, n)) {
        puts("ABORT READY/ON changed before SET"); goto done;
    }
    if (mode == 1) {
        if (QUERY(0x0704, 12) || !response_ok(reply, n, 0x0704)) {
            puts("ABORT auto-selection SET failed"); goto done;
        }
        puts("selection auto SET accepted");
    } else {
        puts("selection already auto; 0x0704 skipped");
    }
    if (QUERY(0x0200, 12) || !sim_ready(reply, n) ||
        QUERY(0x0801, 12) || !radio_on(reply, n)) {
        puts("ABORT READY/ON changed before AllowData"); goto done;
    }
    if (QUERY(0x0710, 13) || !response_ok(reply, n, 0x0710)) {
        puts("ABORT AllowData SET failed"); goto done;
    }
    puts("AllowData(1) SET accepted");
    int polled = 0;
    for (int i = 0; i < 4 && now_ms() < end; ++i) {
        if (QUERY(0x0701, 12) || !response_ok(reply, n, 0x0701) || n < 16) {
            puts("ABORT data-registration GET failed"); goto done;
        }
        printf("poll=%d registration_raw=%u reject_raw=%u tech_raw=%u\n",
               i, reply[12], reply[13], reply[15]);
        polled++;
        print_rmnet();
        if (i < 3) {
            struct timespec pause = {3, 0};
            while (nanosleep(&pause, &pause) && errno == EINTR) { }
        }
    }
    if (!polled) { puts("ABORT no registration response before deadline"); goto done; }
    puts("RESULT diagnostic complete; network service not inferred from SIT ACKs");
    rc = 0;
done:
    close(fd);
    close(lock);
    return rc;
#undef QUERY
}
