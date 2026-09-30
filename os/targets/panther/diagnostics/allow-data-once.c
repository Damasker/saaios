/* One-shot BuildAllowData: id 0x0710, length 13, byte12=allow. */
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
    uint8_t req[13] = {0};
    req[2] = 0x10;
    req[3] = 0x07;
    req[4] = 13;
    req[6] = 251;
    req[12] = 1;
    if (write(ipc, req, 13) != 13) { puts("write fail"); return 1; }
    uint8_t buf[4096];
    size_t used = 0;
    int64_t end = now_ms() + 8000;
    while (now_ms() < end) {
        struct pollfd pfd[2] = {{ipc, POLLIN, 0}, {rfs, POLLIN, 0}};
        poll(pfd, 2, 200);
        if (pfd[1].revents & POLLIN) {
            uint8_t junk[512];
            (void)read(rfs, junk, sizeof junk);
        }
        if (pfd[0].revents & POLLIN && used < sizeof buf) {
            ssize_t n = read(ipc, buf + used, sizeof buf - used);
            if (n > 0) used += (size_t)n;
        }
        size_t off = 0;
        while (off < used) {
            int len = frame_size(buf + off, used - off);
            if (len < 0) return 1;
            if (!len) break;
            if (le16(buf + off + 2) == 0x0710 && le32(buf + off + 6) == 251) {
                printf("AllowData length=%d error_raw=%u\n", len,
                       len > 10 ? buf[off + 10] : 0u);
                close(ipc); close(rfs); close(lock);
                return 0;
            }
            off += (size_t)len;
        }
        if (off) { memmove(buf, buf + off, used - off); used -= off; }
    }
    puts("AllowData timeout");
    return 1;
}
