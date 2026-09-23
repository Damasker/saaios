#define read simulated_read
#define poll simulated_poll
#define main unused_original_main
#ifndef CP_BOOT_SOURCE
#error "Define CP_BOOT_SOURCE to the absolute path of the loader snapshot under test"
#endif
#include CP_BOOT_SOURCE
#undef main
#undef read
#undef poll
#include <assert.h>
static unsigned char wire[] = {0x2b, 0xc1, 0, 0, 0x20, 0xc1, 0, 0};
static size_t position;
static int injected, transient;
ssize_t simulated_read(int fd, void *out, size_t count) {
    (void)fd;
    if (transient && position == 2 && !injected++) {
        if (transient == 1) return 0;
        errno = transient == 2 ? EINTR : EAGAIN;
        return -1;
    }
    size_t n = sizeof(wire) - position;
    if (count < n) n = count;
    if (transient && position == 0 && n > 2) n = 2;
    memcpy(out, wire + position, n);
    position += n;
    return (ssize_t)n;
}
int simulated_poll(struct pollfd *p, nfds_t count, int timeout) {
    (void)count; (void)timeout;
    p->revents = POLLIN;
    return 1;
}
int main(void) {
    int failed = 0;
    for (transient = 0; transient <= 3; transient++) {
        position = 0; injected = 0;
        uint32_t first = 0, second = 0;
        int a = sit_wait_u32(&first, 10, NULL, NULL);
        int b = sit_wait_u32(&second, 10, NULL, NULL);
        int ok = a == 0 && b == 0 && first == 0xc12b && second == 0xc120;
        printf("case=%d %s first=%x second=%x\n", transient, ok ? "PASS" : "FAIL", first, second);
        failed += !ok;
    }
    return failed ? 1 : 0;
}
