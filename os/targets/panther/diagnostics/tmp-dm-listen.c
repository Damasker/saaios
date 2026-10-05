#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <time.h>
#include <unistd.h>

/* RO listen umts_dm0 + oem_ipc0..7. Count ASCII needles only. No dumps. */
static const char *needles[] = {
    "CDMA_MEAS_RESULT_IND",
    "CDMA_TIMING_LATCH",
    "MMC_LTEL1_CDMA",
    "SET_APP",
    "No CDMA",
    "SupportedRatMap",
    "QM_MM_INIT",
    "GMC",
    NULL
};

static int open_class(const char *name) {
    char path[128];
    unsigned maj = 0, min = 0;
    snprintf(path, sizeof path, "/sys/class/cpif/%s/dev", name);
    FILE *f = fopen(path, "r");
    if (!f) return -1;
    if (fscanf(f, "%u:%u", &maj, &min) != 2) { fclose(f); return -1; }
    fclose(f);
    snprintf(path, sizeof path, "/dev/%s", name);
    unlink(path);
    if (mknod(path, S_IFCHR | 0600, makedev(maj, min))) return -1;
    int fd = open(path, O_RDONLY | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0) return -1;
    struct stat st;
    if (fstat(fd, &st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev) != maj || minor(st.st_rdev) != min) {
        close(fd);
        return -1;
    }
    return fd;
}

int main(void) {
    FILE *ms = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!ms || fscanf(ms, "%31s", state) != 1) return 1;
    fclose(ms);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }

    const char *names[] = {
        "umts_dm0", "oem_ipc0", "oem_ipc1", "oem_ipc2", "oem_ipc3",
        "oem_ipc4", "oem_ipc5", "oem_ipc6", "oem_ipc7", NULL
    };
    int fds[9];
    int nfd = 0;
    for (int i = 0; names[i]; i++) {
        fds[i] = open_class(names[i]);
        printf("open %s fd=%d errno=%d\n", names[i], fds[i], fds[i] < 0 ? errno : 0);
        if (fds[i] >= 0) nfd++;
    }

    unsigned long bytes[9] = {0};
    unsigned hits[9][8] = {{0}};
    unsigned char tail[9][64];
    int tlen[9] = {0};
    memset(tail, 0, sizeof tail);

    struct timespec t0;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    const int window_s = 40;
    while (1) {
        struct timespec now;
        clock_gettime(CLOCK_MONOTONIC, &now);
        if (now.tv_sec - t0.tv_sec >= window_s) break;
        struct pollfd pfd[9];
        int np = 0;
        int map[9];
        for (int i = 0; names[i]; i++) {
            if (fds[i] < 0) continue;
            pfd[np].fd = fds[i];
            pfd[np].events = POLLIN;
            pfd[np].revents = 0;
            map[np] = i;
            np++;
        }
        if (!np) break;
        int pr = poll(pfd, (nfds_t)np, 500);
        if (pr < 0 && errno == EINTR) continue;
        if (pr <= 0) continue;
        unsigned char buf[4096];
        for (int k = 0; k < np; k++) {
            if (!(pfd[k].revents & POLLIN)) continue;
            int i = map[k];
            ssize_t n = read(fds[i], buf, sizeof buf);
            if (n <= 0) continue;
            bytes[i] += (unsigned long)n;
            /* Scan overlapping with 32-byte tail. */
            int old = tlen[i];
            unsigned char win[64 + 4096];
            memcpy(win, tail[i], (size_t)old);
            memcpy(win + old, buf, (size_t)n);
            int wlen = old + (int)n;
            for (int ni = 0; needles[ni]; ni++) {
                size_t sl = strlen(needles[ni]);
                for (int p = 0; p + (int)sl <= wlen; p++) {
                    if (memcmp(win + p, needles[ni], sl) == 0)
                        hits[i][ni]++;
                }
            }
            int keep = wlen > 63 ? 63 : wlen;
            memcpy(tail[i], win + (wlen - keep), (size_t)keep);
            tlen[i] = keep;
        }
    }

    printf("listen_s=%d nodes_open=%d\n", window_s, nfd);
    for (int i = 0; names[i]; i++) {
        if (fds[i] < 0) continue;
        printf("%s bytes=%lu", names[i], bytes[i]);
        for (int ni = 0; needles[ni]; ni++)
            if (hits[i][ni])
                printf(" %s=%u", needles[ni], hits[i][ni]);
        puts("");
        close(fds[i]);
    }
    return 0;
}
