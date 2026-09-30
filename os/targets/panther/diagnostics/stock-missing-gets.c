/* ONE live: stock-evidenced empty GETs we never sent on soft-lock path.
 * BuildSim3GPbCapa 0x0245 len12 (UiccPhonebookHandler::DoGet3GPbCapa)
 * BuildGetPreferredCallCapability 0x0930 len12 (MiscService)
 * Bookend with 0x0200 + radio/allow/pref already-on checks. No PIN digits,
 * no CardPower, no POWER_OFF, no EFS, no rild. */
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

static int g_app = -1, g_pin = -1, g_card = -1, g_apps = -1, g_remain = -1;

static unsigned le16(const uint8_t *p) { return p[0] | ((unsigned)p[1] << 8); }
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
    if (n == 12 && buf[0] == 0x07) {
        static const uint8_t reply[16] = {
            0x03, 0, 0, 0, 0x08, 0, 0, 0, 0, 0, 0, 0, 0x03, 0, 0, 0
        };
        (void)write(fd, reply, sizeof reply);
    }
}
static void note_sim(const uint8_t *p, int len) {
    if (len < 75 || p[0] != 1 || le16(p + 2) != 0x0200) return;
    g_card = p[12];
    g_apps = p[14];
    g_app = p[17];
    g_pin = p[72];
    g_remain = p[74];
}
static const char *app_name(int a) {
    switch (a) {
    case 0: return "UNKNOWN";
    case 1: return "DETECTED";
    case 2: return "PIN";
    case 3: return "PUK";
    case 4: return "PERSO";
    case 5: return "READY";
    default: return "?";
    }
}
static const char *present_infer(int app) {
    /* STATUS Present→SET: 0→PIN 1→PUK 2→READY 3→PERSO */
    if (app == 2) return "notin_1_2_3";
    if (app == 3) return "was_1";
    if (app == 5) return "was_2";
    if (app == 4) return "was_3";
    if (app == 1) return "detected_no_present_map";
    return "unknown";
}
static void print_sim(const char *tag) {
    printf("%s card=%d apps=%d app=%d(%s) pin1=%d remain=%d present_infer=%s\n",
           tag, g_card, g_apps, g_app, app_name(g_app), g_pin, g_remain,
           present_infer(g_app));
}
static int exchange(int ipc, int rfs, uint8_t *req, int reqlen, unsigned want_id,
                    int32_t want_tok, uint8_t *resp, int *out_n) {
    if (write(ipc, req, reqlen) != reqlen) return -1;
    uint8_t buf[8192];
    size_t used = 0;
    int64_t end = now_ms() + 15000;
    while (now_ms() < end) {
        struct pollfd pfd[2] = {{ipc, POLLIN, 0}, {rfs, POLLIN, 0}};
        poll(pfd, 2, 200);
        if (pfd[1].revents & POLLIN) service_rfs(rfs);
        if (pfd[0].revents & POLLIN && used < sizeof buf) {
            ssize_t n = read(ipc, buf + used, sizeof buf - used);
            if (n > 0) used += (size_t)n;
        }
        size_t off = 0;
        while (off < used) {
            int fs = frame_size(buf + off, used - off);
            if (fs < 0) {
                off++;
                continue;
            }
            if (fs == 0) break;
            unsigned fid = le16(buf + off + 2);
            if (fid == 0x0200) note_sim(buf + off, fs);
            if (buf[off] == 1 && fid == want_id) {
                int32_t tok = (int32_t)(buf[off + 6] | (buf[off + 7] << 8) |
                                        (buf[off + 8] << 16) | (buf[off + 9] << 24));
                if (want_tok < 0 || tok == want_tok) {
                    if (fs > 512) fs = 512;
                    memcpy(resp, buf + off, (size_t)fs);
                    *out_n = fs;
                    return 0;
                }
            }
            off += (size_t)fs;
        }
        if (off > 0 && off < used) memmove(buf, buf + off, used - off);
        if (off > 0) used -= off;
        if (off >= used) used = 0;
    }
    return 1;
}
static void fill_empty(uint8_t *req, unsigned id, uint32_t tok) {
    memset(req, 0, 12);
    req[2] = (uint8_t)(id & 0xff);
    req[3] = (uint8_t)((id >> 8) & 0xff);
    req[4] = 12;
    req[6] = (uint8_t)(tok & 0xff);
    req[7] = (uint8_t)((tok >> 8) & 0xff);
    req[8] = (uint8_t)((tok >> 16) & 0xff);
    req[9] = (uint8_t)((tok >> 24) & 0xff);
}
static void print_rmnet(void) {
    for (int i = 0; i < 4; i++) {
        char path[64];
        unsigned long rx = 0, tx = 0;
        snprintf(path, sizeof path, "/sys/class/net/rmnet%d/statistics/rx_bytes", i);
        FILE *f = fopen(path, "r");
        if (f) {
            fscanf(f, "%lu", &rx);
            fclose(f);
        }
        snprintf(path, sizeof path, "/sys/class/net/rmnet%d/statistics/tx_bytes", i);
        f = fopen(path, "r");
        if (f) {
            fscanf(f, "%lu", &tx);
            fclose(f);
        }
        printf("rmnet%d rx=%lu tx=%lu\n", i, rx, tx);
    }
    FILE *p = popen("ip -4 -o addr show 2>/dev/null | grep -E 'rmnet|ccmni' || echo no_rmnet_ipv4", "r");
    if (p) {
        char line[256];
        while (fgets(line, sizeof line, p)) fputs(line, stdout);
        pclose(p);
    }
}

int main(void) {
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1 || strcmp(state, "ONLINE")) {
        puts("requires ONLINE");
        return 1;
    }
    fclose(f);

    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        puts("lock busy");
        return 1;
    }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) {
        puts("open fail");
        return 1;
    }

    uint8_t req[16], resp[512];
    int n = 0;

    puts("EXP stock-missing-empty-GETs: 0x0245 + 0x0930 (never live on soft-lock path)");
    puts("baseline SIM");
    fill_empty(req, 0x0200, 1);
    if (exchange(ipc, rfs, req, 12, 0x0200, 1, resp, &n))
        puts("SIM baseline timeout");
    else {
        printf("SIM length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
        print_sim("pre");
    }

    /* radio state GET — stock step 1 */
    fill_empty(req, 0x0801, 2);
    if (exchange(ipc, rfs, req, 12, 0x0801, 2, resp, &n))
        puts("radio-state timeout");
    else
        printf("radio 0x0801 length=%d error=%u state=%u\n", n, n > 10 ? resp[10] : 0,
               n >= 13 ? resp[12] : 0);

    /* preferred GET — stock */
    fill_empty(req, 0x070b, 3);
    if (exchange(ipc, rfs, req, 12, 0x070b, 3, resp, &n))
        puts("pref-get timeout");
    else
        printf("pref 0x070b length=%d error=%u preferred_raw=%u\n", n, n > 10 ? resp[10] : 0,
               n >= 13 ? resp[12] : 0);

    /* MISSING #1: BuildSim3GPbCapa */
    fill_empty(req, 0x0245, 10);
    puts("send BuildSim3GPbCapa 0x0245 len12");
    if (exchange(ipc, rfs, req, 12, 0x0245, 10, resp, &n))
        puts("0x0245 timeout");
    else
        printf("0x0245 length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);

    /* MISSING #2: BuildGetPreferredCallCapability */
    fill_empty(req, 0x0930, 11);
    puts("send BuildGetPreferredCallCapability 0x0930 len12");
    if (exchange(ipc, rfs, req, 12, 0x0930, 11, resp, &n))
        puts("0x0930 timeout");
    else
        printf("0x0930 length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);

    usleep(500000);
    puts("post SIM");
    fill_empty(req, 0x0200, 20);
    if (exchange(ipc, rfs, req, 12, 0x0200, 20, resp, &n))
        puts("SIM post timeout");
    else {
        printf("SIM length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
        print_sim("post");
    }

    print_rmnet();
    printf("RESULT ready=%d app=%d pin1=%d present_infer=%s\n",
           g_app == 5, g_app, g_pin, present_infer(g_app));
    close(ipc);
    close(rfs);
    close(lock);
    return 0;
}
