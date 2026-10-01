/* Bounded, redacted SIT observer for a diagnostic Pixel 7 boot.
 * Holds umts_ipc0 open for 30 seconds and reports frame headers only.
 * No SIT requests, RFS access, PIN, APN, or NV/EFS operations.
 */
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

enum { RX_CAP = 65536, MAX_FRAMES = 256, WINDOW_MS = 30000 };

static unsigned le16(const uint8_t *p) { return p[0] | ((unsigned)p[1] << 8); }

static int64_t now_ms(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts)) return -1;
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

static int frame_size(const uint8_t *p, size_t n) {
    if (n < 6) return 0;
    if (p[0] > 2) return -1;
    unsigned min = p[0] == 2 ? 8U : 12U;
    unsigned len = le16(p + 4);
    if (len < min || len > RX_CAP) return -1;
    return n < len ? 0 : (int)len;
}

static int fixture(void) {
    uint8_t unsol[8] = {2, 0, 0x34, 0x12, 8, 0, 0, 0};
    if (frame_size(unsol, 7) != 0 || frame_size(unsol, 8) != 8) return 1;
    unsol[4] = 7;
    if (frame_size(unsol, 8) != -1) return 1;
    unsol[0] = 1;
    unsol[4] = 12;
    if (frame_size(unsol, 8) != 0) return 1;
    puts("PASS SIT redacted observer framing");
    return 0;
}

static int online(void) {
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f) return 0;
    int ok = fscanf(f, "%31s", state) == 1 && strcmp(state, "ONLINE") == 0;
    fclose(f);
    return ok;
}

static int open_verified_ipc(void) {
    unsigned maj = 0, min = 0;
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

static int observe(void) {
    int lock = open("/run/saaios-sit-status.lock",
                    O_CREAT | O_RDWR | O_CLOEXEC | O_NOFOLLOW, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        fputs("ABORT common SIT lock busy or unavailable\n", stderr);
        if (lock >= 0) close(lock);
        return 1;
    }
    if (!online()) { fputs("ABORT CP not ONLINE\n", stderr); close(lock); return 1; }
    int fd = open_verified_ipc();
    if (fd < 0) { fputs("ABORT unverified IPC device\n", stderr); close(lock); return 1; }
    int64_t start = now_ms();
    if (start < 0) { close(fd); close(lock); return 1; }
    int64_t end = start + WINDOW_MS;
    uint8_t rx[RX_CAP];
    size_t used = 0;
    unsigned frames = 0;
    int rc = 0;
    puts("observer=started duration_ms=30000 payload=redacted");
    fflush(stdout);
    while (frames < MAX_FRAMES) {
        int64_t now = now_ms();
        if (now < 0) { rc = 1; break; }
        int64_t remain = end - now;
        if (remain <= 0) break;
        struct pollfd pfd = {fd, POLLIN, 0};
        int ready = poll(&pfd, 1, remain > 250 ? 250 : (int)remain);
        if (ready < 0 && errno == EINTR) continue;
        if (ready < 0 || (pfd.revents & (POLLERR | POLLHUP | POLLNVAL))) {
            rc = 1; break;
        }
        if (!(pfd.revents & POLLIN)) continue;
        if (used == sizeof rx) { rc = 1; break; }
        ssize_t n = read(fd, rx + used, sizeof rx - used);
        if (n < 0 && (errno == EINTR || errno == EAGAIN)) continue;
        if (n <= 0) { rc = 1; break; }
        used += (size_t)n;
        size_t off = 0;
        while (off < used) {
            int len = frame_size(rx + off, used - off);
            if (len < 0) { rc = 1; break; }
            if (!len) break;
            const uint8_t *p = rx + off;
            int64_t elapsed = now_ms() - start;
            printf("frame elapsed_ms=%lld type=%u id=0x%04x length=%d\n",
                   (long long)elapsed, p[0], le16(p + 2), len);
            fflush(stdout);
            frames++;
            off += (size_t)len;
            if (frames >= MAX_FRAMES) break;
        }
        if (rc) break;
        if (off) { memmove(rx, rx + off, used - off); used -= off; }
    }
    printf("observer=done frames=%u result=%s\n", frames, rc ? "error" : "bounded");
    close(fd);
    close(lock);
    return rc;
}

int main(int argc, char **argv) {
    if (argc == 2 && strcmp(argv[1], "self-test") == 0) return fixture();
    if (argc == 2 && strcmp(argv[1], "observe-30") == 0) return observe();
    fputs("usage: sit-unsol-observe self-test|observe-30\n", stderr);
    return 64;
}
