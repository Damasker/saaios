#include <fcntl.h>
#include <poll.h>
#include <stdio.h>
#include <unistd.h>

/* Print a 12-byte RFS header as three words. Longer reads stay unprinted. */
int main(void) {
    int fd = open("/dev/umts_rfs0", O_RDWR | O_NONBLOCK | O_CLOEXEC);
    if (fd < 0) {
        perror("open");
        return 1;
    }
    struct pollfd pfd = {.fd = fd, .events = POLLIN};
    int ready = poll(&pfd, 1, 1500);
    unsigned char buf[64];
    ssize_t n = 0;
    if (ready > 0)
        n = read(fd, buf, sizeof(buf));
    printf("ready=%d bytes=%zd\n", ready, n);
    if (n == 12) {
        unsigned w0 = buf[0] | ((unsigned)buf[1] << 8) | ((unsigned)buf[2] << 16) | ((unsigned)buf[3] << 24);
        unsigned w1 = buf[4] | ((unsigned)buf[5] << 8) | ((unsigned)buf[6] << 16) | ((unsigned)buf[7] << 24);
        unsigned w2 = buf[8] | ((unsigned)buf[9] << 8) | ((unsigned)buf[10] << 16) | ((unsigned)buf[11] << 24);
        printf("words %08x %08x %08x\n", w0, w1, w2);
    }
    return 0;
}
