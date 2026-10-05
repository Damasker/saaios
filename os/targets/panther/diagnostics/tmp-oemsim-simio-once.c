/* ONE OemSim soft path: SIT SIM_IO 0x0208 STATUS (cmd 0xF2) with empty defaults.
   Factory: BuildSimIO id 0x0208 length 0x23c; BuildOemSimRequest RIL 28→0x208.
   No secrets printed. No POWER_OFF / crash / cbd / rild / EFS RW. */
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
    if (len < min || len > 65536) return -1;
    return n < len ? 0 : (int)len;
}
static int open_node(const char *node, const char *sysdev) {
    unsigned maj = 0, min = 0;
    FILE *f = fopen(sysdev, "r");
    if (!f || fscanf(f, "%u:%u", &maj, &min) != 2) {
        if (f) fclose(f);
        return -1;
    }
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
    if (n <= 0) return;
    if (n == (ssize_t)sizeof buf) {
        uint8_t dump[256];
        while (read(fd, dump, sizeof dump) > 0) {
        }
        puts("rfs large drained");
        return;
    }
    if (n == 12 && buf[0] == 0x07 && !buf[1] && !buf[2] && !buf[3] &&
        buf[4] == 0x04 && !buf[5] && !buf[6] && !buf[7] &&
        buf[8] == 0x03 && !buf[9] && !buf[10] && !buf[11]) {
        static const uint8_t reply[16] = {
            0x03, 0, 0, 0, 0x08, 0, 0, 0, 0, 0, 0, 0, 0x03, 0, 0, 0};
        (void)write(fd, reply, sizeof reply);
        puts("rfs unprotect reply");
        return;
    }
    puts("rfs ignored");
}

static int g_app = -1, g_pin = -1, g_remain = -1, g_card = -1, g_apps = -1;

static void note_sim(const uint8_t *b, int len) {
    if (len < 75 || b[10]) return;
    g_card = b[12];
    g_apps = b[14];
    g_app = b[17];
    g_pin = b[72];
    g_remain = b[74];
    printf("SIM card=%d apps=%d app=%d pin1=%d remain=%d len=%d\n",
           g_card, g_apps, g_app, g_pin, g_remain, len);
}

static int exchange(int ipc, int rfs, const uint8_t *req, size_t req_len,
                    unsigned id, uint32_t tok, int timeout_ms,
                    uint8_t *resp, int *resp_len) {
    ssize_t wr = write(ipc, req, req_len);
    if (wr != (ssize_t)req_len) {
        printf("write fail wr=%zd errno=%d\n", wr, errno);
        return -1;
    }
    uint8_t buf[16384];
    size_t used = 0;
    int64_t end = now_ms() + timeout_ms;
    while (now_ms() < end) {
        struct pollfd pfd[2] = {{ipc, POLLIN, 0}, {rfs, POLLIN, 0}};
        if (poll(pfd, 2, 250) < 0) {
            if (errno == EINTR) continue;
            return -1;
        }
        if (pfd[1].revents & POLLIN) service_rfs(rfs);
        if (pfd[0].revents & POLLIN && used < sizeof buf) {
            ssize_t n = read(ipc, buf + used, sizeof buf - used);
            if (n > 0) used += (size_t)n;
        }
        size_t off = 0;
        while (off < used) {
            int len = frame_size(buf + off, used - off);
            if (len < 0) return -1;
            if (!len) break;
            unsigned fid = le16(buf + off + 2);
            uint32_t ftok = le32(buf + off + 6);
            if (fid == 0x0200) note_sim(buf + off, len);
            if (buf[off] == 1 && fid == id && ftok == tok) {
                if (resp && resp_len) {
                    int c = len < 512 ? len : 512;
                    memcpy(resp, buf + off, (size_t)c);
                    *resp_len = c;
                }
                printf("rsp id=0x%04x tok=%u length=%d error_raw=%u\n",
                       id, tok, len, len > 10 ? buf[off + 10] : 0);
                if (id == 0x0208 && len >= 14 && !buf[off + 10])
                    printf("simio sw=%02x%02x body_hint=%d\n",
                           buf[off + 12], buf[off + 13], len);
                return 0;
            }
            off += (size_t)len;
        }
        if (off) {
            used -= off;
            memmove(buf, buf + off, used);
        }
    }
    printf("timeout id=0x%04x tok=%u\n", id, tok);
    return -2;
}

static int get_status(int ipc, int rfs, uint32_t tok) {
    uint8_t req[12] = {0};
    req[2] = 0x00;
    req[3] = 0x02;
    req[4] = 12;
    req[6] = (uint8_t)tok;
    return exchange(ipc, rfs, req, 12, 0x0200, tok, 12000, NULL, NULL);
}

/* Factory BuildSimIO: len 0x23c; +12 cmd, +13 fileid_lo, +14 u16,
   +16 path_len, +17 path, +29 p1, +30 p2, +31 p3, +32 data_len, +34 data,
   +546 pin2_len, +547 pin2, +555 aid_len, +556 aid. Empty defaults = zero. */
static int sim_io_status(int ipc, int rfs, uint32_t tok) {
    uint8_t req[0x23c];
    int n = 0;
    uint8_t resp[512];
    memset(req, 0, sizeof req);
    req[0] = 0;
    req[2] = 0x08;
    req[3] = 0x02;
    req[4] = 0x3c;
    req[5] = 0x02;
    req[6] = (uint8_t)tok;
    req[12] = 0xF2; /* STATUS — documented SIM_IO command in BuildSimIO switch */
    printf("send SIM_IO STATUS 0x0208 len=0x23c tok=%u (empty defaults)\n", tok);
    return exchange(ipc, rfs, req, sizeof req, 0x0208, tok, 20000, resp, &n);
}

int main(void) {
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) {
        printf("requires ONLINE got=%s\n", state);
        return 1;
    }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        puts("lock busy");
        return 1;
    }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) {
        printf("open fail ipc=%d rfs=%d\n", ipc, rfs);
        return 1;
    }
    puts("PRE");
    if (get_status(ipc, rfs, 91) != 0) return 2;
    int pre_app = g_app, pre_pin = g_pin;
    if (sim_io_status(ipc, rfs, 92) != 0) puts("SIM_IO exchange failed (still polling status)");
    usleep(500000);
    puts("POST");
    if (get_status(ipc, rfs, 93) != 0) return 3;
    printf("delta pre_app=%d pre_pin=%d -> app=%d pin1=%d remain=%d\n",
           pre_app, pre_pin, g_app, g_pin, g_remain);
    if (g_app == 1 || g_app == 4 || g_app == 5)
        puts("EDGE_READY_CLASS");
    else
        puts("STILL_NOT_READY");
    close(ipc);
    close(rfs);
    close(lock);
    return 0;
}
