#define _GNU_SOURCE
/* ONE live race: poll 0x0200 during CardPower 4→1; on DETECTED pulse Radio ON. */
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
    if (len < min) return -1;
    return n < len ? 0 : (int)len;
}

static int open_node(const char *node, const char *sysdev) {
    unsigned maj = 0, min = 0;
    FILE *f = fopen(sysdev, "r");
    if (!f) return -1;
    if (fscanf(f, "%u:%u", &maj, &min) != 2) { fclose(f); return -1; }
    fclose(f);
    int fd = open(node, O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0 && errno == ENOENT) {
        if (mknod(node, S_IFCHR | 0600, makedev(maj, min)) && errno != EEXIST) return -1;
        fd = open(node, O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    }
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
    if (n == 12 && buf[0] == 0x07 && !buf[1] && !buf[2] && !buf[3] &&
        buf[4] == 0x04 && !buf[5] && !buf[6] && !buf[7] &&
        buf[8] == 0x03 && !buf[9] && !buf[10] && !buf[11]) {
        static const uint8_t reply[16] = {
            0x03, 0, 0, 0, 0x08, 0, 0, 0, 0, 0, 0, 0, 0x03, 0, 0, 0
        };
        (void)write(fd, reply, sizeof reply);
        return;
    }
    if (n == (ssize_t)sizeof buf) {
        uint8_t dump[256];
        while (read(fd, dump, sizeof dump) > 0) {}
    }
}

/* Drain unrelated IPC; match id+token. timeout_ms bounded. */
static int exchange_ms(int ipc, int rfs, const uint8_t *req, size_t req_len,
                       unsigned id, uint32_t token, uint8_t *resp, int *resp_len,
                       int timeout_ms) {
    if (write(ipc, req, req_len) != (ssize_t)req_len) return -1;
    uint8_t buf[4096];
    size_t used = 0;
    int64_t end = now_ms() + timeout_ms;
    while (now_ms() < end) {
        struct pollfd pfd[2] = {{ipc, POLLIN, 0}, {rfs, POLLIN, 0}};
        int to = (int)(end - now_ms());
        if (to < 1) to = 1;
        if (to > 50) to = 50;
        int ready = poll(pfd, 2, to);
        if (ready < 0) {
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
            uint32_t ftok = le32(buf + off + 6);
            if (fid == id && ftok == token) {
                int keep = len > 512 ? 512 : len;
                memcpy(resp, buf + off, (size_t)keep);
                *resp_len = keep;
                return 0;
            }
            off += (size_t)len;
        }
        if (off) {
            memmove(buf, buf + off, used - off);
            used -= off;
        }
    }
    return 1;
}

static const char *app_name(int s) {
    switch (s) {
    case 0: return "UNKNOWN";
    case 1: return "DETECTED";
    case 2: return "PIN";
    case 3: return "PUK";
    case 4: return "SUBSCRIPTION_PERSO";
    case 5: return "READY";
    case 6: return "NOT_READY";
    case 7: return "PERM_DISABLED";
    default: return "?";
    }
}

static int saw_absent;
static int saw_detected;
static int radio_pulsed;
static int last_app = -1;
static int last_card = -1;
static int64_t t0;

static void log_sim(const uint8_t *b, int len, const char *tag) {
    int64_t dt = now_ms() - t0;
    if (len < 15 || b[10]) {
        printf("t=%lld %s SIM err length=%d error_raw=%u\n",
               (long long)dt, tag, len, len > 10 ? b[10] : 0);
        return;
    }
    unsigned card = b[12];
    unsigned apps = b[14];
    int app = -1, pin1 = -1;
    if (apps >= 1 && len >= 18) app = b[17];
    if (apps >= 1 && len >= 75) pin1 = b[72];
    if (card == 0) saw_absent = 1;
    if (app == 1) saw_detected = 1;
    if (card != (unsigned)last_card || app != last_app) {
        printf("t=%lld %s card=%u apps=%u app=%d(%s) pin1=%d\n",
               (long long)dt, tag, card, apps, app, app >= 0 ? app_name(app) : "n/a", pin1);
        last_card = (int)card;
        last_app = app;
    } else {
        printf("t=%lld %s same card=%u app=%d\n", (long long)dt, tag, card, app);
    }
}

static int poll_sim(int ipc, int rfs, uint32_t token, const char *tag, int timeout_ms,
                    uint8_t *resp, int *n) {
    uint8_t req[12];
    memset(req, 0, sizeof req);
    req[2] = 0x00;
    req[3] = 0x02;
    req[4] = 12;
    req[6] = (uint8_t)token;
    req[7] = (uint8_t)(token >> 8);
    req[8] = (uint8_t)(token >> 16);
    req[9] = (uint8_t)(token >> 24);
    int rc = exchange_ms(ipc, rfs, req, 12, 0x0200, token, resp, n, timeout_ms);
    if (!rc) log_sim(resp, *n, tag);
    else printf("t=%lld %s SIM timeout\n", (long long)(now_ms() - t0), tag);
    return rc;
}

static int send_card_power(int ipc, int rfs, uint8_t state, uint32_t token) {
    uint8_t req[13], resp[512];
    int n = 0;
    memset(req, 0, sizeof req);
    req[2] = 0x4c;
    req[3] = 0x02;
    req[4] = 13;
    req[6] = (uint8_t)token;
    req[7] = (uint8_t)(token >> 8);
    req[12] = state;
    printf("t=%lld CardPower state=%u\n", (long long)(now_ms() - t0), state);
    int rc = exchange_ms(ipc, rfs, req, 13, 0x024c, token, resp, &n, 5000);
    if (rc) {
        printf("t=%lld CardPower timeout\n", (long long)(now_ms() - t0));
        return rc;
    }
    printf("t=%lld CardPower rsp length=%d error_raw=%u\n",
           (long long)(now_ms() - t0), n, n > 10 ? resp[10] : 0);
    return 0;
}

static int send_radio_on(int ipc, int rfs, uint32_t token) {
    uint8_t req[18], resp[512];
    int n = 0;
    memset(req, 0, sizeof req);
    req[2] = 0x00;
    req[3] = 0x08;
    req[4] = 18;
    req[6] = (uint8_t)token;
    req[7] = (uint8_t)(token >> 8);
    req[12] = 2; /* ON */
    printf("t=%lld RadioPower ON\n", (long long)(now_ms() - t0));
    int rc = exchange_ms(ipc, rfs, req, 18, 0x0800, token, resp, &n, 5000);
    if (rc) {
        printf("t=%lld RadioPower timeout\n", (long long)(now_ms() - t0));
        return rc;
    }
    printf("t=%lld RadioPower rsp length=%d error_raw=%u\n",
           (long long)(now_ms() - t0), n, n > 10 ? resp[10] : 0);
    radio_pulsed = 1;
    return 0;
}

static void query_reg(int ipc, int rfs, unsigned id, uint32_t token, const char *tag) {
    uint8_t req[12], resp[512];
    int n = 0;
    memset(req, 0, sizeof req);
    req[2] = (uint8_t)id;
    req[3] = (uint8_t)(id >> 8);
    req[4] = 12;
    req[6] = (uint8_t)token;
    req[7] = (uint8_t)(token >> 8);
    int rc = exchange_ms(ipc, rfs, req, 12, id, token, resp, &n, 5000);
    if (rc) {
        printf("%s timeout\n", tag);
        return;
    }
    printf("%s length=%d error_raw=%u", tag, n, n > 10 ? resp[10] : 0);
    if (!resp[10] && n >= 16)
        printf(" registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u",
               resp[12], resp[13], resp[15]);
    puts("");
}

static void print_rmnet(void) {
    FILE *f;
    unsigned long rx = 0, tx = 0;
    f = fopen("/sys/class/net/rmnet0/statistics/rx_bytes", "r");
    if (f) {
        if (fscanf(f, "%lu", &rx) != 1) rx = 0;
        fclose(f);
    }
    f = fopen("/sys/class/net/rmnet0/statistics/tx_bytes", "r");
    if (f) {
        if (fscanf(f, "%lu", &tx) != 1) tx = 0;
        fclose(f);
    }
    printf("rmnet0 rx=%lu tx=%lu\n", rx, tx);
    f = popen("ip -4 -o addr show rmnet0 2>/dev/null", "r");
    if (f) {
        char line[256];
        if (fgets(line, sizeof line, f)) printf("rmnet0 ipv4: %s", line);
        else puts("rmnet0 ipv4: none");
        pclose(f);
    }
}

/* Maybe trigger Radio ON if DETECTED seen (or ABSENT→DETECTED). */
static void maybe_pulse(int ipc, int rfs, uint32_t *tok) {
    if (radio_pulsed) return;
    if (saw_detected) {
        printf("t=%lld DETECTED seen — pulse Radio ON\n", (long long)(now_ms() - t0));
        send_radio_on(ipc, rfs, (*tok)++);
    }
}

static int poll_burst(int ipc, int rfs, uint32_t *tok, int duration_ms, const char *tag) {
    int64_t end = now_ms() + duration_ms;
    uint8_t resp[512];
    int n = 0;
    while (now_ms() < end) {
        poll_sim(ipc, rfs, (*tok)++, tag, 200, resp, &n);
        maybe_pulse(ipc, rfs, tok);
        if (last_app == 5) return 1; /* READY */
    }
    return 0;
}

int main(void) {
    alarm(90);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) {
        puts("requires ONLINE");
        return 1;
    }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        puts("status lock busy");
        return 1;
    }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) {
        puts("channel open failed");
        return 1;
    }

    t0 = now_ms();
    uint32_t tok = 300;
    uint8_t resp[512];
    int n = 0;
    puts("RACE begin: poll 0x0200 + CardPower 4→1");
    poll_sim(ipc, rfs, tok++, "pre", 2000, resp, &n);

    if (send_card_power(ipc, rfs, 4, tok++)) goto out;
    /* Fast poll while down / transitioning */
    poll_burst(ipc, rfs, &tok, 2500, "down");

    if (send_card_power(ipc, rfs, 1, tok++)) goto out;
    poll_burst(ipc, rfs, &tok, 8000, "up");

    /* If DETECTED was seen but pulse failed timing, one more ON */
    if (saw_detected && !radio_pulsed) send_radio_on(ipc, rfs, tok++);
    else if (saw_detected && radio_pulsed) {
        /* re-pulse once to refresh START_NETWORK */
        printf("t=%lld re-pulse Radio ON\n", (long long)(now_ms() - t0));
        radio_pulsed = 0;
        send_radio_on(ipc, rfs, tok++);
        poll_burst(ipc, rfs, &tok, 3000, "post-radio");
    } else {
        puts("never saw DETECTED(1) during race");
        /* still ensure radio ON for baseline post checks */
        send_radio_on(ipc, rfs, tok++);
        poll_burst(ipc, rfs, &tok, 2000, "post-miss");
    }

    puts("--- final ---");
    poll_sim(ipc, rfs, tok++, "final", 3000, resp, &n);
    query_reg(ipc, rfs, 0x0700, tok++, "voice");
    query_reg(ipc, rfs, 0x0701, tok++, "data");
    print_rmnet();
    printf("RESULT saw_absent=%d saw_detected=%d radio_pulsed=%d last_app=%d last_card=%d\n",
           saw_absent, saw_detected, radio_pulsed, last_app, last_card);

out:
    close(ipc);
    close(rfs);
    close(lock);
    return 0;
}
