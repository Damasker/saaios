/* ONE-SHOT SitOem Ping on /dev/oem_ipc0 — evidenced protobuf shape only.
 * From factory carved SitOem (oem_ipc0 ELF):
 *   IpcMessageType REQUEST = 1 (sitSendPingReq → PingMessageModemData ctor)
 *   PayloadCase ping = 5 (Ping ctor MOVZ #5 → initialMessageHeader)
 *   PingMessage ipc oneof request = 1 (fillInput MOVZ #1)
 *   PingRequest carries string from fillInput(const char*) as field 1 (len-delim)
 * Userspace write = raw protobuf (kernel prepends EXYNOS 12B). Not catalog 0x2f50.
 * Prints lengths / first-byte tags only — never ping string content beyond "x".
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <unistd.h>

/* Minimal evidenced Ping REQUEST, token=1, ping string "x" (1 byte, non-secret).
 * Wire (protobuf):
 *   08 01          type=1 REQUEST
 *   10 01          token=1
 *   2a 05          ping { ... } len=5
 *     0a 03        request { ... } len=3
 *       0a 01 78   data="x"
 */
static const uint8_t PING_REQ[] = {
    0x08, 0x01,
    0x10, 0x01,
    0x2a, 0x05,
        0x0a, 0x03,
            0x0a, 0x01, 0x78
};

static void print_tags(const uint8_t *p, int n, const char *tag) {
    printf("%s: n=%d tags=", tag, n);
    int lim = n < 16 ? n : 16;
    for (int i = 0; i < lim; i++) printf("%02x%s", p[i], i + 1 == lim ? "" : " ");
    if (n > 16) printf(" ...");
    puts("");
}

int main(void) {
    unsigned maj = 0, min = 0;
    FILE *f = fopen("/sys/class/cpif/oem_ipc0/dev", "r");
    if (!f) {
        /* fallback path variants */
        f = fopen("/sys/devices/platform/cpif/oem_ipc0/dev", "r");
    }
    if (f) {
        if (fscanf(f, "%u:%u", &maj, &min) != 2) { fclose(f); return 1; }
        fclose(f);
    }
    /* Ensure node exists (prior mknod 493:12). */
    struct stat st;
    if (stat("/dev/oem_ipc0", &st) || !S_ISCHR(st.st_mode)) {
        if (!maj) return 1;
        unlink("/dev/oem_ipc0");
        if (mknod("/dev/oem_ipc0", S_IFCHR | 0666, makedev(maj, min))) return 1;
    }
    int fd = open("/dev/oem_ipc0", O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0) { perror("open oem_ipc0"); return 1; }
    if (fstat(fd, &st) || !S_ISCHR(st.st_mode)) { close(fd); return 1; }
    printf("oem_ipc0 open ok maj=%u min=%u write_len=%zu\n",
           major(st.st_rdev), minor(st.st_rdev), sizeof PING_REQ);
    print_tags(PING_REQ, (int)sizeof PING_REQ, "TX");
    ssize_t w = write(fd, PING_REQ, sizeof PING_REQ);
    if (w != (ssize_t)sizeof PING_REQ) {
        printf("write_ret=%zd errno=%d\n", w, errno);
        close(fd);
        return 2;
    }
    puts("write_ok");
    /* Read any response / indication for a short window. */
    uint8_t buf[512];
    int got = 0;
    for (int i = 0; i < 20; i++) {
        struct pollfd pfd = {fd, POLLIN, 0};
        int pr = poll(&pfd, 1, 100);
        if (pr < 0 && errno == EINTR) continue;
        if (pr <= 0) continue;
        if (!(pfd.revents & POLLIN)) break;
        ssize_t n = read(fd, buf, sizeof buf);
        if (n < 0 && (errno == EAGAIN || errno == EINTR)) continue;
        if (n <= 0) { printf("read_ret=%zd errno=%d\n", n, errno); break; }
        print_tags(buf, (int)n, "RX");
        got = 1;
        break;
    }
    if (!got) puts("RX: none (timeout 2s)");
    close(fd);
    return got ? 0 : 3;
}
