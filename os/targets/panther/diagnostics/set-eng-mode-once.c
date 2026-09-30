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

/* ONE-SHOT: SetEngMode 0x0908 len 13 mode@12 — factory ProtocolMiscDebugBuilder::SetEngMode(h). */
static unsigned le16(const uint8_t *p) { return p[0] | ((unsigned)p[1] << 8); }
static int64_t now_ms(void) {
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}

static int open_node(const char *dev, const char *sys) {
    unsigned maj = 0, min = 0;
    FILE *f = fopen(sys, "r");
    if (!f) return -1;
    if (fscanf(f, "%u:%u", &maj, &min) != 2) { fclose(f); return -1; }
    fclose(f);
    unlink(dev);
    if (mknod(dev, S_IFCHR | 0600, makedev(maj, min))) return -1;
    return open(dev, O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
}

static int exchange(int ipc, int rfs, const uint8_t *req, size_t req_len,
                    unsigned id, uint32_t token, uint8_t *resp, int *n_out) {
    if (write(ipc, req, req_len) != (ssize_t)req_len) return -1;
    int64_t deadline = now_ms() + 8000;
    *n_out = 0;
    while (now_ms() < deadline) {
        struct pollfd p[2] = {{ipc, POLLIN, 0}, {rfs, POLLIN, 0}};
        int pr = poll(p, 2, 200);
        if (pr < 0 && errno == EINTR) continue;
        for (int i = 0; i < 2; i++) {
            if (!(p[i].revents & POLLIN)) continue;
            uint8_t buf[1024];
            ssize_t n = read(p[i].fd, buf, sizeof buf);
            if (n < 12) continue;
            if (le16(buf + 2) == id && (uint32_t)buf[6] == (token & 0xff)) {
                if (n > 512) n = 512;
                memcpy(resp, buf, (size_t)n);
                *n_out = (int)n;
                return 0;
            }
        }
    }
    return -1;
}

static void print_sim(const uint8_t *resp, int n, const char *tag) {
    if (n < 18 || resp[10]) {
        printf("%s: n=%d err=%u\n", tag, n, n > 10 ? resp[10] : 0);
        return;
    }
    unsigned card = resp[12];
    unsigned apps = resp[14];
    unsigned app = n > 17 ? resp[17] : 0;
    unsigned pin1 = n > 72 ? resp[72] : 0xff;
    printf("%s: card=%u apps=%u app=%u pin1=%u\n", tag, card, apps, app, pin1);
}

static void print_rmnet(void) {
    for (int i = 0; i < 3; i++) {
        char path[64];
        unsigned long long rx = 0;
        snprintf(path, sizeof path, "/sys/class/net/rmnet%d/statistics/rx_bytes", i);
        FILE *f = fopen(path, "r");
        if (f) { if (fscanf(f, "%llu", &rx) != 1) rx = 0; fclose(f); }
        printf("rmnet%d rx=%llu\n", i, rx);
    }
}

int main(void) {
    signal(SIGPIPE, SIG_IGN);
    alarm(60);
    FILE *st = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!st || fscanf(st, "%31s", state) != 1) return 1;
    fclose(st);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }

    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 2; }

    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }

    uint8_t req[16], resp[512];
    int n = 0;

    /* baseline SIM */
    memset(req, 0, sizeof req);
    req[2] = 0x00; req[3] = 0x02; req[4] = 12; req[6] = 1;
    if (exchange(ipc, rfs, req, 12, 0x0200, 1, resp, &n)) puts("SIM baseline timeout");
    else print_sim(resp, n, "SIM_before");

    /* SetEngMode: id 0x0908, len 13, mode@12 = 1 */
    memset(req, 0, sizeof req);
    req[2] = 0x08; req[3] = 0x09; req[4] = 13; req[6] = 2; req[12] = 1;
    puts("SetEngMode 0x0908 len=13 mode=1");
    if (exchange(ipc, rfs, req, 13, 0x0908, 2, resp, &n)) {
        puts("SetEngMode timeout");
    } else {
        printf("SetEngMode rsp: n=%d err=%u\n", n, n > 10 ? resp[10] : 0);
        if (n >= 16)
            printf("SetEngMode payload_hex=");
        for (int i = 12; i < n && i < 20; i++) printf("%02x", resp[i]);
        if (n >= 16) puts("");
    }
    memset(req, 0, sizeof req);

    /* post SIM + short poll */
    for (int i = 0; i < 4; i++) {
        usleep(500000);
        memset(req, 0, sizeof req);
        req[2] = 0x00; req[3] = 0x02; req[4] = 12; req[6] = (uint8_t)(10 + i);
        if (exchange(ipc, rfs, req, 12, 0x0200, 10 + (uint32_t)i, resp, &n))
            printf("SIM_after[%d] timeout\n", i);
        else
            print_sim(resp, n, "SIM_after");
    }

    /* data reg 0x0701 */
    memset(req, 0, sizeof req);
    req[2] = 0x01; req[3] = 0x07; req[4] = 12; req[6] = 20;
    if (exchange(ipc, rfs, req, 12, 0x0701, 20, resp, &n))
        puts("reg timeout");
    else
        printf("reg 0x0701: n=%d err=%u tech=%u\n", n, n > 10 ? resp[10] : 0,
               n > 13 ? resp[13] : 0);

    print_rmnet();
    close(ipc); close(rfs); close(lock);
    return 0;
}
