/*
 * Unsafe, opt-in one-shot RFS poll. It does not read or answer packets, but
 * ipc_release() purges the RX queue when this is the last open RFS descriptor.
 * A CP request can therefore be lost when the probe closes.
 */
#define _GNU_SOURCE
#include <stdio.h>
#include <string.h>
#ifndef _WIN32
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <unistd.h>
#endif

int main(int argc, char **argv) {
    if (argc != 2 || strcmp(argv[1], "--unsafe-poll-once") != 0) {
        fputs("Refusing to open RFS: closing its last descriptor purges queued CP requests.\n"
              "Use --unsafe-poll-once only if that packet-loss risk is acceptable.\n", stderr);
        return 64;
    }
#ifdef _WIN32
    fputs("RFS device poll is available only on Linux.\n", stderr);
    return 69;
#else
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) return 2;
    unsigned maj = 0, min = 0;
    f = fopen("/sys/class/cpif/umts_rfs0/dev", "r");
    if (!f || fscanf(f, "%u:%u", &maj, &min) != 2) return 3;
    fclose(f);
    int fd = open("/dev/umts_rfs0", O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    struct stat st;
    if (fd < 0 || fstat(fd, &st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev) != maj || minor(st.st_rdev) != min) return 4;
    struct pollfd pfd = {.fd=fd, .events=POLLIN};
    int n = poll(&pfd, 1, 1000);
    if (n < 0) { fprintf(stderr, "poll: %s\n", strerror(errno)); return 5; }
    printf("rfs_pending=%d poll_events=0x%x\n", n > 0 && !!(pfd.revents & POLLIN), pfd.revents);
    fputs("WARNING: closing the last RFS descriptor may purge queued CP requests.\n", stderr);
    close(fd);
    return 0;
#endif
}
