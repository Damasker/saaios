/* ONE-SHOT: dump 0x0200 layout bytes only (no AID/IMSI). Validates type@15 vs state@17. */
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
int main(void) {
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    unsigned maj = 0, min = 0;
    f = fopen("/sys/class/cpif/umts_ipc0/dev", "r");
    if (!f || fscanf(f, "%u:%u", &maj, &min) != 2) return 1;
    fclose(f);
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("lock busy"); return 2; }
    int fd = open("/dev/umts_ipc0", O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    struct stat st;
    if (fd < 0 || fstat(fd, &st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev) != maj || minor(st.st_rdev) != min) return 1;
    const uint8_t req[12] = {0, 0, 0x00, 0x02, 12, 0, 0x42, 0, 0, 0, 0, 0};
    if (write(fd, req, 12) != 12) return 1;
    uint8_t buf[8192];
    size_t used = 0;
    int64_t end = now_ms() + 8000;
    while (now_ms() < end) {
        struct pollfd pfd = {fd, POLLIN, 0};
        if (poll(&pfd, 1, 200) <= 0) continue;
        ssize_t n = read(fd, buf + used, sizeof buf - used);
        if (n <= 0) continue;
        used += (size_t)n;
        size_t off = 0;
        while (off < used) {
            int fs = frame_size(buf + off, used - off);
            if (fs < 0) { off++; continue; }
            if (!fs) break;
            if (buf[off] == 1 && le16(buf + off + 2) == 0x0200 &&
                le32(buf + off + 6) == 0x42 && fs >= 75 && buf[off + 10] == 0) {
                const uint8_t *b = buf + off;
                printf("len=%d card=%u upin=%u apps=%u type=%u state=%u "
                       "b16=%u b18=%u pin1=%u pin2=%u remain1=%u remain2=%u\n",
                       fs, b[12], b[13], b[14], b[15], b[17], b[16], b[18],
                       b[72], b[73], b[74], b[75]);
                printf("byte12_20=");
                for (int i = 12; i <= 20; i++) printf("%u%s", b[i], i == 20 ? "\n" : ",");
                /* If apps>1, second slot starts at 15+63=78 */
                if (b[14] > 1 && fs > 80)
                    printf("app1_type=%u app1_state=%u\n", b[78], b[80]);
                close(fd); close(lock);
                return 0;
            }
            off += (size_t)fs;
        }
        if (off) { memmove(buf, buf + off, used - off); used -= off; }
    }
    puts("NO_MATCH");
    close(fd); close(lock);
    return 3;
}
