/* Radio OFF→ON only. No CardPower, no VerifyPin. */
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
    if (len < min) return -1;
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
    if (n == 12 && buf[0] == 0x07) {
        static const uint8_t reply[16] = {
            0x03,0,0,0, 0x08,0,0,0, 0,0,0,0, 0x03,0,0,0
        };
        (void)write(fd, reply, sizeof reply);
    }
}
static int exchange(int ipc, int rfs, const uint8_t *req, size_t req_len,
                    unsigned id, uint32_t tok, int timeout_ms) {
    if (write(ipc, req, req_len) != (ssize_t)req_len) return -1;
    uint8_t buf[4096];
    size_t used = 0;
    int64_t end = now_ms() + timeout_ms;
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
            if (len <= 0) break;
            if (le16(buf + off + 2) == id && le32(buf + off + 6) == tok) {
                printf("id=0x%x length=%d error_raw=%u\n", id, len,
                       len > 10 ? buf[off + 10] : 0);
                return 0;
            }
            off += (size_t)len;
        }
        if (off) { memmove(buf, buf + off, used - off); used -= off; }
    }
    printf("timeout id=0x%x\n", id);
    return 1;
}
int main(void) {
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1 || strcmp(state, "ONLINE")) {
        puts("requires ONLINE");
        return 1;
    }
    fclose(f);
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("open fail"); return 1; }
    uint8_t req[18] = {0};
    req[2] = 0x00; req[3] = 0x08; req[4] = 18;
    req[6] = 191; req[12] = 1;
    puts("radio OFF");
    (void)exchange(ipc, rfs, req, 18, 0x0800, 191, 10000);
    usleep(3000000);
    memset(req, 0, sizeof req);
    req[2] = 0x00; req[3] = 0x08; req[4] = 18;
    req[6] = 192; req[12] = 2;
    puts("radio ON");
    (void)exchange(ipc, rfs, req, 18, 0x0800, 192, 10000);
    close(ipc); close(rfs); close(lock);
    return 0;
}
