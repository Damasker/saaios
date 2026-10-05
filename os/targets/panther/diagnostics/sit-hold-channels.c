/* Hold the factory IPC and RFS devices open and poll SIM status.
 * Replies only to the 12-byte protected-NV unprotect. OP_STATUS and an
 * NV write (op 2) get no reply and no file access. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/file.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <time.h>
#include <unistd.h>

static FILE *logf;

static unsigned le16(const uint8_t *p) { return p[0] | ((unsigned)p[1] << 8); }
static uint32_t le32(const uint8_t *p) {
    return (uint32_t)le16(p) | ((uint32_t)le16(p + 2) << 16);
}
static int64_t now_ms(void) {
    struct timespec t;
    if (clock_gettime(CLOCK_MONOTONIC, &t)) return 0;
    return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}
static void note(const char *s) {
    puts(s);
    fflush(stdout);
    if (logf) { fputs(s, logf); fputc('\n', logf); fflush(logf); }
}

static const char *app_state_name(unsigned state) {
    switch (state) {
    case 0: return "UNKNOWN";
    case 1: return "DETECTED";
    case 2: return "PIN";
    case 3: return "PUK";
    case 4: return "SUBSCRIPTION_PERSO";
    case 5: return "READY";
    default: return "unknown";
    }
}

static int frame_size(const uint8_t *p, size_t n) {
    if (n < 6) return 0;
    if (p[0] > 2) return -1;
    unsigned min = p[0] == 2 ? 8 : 12;
    unsigned len = le16(p + 4);
    if (len < min || len > 65536) return -1;
    return n < len ? 0 : (int)len;
}

static int open_cpif(const char *node, const char *sysdev) {
    unsigned maj = 0, min = 0;
    FILE *f = fopen(sysdev, "r");
    if (!f) return -1;
    int got = fscanf(f, "%u:%u", &maj, &min);
    fclose(f);
    if (got != 2) return -1;
    int fd = open(node, O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    struct stat st;
    if (fd < 0 || fstat(fd, &st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev) != maj || minor(st.st_rdev) != min) {
        if (fd >= 0) close(fd);
        return -1;
    }
    return fd;
}

/* Exact factory unprotect: command 7, length word 4, file id 3. */
static int is_unprotect(const uint8_t *b, ssize_t n) {
    return n == 12 && b[0] == 0x07 && b[1] == 0 && b[2] == 0 && b[3] == 0 &&
           b[4] == 0x04 && b[5] == 0 && b[6] == 0 && b[7] == 0 &&
           b[8] == 0x03 && b[9] == 0 && b[10] == 0 && b[11] == 0;
}

static void service_rfs(int fd) {
    uint8_t buf[256];
    ssize_t n = read(fd, buf, sizeof buf);
    if (n < 0) {
        if (errno == EAGAIN || errno == EINTR) return;
        note("rfs read failed");
        return;
    }
    if (n == 0) return;
    if (n == (ssize_t)sizeof buf) {
        uint8_t dump[256];
        while (read(fd, dump, sizeof dump) > 0) {}
        note("rfs large request drained no reply");
        return;
    }
    char line[80];
    snprintf(line, sizeof line, "rfs bytes=%zd cmd=%u", n, buf[0]);
    note(line);
    if (is_unprotect(buf, n)) {
        static const uint8_t reply[16] = {
            0x03, 0x00, 0x00, 0x00,
            0x08, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x03, 0x00, 0x00, 0x00
        };
        ssize_t wr = write(fd, reply, sizeof reply);
        snprintf(line, sizeof line, "rfs unprotect reply bytes=%zd", wr);
        note(line);
        return;
    }
    if (n == 20 && buf[0] == 0x03 && le32(buf + 4) == 12) {
        note("rfs op_status no reply");
        return;
    }
    if (n >= 24 && buf[0] == 0x06) {
        unsigned op = le32(buf + 20);
        if (op == 2) note("rfs write op=2 refused no reply");
        else note("rfs io no reply");
        return;
    }
    note("rfs ignored no reply");
}

static uint8_t ibuf[65536];
static size_t iused;
static int last_app = -1;
static int last_pin1 = -1;
static int last_pref = -1;
static int saw_ready;

static void on_sim(const uint8_t *b, int len) {
    char line[160];
    if (len < 15 || b[10]) {
        snprintf(line, sizeof line, "SIM length=%d error_raw=%u", len, len > 10 ? b[10] : 0);
        note(line);
        return;
    }
    unsigned apps = b[14];
    int type = -1, state = -1, pin1 = -1;
    const char *name = "none";
    if (apps >= 1 && len >= 18) {
        type = b[15];
        state = b[17];
        name = app_state_name((unsigned)state);
        last_app = state;
        if (state == 5) saw_ready = 1;
    }
    if (apps >= 1 && len >= 75) {
        pin1 = b[72];
        last_pin1 = pin1;
    }
    if (pin1 >= 0)
        snprintf(line, sizeof line,
            "SIM length=%d error_raw=0 card_state_raw=%u applications=%u app0_type_raw=%d app0_state_raw=%d app0_state=%s pin1_state_raw=%d",
            len, b[12], apps, type, state, name, pin1);
    else
        snprintf(line, sizeof line,
            "SIM length=%d error_raw=0 card_state_raw=%u applications=%u app0_type_raw=%d app0_state_raw=%d app0_state=%s",
            len, b[12], apps, type, state, name);
    note(line);
}

static int consume_frames(int want, unsigned id, uint32_t token) {
    int found = 0;
    while (iused) {
        int len = frame_size(ibuf, iused);
        if (len < 0) { iused = 0; note("ipc framing drop"); return -1; }
        if (!len) break;
        unsigned fid = le16(ibuf + 2);
        uint32_t ftok = le32(ibuf + 6);
        unsigned err = len > 10 ? ibuf[10] : 0;
        if (fid == 0x0200) on_sim(ibuf, len);
        if (want && fid == id && ftok == token) {
            char line[96];
            if (fid != 0x0200) {
                snprintf(line, sizeof line, "response id=%u length=%d error_raw=%u", fid, len, err);
                note(line);
            }
            if (fid == 0x0701 && !err && len >= 16) {
                snprintf(line, sizeof line,
                    "registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u",
                    ibuf[12], ibuf[13], ibuf[15]);
                note(line);
            }
            if (fid == 0x070b && !err && len >= 16) {
                last_pref = (int)le32(ibuf + 12);
                snprintf(line, sizeof line, "preferred_raw=%u", (unsigned)last_pref);
                note(line);
            }
            found = 1;
        } else if (fid != 0x0200) {
            char line[64];
            snprintf(line, sizeof line, "ipc id=%u length=%d", fid, len);
            note(line);
        }
        iused -= (size_t)len;
        memmove(ibuf, ibuf + len, iused);
    }
    return found;
}

static int send_req(int fd, unsigned id, unsigned len, uint32_t token, uint32_t word) {
    uint8_t req[18];
    if (len != 12 && len != 16 && len != 18) return -1;
    memset(req, 0, sizeof req);
    req[2] = (uint8_t)id;
    req[3] = (uint8_t)(id >> 8);
    req[4] = (uint8_t)len;
    req[6] = (uint8_t)token;
    req[7] = (uint8_t)(token >> 8);
    if (len >= 16) {
        req[12] = (uint8_t)word;
        req[13] = (uint8_t)(word >> 8);
        req[14] = (uint8_t)(word >> 16);
        req[15] = (uint8_t)(word >> 24);
    }
    ssize_t wr = write(fd, req, len);
    memset(req, 0, sizeof req);
    return wr == (ssize_t)len ? 0 : -1;
}

/* Wait until a response with this id and token is consumed by drain, or timeout.
 * SIM frames update last_app as they arrive. */
static int wait_token(int ipc, int rfs, int want, unsigned id, uint32_t token, int ms) {
    int64_t end = now_ms() + ms;
    while (now_ms() < end) {
        int slice = (int)(end - now_ms());
        if (slice > 200) slice = 200;
        if (slice < 1) slice = 1;
        struct pollfd pfd[2] = {
            { ipc, POLLIN, 0 },
            { rfs, POLLIN, 0 }
        };
        int ready = poll(pfd, 2, slice);
        if (ready < 0) { if (errno == EINTR) continue; return -1; }
        if (pfd[1].revents & POLLIN) service_rfs(rfs);
        if (pfd[0].revents & POLLIN) {
            if (iused >= sizeof ibuf) { iused = 0; note("ipc overflow drop"); }
            ssize_t n = read(ipc, ibuf + iused, sizeof ibuf - iused);
            if (n > 0) iused += (size_t)n;
        }
        int got = consume_frames(want, id, token);
        if (got < 0) return -1;
        if (got > 0) return 0;
        if ((pfd[0].revents | pfd[1].revents) & (POLLERR | POLLHUP | POLLNVAL)) return -1;
    }
    return 1;
}

static void print_rmnet(void) {
    FILE *f;
    char b[32];
    unsigned long long rx = 0, tx = 0;
    f = fopen("/sys/class/net/rmnet0/statistics/rx_bytes", "r");
    if (f) { if (fscanf(f, "%llu", &rx) != 1) rx = 0; fclose(f); }
    f = fopen("/sys/class/net/rmnet0/statistics/tx_bytes", "r");
    if (f) { if (fscanf(f, "%llu", &tx) != 1) tx = 0; fclose(f); }
    snprintf(b, sizeof b, "rmnet_rx=%llu rmnet_tx=%llu", rx, tx);
    note(b);
}

int main(void) {
    signal(SIGHUP, SIG_IGN);
    signal(SIGPIPE, SIG_IGN);
    alarm(360);
    mkdir("/data/saaios/var", 0700);
    logf = fopen("/data/saaios/var/hold-channels.log", "w");
    FILE *pidf = fopen("/data/saaios/var/hold-channels.pid", "w");
    if (pidf) { fprintf(pidf, "%d\n", (int)getpid()); fclose(pidf); }
    FILE *st = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!st || fscanf(st, "%31s", state) != 1) { note("modem state unreadable"); return 1; }
    fclose(st);
    if (strcmp(state, "ONLINE")) { note("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { note("status lock busy"); return 1; }
    int ipc = open_cpif("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_cpif("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { note("channel open failed"); return 1; }
    note("channels open");
    int64_t start = now_ms();
    int64_t end = start + 180000;
    int64_t next = start;
    uint32_t token = 1;
    while (now_ms() < end && !saw_ready) {
        if (now_ms() >= next) {
            char line[64];
            snprintf(line, sizeof line, "SIM query token=%u", token);
            note(line);
            if (send_req(ipc, 0x0200, 12, token, 0)) note("SIM write failed");
            else if (wait_token(ipc, rfs, 1, 0x0200, token, 10000) != 0) note("SIM query timeout");
            token++;
            next = now_ms() + 15000;
        } else {
            int left = (int)(next - now_ms());
            if (left > 1000) left = 1000;
            if (left < 1) left = 1;
            wait_token(ipc, rfs, 0, 0, 0, left);
        }
    }
    int radio = 0;
    if (!saw_ready) {
        /* DoRadioPower passes flag bytes 16 and 17 as 0 when the request
         * has fewer than three integers. OFF is word 1, ON is word 2. */
        note("radio power OFF");
        radio = 1;
        if (send_req(ipc, 0x0800, 18, 40, 1)) note("radio OFF write failed");
        else if (wait_token(ipc, rfs, 1, 0x0800, 40, 10000) != 0) note("radio OFF timeout");
        note("radio power ON");
        if (send_req(ipc, 0x0800, 18, 41, 2)) note("radio ON write failed");
        else if (wait_token(ipc, rfs, 1, 0x0800, 41, 10000) != 0) note("radio ON timeout");
        note("SIM query after radio");
        if (send_req(ipc, 0x0200, 12, 42, 0)) note("SIM write failed");
        else if (wait_token(ipc, rfs, 1, 0x0200, 42, 10000) != 0) note("SIM query timeout");
    }
    if (saw_ready) {
        note("preferred query");
        if (!send_req(ipc, 0x070b, 12, 50, 0))
            wait_token(ipc, rfs, 1, 0x070b, 50, 10000);
        if (last_pref >= 0 && last_pref != 11) {
            note("preferred set LTE_ONLY");
            if (!send_req(ipc, 0x070a, 16, 51, 11))
                wait_token(ipc, rfs, 1, 0x070a, 51, 10000);
        }
        for (int i = 0; i < 4; i++) {
            uint32_t t = 60 + (uint32_t)i;
            note("registration query");
            if (!send_req(ipc, 0x0701, 12, t, 0))
                wait_token(ipc, rfs, 1, 0x0701, t, 10000);
            if (i != 3) wait_token(ipc, rfs, 0, 0, 0, 15000);
        }
    }
    print_rmnet();
    {
        char line[80];
        snprintf(line, sizeof line, "RESULT app_state=%d pin1=%d radio_cycled=%d ready=%d",
            last_app, last_pin1, radio, saw_ready);
        note(line);
    }
    close(ipc);
    close(rfs);
    close(lock);
    return saw_ready ? 0 : 2;
}
