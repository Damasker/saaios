/* BuildSetupDataCall simple: id 0x0600, length 246 (sit-stream 0x78c80).
 * APN from /data/saaios/etc/apn or argv[1] — never invent a carrier string.
 * Layout (RE): hdr 12; cid@12; proto@13 (factory default IPV4V6=3);
 * APN@16 max 100; user@117; pass@167; auth@217. No PIN/CardPower/EFS. */
#define _GNU_SOURCE
#include <ctype.h>
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

#define SETUP_ID 0x0600
#define SETUP_LEN 246
#define APN_OFF 16
#define APN_MAX 100
#define USER_OFF 117
#define PASS_OFF 167
#define AUTH_OFF 217
#define PROTO_IPV4V6 3
#define DEFAULT_APN_PATH "/data/saaios/etc/apn"

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
static int apn_usable(const char *s) {
    size_t n = 0;
    if (!s || !*s) return 0;
    for (; s[n]; n++) {
        unsigned char c = (unsigned char)s[n];
        if (n >= APN_MAX) return 0;
        if (c == '.') {
            if (n == 0 || s[n - 1] == '.' || s[n + 1] == 0) return 0;
        } else if (!((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
                     (c >= '0' && c <= '9') || c == '-' || c == '_')) {
            return 0;
        }
    }
    if (!strcasecmp(s, "none") || !strcasecmp(s, "null")) return 0;
    return 1;
}

static int apn_fixture(void) {
    const char *valid[] = {"internet", "fast.t-mobile.com", "a_b-1"};
    const char *invalid[] = {"", "none", "NULL", ".internet", "internet.",
                             "a..b", "bad apn", "a/b"};
    char oversized[APN_MAX + 2];
    memset(oversized, 'a', sizeof oversized - 1);
    oversized[sizeof oversized - 1] = 0;
    for (size_t i = 0; i < sizeof valid / sizeof *valid; i++)
        if (!apn_usable(valid[i])) return 1;
    for (size_t i = 0; i < sizeof invalid / sizeof *invalid; i++)
        if (apn_usable(invalid[i])) return 2;
    if (apn_usable(oversized)) return 3;
    puts("PASS: APN validation accepts single-label and dotted names");
    return 0;
}

/* Read first usable APN line. Never logs the APN string. */
static int load_apn(const char *path, const char *argv1, char *out, size_t out_sz) {
    if (argv1 && apn_usable(argv1)) {
        snprintf(out, out_sz, "%s", argv1);
        return 0;
    }
    FILE *f = fopen(path ? path : DEFAULT_APN_PATH, "r");
    if (!f) return -1;
    char line[160];
    while (fgets(line, sizeof line, f)) {
        char *p = line;
        while (*p && isspace((unsigned char)*p)) p++;
        if (*p == '#' || !*p) continue;
        size_t L = strlen(p);
        while (L && (p[L - 1] == '\n' || p[L - 1] == '\r' || isspace((unsigned char)p[L - 1])))
            p[--L] = 0;
        const char *v = p;
        if (!strncasecmp(p, "apn=", 4)) v = p + 4;
        while (*v && isspace((unsigned char)*v)) v++;
        if (apn_usable(v)) {
            snprintf(out, out_sz, "%s", v);
            fclose(f);
            return 0;
        }
    }
    fclose(f);
    return -1;
}

/* A data call is meaningful only after PS registration. Never log the APN. */
static int data_registered(int ipc) {
    static const uint8_t request[12] = {
        0, 0, 0x01, 0x07, 12, 0, 90, 0, 0, 0, 0, 0
    };
    if (write(ipc, request, sizeof request) != sizeof request) return 0;
    uint8_t buf[4096];
    size_t used = 0;
    int64_t end = now_ms() + 8000;
    while (now_ms() < end) {
        struct pollfd pfd = {ipc, POLLIN, 0};
        int ready = poll(&pfd, 1, 200);
        if (ready < 0 && errno == EINTR) continue;
        if (ready < 0 || (pfd.revents & (POLLERR | POLLHUP | POLLNVAL))) return 0;
        if (!(pfd.revents & POLLIN)) continue;
        if (used == sizeof buf) return 0;
        ssize_t got = read(ipc, buf + used, sizeof buf - used);
        if (got < 0 && (errno == EAGAIN || errno == EINTR)) continue;
        if (got <= 0) return 0;
        used += (size_t)got;
        size_t off = 0;
        while (off < used) {
            int len = frame_size(buf + off, used - off);
            if (len < 0) return 0;
            if (!len) break;
            const uint8_t *frame = buf + off;
            if (frame[0] == 1 && le16(frame + 2) == 0x0701 &&
                le32(frame + 6) == 90) {
                if (len < 16 || frame[10] != 0) return 0;
                printf("DataRegistration raw=%u\n", frame[12]);
                return frame[12] == 1 || frame[12] == 5;
            }
            off += (size_t)len;
        }
        if (off) { memmove(buf, buf + off, used - off); used -= off; }
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "--self-test")) return apn_fixture();
    char apn[APN_MAX + 1];
    const char *path = DEFAULT_APN_PATH;
    const char *argv_apn = NULL;
    for (int i = 1; i < argc; i++) {
        if (!strcmp(argv[i], "--apn-file") && i + 1 < argc) {
            path = argv[++i];
        } else if (argv[i][0] != '-') {
            argv_apn = argv[i];
        }
    }
    if (load_apn(path, argv_apn, apn, sizeof apn)) {
        puts("SetupDataCall deferred_no_apn");
        printf("apn_hint=write_carrier_apn_to_%s\n", DEFAULT_APN_PATH);
        return 2;
    }

    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1 || strcmp(state, "ONLINE")) {
        puts("requires ONLINE");
        return 1;
    }
    fclose(f);
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    /* RFS has its own broker. This client must never consume its messages. */
    if (ipc < 0) { puts("open fail"); return 1; }
    if (!data_registered(ipc)) {
        puts("SetupDataCall deferred_not_registered");
        close(ipc); close(lock);
        return 4;
    }

    uint8_t req[SETUP_LEN];
    memset(req, 0, sizeof req);
    req[2] = (uint8_t)(SETUP_ID & 0xff);
    req[3] = (uint8_t)((SETUP_ID >> 8) & 0xff);
    req[4] = (uint8_t)(SETUP_LEN & 0xff);
    req[5] = (uint8_t)((SETUP_LEN >> 8) & 0xff);
    req[6] = 91; /* token */
    req[12] = 1; /* cid */
    req[13] = PROTO_IPV4V6; /* factory default when DB has no protocol */
    size_t alen = strlen(apn);
    if (alen > APN_MAX) alen = APN_MAX;
    memcpy(req + APN_OFF, apn, alen);
    /* user@117 / pass@167 left zero; auth@217 = 0 (no auth) */
    (void)USER_OFF;
    (void)PASS_OFF;
    (void)AUTH_OFF;

    printf("SetupDataCall send id=0x%04x len=%u apn_len=%zu proto=%u cid=1\n",
           SETUP_ID, SETUP_LEN, alen, (unsigned)PROTO_IPV4V6);

    if (write(ipc, req, SETUP_LEN) != SETUP_LEN) {
        puts("write fail");
        return 1;
    }
    uint8_t buf[4096];
    size_t used = 0;
    int64_t end = now_ms() + 20000;
    while (now_ms() < end) {
        struct pollfd pfd = {ipc, POLLIN, 0};
        poll(&pfd, 1, 200);
        if (pfd.revents & POLLIN && used < sizeof buf) {
            ssize_t n = read(ipc, buf + used, sizeof buf - used);
            if (n > 0) used += (size_t)n;
        }
        size_t off = 0;
        while (off < used) {
            int len = frame_size(buf + off, used - off);
            if (len <= 0) break;
            if (le16(buf + off + 2) == SETUP_ID) {
                unsigned err = len > 10 ? buf[off + 10] : 0;
                printf("SetupDataCall length=%d error_raw=%u tok=%u\n",
                       len, err, le32(buf + off + 6));
                close(ipc); close(lock);
                return err == 0 ? 0 : 3;
            }
            if (le16(buf + off + 2) != 0x0200)
                printf("ipc id=%u length=%d\n", le16(buf + off + 2), len);
            off += (size_t)len;
        }
        if (off) { memmove(buf, buf + off, used - off); used -= off; }
    }
    puts("SetupDataCall timeout");
    return 1;
}
