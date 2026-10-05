/* ONE post-VerifyPin soft path: OpenChannelWithP2 + channel SELECT + SIM_IO STATUS+AID.
   Factory:
     BuildSimOpenChannelWithP2 0x0247 len 30: aid_len@12 aid@13..28 p2@29
     BuildSimTransmitApduChannel 0x020f base 38: session@12 CLA@16 INS@20 P1@24 P2@28 P3@32 data_len@36 data@38
     BuildSimIO 0x0208 len 0x23c: cmd@12 ... aid_len@555 aid@556 (STATUS 0xF2 accepted; A4 not in builder switch)
   AID taken from GetSimStatus only; never printed. No POWER_OFF/crash/cbd/rild/EFS. */
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
static void put_le16(uint8_t *p, unsigned v) { p[0] = (uint8_t)v; p[1] = (uint8_t)(v >> 8); }
static void put_le32(uint8_t *p, uint32_t v) {
    put_le16(p, (unsigned)(v & 0xffff));
    put_le16(p + 2, (unsigned)(v >> 16));
}
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
        if (write(fd, reply, sizeof reply) < 0)
            puts("rfs unprotect write fail");
        else
            puts("rfs unprotect reply");
        return;
    }
    puts("rfs ignored");
}

static int g_app = -1, g_pin = -1, g_remain = -1, g_card = -1, g_apps = -1;
static int g_aid_len = 0;
static uint8_t g_aid[16];
static int g_session = -1;

static void note_sim(const uint8_t *b, int len) {
    if (len < 75 || b[10]) return;
    g_card = b[12];
    g_apps = b[14];
    g_app = b[17];
    g_pin = b[72];
    g_remain = b[74];
    g_aid_len = 0;
    memset(g_aid, 0, sizeof g_aid);
    if (len > 18) {
        unsigned n = b[18];
        if (n > 16) n = 16;
        if (19 + (int)n <= len) {
            g_aid_len = (int)n;
            if (n) memcpy(g_aid, b + 19, n);
        }
    }
    printf("SIM card=%d apps=%d app=%d pin1=%d remain=%d aid_len=%d len=%d\n",
           g_card, g_apps, g_app, g_pin, g_remain, g_aid_len, len);
}

static int exchange(int ipc, int rfs, const uint8_t *req, size_t req_len,
                    unsigned id, uint32_t tok, int timeout_ms,
                    uint8_t *resp, int *resp_len) {
    ssize_t wr = write(ipc, req, req_len);
    if (wr != (ssize_t)req_len) {
        printf("write fail id=0x%04x wr=%zd errno=%d\n", id, wr, errno);
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
                if (id == 0x0247 && len >= 18 && !buf[off + 10]) {
                    /* Adapter: session u32@12, SW1@16 SW2@17 */
                    g_session = (int)le32(buf + off + 12);
                    printf("open session=%d sw=%02x%02x\n",
                           g_session, buf[off + 16], buf[off + 17]);
                }
                if (id == 0x020f && len >= 14 && !buf[off + 10]) {
                    printf("ch-apdu sw=%02x%02x apdu_hint=%u\n",
                           buf[off + 12], buf[off + 13],
                           len >= 16 ? le16(buf + off + 14) : 0);
                }
                if (id == 0x0208 && len >= 14 && !buf[off + 10]) {
                    printf("simio sw=%02x%02x body_hint=%d\n",
                           buf[off + 12], buf[off + 13], len);
                }
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

static int open_channel_p2(int ipc, int rfs, uint32_t tok, int p2) {
    uint8_t req[30];
    uint8_t resp[512];
    int n = 0;
    memset(req, 0, sizeof req);
    req[2] = 0x47;
    req[3] = 0x02;
    req[4] = 30;
    req[6] = (uint8_t)tok;
    if (g_aid_len <= 0 || g_aid_len > 16) {
        puts("open needs aid");
        return -1;
    }
    req[12] = (uint8_t)g_aid_len;
    memcpy(req + 13, g_aid, (size_t)g_aid_len);
    req[29] = (uint8_t)p2;
    printf("send OpenChannelWithP2 0x0247 len=30 tok=%u p2=%d aid_len=%d\n",
           tok, p2, g_aid_len);
    return exchange(ipc, rfs, req, sizeof req, 0x0247, tok, 20000, resp, &n);
}

/* Channel SELECT by DF name (USIM ADF). Open already SELECTs; stock may re-SELECT. */
static int ch_select_adf(int ipc, int rfs, uint32_t tok, int session) {
    size_t total = 38 + (size_t)g_aid_len;
    uint8_t req[38 + 16];
    uint8_t resp[512];
    int n = 0;
    if (g_aid_len <= 0 || g_aid_len > 16) return -1;
    if (total > sizeof req) return -1;
    memset(req, 0, total);
    req[2] = 0x0f;
    req[3] = 0x02;
    put_le16(req + 4, (unsigned)total);
    req[6] = (uint8_t)tok;
    put_le32(req + 12, (uint32_t)session);
    put_le32(req + 16, 0x00); /* CLA — handler may OR channel */
    put_le32(req + 20, 0xA4); /* SELECT */
    put_le32(req + 24, 0x04); /* by DF name */
    put_le32(req + 28, 0x00); /* return FCI */
    put_le32(req + 32, (uint32_t)g_aid_len);
    put_le16(req + 36, (unsigned)g_aid_len);
    memcpy(req + 38, g_aid, (size_t)g_aid_len);
    printf("send TransmitApduChannel 0x020f SELECT tok=%u session=%d aid_len=%d len=%zu\n",
           tok, session, g_aid_len, total);
    return exchange(ipc, rfs, req, total, 0x020f, tok, 20000, resp, &n);
}

/* SIM_IO STATUS with AID filled (empty defaults already live-negated). */
static int sim_io_status_aid(int ipc, int rfs, uint32_t tok) {
    uint8_t req[0x23c];
    uint8_t resp[512];
    int n = 0;
    if (g_aid_len <= 0 || g_aid_len > 16) return -1;
    memset(req, 0, sizeof req);
    req[2] = 0x08;
    req[3] = 0x02;
    req[4] = 0x3c;
    req[5] = 0x02;
    req[6] = (uint8_t)tok;
    req[12] = 0xF2; /* STATUS — accepted by BuildSimIO switch */
    req[555] = (uint8_t)g_aid_len;
    memcpy(req + 556, g_aid, (size_t)g_aid_len);
    printf("send SIM_IO STATUS 0x0208 len=0x23c tok=%u aid_len=%d (AID filled)\n",
           tok, g_aid_len);
    return exchange(ipc, rfs, req, sizeof req, 0x0208, tok, 20000, resp, &n);
}

static int close_channel(int ipc, int rfs, uint32_t tok, int session) {
    uint8_t req[16];
    uint8_t resp[128];
    int n = 0;
    memset(req, 0, sizeof req);
    req[2] = 0x0e;
    req[3] = 0x02;
    req[4] = 16;
    req[6] = (uint8_t)tok;
    put_le32(req + 12, (uint32_t)session);
    printf("send CloseChannel 0x020e tok=%u session=%d\n", tok, session);
    return exchange(ipc, rfs, req, sizeof req, 0x020e, tok, 12000, resp, &n);
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

    puts("SEQ opcodes: 0x0200 -> 0x0247 -> 0x020f(SELECT) -> 0x0208(STATUS+AID) -> 0x0200");
    puts("PRE");
    if (get_status(ipc, rfs, 101) != 0) return 2;
    int pre_app = g_app, pre_pin = g_pin;
    if (g_aid_len <= 0) {
        puts("no AID in status — abort");
        return 3;
    }

    if (open_channel_p2(ipc, rfs, 102, 0) != 0) {
        puts("OpenChannel failed");
        get_status(ipc, rfs, 109);
        return 4;
    }
    int session = g_session >= 0 ? g_session : 1;

    if (ch_select_adf(ipc, rfs, 103, session) != 0)
        puts("channel SELECT exchange failed (continuing)");

    if (sim_io_status_aid(ipc, rfs, 104) != 0)
        puts("SIM_IO STATUS+AID exchange failed (continuing)");

    usleep(400000);
    puts("POST");
    if (get_status(ipc, rfs, 105) != 0) return 5;

    if (session > 0)
        (void)close_channel(ipc, rfs, 106, session);

    printf("delta pre_app=%d pre_pin=%d -> app=%d pin1=%d remain=%d\n",
           pre_app, pre_pin, g_app, g_pin, g_remain);
    if (g_app == 1 || g_app == 4 || g_app == 5)
        puts("EDGE_READY_CLASS");
    else
        puts("STILL_NOT_READY");

    memset(g_aid, 0, sizeof g_aid);
    close(ipc);
    close(rfs);
    close(lock);
    return 0;
}
