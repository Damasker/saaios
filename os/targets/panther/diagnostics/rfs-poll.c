/* One-shot, non-consuming RFS readiness check. No response or NV access. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <unistd.h>

int main(void) {
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
    close(fd);
    return 0;
}
