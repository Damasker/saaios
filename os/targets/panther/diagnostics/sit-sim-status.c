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
#include <signal.h>

static unsigned le16(const uint8_t *p) { return p[0] | ((unsigned)p[1] << 8); }
static uint32_t le32(const uint8_t *p) { return le16(p) | ((uint32_t)le16(p+2) << 16); }
static int64_t now_ms(void) {
    struct timespec t;
    if (clock_gettime(CLOCK_MONOTONIC, &t)) return -1;
    return (int64_t)t.tv_sec*1000 + t.tv_nsec/1000000;
}

/* 0=incomplete, -1=malformed, positive=one complete frame size. */
/* Factory covertCardStateToString: 0 ABSENT, 1 PRESENT, 2 ERROR, 3 RESTRICTED.
   ProtocolSimStatusAdapter Init rewrites wire byte 3 to stored 1 and wire byte 4 to stored 3. */
static unsigned stored_card_state(unsigned raw) {
    if (raw == 3) return 1;
    if (raw == 4) return 3;
    return raw;
}
static const char *card_state_name(unsigned stored) {
    switch (stored) {
    case 0: return "ABSENT";
    case 1: return "PRESENT";
    case 2: return "ERROR";
    case 3: return "RESTRICTED";
    default: return "unknown";
    }
}
/* Factory BuildRilCardStatusApplications: types 1 and 2 share the GSM/UMTS slot,
   3 and 4 the RUIM/CSIM slot, 5 the ISIM slot. */
static const char *app_class_name(unsigned type) {
    if (type == 1 || type == 2) return "SIM/USIM";
    if (type == 3 || type == 4) return "RUIM/CSIM";
    if (type == 5) return "ISIM";
    return "unknown";
}
/* Factory covertAppStateToString. */
static const char *app_state_name(unsigned state) {
    switch (state) {
    case 0: return "UNKNOWN";
    case 1: return "DETECTED";
    case 2: return "PIN";
    case 3: return "PUK";
    case 4: return "SUBSCRIPTION_PERSO";
    case 5: return "READY";
    default: return "unknown";
    }
}

/* Infer Present enum published by last STATUS->SET_APP decision (RO MAIN):
   Present 1->PUK(#3), 2->READY(#5), 3->PERSO(#4), else (incl 0)->PIN(#2).
   DETECTED(#1) has no Present gate. 0x0200 byte17 = GET_APP +0xBF4 only —
   does NOT carry +0xBF6. Validated 2026-09-30 (not a mis-read of Present).
   Caveat: while app==PIN, STATUS may skip Present re-eval; label is last
   published decision. HotSwap live: PIN + notin_1_2_3 (Present was 0/else). */
static const char *present_infer_from_app(int apps, int app) {
    if (apps <= 0 || app < 0) return "n/a(no-app)";
    switch (app) {
    case 5: return "was_2";
    case 3: return "was_1";
    case 4: return "was_3";
    case 2: return "notin_1_2_3";
    case 1: return "n/a(DETECTED)";
    case 0: return "n/a(UNKNOWN)";
    default: return "n/a";
    }
}
/* sit-base toRadioTech: values 1..16 pass through; 14 and 20 both become RIL LTE; 21 becomes NR.
   Pass-through 3 is AOSP RADIO_TECH_UMTS. */
static const char *radio_tech_name(unsigned sit) {
    if (sit == 14 || sit == 20) return "LTE";
    if (sit == 21) return "NR";
    if (sit == 3) return "UMTS";
    if (sit == 0) return "none";
    return NULL;
}

/* Factory BuildSimVerifyPin with the first argument 0: id 0x0201, length 38.
   DoVerifyPin uses that path for PIN1 and passes a null AID, so byte 21 and
   the following 16 bytes stay zero. Byte 12 is the character count. The
   characters start at byte 13 and are capped at 8. Only attempt indexes
   1 and 2 are built, and they are not stored as text. */
static int fill_verify_pin(uint8_t *req, uint32_t token, int which) {
    if (which != 1 && which != 2) return -1;
    memset(req, 0, 38);
    req[2] = 0x01;
    req[3] = 0x02;
    req[4] = 38;
    req[6] = (uint8_t)token;
    req[12] = 4;
    uint8_t digit = which == 1 ? '0' : '1';
    for (int i = 0; i < 4; i++) req[13 + i] = digit;
    return 38;
}

/* BuildSimVerifyPin writes AID only when its hex-string argument is non-null.
   HexString2Value stores the byte count at packet offset 21 and the bytes at
   offset 22, at most 16. GetAID's length is status byte 18 and its bytes
   follow at byte 19. Those bytes stay in this buffer and are never printed. */
static int fill_verify_pin_aid(uint8_t *req, uint32_t token, int which,
                               const uint8_t *aid, unsigned aid_len) {
    if (fill_verify_pin(req, token, which) != 38) return -1;
    if (!aid || aid_len == 0 || aid_len > 16) return -1;
    req[21] = (uint8_t)aid_len;
    memcpy(req + 22, aid, aid_len);
    return 38;
}

static int frame_size(const uint8_t *p, size_t n) {
    if (n < 6) return 0;
    if (p[0] > 2) return -1;
    unsigned min = p[0] == 2 ? 8 : 12;
    unsigned len = le16(p+4);
    if (len < min) return -1;
    return n < len ? 0 : (int)len;
}

static int open_node(const char *node, const char *sysdev) {
    unsigned maj = 0, min = 0;
    FILE *f = fopen(sysdev, "r");
    if (!f) return -1;
    int got = fscanf(f, "%u:%u", &maj, &min);
    fclose(f);
    if (got != 2) return -1;
    int fd = open(node, O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0 && errno == ENOENT) {
        if (mknod(node, S_IFCHR | 0600, makedev(maj, min)) && errno != EEXIST)
            return -1;
        fd = open(node, O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    }
    struct stat st;
    if (fd < 0 || fstat(fd, &st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev) != maj || minor(st.st_rdev) != min) {
        if (fd >= 0) close(fd);
        return -1;
    }
    return fd;
}

static int g_app = -1, g_pin = -1, g_remain = -1, g_pref = -1, g_ready = 0, g_aid_len = -1, g_card = -1, g_apps = -1;
static uint8_t g_aid[16];
static int g_quiet_sim = 0;

static void note_sim(const uint8_t *b, int len) {
    if (len < 15 || b[10]) {
        if (!g_quiet_sim) printf("SIM length=%d error_raw=%u\n", len, len > 10 ? b[10] : 0);
        return;
    }
    unsigned apps = b[14];
    int type = -1, state = -1, pin1 = -1, remain = -1;
    g_card = (int)stored_card_state(b[12]);
    g_apps = (int)apps;
    g_aid_len = 0;
    memset(g_aid, 0, sizeof g_aid);
    if (apps >= 1 && len >= 18) {
        type = b[15];
        state = b[17];
        g_app = state;
        if (state == 5) g_ready = 1;
    } else if (apps == 0) {
        g_app = -1;
        g_pin = -1;
    }
    if (apps >= 1 && len >= 19) {
        unsigned n = b[18];
        if (n > 16) n = 16;
        if ((unsigned)len >= 19u + n) {
            g_aid_len = (int)n;
            if (n) memcpy(g_aid, b + 19, n);
        }
    }
    if (apps >= 1 && len >= 75) {
        pin1 = b[72];
        remain = b[74];
        g_pin = pin1;
        g_remain = remain;
    }
    if (!g_quiet_sim)
        printf("SIM length=%d error_raw=0 card_state_raw=%u applications=%u app0_type_raw=%d app0_state_raw=%d app0_state=%s pin1_state_raw=%d pin1_remain_raw=%d\n",
            len, b[12], apps, type, state, app_state_name((unsigned)state), pin1, remain);
}

static void service_rfs(int fd) {
    uint8_t buf[256];
    ssize_t n = read(fd, buf, sizeof buf);
    if (n <= 0) return;
    if (n == (ssize_t)sizeof buf) {
        uint8_t dump[256];
        while (read(fd, dump, sizeof dump) > 0) {}
        puts("rfs large request drained no reply");
        return;
    }
    printf("rfs bytes=%zd cmd=%u\n", n, buf[0]);
    if (n == 12 && buf[0] == 0x07 && !buf[1] && !buf[2] && !buf[3] &&
        buf[4] == 0x04 && !buf[5] && !buf[6] && !buf[7] &&
        buf[8] == 0x03 && !buf[9] && !buf[10] && !buf[11]) {
        static const uint8_t reply[16] = {
            0x03,0,0,0, 0x08,0,0,0, 0,0,0,0, 0x03,0,0,0
        };
        ssize_t wr = write(fd, reply, sizeof reply);
        printf("rfs unprotect reply bytes=%zd\n", wr);
        return;
    }
    if (n == 20 && buf[0] == 0x03 && le32(buf + 4) == 12) { puts("rfs op_status no reply"); return; }
    if (n >= 24 && buf[0] == 0x06) {
        puts(le32(buf + 20) == 2 ? "rfs write op=2 refused no reply" : "rfs io no reply");
        return;
    }
    puts("rfs ignored no reply");
}

static int exchange(int ipc, int rfs, const uint8_t *req, size_t req_len,
                    unsigned id, uint32_t token, uint8_t *resp, int *resp_len) {
    ssize_t wr = write(ipc, req, req_len);
    if (wr != (ssize_t)req_len) return -1;
    uint8_t buf[4096];
    size_t used = 0;
    int64_t end = now_ms() + 10000;
    while (now_ms() < end) {
        struct pollfd pfd[2] = { { ipc, POLLIN, 0 }, { rfs, POLLIN, 0 } };
        int ready = poll(pfd, 2, 200);
        if (ready < 0) { if (errno == EINTR) continue; return -1; }
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
            if (fid == id && ftok == token) {
                int keep = len > 512 ? 512 : len;
                memcpy(resp, buf + off, (size_t)keep);
                *resp_len = keep;
                return 0;
            }
            if (fid != 0x0200 && !g_quiet_sim) printf("ipc id=%u length=%d\n", fid, len);
            off += (size_t)len;
        }
        if (off) { memmove(buf, buf + off, used - off); used -= off; }
    }
    return 1;
}

static void follow_ready(int ipc, int rfs) {
    uint8_t resp[512];
    int n = 0;
    uint8_t req[16];
    memset(req, 0, sizeof req);
    req[2] = 0x0b; req[3] = 0x07; req[4] = 12; req[6] = 50;
    puts("preferred query");
    if (!exchange(ipc, rfs, req, 12, 0x070b, 50, resp, &n) && n >= 16 && !resp[10]) {
        g_pref = (int)le32(resp + 12);
        printf("preferred_raw=%u\n", (unsigned)g_pref);
    }
    if (g_pref >= 0 && g_pref != 11) {
        memset(req, 0, sizeof req);
        req[2] = 0x0a; req[3] = 0x07; req[4] = 16; req[6] = 51; req[12] = 11;
        puts("preferred set LTE_ONLY");
        exchange(ipc, rfs, req, 16, 0x070a, 51, resp, &n);
    }
    memset(req, 0, sizeof req);
    req[2] = 0x04; req[3] = 0x07; req[4] = 12; req[6] = 52;
    puts("network selection auto");
    if (!exchange(ipc, rfs, req, 12, 0x0704, 52, resp, &n))
        printf("selection length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    for (int i = 0; i < 4; i++) {
        memset(req, 0, sizeof req);
        req[2] = 0x01; req[3] = 0x07; req[4] = 12; req[6] = (uint8_t)(60 + i);
        puts("registration query");
        if (!exchange(ipc, rfs, req, 12, 0x0701, 60 + (uint32_t)i, resp, &n) && n >= 16 && !resp[10])
            printf("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u\n", resp[12], resp[13], resp[15]);
        if (i != 3) {
            int64_t until = now_ms() + 15000;
            while (now_ms() < until) {
                struct pollfd pfd[2] = { { ipc, POLLIN, 0 }, { rfs, POLLIN, 0 } };
                poll(pfd, 2, 500);
                if (pfd[1].revents & POLLIN) service_rfs(rfs);
            }
        }
    }
    unsigned long long rx = 0, tx = 0;
    FILE *f = fopen("/sys/class/net/rmnet0/statistics/rx_bytes", "r");
    if (f) { if (fscanf(f, "%llu", &rx) != 1) rx = 0; fclose(f); }
    f = fopen("/sys/class/net/rmnet0/statistics/tx_bytes", "r");
    if (f) { if (fscanf(f, "%llu", &tx) != 1) tx = 0; fclose(f); }
    printf("rmnet_rx=%llu rmnet_tx=%llu\n", rx, tx);
}

static int send_one_verify(int ipc, int rfs, int which, uint32_t token) {
    uint8_t req[38], resp[512];
    int n = 0;
    if (fill_verify_pin_aid(req, token, which, g_aid, (unsigned)g_aid_len) != 38) return -1;
    printf("PIN verify attempt=%d\n", which);
    int rc = exchange(ipc, rfs, req, 38, 0x0201, token, resp, &n);
    memset(req, 0, sizeof req);
    if (rc) { puts("PIN verify timeout"); return rc; }
    unsigned err = n > 10 ? resp[10] : 0;
    unsigned remain = n >= 16 ? le32(resp + 12) : 0;
    printf("PIN verify response: length=%d error_raw=%u remain_raw=%u\n", n, err, remain);
    if (n >= 16 && remain <= 1) g_remain = (int)remain;
    return 0;
}

static int run_verify_with_aid(void) {
    alarm(120);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[12], resp[512];
    int n = 0;
    memset(req, 0, sizeof req);
    req[2] = 0x00; req[3] = 0x02; req[4] = 12; req[6] = 1;
    puts("SIM query before verify");
    if (exchange(ipc, rfs, req, 12, 0x0200, 1, resp, &n)) { puts("SIM query timeout"); return 1; }
    if (g_ready) { follow_ready(ipc, rfs); return 0; }
    if (g_app == 3 || (g_remain >= 0 && g_remain <= 1)) {
        puts("verify not sent");
        return 2;
    }
    if (g_aid_len <= 0) {
        puts("aid_len=0; verify not sent");
        return 2;
    }
    send_one_verify(ipc, rfs, 1, 7);
    memset(req, 0, sizeof req);
    req[2] = 0x00; req[3] = 0x02; req[4] = 12; req[6] = 2;
    puts("SIM query after first");
    exchange(ipc, rfs, req, 12, 0x0200, 2, resp, &n);
    if (!g_ready && g_app == 2 && g_app != 3 && g_remain > 1)
        send_one_verify(ipc, rfs, 2, 8);
    if (!g_ready && g_app == 2) {
        memset(req, 0, sizeof req);
        req[2] = 0x00; req[3] = 0x02; req[4] = 12; req[6] = 3;
        puts("SIM query after second");
        exchange(ipc, rfs, req, 12, 0x0200, 3, resp, &n);
    }
    memset(g_aid, 0, sizeof g_aid);
    if (g_ready) follow_ready(ipc, rfs);
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return g_ready ? 0 : 2;
}

/* BuildSetSimCardPower: id 0x024c, length 13, state byte at offset 12.
   HAL >= 0x16 rewrites request 0 to 4 before the builder (POWER_DOWN).
   Value 1 is POWER_UP. Value 2 is PASS_THROUGH. */
static int fill_card_power(uint8_t *req, uint32_t token, uint8_t state) {
    if (state != 1 && state != 2 && state != 4) return -1;
    memset(req, 0, 13);
    req[2] = 0x4c;
    req[3] = 0x02;
    req[4] = 13;
    req[6] = (uint8_t)token;
    req[12] = state;
    return 13;
}

static void wait_ms(int ipc, int rfs, int ms) {
    int64_t until = now_ms() + ms;
    while (now_ms() < until) {
        struct pollfd pfd[2] = { { ipc, POLLIN, 0 }, { rfs, POLLIN, 0 } };
        int left = (int)(until - now_ms());
        if (left < 1) break;
        if (left > 500) left = 500;
        if (poll(pfd, 2, left) < 0 && errno != EINTR) return;
        if (pfd[1].revents & POLLIN) service_rfs(rfs);
        if (pfd[0].revents & POLLIN) {
            uint8_t dump[512];
            if (read(ipc, dump, sizeof dump) < 0 && errno != EAGAIN && errno != EINTR) return;
        }
    }
}

static void print_rmnet(void) {
    unsigned long long rx = 0, tx = 0;
    FILE *f = fopen("/sys/class/net/rmnet0/statistics/rx_bytes", "r");
    if (f) { if (fscanf(f, "%llu", &rx) != 1) rx = 0; fclose(f); }
    f = fopen("/sys/class/net/rmnet0/statistics/tx_bytes", "r");
    if (f) { if (fscanf(f, "%llu", &tx) != 1) tx = 0; fclose(f); }
    printf("rmnet_rx=%llu rmnet_tx=%llu\n", rx, tx);
}

static int query_sim(int ipc, int rfs, uint32_t token) {
    uint8_t req[12], resp[512];
    int n = 0;
    memset(req, 0, sizeof req);
    req[2] = 0x00; req[3] = 0x02; req[4] = 12; req[6] = (uint8_t)token;
    if (!g_quiet_sim) puts("SIM query");
    return exchange(ipc, rfs, req, 12, 0x0200, token, resp, &n);
}

static int send_card_power(int ipc, int rfs, uint8_t state, uint32_t token) {
    uint8_t req[13], resp[512];
    int n = 0;
    if (fill_card_power(req, token, state) != 13) return -1;
    printf("card power state=%u\n", state);
    int rc = exchange(ipc, rfs, req, 13, 0x024c, token, resp, &n);
    if (rc) { puts("card power timeout"); return rc; }
    printf("card power response: length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    return 0;
}

static int run_card_power_up(void) {
    alarm(180);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    if (send_card_power(ipc, rfs, 1, 70)) { close(ipc); close(rfs); close(lock); return 1; }
    puts("wait after card power");
    wait_ms(ipc, rfs, 4000);
    if (query_sim(ipc, rfs, 71)) puts("SIM query timeout");
    if (!g_ready && g_card == 0) {
        puts("card ABSENT after up; one pass-through");
        if (!send_card_power(ipc, rfs, 2, 72)) {
            wait_ms(ipc, rfs, 4000);
            if (query_sim(ipc, rfs, 73)) puts("SIM query timeout");
        }
    }
    if (g_ready) follow_ready(ipc, rfs);
    print_rmnet();
    printf("RESULT card=%d app_state=%d pin1=%d ready=%d\n", g_card, g_app, g_pin, g_ready);
    close(ipc); close(rfs); close(lock);
    return g_ready ? 0 : 2;
}

/* Empty GETs from the factory radio-up handlers. Signal is 0x0900 length
   12. Baseband is 0x0901 length 13 and the handler passes 0xFF. TTY is
   0x0904 length 12; the mode word is at response offset 12. */
static int fill_empty_get(uint8_t *req, unsigned id, uint32_t token, size_t len) {
    if (len != 12 && len != 13) return -1;
    memset(req, 0, len);
    req[2] = (uint8_t)id;
    req[3] = (uint8_t)(id >> 8);
    req[4] = (uint8_t)len;
    req[6] = (uint8_t)token;
    return (int)len;
}

static int send_named_get(int ipc, int rfs, const char *name, unsigned id,
                          uint32_t token, size_t len, uint8_t extra) {
    uint8_t req[13], resp[512];
    int n = 0;
    if (fill_empty_get(req, id, token, len) != (int)len) return -1;
    if (len == 13) req[12] = extra;
    printf("%s\n", name);
    int rc = exchange(ipc, rfs, req, len, id, token, resp, &n);
    memset(req, 0, sizeof req);
    if (rc) { printf("%s timeout\n", name); return rc; }
    printf("%s response: length=%d error_raw=%u\n", name, n, n > 10 ? resp[10] : 0);
    if (id == 0x0904 && n >= 16 && !resp[10])
        printf("tty_mode_raw=%u\n", le32(resp + 12));
    return 0;
}

static int run_stock_gets(void) {
    alarm(90);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    /* Signal 0x0900 was already sent once on this boot. */
    send_named_get(ipc, rfs, "baseband", 0x0901, 81, 13, 0xff);
    send_named_get(ipc, rfs, "tty", 0x0904, 82, 12, 0);
    query_sim(ipc, rfs, 83);
    {
        uint8_t req[12], resp[512];
        int n = 0;
        memset(req, 0, sizeof req);
        req[2] = 0x01; req[3] = 0x07; req[4] = 12; req[6] = 84;
        puts("registration query");
        if (!exchange(ipc, rfs, req, 12, 0x0701, 84, resp, &n) && n >= 16 && !resp[10])
            printf("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u\n",
                resp[12], resp[13], resp[15]);
        else
            printf("registration length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    }
    print_rmnet();
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* BuildSetModemsConfig: id 0x093f, length 13. Byte 12 is 0 when the
   argument is 1 and 1 otherwise. Stock phone count is 2 when
   persist.radio.multisim.config is dsds, and the send is allowed when
   persist.vendor.radio.multisim_switch_support is true. */
static int fill_modems_config(uint8_t *req, uint32_t token) {
    memset(req, 0, 13);
    req[2] = 0x3f;
    req[3] = 0x09;
    req[4] = 13;
    req[6] = (uint8_t)token;
    req[12] = 1;
    return 13;
}

static int run_set_modems_config(void) {
    alarm(40);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[13], resp[512];
    int n = 0;
    fill_modems_config(req, 90);
    puts("modems config");
    int rc = exchange(ipc, rfs, req, 13, 0x093f, 90, resp, &n);
    memset(req, 0, sizeof req);
    if (rc) puts("modems config timeout");
    else printf("modems config response: length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    if (query_sim(ipc, rfs, 91)) puts("SIM query timeout");
    {
        uint8_t reg[12];
        n = 0;
        memset(reg, 0, sizeof reg);
        reg[2] = 0x01; reg[3] = 0x07; reg[4] = 12; reg[6] = 92;
        puts("registration query");
        if (!exchange(ipc, rfs, reg, 12, 0x0701, 92, resp, &n) && n >= 16 && !resp[10])
            printf("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u\n",
                resp[12], resp[13], resp[15]);
        else
            printf("registration length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    }
    print_rmnet();
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* BuildSetApSystemTime: id 0x0949, length 18. Bytes 12..17 are the low
   bytes of tm_year, tm_mon, tm_mday, tm_hour, tm_min, tm_sec. */
static int fill_ap_time(uint8_t *req, uint32_t token) {
    time_t now = time(NULL);
    struct tm *tm = localtime(&now);
    if (!tm) return -1;
    memset(req, 0, 18);
    req[2] = 0x49;
    req[3] = 0x09;
    req[4] = 18;
    req[6] = (uint8_t)token;
    req[12] = (uint8_t)tm->tm_year;
    req[13] = (uint8_t)tm->tm_mon;
    req[14] = (uint8_t)tm->tm_mday;
    req[15] = (uint8_t)tm->tm_hour;
    req[16] = (uint8_t)tm->tm_min;
    req[17] = (uint8_t)tm->tm_sec;
    return 18;
}

/* SendDeviceInfo: id 0x0922, length 140. Four 32-byte fields at offsets
   12, 44, 76 and 108. Each is strncpy of at most 31 bytes. */
static int fill_device_info(uint8_t *req, uint32_t token,
                            const char *a, const char *b, const char *c, const char *d) {
    if (!a || !b || !c || !d) return -1;
    memset(req, 0, 140);
    req[2] = 0x22;
    req[3] = 0x09;
    req[4] = 140;
    req[6] = (uint8_t)token;
    strncpy((char *)req + 12, a, 31);
    strncpy((char *)req + 44, b, 31);
    strncpy((char *)req + 76, c, 31);
    strncpy((char *)req + 108, d, 31);
    return 140;
}

static int read_prop(const char *key, char *out, size_t cap) {
    char cmd[96];
    if (cap < 2) return -1;
    out[0] = 0;
    if (snprintf(cmd, sizeof cmd, "getprop %s", key) >= (int)sizeof cmd) return -1;
    FILE *f = popen(cmd, "r");
    if (!f) return -1;
    if (!fgets(out, (int)cap, f)) out[0] = 0;
    int rc = pclose(f);
    size_t n = strlen(out);
    if (n && out[n - 1] == '\n') out[n - 1] = 0;
    return rc == 0 ? 0 : -1;
}

static int run_verify_after_config(void) {
    alarm(50);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    puts("modems config already sent this boot");
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    if (query_sim(ipc, rfs, 93)) { puts("SIM query timeout"); close(ipc); close(rfs); close(lock); return 1; }
    int verify_err = -1;
    if (g_ready) {
        puts("already READY; verify not sent");
    } else if (g_app == 3 || (g_remain >= 0 && g_remain <= 1)) {
        puts("verify not sent");
    } else {
        uint8_t req[38], resp[512];
        int n = 0;
        int filled = g_aid_len > 0
            ? fill_verify_pin_aid(req, 94, 1, g_aid, (unsigned)g_aid_len)
            : fill_verify_pin(req, 94, 1);
        printf("PIN verify first aid_present=%d\n", g_aid_len > 0 ? 1 : 0);
        if (filled != 38) { puts("verify not sent"); }
        else {
            int rc = exchange(ipc, rfs, req, 38, 0x0201, 94, resp, &n);
            memset(req, 0, sizeof req);
            memset(g_aid, 0, sizeof g_aid);
            if (rc) puts("PIN verify timeout");
            else {
                verify_err = n > 10 ? (int)resp[10] : 0;
                unsigned remain = n >= 16 ? le32(resp + 12) : 0;
                printf("PIN verify response: length=%d error_raw=%u remain_raw=%u\n", n, (unsigned)verify_err, remain);
                if (n >= 16) g_remain = (int)remain;
            }
        }
        if (query_sim(ipc, rfs, 95)) puts("SIM query timeout");
        if (verify_err == 6) puts("second candidate not sent");
        if (g_app == 3 || (g_remain >= 0 && g_remain <= 1)) puts("stop after verify");
    }
    memset(g_aid, 0, sizeof g_aid);
    {
        uint8_t req[18], resp[512];
        int n = 0;
        if (fill_ap_time(req, 96) == 18) {
            puts("ap system time");
            int rc = exchange(ipc, rfs, req, 18, 0x0949, 96, resp, &n);
            memset(req, 0, sizeof req);
            if (rc) puts("ap system time timeout");
            else printf("ap system time response: length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
        }
    }
    {
        char model[40], build_id[40], name[40], release[40];
        uint8_t req[140], resp[512];
        int n = 0;
        if (read_prop("ro.product.model", model, sizeof model) ||
            read_prop("ro.build.id", build_id, sizeof build_id) ||
            read_prop("ro.product.name", name, sizeof name) ||
            read_prop("ro.build.version.release", release, sizeof release)) {
            puts("device info skipped");
        } else if (fill_device_info(req, 97, model, build_id, name, release) == 140) {
            printf("device info props model=%d id=%d name=%d release=%d\n",
                model[0] != 0, build_id[0] != 0, name[0] != 0, release[0] != 0);
            puts("device info");
            int rc = exchange(ipc, rfs, req, 140, 0x0922, 97, resp, &n);
            memset(req, 0, sizeof req);
            memset(model, 0, sizeof model);
            memset(build_id, 0, sizeof build_id);
            memset(name, 0, sizeof name);
            memset(release, 0, sizeof release);
            if (rc) puts("device info timeout");
            else printf("device info response: length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
        }
    }
    if (g_ready) follow_ready(ipc, rfs);
    else {
        uint8_t req[12], resp[512];
        int n = 0;
        memset(req, 0, sizeof req);
        req[2] = 0x01; req[3] = 0x07; req[4] = 12; req[6] = 98;
        puts("registration query");
        if (!exchange(ipc, rfs, req, 12, 0x0701, 98, resp, &n) && n >= 16 && !resp[10])
            printf("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u\n",
                resp[12], resp[13], resp[15]);
        else
            printf("registration length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
        print_rmnet();
    }
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* BuildSimTransmitApduBasic: id 0x020c. Session word at offset 12.
   Halfword at 16 starts at 5 and grows by the decoded data length.
   Bytes 18..22 are CLA, INS, P1, P2, P3. Data begins at offset 23.
   The header length is 23 plus the hex-string length, which is 16
   for eight data bytes. CLA 00 VERIFY CHV is the basic builder, not
   the channel builder 0x020f. Digits are left-justified and the other
   four bytes are 0xFF. Those bytes are not printed. */
static int fill_basic_apdu(uint8_t *req, uint32_t token, uint8_t cla, uint8_t ins,
                           uint8_t p1, uint8_t p2, const uint8_t *data, unsigned dlen) {
    if (dlen > 16) return -1;
    unsigned pkt = 23u + dlen * 2u;
    memset(req, 0, pkt);
    req[2] = 0x0c;
    req[3] = 0x02;
    req[4] = (uint8_t)pkt;
    req[6] = (uint8_t)token;
    req[16] = (uint8_t)(5u + dlen);
    req[18] = cla;
    req[19] = ins;
    req[20] = p1;
    req[21] = p2;
    req[22] = (uint8_t)dlen;
    if (dlen) memcpy(req + 23, data, dlen);
    return (int)pkt;
}

static int fill_transmit_chv_p2(uint8_t *req, uint32_t token, int which, uint8_t p2) {
    if (which != 1 && which != 2) return -1;
    uint8_t body[8];
    uint8_t digit = which == 1 ? '0' : '1';
    for (int i = 0; i < 4; i++) body[i] = digit;
    for (int i = 4; i < 8; i++) body[i] = 0xff;
    int n = fill_basic_apdu(req, token, 0x00, 0x20, 0x00, p2, body, 8);
    memset(body, 0, sizeof body);
    return n;
}

static int fill_transmit_chv(uint8_t *req, uint32_t token, int which) {
    return fill_transmit_chv_p2(req, token, which, 0x01);
}

static int apdu_sw(const uint8_t *b, int n, unsigned *sw) {
    if (n < 16 || le16(b + 2) != 0x020c) return -1;
    unsigned alen = le16(b + 12);
    if (alen < 2 || 14u + alen > (unsigned)n) return -1;
    *sw = ((unsigned)b[14 + alen - 2] << 8) | b[14 + alen - 1];
    return 0;
}

static int send_chv(int ipc, int rfs, int which, uint32_t token, unsigned *sw) {
    uint8_t req[39], resp[512];
    int n = 0;
    *sw = 0;
    if (fill_transmit_chv(req, token, which) != 39) return -1;
    printf("transmit chv which=%d\n", which);
    int rc = exchange(ipc, rfs, req, 39, 0x020c, token, resp, &n);
    memset(req, 0, sizeof req);
    if (rc) { puts("transmit chv timeout"); return -1; }
    int have = apdu_sw(resp, n, sw) == 0;
    if (have)
        printf("transmit chv response: length=%d error_raw=%u sw=%04x\n", n, n > 10 ? resp[10] : 0, *sw);
    else
        printf("transmit chv response: length=%d error_raw=%u sw_present=0\n", n, n > 10 ? resp[10] : 0);
    memset(resp, 0, sizeof resp);
    return have;
}

static int chv_second_ok(int have, unsigned sw) {
    if (g_ready || g_app == 3 || (g_remain >= 0 && g_remain <= 1) || g_app != 2) return 0;
    if (have != 1) return 0;
    if (sw == 0x6983 || sw == 0x6984 || sw == 0x9840 || sw == 0x9000) return 0;
    if ((sw & 0xfff0) == 0x63c0) return (sw & 0x0f) > 1;
    if (sw == 0x9804) return 1;
    return 0;
}

static void one_registration(int ipc, int rfs, uint32_t token) {
    uint8_t req[12], resp[512];
    int n = 0;
    memset(req, 0, sizeof req);
    req[2] = 0x01; req[3] = 0x07; req[4] = 12; req[6] = (uint8_t)token;
    puts("registration query");
    if (!exchange(ipc, rfs, req, 12, 0x0701, token, resp, &n) && n >= 16 && !resp[10])
        printf("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u\n",
            resp[12], resp[13], resp[15]);
    else
        printf("registration length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    print_rmnet();
}

static int run_transmit_chv(void) {
    alarm(45);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    if (query_sim(ipc, rfs, 100)) { puts("SIM query timeout"); close(ipc); close(rfs); close(lock); return 1; }
    unsigned sw = 0;
    int have = 0;
    if (g_ready) puts("already READY; chv not sent");
    else if (g_app == 3 || (g_remain >= 0 && g_remain <= 1)) puts("chv not sent");
    else {
        have = send_chv(ipc, rfs, 1, 101, &sw);
        if (query_sim(ipc, rfs, 102)) puts("SIM query timeout");
        if (chv_second_ok(have, sw)) {
            have = send_chv(ipc, rfs, 2, 103, &sw);
            if (query_sim(ipc, rfs, 104)) puts("SIM query timeout");
        } else if (!g_ready) {
            puts("second chv not sent");
        }
    }
    if (g_ready || (have > 0 && sw == 0x9000)) follow_ready(ipc, rfs);
    else one_registration(ipc, rfs, 105);
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* Extract APDU body (no SW) from TransmitApduBasic/Channel response.
   Body starts at offset 14; halfword@12 is body+SW length. */
static int apdu_body(const uint8_t *b, int n, unsigned expect_id,
                     const uint8_t **body, unsigned *blen, unsigned *sw) {
    *body = NULL; *blen = 0; *sw = 0;
    if (n < 16 || le16(b + 2) != expect_id) return -1;
    unsigned alen = le16(b + 12);
    if (alen < 2 || 14u + alen > (unsigned)n) return -1;
    *sw = ((unsigned)b[14 + alen - 2] << 8) | b[14 + alen - 1];
    *body = b + 14;
    *blen = alen - 2;
    return 0;
}

/* TS 102.221 PIN Status Template (tag C6) / PS_DO (tag 90).
   Report presence and whether first PS bit (MSB) indicates PIN enabled;
   never print AID/ICCID/IMSI or full template dumps. */
static void decode_pin_status_c6(const uint8_t *body, unsigned blen, int cp_pin1) {
    unsigned i = 0;
    int c6 = 0, ps = -1, life = -1;
    while (i + 2 <= blen) {
        unsigned tag = body[i++];
        if ((tag & 0x1f) == 0x1f) {
            while (i < blen && (body[i] & 0x80)) i++;
            if (i < blen) i++;
            if (i >= blen) break;
            tag = 0xff;
        }
        unsigned len = body[i++];
        if (len & 0x80) {
            unsigned nb = len & 0x7f;
            if (i + nb > blen) break;
            len = 0;
            while (nb--) len = (len << 8) | body[i++];
        }
        if (i + len > blen) break;
        /* Enter FCP (0x62) constructed content without printing it. */
        if (tag == 0x62 && (tag & 0x20)) {
            /* fall through to nested scan via temporarily shrinking — handled below */
        }
        if (tag == 0xc6) {
            c6 = 1;
            unsigned j = 0;
            while (j + 2 <= len) {
                unsigned t2 = body[i + j++];
                unsigned l2 = body[i + j++];
                if (l2 & 0x80) break;
                if (j + l2 > len) break;
                if (t2 == 0x90 && l2 >= 1)
                    ps = body[i + j];
                j += l2;
            }
        } else if (tag == 0x8a && len >= 1)
            life = body[i];
        else if (tag == 0x62) {
            /* Recurse one level into FCP for C6/8A only. */
            const uint8_t *inner = body + i;
            unsigned k = 0;
            while (k + 2 <= len) {
                unsigned t2 = inner[k++];
                unsigned l2 = inner[k++];
                if (l2 & 0x80) break;
                if (k + l2 > len) break;
                if (t2 == 0xc6) {
                    c6 = 1;
                    unsigned m = 0;
                    while (m + 2 <= l2) {
                        unsigned t3 = inner[k + m++];
                        unsigned l3 = inner[k + m++];
                        if (l3 & 0x80) break;
                        if (m + l3 > l2) break;
                        if (t3 == 0x90 && l3 >= 1)
                            ps = inner[k + m];
                        m += l3;
                    }
                } else if (t2 == 0x8a && l2 >= 1)
                    life = inner[k];
                k += l2;
            }
        }
        i += len;
    }
    int card_pin_enabled = (ps >= 0) ? ((ps & 0x80) != 0) : -1;
    printf("tlv_c6_present=%d ps_do_present=%d card_pin1_enabled=%d cp_pin1=%d",
        c6, ps >= 0 ? 1 : 0, card_pin_enabled, cp_pin1);
    if (life >= 0) printf(" life_cycle=0x%02x", life);
    if (c6 && ps >= 0 && cp_pin1 == 3 && card_pin_enabled == 0)
        printf(" match=card_disabled_vs_cp_disabled");
    else if (c6 && ps >= 0 && cp_pin1 == 3 && card_pin_enabled == 1)
        printf(" match=card_enabled_vs_cp_disabled");
    else if (c6 && ps >= 0)
        printf(" match=other");
    else
        printf(" match=no_c6_or_ps");
    puts("");
}

static int send_basic_sw_body(int ipc, int rfs, uint8_t *req, int pkt, uint32_t token,
                              const char *name, unsigned *sw,
                              uint8_t *out, unsigned *out_len) {
    uint8_t resp[512];
    int n = 0;
    *sw = 0;
    if (out_len) *out_len = 0;
    printf("%s\n", name);
    int rc = exchange(ipc, rfs, req, (size_t)pkt, 0x020c, token, resp, &n);
    memset(req, 0, (size_t)pkt);
    if (rc) { printf("%s timeout\n", name); return -1; }
    const uint8_t *body = NULL;
    unsigned blen = 0;
    int have = apdu_body(resp, n, 0x020c, &body, &blen, sw) == 0;
    if (have) {
        printf("%s response: length=%d error_raw=%u sw=%04x body_len=%u\n",
            name, n, n > 10 ? resp[10] : 0, *sw, blen);
        if (out && out_len && blen && blen <= 256) {
            memcpy(out, body, blen);
            *out_len = blen;
        }
    } else
        printf("%s response: length=%d error_raw=%u sw_present=0\n", name, n, n > 10 ? resp[10] : 0);
    memset(resp, 0, sizeof resp);
    return have;
}

static int send_basic_sw(int ipc, int rfs, uint8_t *req, int pkt, uint32_t token,
                         const char *name, unsigned *sw) {
    return send_basic_sw_body(ipc, rfs, req, pkt, token, name, sw, NULL, NULL);
}

static int select_ok(int have, unsigned sw) {
    return have == 1 && (sw == 0x9000 || (sw & 0xff00) == 0x6100);
}

static int p2_retry_ok(int have, unsigned sw) {
    if (have != 1 || g_ready || g_app == 3 || g_app != 2) return 0;
    if (g_remain >= 0 && g_remain <= 1) return 0;
    return sw == 0x6984 || sw == 0x6985 || sw == 0x6a86 || sw == 0x6a88;
}

static int run_select_verify(void) {
    alarm(50);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    if (query_sim(ipc, rfs, 110)) { puts("SIM query timeout"); close(ipc); close(rfs); close(lock); return 1; }
    unsigned sw = 0;
    int have = 0;
    int aid_n = g_aid_len;
    uint8_t aid[16];
    memset(aid, 0, sizeof aid);
    if (aid_n > 0 && aid_n <= 16) memcpy(aid, g_aid, (size_t)aid_n);
    memset(g_aid, 0, sizeof g_aid);
    if (g_ready || g_app == 3 || (g_remain >= 0 && g_remain <= 1) || aid_n <= 0) {
        puts("select not sent");
        memset(aid, 0, sizeof aid);
    } else {
        uint8_t req[64];
        int pkt = fill_basic_apdu(req, 111, 0x00, 0xa4, 0x04, 0x00, aid, (unsigned)aid_n);
        memset(aid, 0, sizeof aid);
        if (pkt < 23) puts("select not sent");
        else have = send_basic_sw(ipc, rfs, req, pkt, 111, "select", &sw);
        if (!select_ok(have, sw)) {
            puts("verify not sent");
        } else {
            uint8_t creq[39];
            int cpkt = fill_transmit_chv_p2(creq, 112, 1, 0x01);
            have = send_basic_sw(ipc, rfs, creq, cpkt, 112, "verify p2=01", &sw);
            if (query_sim(ipc, rfs, 113)) puts("SIM query timeout");
            if (g_ready || (have == 1 && sw == 0x9000)) {
                /* registration follows */
            } else if (chv_second_ok(have, sw)) {
                cpkt = fill_transmit_chv_p2(creq, 114, 2, 0x01);
                have = send_basic_sw(ipc, rfs, creq, cpkt, 114, "verify p2=01 second", &sw);
                if (query_sim(ipc, rfs, 115)) puts("SIM query timeout");
            } else if (p2_retry_ok(have, sw)) {
                cpkt = fill_transmit_chv_p2(creq, 116, 1, 0x00);
                have = send_basic_sw(ipc, rfs, creq, cpkt, 116, "verify p2=00", &sw);
                if (query_sim(ipc, rfs, 117)) puts("SIM query timeout");
                if (chv_second_ok(have, sw)) {
                    cpkt = fill_transmit_chv_p2(creq, 118, 2, 0x00);
                    have = send_basic_sw(ipc, rfs, creq, cpkt, 118, "verify p2=00 second", &sw);
                    if (query_sim(ipc, rfs, 119)) puts("SIM query timeout");
                } else if (!g_ready) puts("second chv not sent");
            } else if (!g_ready) puts("second chv not sent");
            memset(creq, 0, sizeof creq);
        }
    }
    memset(g_aid, 0, sizeof g_aid);
    if (g_ready || (have == 1 && sw == 0x9000)) follow_ready(ipc, rfs);
    else one_registration(ipc, rfs, 120);
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* BuildGetSimLockInfo: id 0x4104, length 12, no payload. Policy, status
   and lock type are the low nibbles of bytes 12, 13 and 14. Max retry
   is byte 15 and remain is byte 16. The lock-code bytes start later
   and are not read. */
static int lock_fields(const uint8_t *b, int n, unsigned *policy, unsigned *status,
                       unsigned *kind, unsigned *max_retry, unsigned *remain) {
    if (n < 17 || le16(b + 2) != 0x4104 || b[10]) return -1;
    *policy = b[12] & 0xfu;
    *status = b[13] & 0xfu;
    *kind = b[14] & 0xfu;
    *max_retry = b[15];
    *remain = b[16];
    return 0;
}

static int run_query_lock_info(void) {
    alarm(30);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[12], resp[512];
    int n = 0;
    if (fill_empty_get(req, 0x4104, 130, 12) != 12) return 1;
    puts("sim lock info");
    int rc = exchange(ipc, rfs, req, 12, 0x4104, 130, resp, &n);
    memset(req, 0, sizeof req);
    unsigned policy = 0, status = 0, kind = 0, max_retry = 0, remain = 0;
    if (rc) puts("sim lock info timeout");
    else if (lock_fields(resp, n, &policy, &status, &kind, &max_retry, &remain))
        printf("sim lock info length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    else
        printf("sim lock info length=%d error_raw=0 policy=%u status=%u lock_type=%u max_retry=%u remain=%u\n",
            n, policy, status, kind, max_retry, remain);
    memset(resp, 0, sizeof resp);
    if (query_sim(ipc, rfs, 131)) puts("SIM query timeout");
    if (g_ready) follow_ready(ipc, rfs);
    else one_registration(ipc, rfs, 132);
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* BuildSimGetFacilityLock: id 0x0209, length 71. Byte 12 is the facility
   table index (SC=3). Empty password stores length 0 at byte 13. Byte 53
   is the service-class low byte. Optional AID: length at byte 54, bytes
   at offset 55. Stock UiccCardApplication.queryPin1State uses SC, empty
   password, service class 7, and the application AID. */
static int fill_get_facility_sc(uint8_t *req, uint32_t token,
                                const uint8_t *aid, unsigned aid_len) {
    if (aid_len > 16) return -1;
    memset(req, 0, 71);
    req[2] = 0x09;
    req[3] = 0x02;
    req[4] = 71;
    req[6] = (uint8_t)token;
    req[12] = 3;
    req[53] = 7;
    if (aid && aid_len) {
        req[54] = (uint8_t)aid_len;
        memcpy(req + 55, aid, aid_len);
    }
    return 71;
}

static int run_query_facility_sc(void) {
    alarm(30);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    if (query_sim(ipc, rfs, 140)) puts("SIM query timeout");
    uint8_t aid[16];
    unsigned aid_n = 0;
    if (g_aid_len > 0 && g_aid_len <= 16) {
        aid_n = (unsigned)g_aid_len;
        memcpy(aid, g_aid, aid_n);
    }
    memset(g_aid, 0, sizeof g_aid);
    uint8_t req[71], resp[512];
    int n = 0;
    int pkt = fill_get_facility_sc(req, 141, aid_n ? aid : NULL, aid_n);
    memset(aid, 0, sizeof aid);
    if (pkt != 71) return 1;
    puts("facility SC get");
    int rc = exchange(ipc, rfs, req, 71, 0x0209, 141, resp, &n);
    memset(req, 0, sizeof req);
    if (rc) puts("facility SC get timeout");
    else {
        unsigned err = n > 10 ? resp[10] : 0;
        printf("facility SC get length=%d error_raw=%u", n, err);
        if (!err && n > 13) printf(" status_raw=%u", resp[13]);
        puts("");
    }
    memset(resp, 0, sizeof resp);
    if (query_sim(ipc, rfs, 142)) puts("SIM query timeout");
    if (g_ready) follow_ready(ipc, rfs);
    else one_registration(ipc, rfs, 143);
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* BuildSimSetFacilityLock: id 0x020a, length 72 (factory sit-stream 0x7e910).
   Byte 12 = facility index (SC=3), byte 13 = lock mode (1=enable),
   byte 14 = password length, digits at offset 15, byte 54 = service class.
   Optional AID: length at byte 55, bytes at 56 (HexString2Value path).
   Stock setIccLockEnabled(true) uses SC + class 7 + PIN digits (not empty).
   GET sibling is 0x0209/71; SET is a different opcode. */
static int fill_set_facility_sc_enable(uint8_t *req, uint32_t token, int which,
                                       const uint8_t *aid, unsigned aid_len) {
    if (which != 1 && which != 2) return -1;
    if (aid_len > 16) return -1;
    memset(req, 0, 72);
    req[2] = 0x0a;
    req[3] = 0x02;
    req[4] = 72;
    req[6] = (uint8_t)token;
    req[12] = 3;
    req[13] = 1;
    req[14] = 4;
    {
        uint8_t digit = which == 1 ? '0' : '1';
        for (int i = 0; i < 4; i++) req[15 + i] = digit;
    }
    req[54] = 7;
    if (aid && aid_len) {
        req[55] = (uint8_t)aid_len;
        memcpy(req + 56, aid, aid_len);
    }
    return 72;
}

/* One-shot: SET SC lock enable with candidate A only. If pin1 becomes
   ENABLED(1), send ONE VerifyPin A. Never candidate B here. Stop if
   remain<=1 before or after. Never print PIN digits. */
static int run_enable_sc_lock_a(void) {
    alarm(90);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }

    int remain_before = -1;
    if (query_sim(ipc, rfs, 240)) { puts("SIM query timeout"); return 1; }
    remain_before = g_remain;
    printf("pre SET pin1=%d remain=%d app=%d\n", g_pin, g_remain, g_app);
    if (g_remain >= 0 && g_remain <= 1) {
        puts("STOP remain<=1; SET not sent");
        close(ipc); close(rfs); close(lock);
        return 2;
    }
    if (g_ready || g_app == 5 || g_app == 6 || g_app == 7) {
        puts("already leave soft-lock; SET skipped");
        follow_ready(ipc, rfs);
        close(ipc); close(rfs); close(lock);
        return 0;
    }

    uint8_t aid[16];
    unsigned aid_n = 0;
    if (g_aid_len > 0 && g_aid_len <= 16) {
        aid_n = (unsigned)g_aid_len;
        memcpy(aid, g_aid, aid_n);
    }
    memset(g_aid, 0, sizeof g_aid);

    uint8_t req[72], resp[512];
    int n = 0;
    if (fill_set_facility_sc_enable(req, 241, 1, aid_n ? aid : NULL, aid_n) != 72) return 1;
    memset(aid, 0, sizeof aid);
    puts("facility SC SET enable candidate=A serviceClass=7");
    int rc = exchange(ipc, rfs, req, 72, 0x020a, 241, resp, &n);
    memset(req, 0, sizeof req);
    if (rc) {
        puts("facility SC SET timeout");
        memset(resp, 0, sizeof resp);
        close(ipc); close(rfs); close(lock);
        return 1;
    }
    {
        unsigned err = n > 10 ? resp[10] : 0;
        printf("facility SC SET length=%d error_raw=%u\n", n, err);
    }
    memset(resp, 0, sizeof resp);

    if (query_sim(ipc, rfs, 242)) puts("SIM query timeout");
    printf("post SET pin1=%d remain=%d app=%d\n", g_pin, g_remain, g_app);
    if (g_remain >= 0 && g_remain <= 1) {
        puts("STOP remain<=1 after SET; VerifyPin not sent");
        close(ipc); close(rfs); close(lock);
        return 2;
    }
    if (remain_before >= 0 && g_remain >= 0 && g_remain < remain_before &&
        g_pin != 1 && !g_ready) {
        puts("STOP remain dropped without pin1 ENABLED; no candidate B");
        close(ipc); close(rfs); close(lock);
        return 2;
    }
    if (g_pin != 1) {
        puts("pin1 still not ENABLED(1); VerifyPin not sent");
        printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
        close(ipc); close(rfs); close(lock);
        return 3;
    }

    puts("pin1 ENABLED; one VerifyPin candidate=A");
    {
        uint8_t vreq[38];
        if (fill_verify_pin(vreq, 243, 1) != 38) return 1;
        n = 0;
        rc = exchange(ipc, rfs, vreq, 38, 0x0201, 243, resp, &n);
        memset(vreq, 0, sizeof vreq);
    }
    if (rc) puts("VerifyPin timeout");
    else {
        unsigned err = n > 10 ? resp[10] : 0;
        unsigned remain = n >= 16 ? le32(resp + 12) : 0;
        printf("PIN verify response: length=%d error_raw=%u remain_raw=%u\n", n, err, remain);
        if (n >= 16) g_remain = (int)remain;
    }
    memset(resp, 0, sizeof resp);

    if (query_sim(ipc, rfs, 244)) puts("SIM query timeout");
    printf("post verify pin1=%d remain=%d app=%d ready=%d\n", g_pin, g_remain, g_app, g_ready);
    if (g_remain >= 0 && g_remain <= 1) {
        puts("STOP remain<=1 after VerifyPin; no candidate B");
        close(ipc); close(rfs); close(lock);
        return 2;
    }
    if (remain_before >= 0 && g_remain >= 0 && g_remain < remain_before &&
        !g_ready && g_app != 5 && g_app != 6 && g_app != 7) {
        puts("STOP remain dropped without READY/6/7; no candidate B");
        close(ipc); close(rfs); close(lock);
        return 2;
    }
    if (g_ready || g_app == 5 || g_app == 6 || g_app == 7) {
        puts("app left PIN; chasing bearer");
        follow_ready(ipc, rfs);
    } else {
        puts("VerifyPin did not reach READY/6/7; no candidate B");
    }
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return (g_ready || g_app == 5 || g_app == 6 || g_app == 7) ? 0 : 4;
}
static int run_query_atr(void) {
    alarm(40);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[29], resp[512];
    int n = 0;
    if (fill_empty_get(req, 0x0212, 150, 12) != 12) return 1;
    puts("get atr");
    int rc = exchange(ipc, rfs, req, 12, 0x0212, 150, resp, &n);
    memset(req, 0, 12);
    if (rc) puts("get atr timeout");
    else {
        unsigned err = n > 10 ? resp[10] : 0;
        printf("get atr length=%d error_raw=%u", n, err);
        if (!err && n > 13)
            printf(" result_raw=%u atr_len=%u", resp[12], resp[13]);
        puts("");
    }
    memset(resp, 0, sizeof resp);
    if (query_sim(ipc, rfs, 151)) puts("SIM query timeout");
    if (g_ready) {
        follow_ready(ipc, rfs);
        printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
        close(ipc); close(rfs); close(lock);
        return 0;
    }
    /* GetAtrHandler only returns ATR bytes to the framework; it never
       rewrites application state. Next proven probe: open channel 0x020d. */
    uint8_t aid[16];
    unsigned aid_n = 0;
    if (g_aid_len > 0 && g_aid_len <= 16) {
        aid_n = (unsigned)g_aid_len;
        memcpy(aid, g_aid, aid_n);
    }
    memset(g_aid, 0, sizeof g_aid);
    if (!aid_n) {
        puts("open channel skipped no aid");
        one_registration(ipc, rfs, 153);
        printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
        close(ipc); close(rfs); close(lock);
        return 0;
    }
    memset(req, 0, 29);
    req[2] = 0x0d;
    req[3] = 0x02;
    req[4] = 29;
    req[6] = 152;
    req[12] = (uint8_t)aid_n;
    memcpy(req + 13, aid, aid_n);
    memset(aid, 0, sizeof aid);
    puts("open channel");
    rc = exchange(ipc, rfs, req, 29, 0x020d, 152, resp, &n);
    memset(req, 0, sizeof req);
    if (rc) puts("open channel timeout");
    else {
        unsigned err = n > 10 ? resp[10] : 0;
        printf("open channel length=%d error_raw=%u", n, err);
        if (!err && n >= 18)
            printf(" session_raw=%u sw=%02x%02x", le32(resp + 12), resp[16], resp[17]);
        puts("");
    }
    memset(resp, 0, sizeof resp);
    if (query_sim(ipc, rfs, 154)) puts("SIM query timeout");
    if (g_ready) follow_ready(ipc, rfs);
    else one_registration(ipc, rfs, 155);
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* BuildSimCloseChannel: id 0x020e, length 16, session word at offset 12.
   The bool argument only selects a log string. */
static int fill_close_channel(uint8_t *req, uint32_t token, uint32_t session) {
    memset(req, 0, 16);
    req[2] = 0x0e;
    req[3] = 0x02;
    req[4] = 16;
    req[6] = (uint8_t)token;
    req[12] = (uint8_t)session;
    req[13] = (uint8_t)(session >> 8);
    req[14] = (uint8_t)(session >> 16);
    req[15] = (uint8_t)(session >> 24);
    return 16;
}

/* Audit incongruent PIN: print non-secret status fields matching factory
   GetPinState/GetAppState offsets, then close the open logical channel
   (session 1 from prior 0x020d) and re-query. */
static int run_audit_close(void) {
    alarm(40);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[16], resp[512];
    int n = 0;
    if (fill_empty_get(req, 0x0200, 160, 12) != 12) return 1;
    puts("SIM audit");
    if (exchange(ipc, rfs, req, 12, 0x0200, 160, resp, &n)) {
        puts("SIM query timeout");
    } else if (n >= 75 && !resp[10]) {
        /* Factory: card@12, apps@14, type@15, state@17, pin1@72, pin2@73, remain1@74. */
        printf("audit card=%u apps=%u type=%u state=%u pin1=%u pin2=%u remain1=%u\n",
            resp[12], resp[14], resp[15], resp[17], resp[72], resp[73], resp[74]);
        note_sim(resp, n);
    } else {
        printf("SIM length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    }
    memset(resp, 0, sizeof resp);
    memset(req, 0, sizeof req);
    if (fill_close_channel(req, 161, 1) != 16) return 1;
    puts("close channel");
    int rc = exchange(ipc, rfs, req, 16, 0x020e, 161, resp, &n);
    memset(req, 0, sizeof req);
    if (rc) puts("close channel timeout");
    else printf("close channel length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    memset(resp, 0, sizeof resp);
    if (query_sim(ipc, rfs, 162)) puts("SIM query timeout");
    if (g_ready) follow_ready(ipc, rfs);
    else one_registration(ipc, rfs, 163);
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* Empty BuildSimGetSlotStatus 0x024d length 12, then SIM status, then the
   same LTE_ONLY / auto / registration path used after READY — even if the
   CP still reports app_state PIN. Slot response: length+error only. */
static int run_slot_then_attach(void) {
    alarm(90);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[16], resp[512];
    int n = 0;
    if (fill_empty_get(req, 0x024d, 170, 12) != 12) return 1;
    puts("slot status");
    if (exchange(ipc, rfs, req, 12, 0x024d, 170, resp, &n))
        puts("slot status timeout");
    else
        printf("slot status length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    memset(resp, 0, sizeof resp);
    memset(req, 0, sizeof req);
    puts("SIM query");
    if (query_sim(ipc, rfs, 171)) puts("SIM query timeout");
    puts("attach despite pin");
    follow_ready(ipc, rfs);
    memset(resp, 0, sizeof resp);
    memset(req, 0, sizeof req);
    puts("SIM query after attach");
    if (query_sim(ipc, rfs, 172)) puts("SIM query timeout");
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* One HAL1.6 card-power cycle: DOWN (builder byte 4) then UP (1), then
   GetSimStatus. Apps: print only non-secret count/type/state/pin1. */
static int run_card_power_cycle(void) {
    alarm(60);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    puts("SIM before power cycle");
    if (query_sim(ipc, rfs, 180)) puts("SIM query timeout");
    if (send_card_power(ipc, rfs, 4, 181)) { close(ipc); close(rfs); close(lock); return 1; }
    puts("wait after card power down");
    wait_ms(ipc, rfs, 3000);
    if (send_card_power(ipc, rfs, 1, 182)) { close(ipc); close(rfs); close(lock); return 1; }
    puts("wait after card power up");
    wait_ms(ipc, rfs, 5000);
    puts("SIM after power cycle");
    if (query_sim(ipc, rfs, 183)) puts("SIM query timeout");
    if (g_ready) follow_ready(ipc, rfs);
    else one_registration(ipc, rfs, 184);
    print_rmnet();
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* BuildRadioPower: id 0x0800, length 18.
   DoRadioPower: request ON (int>0) → builder arg0=1 → word@12=2;
   OFF (int<=0) → arg0=0 → word@12=1. Bytes 16/17 are optional flags (0). */
static int fill_radio_power(uint8_t *req, uint32_t token, int on) {
    memset(req, 0, 18);
    req[2] = 0x00;
    req[3] = 0x08;
    req[4] = 18;
    req[6] = (uint8_t)token;
    req[12] = on ? 2 : 1;
    return 18;
}

/* Empty PIN1 verify: factory layout with count 0, no digit bytes, no AID.
   DoVerifyPin never sends this; used only to re-check CP error after radio. */
static int fill_verify_pin_empty(uint8_t *req, uint32_t token) {
    memset(req, 0, 38);
    req[2] = 0x01;
    req[3] = 0x02;
    req[4] = 38;
    req[6] = (uint8_t)token;
    /* byte 12 = 0 character count; 13.. remain zero */
    return 38;
}

static int send_radio_power(int ipc, int rfs, int on, uint32_t token) {
    uint8_t req[18], resp[512];
    int n = 0;
    if (fill_radio_power(req, token, on) != 18) return -1;
    printf("radio power %s\n", on ? "on" : "off");
    int rc = exchange(ipc, rfs, req, 18, 0x0800, token, resp, &n);
    if (rc) { puts("radio power timeout"); return rc; }
    printf("radio power response: length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    return 0;
}

static int run_radio_power_cycle(void) {
    alarm(90);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    puts("SIM before radio power");
    if (query_sim(ipc, rfs, 190)) puts("SIM query timeout");
    if (send_radio_power(ipc, rfs, 0, 191)) { close(ipc); close(rfs); close(lock); return 1; }
    puts("wait after radio off");
    wait_ms(ipc, rfs, 3000);
    if (send_radio_power(ipc, rfs, 1, 192)) { close(ipc); close(rfs); close(lock); return 1; }
    puts("wait after radio on");
    wait_ms(ipc, rfs, 5000);
    puts("SIM after radio power");
    if (query_sim(ipc, rfs, 193)) puts("SIM query timeout");
    if (g_ready) {
        follow_ready(ipc, rfs);
    } else {
        /* One empty 0x0201 — error code only, no digits. */
        uint8_t req[38], resp[512];
        int n = 0;
        if (fill_verify_pin_empty(req, 194) != 38) return 1;
        puts("empty verify pin1");
        if (exchange(ipc, rfs, req, 38, 0x0201, 194, resp, &n))
            puts("empty verify timeout");
        else
            printf("empty verify length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
        memset(req, 0, sizeof req);
        memset(resp, 0, sizeof resp);
        if (query_sim(ipc, rfs, 195)) puts("SIM query timeout");
        one_registration(ipc, rfs, 196);
    }
    print_rmnet();
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* After radio ON: LTE_ONLY + auto, poll data reg ~60s, then at most one
   PIN1 candidate (A) with stock strlen layout; optional B only if remain
   still >=2 and A did not unlock / did not freeze remain. */
static int run_attach_then_pin(void) {
    alarm(120);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[38], resp[512];
    int n = 0;
    int saw_reg = 0, saw_tech = 0;
    puts("SIM before attach poll");
    if (query_sim(ipc, rfs, 200)) puts("SIM query timeout");
    if (g_ready) {
        follow_ready(ipc, rfs);
        print_rmnet();
        printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
        close(ipc); close(rfs); close(lock);
        return 0;
    }
    memset(req, 0, sizeof req);
    req[2] = 0x0b; req[3] = 0x07; req[4] = 12; req[6] = 201;
    puts("preferred query");
    if (!exchange(ipc, rfs, req, 12, 0x070b, 201, resp, &n) && n >= 16 && !resp[10]) {
        g_pref = (int)le32(resp + 12);
        printf("preferred_raw=%u\n", (unsigned)g_pref);
    }
    if (g_pref != 11) {
        memset(req, 0, sizeof req);
        req[2] = 0x0a; req[3] = 0x07; req[4] = 16; req[6] = 202; req[12] = 11;
        puts("preferred set LTE_ONLY");
        if (!exchange(ipc, rfs, req, 16, 0x070a, 202, resp, &n))
            printf("preferred set length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    }
    memset(req, 0, sizeof req);
    req[2] = 0x04; req[3] = 0x07; req[4] = 12; req[6] = 203;
    puts("network selection auto");
    if (!exchange(ipc, rfs, req, 12, 0x0704, 203, resp, &n))
        printf("selection length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    for (int i = 0; i < 5; i++) {
        memset(req, 0, sizeof req);
        memset(resp, 0, sizeof resp);
        req[2] = 0x01; req[3] = 0x07; req[4] = 12; req[6] = (uint8_t)(210 + i);
        printf("registration sample=%d\n", i);
        if (!exchange(ipc, rfs, req, 12, 0x0701, 210 + (uint32_t)i, resp, &n) && n >= 16 && !resp[10]) {
            unsigned reg = resp[12], rej = resp[13], tech = resp[15];
            printf("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u\n", reg, rej, tech);
            if (reg) saw_reg = 1;
            if (tech) saw_tech = 1;
        } else {
            printf("registration length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
        }
        print_rmnet();
        if (i != 4) wait_ms(ipc, rfs, 12000);
    }
    if (query_sim(ipc, rfs, 220)) puts("SIM query timeout");
    if (g_ready || saw_reg) {
        if (g_ready) follow_ready(ipc, rfs);
        print_rmnet();
        printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
        close(ipc); close(rfs); close(lock);
        return 0;
    }
    /* No registration. Try candidate A once if still PIN and remain safe. */
    if (g_app != 2 || g_remain < 2) {
        puts(g_remain < 2 ? "pin remain too low; candidate not sent" : "not PIN; candidate not sent");
        print_rmnet();
        printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
        close(ipc); close(rfs); close(lock);
        return 0;
    }
    int remain_before = g_remain;
    printf("remain_before=%d\n", remain_before);
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    /* Stock PIN1: id 0x0201, strlen count, chars at 13, null AID, trailing zeros. */
    if (fill_verify_pin(req, 221, 1) != 38) return 1;
    puts("PIN verify candidate=A");
    int rc = exchange(ipc, rfs, req, 38, 0x0201, 221, resp, &n);
    memset(req, 0, sizeof req);
    unsigned verr = 0, vrem = 0;
    if (rc) puts("PIN verify timeout");
    else {
        verr = n > 10 ? resp[10] : 0;
        vrem = n >= 16 ? le32(resp + 12) : 0;
        printf("PIN verify candidate=A length=%d error_raw=%u remain_raw=%u\n", n, verr, vrem);
    }
    memset(resp, 0, sizeof resp);
    if (query_sim(ipc, rfs, 222)) puts("SIM query timeout");
    printf("remain_after_A=%d\n", g_remain);
    if (g_ready) {
        follow_ready(ipc, rfs);
        print_rmnet();
        printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
        close(ipc); close(rfs); close(lock);
        return 0;
    }
    /* Wrong-PIN style is protocol 3. Unchanged remain + non-3 → stop spray. */
    if (verr != 3 && g_remain == remain_before) {
        puts("attempts unchanged; stop digit spray");
        one_registration(ipc, rfs, 223);
        print_rmnet();
        printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
        close(ipc); close(rfs); close(lock);
        return 0;
    }
    if (g_remain <= 1) {
        puts("remain<=1; candidate B not sent");
        print_rmnet();
        printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
        close(ipc); close(rfs); close(lock);
        return 0;
    }
    if (g_remain < 2 || g_app != 2) {
        puts("candidate B not sent");
        print_rmnet();
        printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
        close(ipc); close(rfs); close(lock);
        return 0;
    }
    remain_before = g_remain;
    printf("remain_before_B=%d\n", remain_before);
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    if (fill_verify_pin(req, 224, 2) != 38) return 1;
    puts("PIN verify candidate=B");
    rc = exchange(ipc, rfs, req, 38, 0x0201, 224, resp, &n);
    memset(req, 0, sizeof req);
    if (rc) puts("PIN verify timeout");
    else {
        verr = n > 10 ? resp[10] : 0;
        vrem = n >= 16 ? le32(resp + 12) : 0;
        printf("PIN verify candidate=B length=%d error_raw=%u remain_raw=%u\n", n, verr, vrem);
    }
    if (query_sim(ipc, rfs, 225)) puts("SIM query timeout");
    printf("remain_after_B=%d\n", g_remain);
    if (g_ready) follow_ready(ipc, rfs);
    else one_registration(ipc, rfs, 226);
    print_rmnet();
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d saw_tech=%d\n",
        g_app, g_pin, g_remain, g_ready, saw_tech);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* Query selection mode 0x0703 (byte 12: 0=auto). Resend 0x0704 only if not auto. */
static int run_fix_selection(void) {
    alarm(40);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[16], resp[512];
    int n = 0, mode = -1;
    memset(req, 0, sizeof req);
    req[2] = 0x01; req[3] = 0x08; req[4] = 12; req[6] = 230;
    puts("radio state");
    if (!exchange(ipc, rfs, req, 12, 0x0801, 230, resp, &n) && n >= 16 && !resp[10])
        printf("radio_state_raw=%u\n", le32(resp + 12));
    else
        printf("radio state length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    req[2] = 0x03; req[3] = 0x07; req[4] = 12; req[6] = 231;
    puts("selection mode query");
    if (!exchange(ipc, rfs, req, 12, 0x0703, 231, resp, &n) && n >= 13 && !resp[10]) {
        mode = (int)resp[12];
        printf("selection_mode_raw=%d%s\n", mode, mode == 0 ? " selection=auto" : "");
    } else {
        printf("selection mode length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    }
    if (mode != 0) {
        memset(req, 0, sizeof req);
        memset(resp, 0, sizeof resp);
        req[2] = 0x04; req[3] = 0x07; req[4] = 12; req[6] = 232;
        puts("network selection auto");
        if (!exchange(ipc, rfs, req, 12, 0x0704, 232, resp, &n))
            printf("selection set length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    } else {
        puts("already auto; set skipped");
    }
    if (query_sim(ipc, rfs, 233)) puts("SIM query timeout");
    one_registration(ipc, rfs, 234);
    print_rmnet();
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* BuildSetUicc: id 0x0249, length 13, state byte at offset 12.
   EnableUiccAppHandler passes the request int through (enable=1).
   DoAutoDisableUicc uses 0 — banned. */
static int fill_set_uicc(uint8_t *req, uint32_t token, uint8_t enable) {
    if (enable != 1) return -1; /* only enable path allowed here */
    memset(req, 0, 13);
    req[2] = 0x49;
    req[3] = 0x02;
    req[4] = 13;
    req[6] = (uint8_t)token;
    req[12] = enable;
    return 13;
}

static int run_uicc_enable(void) {
    alarm(100);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[18], resp[512];
    int n = 0;
    puts("SIM before uicc enable");
    if (query_sim(ipc, rfs, 240)) puts("SIM query timeout");
    memset(req, 0, sizeof req);
    if (fill_set_uicc(req, 241, 1) != 13) return 1;
    puts("uicc enable");
    if (exchange(ipc, rfs, req, 13, 0x0249, 241, resp, &n))
        puts("uicc enable timeout");
    else
        printf("uicc enable length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    wait_ms(ipc, rfs, 3000);
    puts("SIM after uicc enable");
    if (query_sim(ipc, rfs, 242)) puts("SIM query timeout");
    /* Radio ON if needed (state 10 = ON from prior boots). */
    memset(req, 0, sizeof req);
    req[2] = 0x01; req[3] = 0x08; req[4] = 12; req[6] = 243;
    puts("radio state");
    unsigned radio = 0;
    if (!exchange(ipc, rfs, req, 12, 0x0801, 243, resp, &n) && n >= 16 && !resp[10]) {
        radio = le32(resp + 12);
        printf("radio_state_raw=%u\n", radio);
    }
    if (radio != 10) {
        if (send_radio_power(ipc, rfs, 1, 244)) { /* fall through */ }
        wait_ms(ipc, rfs, 2000);
    }
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    req[2] = 0x0b; req[3] = 0x07; req[4] = 12; req[6] = 245;
    puts("preferred query");
    if (!exchange(ipc, rfs, req, 12, 0x070b, 245, resp, &n) && n >= 16 && !resp[10]) {
        g_pref = (int)le32(resp + 12);
        printf("preferred_raw=%u\n", (unsigned)g_pref);
    }
    if (g_pref != 11) {
        memset(req, 0, sizeof req);
        req[2] = 0x0a; req[3] = 0x07; req[4] = 16; req[6] = 246; req[12] = 11;
        puts("preferred set LTE_ONLY");
        exchange(ipc, rfs, req, 16, 0x070a, 246, resp, &n);
    }
    if (g_ready) {
        follow_ready(ipc, rfs);
    } else {
        for (int i = 0; i < 4; i++) {
            memset(req, 0, sizeof req);
            memset(resp, 0, sizeof resp);
            req[2] = 0x01; req[3] = 0x07; req[4] = 12; req[6] = (uint8_t)(250 + i);
            printf("registration sample=%d\n", i);
            if (!exchange(ipc, rfs, req, 12, 0x0701, 250 + (uint32_t)i, resp, &n) && n >= 16 && !resp[10]) {
                unsigned reg = resp[12], rej = resp[13], tech = resp[15];
                const char *tn = radio_tech_name(tech);
                printf("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u%s%s\n",
                    reg, rej, tech, tn ? " radio_tech=" : "", tn ? tn : "");
            }
            print_rmnet();
            if (i != 3) wait_ms(ipc, rfs, 12000);
        }
        if (query_sim(ipc, rfs, 255)) puts("SIM query timeout");
    }
    print_rmnet();
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* Facility table at sit-stream 0x2b520, 4-byte names: SC=3, PN=17. */
static int fill_get_facility(uint8_t *req, uint32_t token, uint8_t fac_idx) {
    memset(req, 0, 71);
    req[2] = 0x09;
    req[3] = 0x02;
    req[4] = 71;
    req[6] = (uint8_t)token;
    req[12] = fac_idx;
    req[53] = 7;
    return 71;
}

/* PIN incongruence audit: lock GETs (no secrets), parse slot status
   non-secret fields, SIM status. Preferred tweaks are not the main path. */
static int run_pin_incongruence_audit(void) {
    alarm(90);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[72], resp[512];
    int n = 0;
    /* Tokens must fit req[6] (uint8); >255 truncates and exchange never matches. */
    puts("SIM status");
    if (query_sim(ipc, rfs, 61)) puts("SIM query timeout");

    /* 0x0207 VerifyNetworkLock requires password — not an empty GET; skip. */
    puts("skip 0x0207 verify-network-lock (not empty GET)");

    /* Empty GetFrequencyLock 0x073a. */
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    if (fill_empty_get(req, 0x073a, 62, 12) != 12) return 1;
    puts("frequency lock get");
    if (exchange(ipc, rfs, req, 12, 0x073a, 62, resp, &n))
        puts("frequency lock timeout");
    else {
        /* Adapter GetFrequencyLock copies 7 words from adapter+16..+40.
           With packet at adapter+16 that is resp+16..; first payload word
           often sits at resp+12 after RCM. Print both candidates as enums only. */
        printf("frequency lock length=%d error_raw=%u", n, n > 10 ? resp[10] : 0);
        if (n >= 16 && !resp[10])
            printf(" lock_word0_at12=%u", le32(resp + 12));
        if (n >= 20 && !resp[10])
            printf(" lock_word0_at16=%u", le32(resp + 16));
        puts("");
    }

    /* Facility PN network lock query (same GET layout as SC). */
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    if (fill_get_facility(req, 63, 17) != 71) return 1;
    puts("facility PN get");
    if (exchange(ipc, rfs, req, 71, 0x0209, 63, resp, &n))
        puts("facility PN timeout");
    else {
        printf("facility PN length=%d error_raw=%u", n, n > 10 ? resp[10] : 0);
        if (n >= 14 && !resp[10])
            printf(" lock_status_raw=%u", resp[13]);
        puts("");
    }

    /* GetSlotStatus: non-secret fields only (no ATR/ICCID/EID dump). */
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    if (fill_empty_get(req, 0x024d, 64, 12) != 12) return 1;
    puts("slot status");
    if (exchange(ipc, rfs, req, 12, 0x024d, 64, resp, &n))
        puts("slot status timeout");
    else {
        printf("slot status length=%d error_raw=%u", n, n > 10 ? resp[10] : 0);
        if (n >= 15 && !resp[10]) {
            /* Init (type1, len>=0x1b1): num_slots@12; fill(ptr=pkt+0xd, legacy=0).
               Modern slot record stride 0x69: card@+0, atr_len@+1, atr@+2.
               Port-info count at record+52. SlotStatus has no app_state field.
               GetSlotState returns 0 when not legacy — do not invent slot_state. */
            unsigned nslots = resp[12];
            unsigned card0 = resp[13];
            unsigned atr_len0 = resp[14];
            printf(" num_slots=%u slot0_card_state_raw=%u slot0_atr_len=%u",
                nslots, card0, atr_len0);
            if (n >= 13 + 53)
                printf(" slot0_port_count_raw=%u", resp[13 + 52]);
            printf(" vs_sim_card=%d vs_sim_app_state=%d vs_sim_apps=%d",
                g_card, g_app, g_apps);
            /* GetSimStatus applications count was last note_sim apps; reuse g_card. */
            if (nslots >= 1 && card0 != 0 && g_card == 1)
                printf(" slot_card_present_matches_sim=1");
            else if (nslots >= 1)
                printf(" slot_card_vs_sim_mismatch=1");
        }
        puts("");
        /* wipe response so ATR/ICCID never linger in this buffer for later prints */
        memset(resp, 0, sizeof resp);
    }

    one_registration(ipc, rfs, 65);
    print_rmnet();
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);

    /* SECONDARY only: 1-3 left PIN incongruence with no actionable lock/slot/HAL
       fixup. One preferred=12 (LTE_WCDMA) + short reg poll — not the main path. */
    puts("SECONDARY preferred=12 LTE_WCDMA");
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    req[2] = 0x0a; req[3] = 0x07; req[4] = 16; req[6] = 66; req[12] = 12;
    if (exchange(ipc, rfs, req, 16, 0x070a, 66, resp, &n))
        puts("SECONDARY preferred set timeout");
    else
        printf("SECONDARY preferred set length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    req[2] = 0x0b; req[3] = 0x07; req[4] = 12; req[6] = 67;
    if (!exchange(ipc, rfs, req, 12, 0x070b, 67, resp, &n) && n >= 16 && !resp[10])
        printf("SECONDARY preferred_raw=%u\n", le32(resp + 12));
    for (int i = 0; i < 3; i++) {
        one_registration(ipc, rfs, 68 + (uint32_t)i);
        if (i != 2) wait_ms(ipc, rfs, 8000);
    }
    if (query_sim(ipc, rfs, 71)) puts("SIM query timeout");
    print_rmnet();
    printf("SECONDARY RESULT app_state=%d pin1=%d remain=%d ready=%d\n",
        g_app, g_pin, g_remain, g_ready);
    if (g_ready) follow_ready(ipc, rfs);

    close(ipc); close(rfs); close(lock);
    return 0;
}

/* Dual-slot/port audit. Mapping SET 0x0250 is NOT sent: stock
   MiscService::SetSlotMapping returns without a packet when
   persist.radio.slotmap.config is empty; SimSlotMappingHandler needs
   framework-supplied slot (or slot+port) integers — no proven default.
   One proven never-sent empty GET: BuildGetPhoneCapability 0x0615 len 12.
   Also re-parse GetSlotStatus port logical_id/state (no ATR/ICCID). */
static int run_slot_port_audit(void) {
    alarm(40);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[16], resp[512];
    int n = 0;

    puts("skip 0x0250 slot-mapping SET (needs slotmap ints; property empty)");
    puts("skip 0x0740 preferred-data-modem (needs modem index from request)");

    /* Empty BuildGetPhoneCapability: id 0x0615, length 12. */
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    if (fill_empty_get(req, 0x0615, 80, 12) != 12) return 1;
    puts("phone capability get");
    if (exchange(ipc, rfs, req, 12, 0x0615, 80, resp, &n))
        puts("phone capability timeout");
    else {
        printf("phone capability length=%d error_raw=%u", n, n > 10 ? resp[10] : 0);
        if (n >= 16 && !resp[10])
            printf(" max_active_data=%u max_active_internet=%u lingering=%u logical_modem_list_size=%u",
                resp[12], resp[13], resp[14], resp[15]);
        puts("");
        memset(resp, 0, sizeof resp);
    }

    /* GetSlotStatus: non-secret slot0/1 + port logical/state. */
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    if (fill_empty_get(req, 0x024d, 81, 12) != 12) return 1;
    puts("slot status");
    if (exchange(ipc, rfs, req, 12, 0x024d, 81, resp, &n))
        puts("slot status timeout");
    else {
        printf("slot status length=%d error_raw=%u", n, n > 10 ? resp[10] : 0);
        if (n >= 15 && !resp[10]) {
            unsigned nslots = resp[12];
            printf(" num_slots=%u", nslots);
            /* Modern: payload at pkt+13; slot stride 0x69; port_count@+52;
               ports start at payload+0x40: logical@+0, state@+1, stride 0xd. */
            for (unsigned s = 0; s < nslots && s < 2; s++) {
                unsigned base = 13 + s * 0x69;
                if (n < (int)(base + 53)) break;
                unsigned card = resp[base];
                unsigned atr_len = resp[base + 1];
                unsigned pc = resp[base + 52];
                printf(" slot%u_card=%u slot%u_atr_len=%u slot%u_ports=%u",
                    s, card, s, atr_len, s, pc);
                if (pc >= 1 && pc <= 4) {
                    unsigned pbase = base + 0x40; /* modern: logical@+0x40, state@+0x41, stride 0xd */
                    for (unsigned p = 0; p < pc && p < 4; p++) {
                        unsigned off = pbase + p * 0xd;
                        if (n < (int)(off + 2)) break;
                        printf(" p%u_logical=%u p%u_state=%u",
                            p, resp[off], p, resp[off + 1]);
                    }
                }
            }
        }
        puts("");
        memset(resp, 0, sizeof resp);
    }

    if (query_sim(ipc, rfs, 82)) puts("SIM query timeout");
    one_registration(ipc, rfs, 83);
    print_rmnet();
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    if (g_ready) follow_ready(ipc, rfs);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* Logical binding: SIT header has no phone/slot id (InitRequestHeader).
   BuildSimGetStatus is empty 0x0200 len 12. Stock dual path is separate
   IoChannel nodes /dev/umts_ipc0 and /dev/umts_ipc1 (string in libsitril;
   sysfs maj:min 493:0 / 493:1). Our probes always used ipc0 only = logical
   stack bound to that channel. 0x0250 still not derived from GetSlotStatus
   (SetSlotMapping needs property; SimSlotMappingHandler needs framework
   ints). Query the other stack via ipc1. */
static int run_other_logical_sim(void) {
    alarm(40);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }

    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (rfs < 0) { puts("rfs0 open failed"); return 1; }

    int ipc0 = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int ipc1 = open_node("/dev/umts_ipc1", "/sys/class/cpif/umts_ipc1/dev");
    printf("bind ipc0=%s ipc1=%s (0x0200 has no slot field; channel selects stack)\n",
        ipc0 >= 0 ? "ok" : "fail", ipc1 >= 0 ? "ok" : "fail");
    if (ipc1 < 0) {
        puts("umts_ipc1 unavailable; cannot query other logical");
        if (ipc0 >= 0) close(ipc0);
        close(rfs); close(lock);
        return 1;
    }

    uint8_t req[16], resp[512];
    int n = 0;
    int a0 = -1, p0 = -1, c0 = -1, n0 = -1, r0 = 0;
    int a1 = -1, p1 = -1, c1 = -1, n1 = -1, r1 = 0;

    if (ipc0 >= 0) {
        g_app = g_pin = g_card = g_apps = -1; g_ready = 0;
        memset(req, 0, sizeof req);
        memset(resp, 0, sizeof resp);
        if (fill_empty_get(req, 0x0200, 90, 12) != 12) return 1;
        puts("SIM on umts_ipc0");
        if (exchange(ipc0, rfs, req, 12, 0x0200, 90, resp, &n))
            puts("ipc0 SIM timeout");
        else {
            a0 = g_app; p0 = g_pin; c0 = g_card; n0 = g_apps; r0 = g_ready;
            printf("ipc0 card=%d apps=%d app_state=%d pin1=%d ready=%d\n", c0, n0, a0, p0, r0);
        }
        close(ipc0);
    }

    g_app = g_pin = g_card = g_apps = -1; g_ready = 0; g_remain = -1;
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    if (fill_empty_get(req, 0x0200, 91, 12) != 12) return 1;
    puts("SIM on umts_ipc1 (other logical)");
    if (exchange(ipc1, rfs, req, 12, 0x0200, 91, resp, &n))
        puts("ipc1 SIM timeout");
    else {
        a1 = g_app; p1 = g_pin; c1 = g_card; n1 = g_apps; r1 = g_ready;
        printf("ipc1 card=%d apps=%d app_state=%d pin1=%d remain=%d ready=%d\n",
            c1, n1, a1, p1, g_remain, r1);
    }

    puts("skip 0x0250 (not built from GetSlotStatus ports; property empty)");
    puts("skip 0x0740 (modem index still unproven)");

    if (r1) {
        puts("ipc1 READY — chase reg/rmnet on ipc1");
        follow_ready(ipc1, rfs);
    } else if (r0) {
        puts("only ipc0 READY unexpected");
    } else {
        one_registration(ipc1, rfs, 92);
        print_rmnet();
    }
    printf("COMPARE ipc0_app=%d ipc0_pin1=%d | ipc1_app=%d ipc1_pin1=%d\n", a0, p0, a1, p1);
    printf("RESULT other_ready=%d\n", r1);
    close(ipc1); close(rfs); close(lock);
    return 0;
}

/* ipc1 apps=0 bring-up: stock per-phone cold boot sends RadioPower ON
   via DoRadioPower on that RIL instance's IoChannel (same 0x0800 layout;
   no slot in packet). Also proven on ipc0 earlier: CardPower 0x024c,
   EnableUicc 0x0249=1. UiccSubscription is RIL request creator only —
   no separate empty SIT builder in sit-stream. AllowData has builder
   but needs allow flag from framework. ONE smallest: RadioPower ON on
   ipc1 only (no OFF). Then re-0x0200 both channels. */
static int run_ipc1_radio_on(void) {
    alarm(50);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    int ipc0 = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int ipc1 = open_node("/dev/umts_ipc1", "/sys/class/cpif/umts_ipc1/dev");
    if (rfs < 0 || ipc1 < 0) {
        puts("channel open failed");
        if (ipc0 >= 0) close(ipc0);
        if (ipc1 >= 0) close(ipc1);
        if (rfs >= 0) close(rfs);
        close(lock);
        return 1;
    }
    printf("ipc0=%s ipc1=ok — stock cold-boot SETs for 2nd phone: RadioPower ON, "
           "optionally CardPower UP / EnableUicc=1 (same opcodes, other channel)\n",
        ipc0 >= 0 ? "ok" : "fail");
    puts("ONE bring-up: RadioPower ON on ipc1 only");
    if (send_radio_power(ipc1, rfs, 1, 100)) {
        close(ipc1); if (ipc0 >= 0) close(ipc0); close(rfs); close(lock);
        return 1;
    }
    puts("wait after ipc1 radio on");
    {
        int64_t until = now_ms() + 3000;
        while (now_ms() < until) {
            struct pollfd pfd[2] = { { ipc1, POLLIN, 0 }, { rfs, POLLIN, 0 } };
            poll(pfd, 2, 200);
            if (pfd[1].revents & POLLIN) service_rfs(rfs);
            if (pfd[0].revents & POLLIN) {
                uint8_t dump[256];
                while (read(ipc1, dump, sizeof dump) > 0) {}
            }
        }
    }

    uint8_t req[16], resp[512];
    int n = 0;
    int a0 = -1, p0 = -1, c0 = -1, n0 = -1, r0 = 0;
    int a1 = -1, p1 = -1, c1 = -1, n1 = -1, r1 = 0;

    if (ipc0 >= 0) {
        g_app = g_pin = g_card = g_apps = -1; g_ready = 0; g_remain = -1;
        memset(req, 0, sizeof req);
        memset(resp, 0, sizeof resp);
        if (fill_empty_get(req, 0x0200, 101, 12) != 12) return 1;
        puts("SIM ipc0 after ipc1 radio");
        if (exchange(ipc0, rfs, req, 12, 0x0200, 101, resp, &n))
            puts("ipc0 SIM timeout");
        else {
            a0 = g_app; p0 = g_pin; c0 = g_card; n0 = g_apps; r0 = g_ready;
            printf("ipc0 card=%d apps=%d app_state=%d pin1=%d ready=%d\n",
                c0, n0, a0, p0, r0);
        }
    }

    g_app = g_pin = g_card = g_apps = -1; g_ready = 0; g_remain = -1;
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    if (fill_empty_get(req, 0x0200, 102, 12) != 12) return 1;
    puts("SIM ipc1 after radio on");
    if (exchange(ipc1, rfs, req, 12, 0x0200, 102, resp, &n))
        puts("ipc1 SIM timeout");
    else {
        a1 = g_app; p1 = g_pin; c1 = g_card; n1 = g_apps; r1 = g_ready;
        printf("ipc1 card=%d apps=%d app_state=%d pin1=%d remain=%d ready=%d\n",
            c1, n1, a1, p1, g_remain, r1);
    }

    /* Life SIM phys without ICCID: ipc0 has the only USIM app; cross-map
       logical0→phys1 ⇒ that app lives on physical slot1. */
    puts("hint: ipc0 alone has apps>=1 + map logical0->phys1 => app SIM on phys slot1");

    puts("skip 0x0250 (no invent ints); skip 0x0740 (index unproven)");
    if (r1) {
        puts("ipc1 READY — chase on ipc1");
        follow_ready(ipc1, rfs);
    } else if (r0) {
        puts("ipc0 READY — chase on ipc0");
        if (ipc0 >= 0) follow_ready(ipc0, rfs);
    } else {
        one_registration(ipc1, rfs, 103);
        print_rmnet();
    }
    printf("COMPARE ipc0_app=%d ipc0_pin1=%d apps=%d | ipc1_app=%d ipc1_pin1=%d apps=%d\n",
        a0, p0, n0, a1, p1, n1);
    printf("RESULT ready_ipc0=%d ready_ipc1=%d\n", r0, r1);
    if (ipc0 >= 0) close(ipc0);
    close(ipc1); close(rfs); close(lock);
    return 0;
}

/* ipc1 CardPower UP (0x024c state=1). Factory UP alone is valid; DOWN→UP
   only for HAL1.6 cycle — not required for a single UP. If apps still 0,
   one EnableUicc 0x0249=1 on ipc1 in the same run. */
static int run_ipc1_card_up(void) {
    alarm(60);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    int ipc0 = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int ipc1 = open_node("/dev/umts_ipc1", "/sys/class/cpif/umts_ipc1/dev");
    if (rfs < 0 || ipc1 < 0) {
        puts("channel open failed");
        if (ipc0 >= 0) close(ipc0);
        if (ipc1 >= 0) close(ipc1);
        if (rfs >= 0) close(rfs);
        close(lock);
        return 1;
    }

    puts("ONE: CardPower UP on ipc1 only (no DOWN)");
    if (send_card_power(ipc1, rfs, 1, 110)) {
        if (ipc0 >= 0) close(ipc0);
        close(ipc1); close(rfs); close(lock);
        return 1;
    }
    puts("wait after ipc1 card up");
    wait_ms(ipc1, rfs, 3000);

    uint8_t req[16], resp[512];
    int n = 0;
    int a0 = -1, p0 = -1, n0 = -1, r0 = 0;
    int a1 = -1, p1 = -1, n1 = -1, r1 = 0;

    if (ipc0 >= 0) {
        g_app = g_pin = g_card = g_apps = -1; g_ready = 0; g_remain = -1;
        memset(req, 0, sizeof req);
        memset(resp, 0, sizeof resp);
        if (fill_empty_get(req, 0x0200, 111, 12) != 12) return 1;
        puts("SIM ipc0 after card up");
        if (exchange(ipc0, rfs, req, 12, 0x0200, 111, resp, &n))
            puts("ipc0 SIM timeout");
        else {
            a0 = g_app; p0 = g_pin; n0 = g_apps; r0 = g_ready;
            printf("ipc0 card=%d apps=%d app_state=%d pin1=%d ready=%d\n",
                g_card, n0, a0, p0, r0);
        }
    }

    g_app = g_pin = g_card = g_apps = -1; g_ready = 0; g_remain = -1;
    memset(req, 0, sizeof req);
    memset(resp, 0, sizeof resp);
    if (fill_empty_get(req, 0x0200, 112, 12) != 12) return 1;
    puts("SIM ipc1 after card up");
    if (exchange(ipc1, rfs, req, 12, 0x0200, 112, resp, &n))
        puts("ipc1 SIM timeout");
    else {
        a1 = g_app; p1 = g_pin; n1 = g_apps; r1 = g_ready;
        printf("ipc1 card=%d apps=%d app_state=%d pin1=%d remain=%d ready=%d\n",
            g_card, n1, a1, p1, g_remain, r1);
    }

    if (!r1 && n1 <= 0) {
        puts("ipc1 still apps=0 — ONE EnableUicc=1 on ipc1");
        memset(req, 0, sizeof req);
        memset(resp, 0, sizeof resp);
        if (fill_set_uicc(req, 113, 1) != 13) return 1;
        if (exchange(ipc1, rfs, req, 13, 0x0249, 113, resp, &n))
            puts("uicc enable timeout");
        else
            printf("uicc enable length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
        wait_ms(ipc1, rfs, 3000);

        if (ipc0 >= 0) {
            g_app = g_pin = g_card = g_apps = -1; g_ready = 0;
            memset(req, 0, sizeof req);
            memset(resp, 0, sizeof resp);
            if (fill_empty_get(req, 0x0200, 114, 12) != 12) return 1;
            puts("SIM ipc0 after uicc");
            if (!exchange(ipc0, rfs, req, 12, 0x0200, 114, resp, &n)) {
                a0 = g_app; p0 = g_pin; n0 = g_apps; r0 = g_ready;
                printf("ipc0 card=%d apps=%d app_state=%d pin1=%d ready=%d\n",
                    g_card, n0, a0, p0, r0);
            }
        }
        g_app = g_pin = g_card = g_apps = -1; g_ready = 0; g_remain = -1;
        memset(req, 0, sizeof req);
        memset(resp, 0, sizeof resp);
        if (fill_empty_get(req, 0x0200, 115, 12) != 12) return 1;
        puts("SIM ipc1 after uicc");
        if (!exchange(ipc1, rfs, req, 12, 0x0200, 115, resp, &n)) {
            a1 = g_app; p1 = g_pin; n1 = g_apps; r1 = g_ready;
            printf("ipc1 card=%d apps=%d app_state=%d pin1=%d remain=%d ready=%d\n",
                g_card, n1, a1, p1, g_remain, r1);
        }
    }

    puts("skip 0x0250; skip 0x0740");
    if (r1) {
        puts("ipc1 READY — chase");
        follow_ready(ipc1, rfs);
    } else if (r0) {
        puts("ipc0 READY — chase");
        if (ipc0 >= 0) follow_ready(ipc0, rfs);
    } else {
        one_registration(ipc1, rfs, 116);
        print_rmnet();
    }
    printf("COMPARE ipc0_app=%d pin1=%d apps=%d | ipc1_app=%d pin1=%d apps=%d\n",
        a0, p0, n0, a1, p1, n1);
    printf("RESULT ready_ipc0=%d ready_ipc1=%d\n", r0, r1);
    if (ipc0 >= 0) close(ipc0);
    close(ipc1); close(rfs); close(lock);
    return 0;
}

/* (1) ipc1 CardPower DOWN(4)→UP(1). (2) ipc0 PIN angle (b): Radio OFF→ON
   then poll 0x0200 for transient app_state/pin1 changes.
   (a) skipped — no stock path builds 0x0250 from GetSlotStatus ports.
   (c) OnGetSimStatusDone only BuildSimStatus→RIL notify; no second SIT
   opcode for PIN+DISABLED (AutoVerifyPin needs encrypted prop). */
static int run_ipc1_cycle_ipc0_radio_poll(void) {
    alarm(90);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    int ipc0 = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int ipc1 = open_node("/dev/umts_ipc1", "/sys/class/cpif/umts_ipc1/dev");
    if (rfs < 0 || ipc0 < 0 || ipc1 < 0) {
        puts("channel open failed");
        if (ipc0 >= 0) close(ipc0);
        if (ipc1 >= 0) close(ipc1);
        if (rfs >= 0) close(rfs);
        close(lock);
        return 1;
    }
    uint8_t req[20], resp[512];
    int n = 0;

    puts("angle(a) skip 0x0250 — SetSlotMapping needs property; no GetSlotStatus→mapping builder");
    puts("angle(c) — OnGetSimStatusDone: no second SIT after PIN+DISABLED");

    puts("ONE: ipc1 CardPower DOWN→UP");
    if (send_card_power(ipc1, rfs, 4, 120)) goto out_fail;
    wait_ms(ipc1, rfs, 2000);
    if (send_card_power(ipc1, rfs, 1, 121)) goto out_fail;
    wait_ms(ipc1, rfs, 3000);

    int a0 = -1, p0 = -1, n0 = -1, r0 = 0;
    int a1 = -1, p1 = -1, n1 = -1, r1 = 0;
    g_app = g_pin = g_card = g_apps = -1; g_ready = 0;
    memset(req, 0, sizeof req); memset(resp, 0, sizeof resp);
    if (fill_empty_get(req, 0x0200, 122, 12) != 12) return 1;
    puts("SIM ipc0 after ipc1 DOWN-UP");
    if (!exchange(ipc0, rfs, req, 12, 0x0200, 122, resp, &n)) {
        a0 = g_app; p0 = g_pin; n0 = g_apps; r0 = g_ready;
        printf("ipc0 card=%d apps=%d app_state=%d pin1=%d ready=%d\n",
            g_card, n0, a0, p0, r0);
    }
    g_app = g_pin = g_card = g_apps = -1; g_ready = 0; g_remain = -1;
    memset(req, 0, sizeof req); memset(resp, 0, sizeof resp);
    if (fill_empty_get(req, 0x0200, 123, 12) != 12) return 1;
    puts("SIM ipc1 after DOWN-UP");
    if (!exchange(ipc1, rfs, req, 12, 0x0200, 123, resp, &n)) {
        a1 = g_app; p1 = g_pin; n1 = g_apps; r1 = g_ready;
        printf("ipc1 card=%d apps=%d app_state=%d pin1=%d ready=%d\n",
            g_card, n1, a1, p1, r1);
    }

    puts("angle(b): ipc0 Radio OFF→ON then poll 0x0200 for transient pin1/app_state");
    if (send_radio_power(ipc0, rfs, 0, 124)) goto out_fail;
    wait_ms(ipc0, rfs, 2000);
    if (send_radio_power(ipc0, rfs, 1, 125)) goto out_fail;

    int seen_diff = 0;
    int first_app = -2, first_pin = -2;
    for (int i = 0; i < 8; i++) {
        wait_ms(ipc0, rfs, 2500);
        g_app = g_pin = g_card = g_apps = -1; g_ready = 0; g_remain = -1;
        memset(req, 0, sizeof req); memset(resp, 0, sizeof resp);
        if (fill_empty_get(req, 0x0200, 126 + (uint32_t)i, 12) != 12) return 1;
        printf("poll%d ", i);
        if (exchange(ipc0, rfs, req, 12, 0x0200, 126 + (uint32_t)i, resp, &n)) {
            puts("timeout");
            continue;
        }
        printf("poll%d app_state=%d pin1=%d apps=%d ready=%d\n",
            i, g_app, g_pin, g_apps, g_ready);
        if (first_app == -2) { first_app = g_app; first_pin = g_pin; }
        else if (g_app != first_app || g_pin != first_pin) {
            seen_diff = 1;
            printf("TRANSIENT change from app=%d pin1=%d\n", first_app, first_pin);
        }
        a0 = g_app; p0 = g_pin; n0 = g_apps; r0 = g_ready;
        if (r0) break;
    }
    printf("poll_stable_diff=%d final_app=%d final_pin1=%d\n", seen_diff, a0, p0);

    puts("skip 0x0250; skip 0x0740");
    if (r0) {
        puts("ipc0 READY — chase");
        follow_ready(ipc0, rfs);
    } else if (r1) {
        puts("ipc1 READY — chase");
        follow_ready(ipc1, rfs);
    } else {
        one_registration(ipc0, rfs, 140);
        print_rmnet();
    }
    printf("COMPARE ipc0_app=%d pin1=%d apps=%d | ipc1_app=%d pin1=%d apps=%d\n",
        a0, p0, n0, a1, p1, n1);
    printf("RESULT ready_ipc0=%d ready_ipc1=%d\n", r0, r1);
    close(ipc0); close(ipc1); close(rfs); close(lock);
    return 0;
out_fail:
    close(ipc0); close(ipc1); close(rfs); close(lock);
    return 1;
}

/* BuildGetVoiceOperation: id 0x091b, length 12, empty. Named unsent on
   this boot path; stock misc GET after SIM present. */
static int run_voice_op_get(void) {
    alarm(40);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }
    uint8_t req[16], resp[512];
    int n = 0;
    puts("SIM before voice-op");
    if (query_sim(ipc, rfs, 150)) puts("SIM timeout");
    memset(req, 0, sizeof req);
    if (fill_empty_get(req, 0x091b, 151, 12) != 12) return 1;
    puts("voice operation get 0x091b");
    if (exchange(ipc, rfs, req, 12, 0x091b, 151, resp, &n))
        puts("voice-op timeout");
    else {
        printf("voice-op length=%d error_raw=%u", n, n > 10 ? resp[10] : 0);
        if (n >= 13 && !resp[10]) printf(" mode_raw=%u", resp[12]);
        puts("");
    }
    wait_ms(ipc, rfs, 2000);
    puts("SIM after voice-op");
    if (query_sim(ipc, rfs, 152)) puts("SIM timeout");
    one_registration(ipc, rfs, 153);
    print_rmnet();
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    if (g_ready) follow_ready(ipc, rfs);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* After ONLINE: RadioPower ON, poll 0x0200, then TS 102.221 SELECT MF
   (00 A4 00 0C 02 3F00) + STATUS (80 F2 00 00) via proven 0x020c basic
   TransmitApdu. Report SW only; no CHV/VERIFY. */
static int run_cold_apdu_status(void) {
    alarm(70);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }

    puts("ordered: RadioPower ON");
    if (send_radio_power(ipc, rfs, 1, 160)) {
        close(ipc); close(rfs); close(lock);
        return 1;
    }
    wait_ms(ipc, rfs, 2000);

    int identical = 1;
    int first_app = -2, first_pin = -2;
    int aid_n = 0;
    uint8_t aid[16];
    memset(aid, 0, sizeof aid);
    for (int i = 0; i < 4; i++) {
        g_app = g_pin = g_card = g_apps = -1; g_ready = 0; g_remain = -1;
        g_aid_len = -1; memset(g_aid, 0, sizeof g_aid);
        if (query_sim(ipc, rfs, 161 + (uint32_t)i)) puts("SIM poll timeout");
        else {
            printf("poll%d app=%d pin1=%d apps=%d card=%d ready=%d\n",
                i, g_app, g_pin, g_apps, g_card, g_ready);
            if (first_app == -2) { first_app = g_app; first_pin = g_pin; }
            else if (g_app != first_app || g_pin != first_pin) identical = 0;
            if (aid_n == 0 && g_aid_len > 0 && g_aid_len <= 16) {
                aid_n = g_aid_len;
                memcpy(aid, g_aid, (size_t)aid_n);
            }
        }
        memset(g_aid, 0, sizeof g_aid);
        if (i < 3) wait_ms(ipc, rfs, 2000);
    }
    printf("pin_disabled_identical=%d\n", identical && first_app == 2 && first_pin == 3);

    if (g_card == 0) {
        puts("card ABSENT — CardPower UP");
        send_card_power(ipc, rfs, 1, 170);
        wait_ms(ipc, rfs, 3000);
        query_sim(ipc, rfs, 171);
    }

    unsigned sw = 0;
    uint8_t req[64];
    /* TS 102.221 SELECT by file id, P2=0C (no FCI); FID MF=3F00 */
    uint8_t mf[2] = { 0x3f, 0x00 };
    int pkt = fill_basic_apdu(req, 172, 0x00, 0xa4, 0x00, 0x0c, mf, 2);
    memset(mf, 0, sizeof mf);
    if (pkt < 23) puts("select mf not sent");
    else {
        send_basic_sw(ipc, rfs, req, pkt, 172, "select_mf", &sw);
        printf("select_mf sw=%04x\n", sw);
    }

    /* TS 102.221 STATUS (INS=F2) after MF select — not VERIFY CHV */
    pkt = fill_basic_apdu(req, 173, 0x80, 0xf2, 0x00, 0x00, NULL, 0);
    if (pkt < 23) puts("status_mf not sent");
    else {
        send_basic_sw(ipc, rfs, req, pkt, 173, "status_mf", &sw);
        printf("status_mf sw=%04x\n", sw);
    }

    /* SELECT ADF by AID (from GetSimStatus, not printed) then STATUS */
    if (aid_n > 0) {
        pkt = fill_basic_apdu(req, 174, 0x00, 0xa4, 0x04, 0x00, aid, (unsigned)aid_n);
        if (pkt < 23) puts("select_adf not sent");
        else {
            send_basic_sw(ipc, rfs, req, pkt, 174, "select_adf", &sw);
            printf("select_adf sw=%04x\n", sw);
            if (select_ok(1, sw) || sw == 0x9000 || (sw & 0xff00) == 0x6100) {
                pkt = fill_basic_apdu(req, 175, 0x80, 0xf2, 0x00, 0x00, NULL, 0);
                if (pkt >= 23) {
                    send_basic_sw(ipc, rfs, req, pkt, 175, "status_adf", &sw);
                    printf("status_adf sw=%04x\n", sw);
                }
                /* application-oriented STATUS P2=01 (TS 102.221) */
                pkt = fill_basic_apdu(req, 176, 0x80, 0xf2, 0x00, 0x01, NULL, 0);
                if (pkt >= 23) {
                    send_basic_sw(ipc, rfs, req, pkt, 176, "status_adf_p2", &sw);
                    printf("status_adf_p2 sw=%04x\n", sw);
                }
            }
        }
    } else
        puts("select_adf skipped aid_len=0");
    memset(aid, 0, sizeof aid);

    puts("SIM after STATUS APDUs");
    g_app = g_pin = -1; g_ready = 0;
    if (query_sim(ipc, rfs, 177)) puts("SIM timeout");
    one_registration(ipc, rfs, 178);
    print_rmnet();
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    if (g_ready) follow_ready(ipc, rfs);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* BuildSimOpenChannelWithP2: id 0x0247, length 30.
   Byte 12 = AID length, bytes 13..28 = AID (max 16), byte 29 = P2.
   OpenSimChannelHandler: if request P2 == -1 → BuildSimOpenChannel (0x020d);
   else P2 is the SELECT P2 (stock often 0 = return FCI). */
static int fill_open_channel_p2(uint8_t *req, uint32_t token,
                                const uint8_t *aid, unsigned aid_n, uint8_t p2) {
    if (!aid || aid_n == 0 || aid_n > 16) return -1;
    memset(req, 0, 30);
    req[2] = 0x47;
    req[3] = 0x02;
    req[4] = 30;
    req[6] = (uint8_t)token;
    req[12] = (uint8_t)aid_n;
    memcpy(req + 13, aid, aid_n);
    req[29] = p2;
    return 30;
}

/* BuildSimTransmitApduChannel: id 0x020f, base length 38.
   Words: session@12 CLA@16 INS@20 P1@24 P2@28 P3@32; halfword data_len@36;
   data@38. Handler rewrites CLA with logical-channel bits from session. */
static int fill_channel_apdu(uint8_t *req, uint32_t token, uint32_t session,
                             uint8_t cla, uint8_t ins, uint8_t p1, uint8_t p2,
                             uint8_t p3) {
    /* Encode CLA like TransmitSimApduChannelHandler for session < 4. */
    uint8_t ch_cla = (uint8_t)((cla & 0xfc) | (session & 3));
    memset(req, 0, 38);
    req[2] = 0x0f;
    req[3] = 0x02;
    req[4] = 38;
    req[6] = (uint8_t)token;
    req[12] = (uint8_t)session;
    req[13] = (uint8_t)(session >> 8);
    req[14] = (uint8_t)(session >> 16);
    req[15] = (uint8_t)(session >> 24);
    req[16] = ch_cla;
    req[20] = ins;
    req[24] = p1;
    req[28] = p2;
    req[32] = p3;
    /* data_len halfword @36 stays 0 */
    return 38;
}

static int send_channel_sw(int ipc, int rfs, uint8_t *req, int pkt, uint32_t token,
                           const char *name, unsigned *sw) {
    uint8_t resp[512];
    int n = 0;
    *sw = 0;
    printf("%s\n", name);
    int rc = exchange(ipc, rfs, req, (size_t)pkt, 0x020f, token, resp, &n);
    memset(req, 0, (size_t)pkt);
    if (rc) { printf("%s timeout\n", name); return -1; }
    /* Channel response: SW1@12 SW2@13, apdu_len@14, data@16 (not basic 0x020c). */
    unsigned err = n > 10 ? resp[10] : 0;
    int have = 0;
    if (n >= 14 && le16(resp + 2) == 0x020f && !err) {
        *sw = ((unsigned)resp[12] << 8) | resp[13];
        unsigned alen = (unsigned)(int16_t)le16(resp + 14);
        have = 1;
        printf("%s response: length=%d error_raw=%u sw=%04x apdu_len=%u\n",
            name, n, err, *sw, alen);
    } else
        printf("%s response: length=%d error_raw=%u sw_present=0\n", name, n, err);
    memset(resp, 0, sizeof resp);
    return have;
}

/* Dense 0x0200 watch for physical tray reseat. Logs every app_state
   transition plus Present inference (0x0200 has no +0xBF6). No secrets. */
static int run_tray_watch(void) {
    alarm(200);
    setvbuf(stdout, NULL, _IOLBF, 0);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }

    puts("tray-watch: dense 0x0200 200ms for 180s; Present inferred from app_state only");
    g_quiet_sim = 1;
    int last_card = -2, last_app = -2, last_pin = -2, last_apps = -2;
    int saw_absent = 0, saw_unknown = 0, saw_detected = 0, saw_pin = 0, saw_ready = 0;
    int pin_disabled_identical = 0;
    uint32_t token = 1;
    int64_t end = now_ms() + 180000;
    while (now_ms() < end) {
        g_app = g_pin = g_card = g_apps = -1;
        g_ready = 0;
        uint8_t req[12], resp[512];
        int n = 0;
        memset(req, 0, sizeof req);
        req[2] = 0x00; req[3] = 0x02; req[4] = 12; req[6] = (uint8_t)token;
        req[7] = (uint8_t)(token >> 8);
        int rc = exchange(ipc, rfs, req, 12, 0x0200, token, resp, &n);
        token++;
        if (rc) {
            /* keep draining; do not abort watch */
        } else {
            if (g_card == 0) saw_absent = 1;
            if (g_app == 0) saw_unknown = 1;
            if (g_app == 1) saw_detected = 1;
            if (g_app == 2) {
                saw_pin = 1;
                if (g_pin == 3) pin_disabled_identical = 1;
            }
            if (g_app == 5) saw_ready = 1;
            if (g_card != last_card || g_app != last_app || g_pin != last_pin || g_apps != last_apps) {
                int pin_changed = (last_pin >= 0 && g_pin >= 0 && g_pin != last_pin);
                printf("TRANSITION card=%d apps=%d app=%d(%s) pin1=%d present_infer=%s\n",
                       g_card, g_apps, g_app,
                       g_app < 0 ? "n/a" : app_state_name((unsigned)g_app),
                       g_pin, present_infer_from_app(g_apps, g_app));
                /* ABSENT→PRESENT after tray pull */
                if (last_card == 0 && g_card == 1)
                    puts("tray-watch: ABSENT→PRESENT observed");
                last_card = g_card;
                last_app = g_app;
                last_pin = g_pin;
                last_apps = g_apps;
                if (pin_changed && g_pin != 3) {
                    puts("tray-watch: pin1 changed from DISABLED");
                    break;
                }
            }
            /* Stop early so caller can chase bearer */
            if (g_ready || g_app == 1 || g_app == 5) break;
            if (saw_absent && g_card == 1 && g_app >= 0 && g_app != 2) break;
        }
        wait_ms(ipc, rfs, 200);
    }
    print_rmnet();
    printf("TRAY RESULT saw_absent=%d saw_unknown=%d saw_detected=%d saw_pin=%d "
           "saw_ready=%d pin_disabled_identical=%d last_app=%d last_card=%d "
           "present_infer=%s ready=%d\n",
           saw_absent, saw_unknown, saw_detected, saw_pin, saw_ready,
           pin_disabled_identical, last_app, last_card,
           present_infer_from_app(last_apps, last_app), g_ready);
    close(ipc); close(rfs); close(lock);
    return 0;
}

/* RO inventory of AP-visible surfaces that might relate to Present/+0xBF6.
   Does not poke. Confirms PresentObj is not mapped to userspace. */
static int run_peek_present_surfaces(void) {
    alarm(30);
    setvbuf(stdout, NULL, _IOLBF, 0);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    puts("=== peek-present-surfaces (RO only; no poke) ===");
    printf("modem_state=%s\n", state);

    f = fopen("/sys/devices/platform/cpif/sim/ds_detect", "r");
    if (f) {
        int ds = -1;
        if (fscanf(f, "%d", &ds) == 1)
            printf("sim/ds_detect=%d (Dual-SIM mailbox config ONLY; not Present enum)\n", ds);
        fclose(f);
    } else
        puts("sim/ds_detect: unavailable");

    f = fopen("/sys/devices/platform/cpif/info_region", "r");
    if (f) {
        char line[256];
        puts("info_region (ctrl/srinfo offsets; no PresentObj):");
        while (fgets(line, sizeof line, f)) {
            /* print only non-secret structural lines */
            if (strstr(line, "srinfo") || strstr(line, "capability") ||
                strstr(line, "united_status") || strstr(line, "ap2cp_msg") ||
                strstr(line, "cp2ap_msg") || strstr(line, "version") ||
                strstr(line, "offset"))
                fputs(line, stdout);
        }
        fclose(f);
    } else
        puts("info_region: unavailable");

    f = fopen("/sys/devices/platform/cpif/legacy/status", "r");
    if (f) {
        char line[256];
        int n = 0;
        puts("legacy/status (IPC ring heads only):");
        while (n < 12 && fgets(line, sizeof line, f)) {
            fputs(line, stdout);
            n++;
        }
        fclose(f);
    } else
        puts("legacy/status: unavailable");

    puts("CPIF iod mmap: none in s5300 sources (no userspace CP DRAM map)");
    puts("/dev/mem: not used (prior ENXIO; no PresentObj PA)");

    if (strcmp(state, "ONLINE") == 0) {
        int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
        if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
            puts("status lock busy; skip 0x0200 infer");
        } else {
            int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
            int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
            if (ipc >= 0 && rfs >= 0) {
                g_quiet_sim = 0;
                g_app = g_pin = g_card = g_apps = -1;
                g_ready = 0;
                if (query_sim(ipc, rfs, 1))
                    puts("0x0200 timeout");
                else
                    printf("0x0200 infer: card=%d apps=%d app=%d(%s) pin1=%d present_infer=%s\n",
                           g_card, g_apps, g_app,
                           g_app < 0 ? "n/a" : app_state_name((unsigned)g_app),
                           g_pin, present_infer_from_app(g_apps, g_app));
                close(ipc); close(rfs);
            }
            close(lock);
        }
    }

    puts("GAP: PresentObj/+0xBF6 lives in CP heap; no AP RO mapping to peek byte.");
    puts("GAP: no sysfs/ioctl/debugfs/mailbox write to Present or SET_APP.");
    puts("sim/ds_detect store only sets Dual-SIM module param — not Present=2.");
    return 0;
}

/* SELECT ADF + STATUS (decode C6), then ONE OpenChannelWithP2 (P2=0),
   channel STATUS, recheck 0x0200. No secrets printed. */
static int run_och_p2_status(void) {
    alarm(70);
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) { puts("requires ONLINE"); return 1; }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC, 0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) { puts("status lock busy"); return 1; }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) { puts("channel open failed"); return 1; }

    puts("SIM before och-p2");
    if (query_sim(ipc, rfs, 200)) { puts("SIM timeout"); close(ipc); close(rfs); close(lock); return 1; }
    int cp_pin1 = g_pin;
    int aid_n = g_aid_len;
    uint8_t aid[16];
    memset(aid, 0, sizeof aid);
    if (aid_n > 0 && aid_n <= 16) memcpy(aid, g_aid, (size_t)aid_n);
    memset(g_aid, 0, sizeof g_aid);
    if (aid_n <= 0) {
        puts("no aid; abort");
        close(ipc); close(rfs); close(lock);
        return 1;
    }

    unsigned sw = 0;
    uint8_t req[64], body[256];
    unsigned blen = 0;
    int pkt = fill_basic_apdu(req, 201, 0x00, 0xa4, 0x04, 0x00, aid, (unsigned)aid_n);
    if (pkt >= 23)
        send_basic_sw(ipc, rfs, req, pkt, 201, "select_adf", &sw);
    printf("select_adf sw=%04x\n", sw);

    pkt = fill_basic_apdu(req, 202, 0x80, 0xf2, 0x00, 0x00, NULL, 0);
    if (pkt >= 23) {
        send_basic_sw_body(ipc, rfs, req, pkt, 202, "status_adf", &sw, body, &blen);
        printf("status_adf sw=%04x\n", sw);
        if (sw == 0x9000 || (sw & 0xff00) == 0x6100)
            decode_pin_status_c6(body, blen, cp_pin1);
        else
            puts("tlv_c6 skipped bad sw");
    }
    memset(body, 0, sizeof body);

    /* ONE OpenChannelWithP2, P2=0 (SELECT return FCI) — factory path when P2!=-1 */
    puts("ONE: OpenChannelWithP2 p2=0");
    if (fill_open_channel_p2(req, 203, aid, (unsigned)aid_n, 0) != 30) {
        puts("och-p2 fill failed");
        memset(aid, 0, sizeof aid);
        close(ipc); close(rfs); close(lock);
        return 1;
    }
    memset(aid, 0, sizeof aid);
    uint8_t resp[512];
    int n = 0;
    int rc = exchange(ipc, rfs, req, 30, 0x0247, 203, resp, &n);
    memset(req, 0, 30);
    uint32_t session = 0;
    if (rc) puts("och-p2 timeout");
    else {
        unsigned err = n > 10 ? resp[10] : 0;
        printf("och-p2 length=%d error_raw=%u", n, err);
        if (!err && n >= 18) {
            session = le32(resp + 12);
            printf(" session_raw=%u sw=%02x%02x", session, resp[16], resp[17]);
        }
        puts("");
    }

    if (session > 0 && session < 4) {
        pkt = fill_channel_apdu(req, 204, session, 0x80, 0xf2, 0x00, 0x00, 0x00);
        if (pkt == 38)
            send_channel_sw(ipc, rfs, req, pkt, 204, "status_ch", &sw);
        printf("status_ch sw=%04x session=%u\n", sw, session);
        /* ISO 7816-4 6CXX → retry once with exact Le in P3 */
        if ((sw & 0xff00) == 0x6c00) {
            uint8_t le = (uint8_t)(sw & 0xff);
            pkt = fill_channel_apdu(req, 205, session, 0x80, 0xf2, 0x00, 0x00, le);
            if (pkt == 38)
                send_channel_sw(ipc, rfs, req, pkt, 205, "status_ch_le", &sw);
            printf("status_ch_le sw=%04x le=%u\n", sw, le);
        } else {
            pkt = fill_channel_apdu(req, 205, session, 0x80, 0xf2, 0x00, 0x01, 0x00);
            if (pkt == 38)
                send_channel_sw(ipc, rfs, req, pkt, 205, "status_ch_p2", &sw);
            printf("status_ch_p2 sw=%04x\n", sw);
        }
        if (fill_close_channel(req, 206, session) == 16) {
            puts("close channel");
            n = 0;
            if (!exchange(ipc, rfs, req, 16, 0x020e, 206, resp, &n))
                printf("close channel length=%d error_raw=%u\n", n, n > 10 ? resp[10] : 0);
            memset(req, 0, 16);
        }
    } else
        puts("channel STATUS skipped (no session)");
    memset(resp, 0, sizeof resp);

    puts("SIM after och-p2 + channel STATUS");
    g_app = g_pin = -1; g_ready = 0;
    if (query_sim(ipc, rfs, 207)) puts("SIM timeout");
    one_registration(ipc, rfs, 208);
    print_rmnet();
    printf("RESULT app_state=%d pin1=%d remain=%d ready=%d\n", g_app, g_pin, g_remain, g_ready);
    if (g_ready) follow_ready(ipc, rfs);
    close(ipc); close(rfs); close(lock);
    return 0;
}

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "self-test")) {
        uint8_t p[16] = {1,0,0,2,15,0,1,0,0,0,0,0,0,0,0};
        for (size_t n=0; n<15; n++) if (frame_size(p,n)) return 1;
        if (frame_size(p,15)!=15 || le32(p+6)!=1) return 1;
        p[4]=11; if (frame_size(p,15)!=-1) return 1;
        p[0]=2; p[4]=8; if (frame_size(p,15)!=8) return 1;
        p[0]=3; if (frame_size(p,15)!=-1) return 1;
        uint8_t r[16]={1,0,1,8,16,0,2,0,0,0,0,0,10,0,0,0};
        if (frame_size(r,15)!=0 || frame_size(r,16)!=16 ||
            le16(r+2)!=0x801 || le32(r+6)!=2 || le32(r+12)!=10) return 1;
        r[2]=1; r[3]=7; r[6]=3; r[12]=0; r[13]=0; r[15]=0;
        if (frame_size(r,15)!=0 || frame_size(r,16)!=16 ||
            le16(r+2)!=0x701 || le32(r+6)!=3) return 1;
        uint8_t lte[16]={0,0,0x0a,0x07,16,0,5,0,0,0,0,0,11,0,0,0};
        if (frame_size(lte,15)!=0 || frame_size(lte,16)!=16 ||
            le16(lte+2)!=0x070a || le32(lte+6)!=5 || le32(lte+12)!=11) return 1;
        if (stored_card_state(1)!=1 || stored_card_state(3)!=1 || stored_card_state(4)!=3) return 1;
        if (strcmp(card_state_name(1),"PRESENT") || strcmp(app_state_name(5),"READY")) return 1;
        if (strcmp(app_class_name(2),"SIM/USIM") || strcmp(radio_tech_name(14),"LTE")) return 1;
        if (strcmp(radio_tech_name(3),"UMTS")) return 1;
        uint8_t pin1[38], pin2[38];
        if (fill_verify_pin(pin1, 7, 1) != 38 || fill_verify_pin(pin2, 8, 2) != 38) return 1;
        if (fill_verify_pin(pin1, 7, 0) != -1 || fill_verify_pin(pin1, 7, 3) != -1) return 1;
        if (fill_verify_pin(pin1, 7, 1) != 38) return 1;
        if (frame_size(pin1, 37) != 0 || frame_size(pin1, 38) != 38 || frame_size(pin2, 38) != 38) return 1;
        if (le16(pin1+2) != 0x201 || le32(pin1+6) != 7 || pin1[12] != 4 || pin1[21] != 0) return 1;
        if (le16(pin2+2) != 0x201 || le32(pin2+6) != 8 || pin2[12] != 4 || pin2[21] != 0) return 1;
        if (pin1[13] == pin2[13]) return 1;
        for (int i = 0; i < 4; i++) {
            if (pin1[13+i] != pin1[13] || pin2[13+i] != pin2[13]) return 1;
            if (pin1[13+i] < '0' || pin1[13+i] > '9' || pin2[13+i] < '0' || pin2[13+i] > '9') return 1;
        }
        for (int i = 17; i < 38; i++) if (pin1[i] || pin2[i]) return 1;
        uint8_t aid_fix[2] = {0x11, 0x22};
        if (fill_verify_pin_aid(pin1, 9, 1, aid_fix, 2) != 38) return 1;
        if (pin1[21] != 2 || pin1[22] != aid_fix[0] || pin1[23] != aid_fix[1] || pin1[24]) return 1;
        if (fill_verify_pin_aid(pin1, 9, 1, aid_fix, 0) != -1) return 1;
        memset(aid_fix, 0, sizeof aid_fix);
        memset(pin1, 0, sizeof pin1);
        memset(pin2, 0, sizeof pin2);
        uint8_t power[13];
        if (fill_card_power(power, 70, 1) != 13 || fill_card_power(power, 70, 0) != -1) return 1;
        if (fill_card_power(power, 70, 4) != 13) return 1;
        if (le16(power+2) != 0x024c || power[4] != 13 || le32(power+6) != 70 || power[12] != 4) return 1;
        if (fill_card_power(power, 70, 1) != 13) return 1;
        if (le16(power+2) != 0x024c || power[4] != 13 || le32(power+6) != 70 || power[12] != 1) return 1;
        memset(power, 0, sizeof power);
        uint8_t rp[18], em[38];
        if (fill_radio_power(rp, 191, 0) != 18 || le16(rp+2) != 0x0800 || rp[4] != 18 || le32(rp+12) != 1) return 1;
        if (fill_radio_power(rp, 192, 1) != 18 || le32(rp+12) != 2 || rp[16] || rp[17]) return 1;
        memset(rp, 0, sizeof rp);
        if (fill_verify_pin_empty(em, 194) != 38 || le16(em+2) != 0x0201 || em[4] != 38 || em[12]) return 1;
        for (int i = 13; i < 38; i++) if (em[i]) return 1;
        memset(em, 0, sizeof em);
        uint8_t uicc[13];
        if (fill_set_uicc(uicc, 241, 1) != 13 || fill_set_uicc(uicc, 241, 0) != -1) return 1;
        if (le16(uicc+2) != 0x0249 || uicc[4] != 13 || uicc[12] != 1) return 1;
        memset(uicc, 0, sizeof uicc);
        uint8_t cfg[13];
        if (fill_modems_config(cfg, 90) != 13) return 1;
        if (le16(cfg+2) != 0x093f || cfg[4] != 13 || le32(cfg+6) != 90 || cfg[12] != 1) return 1;
        for (int i = 0; i < 13; i++) if (i != 2 && i != 3 && i != 4 && i != 6 && i != 12 && cfg[i]) return 1;
        memset(cfg, 0, sizeof cfg);
        uint8_t clk[18];
        time_t now = time(NULL);
        struct tm *tm = localtime(&now);
        if (!tm || fill_ap_time(clk, 96) != 18) return 1;
        if (le16(clk+2) != 0x0949 || clk[4] != 18 || le32(clk+6) != 96) return 1;
        if (clk[12] != (uint8_t)tm->tm_year || clk[13] != (uint8_t)tm->tm_mon ||
            clk[14] != (uint8_t)tm->tm_mday || clk[15] != (uint8_t)tm->tm_hour ||
            clk[16] != (uint8_t)tm->tm_min || clk[17] != (uint8_t)tm->tm_sec) return 1;
        memset(clk, 0, sizeof clk);
        uint8_t info[140];
        if (fill_device_info(info, 97, "m", "b", "n", "r") != 140) return 1;
        if (le16(info+2) != 0x0922 || info[4] != 140 || info[12] != 'm' || info[44] != 'b' ||
            info[76] != 'n' || info[108] != 'r' || info[13] || info[45]) return 1;
        memset(info, 0, sizeof info);
        uint8_t chv[39];
        if (fill_transmit_chv(chv, 101, 1) != 39 || fill_transmit_chv(chv, 101, 0) != -1) return 1;
        if (fill_transmit_chv(chv, 101, 1) != 39) return 1;
        if (le16(chv+2) != 0x020c || chv[4] != 39 || le32(chv+12) != 0 || le16(chv+16) != 13) return 1;
        if (chv[18] || chv[19] != 0x20 || chv[20] || chv[21] != 0x01 || chv[22] != 0x08) return 1;
        if (chv[23] != chv[24] || chv[23] != chv[25] || chv[23] != chv[26]) return 1;
        if (chv[27] != 0xff || chv[30] != 0xff || chv[31] || chv[38]) return 1;
        uint8_t chv2[39];
        if (fill_transmit_chv(chv2, 103, 2) != 39 || chv2[23] == chv[23]) return 1;
        memset(chv, 0, sizeof chv);
        uint8_t sel[64];
        uint8_t fake[2] = {0x11, 0x22};
        int spkt = fill_basic_apdu(sel, 111, 0x00, 0xa4, 0x04, 0x00, fake, 2);
        if (spkt != 27 || le16(sel+2) != 0x020c || sel[4] != 27 || le16(sel+16) != 7) return 1;
        if (sel[18] || sel[19] != 0xa4 || sel[20] != 0x04 || sel[21] || sel[22] != 2) return 1;
        if (sel[23] != 0x11 || sel[24] != 0x22 || sel[25] || sel[26]) return 1;
        memset(sel, 0, sizeof sel);
        memset(fake, 0, sizeof fake);
        uint8_t lockreq[12];
        if (fill_empty_get(lockreq, 0x4104, 130, 12) != 12 || le16(lockreq+2) != 0x4104 || lockreq[4] != 12) return 1;
        uint8_t lresp[20] = {0};
        lresp[2] = 0x04; lresp[3] = 0x41; lresp[12] = 0x21; lresp[13] = 0x03; lresp[14] = 0x14; lresp[15] = 2; lresp[16] = 3; lresp[19] = 0x41;
        unsigned policy = 0, st = 0, kind = 0, mx = 0, rm = 0;
        if (lock_fields(lresp, 20, &policy, &st, &kind, &mx, &rm)) return 1;
        if (policy != 1 || st != 3 || kind != 4 || mx != 2 || rm != 3) return 1;
        memset(lockreq, 0, sizeof lockreq);
        memset(lresp, 0, sizeof lresp);
        uint8_t fac[71], faid[2] = {0xaa, 0xbb};
        if (fill_get_facility_sc(fac, 141, NULL, 0) != 71) return 1;
        if (le16(fac + 2) != 0x0209 || fac[4] != 71 || fac[12] != 3 || fac[13] != 0 || fac[53] != 7 || fac[54] != 0) return 1;
        if (fill_get_facility_sc(fac, 141, faid, 2) != 71) return 1;
        if (fac[54] != 2 || fac[55] != 0xaa || fac[56] != 0xbb || fac[53] != 7) return 1;
        memset(fac, 0, sizeof fac);
        memset(faid, 0, sizeof faid);
        uint8_t setf[72];
        if (fill_set_facility_sc_enable(setf, 241, 1, NULL, 0) != 72) return 1;
        if (le16(setf + 2) != 0x020a || setf[4] != 72 || setf[12] != 3 || setf[13] != 1 ||
            setf[14] != 4 || setf[54] != 7 || setf[55] != 0) return 1;
        /* digits present but never asserted as printable text in the test log */
        if (!setf[15] || !setf[16] || !setf[17] || !setf[18]) return 1;
        memset(setf, 0, sizeof setf);
        uint8_t atr[12];
        if (fill_empty_get(atr, 0x0212, 150, 12) != 12 || le16(atr+2) != 0x0212 || atr[4] != 12) return 1;
        memset(atr, 0, sizeof atr);
        uint8_t och[29];
        uint8_t oaid[2] = {0x11, 0x22};
        memset(och, 0, 29);
        och[2] = 0x0d; och[3] = 0x02; och[4] = 29; och[6] = 152; och[12] = 2;
        memcpy(och + 13, oaid, 2);
        if (le16(och+2) != 0x020d || och[4] != 29 || och[12] != 2 || och[13] != 0x11 || och[14] != 0x22) return 1;
        memset(och, 0, sizeof och);
        memset(oaid, 0, sizeof oaid);
        uint8_t clch[16], slot[12];
        if (fill_close_channel(clch, 161, 1) != 16) return 1;
        if (le16(clch+2) != 0x020e || clch[4] != 16 || le32(clch+12) != 1) return 1;
        memset(clch, 0, sizeof clch);
        uint8_t ochp2[30], fake_aid[2] = {0xaa, 0xbb};
        if (fill_open_channel_p2(ochp2, 203, fake_aid, 2, 0) != 30) return 1;
        if (le16(ochp2+2) != 0x0247 || ochp2[4] != 30 || ochp2[12] != 2 || ochp2[13] != 0xaa ||
            ochp2[14] != 0xbb || ochp2[29] != 0) return 1;
        uint8_t chap[38];
        if (fill_channel_apdu(chap, 204, 1, 0x80, 0xf2, 0x00, 0x00, 0x00) != 38) return 1;
        if (le16(chap+2) != 0x020f || chap[4] != 38 || le32(chap+12) != 1 ||
            chap[16] != 0x81 || chap[20] != 0xf2 || chap[24] || chap[28] || chap[32]) return 1;
        memset(ochp2, 0, sizeof ochp2);
        memset(chap, 0, sizeof chap);
        memset(fake_aid, 0, sizeof fake_aid);
        if (fill_empty_get(slot, 0x024d, 170, 12) != 12 || le16(slot+2) != 0x024d || slot[4] != 12) return 1;
        memset(slot, 0, sizeof slot);
        memset(chv2, 0, sizeof chv2);
        uint8_t swb[16] = {0};
        swb[2] = 0x0c; swb[3] = 0x02; swb[12] = 2; swb[14] = 0x90; swb[15] = 0x00;
        unsigned sw = 0;
        if (apdu_sw(swb, 16, &sw) || sw != 0x9000) return 1;
        puts("PASS: framing, tokens, radio, registration, LTE and PIN fixtures"); return 0;
    }
    if (argc == 2 && !strcmp(argv[1], "set-card-power-up")) return run_card_power_up();
    if (argc == 2 && !strcmp(argv[1], "query-stock-gets")) return run_stock_gets();
    if (argc == 2 && !strcmp(argv[1], "set-modems-config")) return run_set_modems_config();
    if (argc == 2 && !strcmp(argv[1], "verify-after-config")) return run_verify_after_config();
    if (argc == 2 && !strcmp(argv[1], "transmit-chv")) return run_transmit_chv();
    if (argc == 2 && !strcmp(argv[1], "select-verify")) return run_select_verify();
    if (argc == 2 && !strcmp(argv[1], "query-sim-lock")) return run_query_lock_info();
    if (argc == 2 && !strcmp(argv[1], "query-facility-sc")) return run_query_facility_sc();
    if (argc == 2 && !strcmp(argv[1], "enable-sc-lock-a")) return run_enable_sc_lock_a();
    if (argc == 2 && !strcmp(argv[1], "query-atr")) return run_query_atr();
    if (argc == 2 && !strcmp(argv[1], "audit-close")) return run_audit_close();
    if (argc == 2 && !strcmp(argv[1], "slot-then-attach")) return run_slot_then_attach();
    if (argc == 2 && !strcmp(argv[1], "card-power-cycle")) return run_card_power_cycle();
    if (argc == 2 && !strcmp(argv[1], "radio-power-cycle")) return run_radio_power_cycle();
    if (argc == 2 && !strcmp(argv[1], "attach-then-pin")) return run_attach_then_pin();
    if (argc == 2 && !strcmp(argv[1], "fix-selection")) return run_fix_selection();
    if (argc == 2 && !strcmp(argv[1], "uicc-enable")) return run_uicc_enable();
    if (argc == 2 && !strcmp(argv[1], "pin-incongruence-audit")) return run_pin_incongruence_audit();
    if (argc == 2 && !strcmp(argv[1], "slot-port-audit")) return run_slot_port_audit();
    if (argc == 2 && !strcmp(argv[1], "other-logical-sim")) return run_other_logical_sim();
    if (argc == 2 && !strcmp(argv[1], "ipc1-radio-on")) return run_ipc1_radio_on();
    if (argc == 2 && !strcmp(argv[1], "ipc1-card-up")) return run_ipc1_card_up();
    if (argc == 2 && !strcmp(argv[1], "ipc1-cycle-ipc0-poll")) return run_ipc1_cycle_ipc0_radio_poll();
    if (argc == 2 && !strcmp(argv[1], "voice-op-get")) return run_voice_op_get();
    if (argc == 2 && !strcmp(argv[1], "cold-apdu-status")) return run_cold_apdu_status();
    if (argc == 2 && !strcmp(argv[1], "och-p2-status")) return run_och_p2_status();
    if (argc == 2 && !strcmp(argv[1], "verify-pin-aid")) return run_verify_with_aid();
    if (argc == 2 && !strcmp(argv[1], "tray-watch")) return run_tray_watch();
    if (argc == 2 && !strcmp(argv[1], "peek-present-surfaces")) return run_peek_present_surfaces();
    int radio = argc == 2 && !strcmp(argv[1], "query-radio-state");
    int registration = argc == 2 && !strcmp(argv[1], "query-data-registration");
    int select_auto = argc == 2 && !strcmp(argv[1], "set-network-selection-auto");
    int prefer_lte = argc == 2 && !strcmp(argv[1], "set-preferred-lte");
    int query_pref = argc == 2 && !strcmp(argv[1], "query-preferred-network");
    int verify_first = argc == 2 && !strcmp(argv[1], "verify-pin-first");
    int verify_second = argc == 2 && !strcmp(argv[1], "verify-pin-second");
    int verify = verify_first || verify_second;
    int listen_sim = argc == 2 && !strcmp(argv[1], "listen-sim");
    int query_op = argc == 2 && !strcmp(argv[1], "query-operator");
    int query_voice = argc == 2 && !strcmp(argv[1], "query-voice-registration");
    int query_nets = argc == 2 && !strcmp(argv[1], "query-available-networks");
    if (argc != 2 || (!radio && !registration && !select_auto && !prefer_lte && !query_pref && !verify && !listen_sim && !query_op && !query_voice && !query_nets && strcmp(argv[1], "query-sim-status"))) {
        fprintf(stderr,"usage: sit-sim-status self-test|query-sim-status|query-radio-state|query-data-registration|set-network-selection-auto|set-preferred-lte|query-preferred-network|verify-pin-first|verify-pin-second|listen-sim|query-operator|tray-watch|peek-present-surfaces\n"); return 64;
    }
    alarm(listen_sim || query_nets ? 45 : 15);
    FILE *f=fopen("/sys/devices/platform/cpif/modem_state","r");
    char state[32]={0};
    if (!f) return 1;
    int got=fscanf(f,"%31s",state); fclose(f);
    if (got!=1 || strcmp(state,"ONLINE")) { puts("requires ONLINE"); return 1; }
    unsigned maj=0,min=0;
    f=fopen("/sys/class/cpif/umts_ipc0/dev","r");
    if (!f) return 1;
    got=fscanf(f,"%u:%u",&maj,&min); fclose(f);
    if (got!=2) return 1;
    int lock=open("/run/saaios-sit-status.lock",O_CREAT|O_RDWR|O_CLOEXEC,0600);
    if (lock<0 || flock(lock,LOCK_EX|LOCK_NB)) return 1;
    int fd=open("/dev/umts_ipc0",O_RDWR|O_NONBLOCK|O_CLOEXEC|O_NOFOLLOW);
    struct stat st;
    if (fd<0 || fstat(fd,&st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev)!=maj || minor(st.st_rdev)!=min) return 1;
    if (listen_sim) {
        /* Hold the channel. Do not write. Print only the factory SIM offsets. */
        uint8_t buffer[65536]; size_t used=0; unsigned frames=0, sim_frames=0;
        int64_t deadline=now_ms()+30000;
        while (now_ms()<deadline && frames<128) {
            struct pollfd pfd={fd,POLLIN,0};
            int ready=poll(&pfd,1,500);
            if (ready<0) { if (errno==EINTR) continue; return 1; }
            if (!ready) continue;
            if (pfd.revents & (POLLERR|POLLHUP|POLLNVAL)) return 1;
            if (!(pfd.revents&POLLIN)) continue;
            ssize_t n=read(fd,buffer+used,sizeof(buffer)-used);
            if (n<0 && (errno==EINTR||errno==EAGAIN)) continue;
            if (n<=0) return 1;
            used+=(size_t)n;
            while (used) {
                int len=frame_size(buffer,used);
                if (len<0) { puts("malformed framing; stop"); return 1; }
                if (!len) break;
                frames++;
                if (le16(buffer+2)==0x0200 && len>=15) {
                    sim_frames++;
                    unsigned raw=buffer[12];
                    printf("unsolicited SIM: length=%d card_state_raw=%u card_state=%s applications=%u\n",
                        len, raw, card_state_name(stored_card_state(raw)), buffer[14]);
                    if (buffer[14]>=1 && len>=18)
                        printf("app0_type_raw=%u app0_class=%s app0_state_raw=%u app0_state=%s\n",
                            buffer[15], app_class_name(buffer[15]), buffer[17], app_state_name(buffer[17]));
                } else {
                    printf("other_frame type=%u id=%u length=%d\n", buffer[0], le16(buffer+2), len);
                }
                used-=(size_t)len; memmove(buffer,buffer+len,used);
            }
            if (used==sizeof(buffer)) return 1;
        }
        printf("listen_frames=%u sim_frames=%u\n", frames, sim_frames);
        close(fd); close(lock);
        return 0;
    }
    /* Factory BuildSimGetStatus: type 0, id 0x0200, length 12, token 1. */
    /* Factory BuildGetRadioState at 0x746a0: id 0x0801, length 12. */
    /* Factory registration builder domain 2: id 0x0701, no payload. */
    /* Factory BuildSetNetworkSelectionAuto at 0x747a0: id 0x0704, length 12, no PLMN. */
    /* Factory BuildSetPreferredNetworkType at 0x74930: id 0x070a, length 16.
       u32 at offset 12 is the SIT type. Index 11 is SIT_NET_PREF_NET_TYPE_LTE_ONLY. */
    /* Factory BuildGetPreferredNetworkType at 0x749d0: id 0x070b, length 12.
       Response u32 at offset 12 is that same SIT type (name table index, max 0x12). */
    /* Factory BuildSimVerifyPin PIN1 path: id 0x0201, length 38, null AID. */
    /* Factory BuildOperator at 0x74450: id 0x0702, length 12, no payload.
       The reply carries name bytes; those are not printed. */
    const unsigned request_id = verify ? 0x0201 : (query_voice ? 0x0700 : (query_nets ? 0x0706 : (query_op ? 0x0702 : (prefer_lte ? 0x070a : (query_pref ? 0x070b : (select_auto ? 0x0704 : (registration ? 0x0701 : (radio ? 0x0801 : 0x0200))))))));
    const uint32_t token = verify_first ? 7 : (verify_second ? 8 : (query_voice ? 10 : (query_nets ? 11 : (query_op ? 9 : (prefer_lte ? 5 : (query_pref ? 6 : (select_auto ? 4 : (registration ? 3 : (radio ? 2 : 1)))))))));
    const size_t req_len = verify ? 38 : (prefer_lte || query_nets ? 16 : 12);
    uint8_t request[38];
    memset(request, 0, sizeof request);
    if (verify) {
        if (fill_verify_pin(request, token, verify_first ? 1 : 2) != 38) return 1;
    } else {
        request[2] = (uint8_t)request_id;
        request[3] = (uint8_t)(request_id >> 8);
        request[4] = (uint8_t)req_len;
        request[6] = (uint8_t)token;
        if (prefer_lte) request[12] = 11;
    }
    ssize_t wrote = write(fd, request, req_len);
    memset(request, 0, sizeof request);
    if (wrote != (ssize_t)req_len) {
        perror("one-shot write"); return 1; /* Never resend or split. */
    }
    uint8_t buffer[65536]; size_t used=0; unsigned frames=0;
    int64_t deadline=now_ms()+(query_nets ? 30000 : 10000);
    while (now_ms()<deadline && frames<128) {
        struct pollfd pfd={fd,POLLIN,0};
        int ready=poll(&pfd,1,500);
        if (ready<0) { if(errno==EINTR) continue; return 1; }
        if (!ready) continue;
        if (pfd.revents & (POLLERR|POLLHUP|POLLNVAL)) return 1;
        if (!(pfd.revents&POLLIN)) continue;
        ssize_t n=read(fd,buffer+used,sizeof(buffer)-used);
        if (n<0 && (errno==EINTR||errno==EAGAIN)) continue;
        if (n<=0) return 1;
        used+=(size_t)n;
        while (used) {
            int len=frame_size(buffer,used);
            if (len<0) { puts("malformed framing; stop"); return 1; }
            if (!len) break;
            frames++;
            if (buffer[0]==1 && le16(buffer+2)==request_id && le32(buffer+6)==token) {
                if (verify) {
                    printf("PIN verify response: length=%d error_raw=%u\n", len, buffer[10]);
                    if (len >= 16) printf("remain_raw=%u\n", le32(buffer + 12));
                    close(fd); close(lock);
                    return buffer[10] ? 2 : (len >= 16 ? 0 : 1);
                }
                if (query_op) {
                    printf("Operator response: length=%d error_raw=%u\n", len, buffer[10]);
                    if (!buffer[10] && len >= 13)
                        printf("plmn_present=%u long_name_present=%u short_name_present=%u\n",
                            buffer[12] ? 1 : 0,
                            len >= 19 && buffer[18] ? 1 : 0,
                            len >= 51 && buffer[50] ? 1 : 0);
                    close(fd); close(lock);
                    return buffer[10] ? 2 : (len >= 13 ? 0 : 1);
                }
                if (query_pref) {
                    printf("Preferred network response: length=%d error_raw=%u\n", len, buffer[10]);
                    if (!buffer[10] && len >= 16) {
                        uint32_t pref = le32(buffer + 12);
                        printf("preferred_raw=%u%s\n", pref, pref == 11 ? " preferred=LTE_ONLY" : "");
                    }
                    close(fd); close(lock);
                    return buffer[10] ? 2 : (len >= 16 ? 0 : 1);
                }
                if (prefer_lte || select_auto) {
                    printf("%s response: length=%d error_raw=%u\n",
                        prefer_lte ? "Preferred LTE" : "Network selection auto", len, buffer[10]);
                    close(fd); close(lock);
                    return buffer[10] ? 2 : 0;
                }
                if (query_voice) {
                    printf("Voice registration response: length=%d error_raw=%u\n", len, buffer[10]);
                    if (!buffer[10] && len >= 15) {
                        const char *tech = radio_tech_name(buffer[14]);
                        printf("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u%s%s\n",
                            buffer[12], buffer[13], buffer[14], tech ? " radio_tech=" : "", tech ? tech : "");
                    }
                    close(fd); close(lock);
                    return buffer[10] ? 2 : (len >= 15 ? 0 : 1);
                }
                if (query_nets) {
                    printf("Available networks response: length=%d error_raw=%u\n", len, buffer[10]);
                    if (!buffer[10] && len >= 16)
                        printf("network_count=%u\n", le32(buffer + 12));
                    close(fd); close(lock);
                    return buffer[10] ? 2 : (len >= 16 ? 0 : 1);
                }
                if (registration) {
                    printf("Data registration response: length=%d error_raw=%u\n",len,buffer[10]);
                    if (!buffer[10] && len>=16) {
                        const char *tech = radio_tech_name(buffer[15]);
                        printf("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u%s%s\n",
                            buffer[12], buffer[13], buffer[15], tech ? " radio_tech=" : "", tech ? tech : "");
                    }
                    close(fd); close(lock);
                    return buffer[10] ? 2 : (len>=16 ? 0 : 1);
                }
                if (radio) {
                    printf("Radio response: length=%d error_raw=%u\n",len,buffer[10]);
                    if (!buffer[10] && len>=16) printf("radio_state_raw=%u\n",le32(buffer+12));
                    close(fd); close(lock);
                    return buffer[10] ? 2 : (len>=16 ? 0 : 1);
                }
                printf("SIM response: length=%d error_raw=%u\n",len,buffer[10]);
                if (!buffer[10] && len>=15) {
                    unsigned raw = buffer[12];
                    unsigned stored = stored_card_state(raw);
                    printf("card_state_raw=%u card_state=%s universal_pin_raw=%u applications=%u\n",
                        raw, card_state_name(stored), buffer[13], buffer[14]);
                    if (buffer[14] >= 1 && len >= 18)
                        printf("app0_type_raw=%u app0_class=%s app0_state_raw=%u app0_state=%s\n",
                            buffer[15], app_class_name(buffer[15]), buffer[17], app_state_name(buffer[17]));
                    /* GetPinState pin 1 is packet byte 72. GetPinRemainCount pin 1 is byte 74.
                       AID length is byte 18 and its bytes follow; those are not printed. */
                    if (buffer[14] >= 1 && len >= 75)
                        printf("pin1_state_raw=%u pin1_remain_raw=%u\n", buffer[72], buffer[74]);
                }
                close(fd); close(lock);
                return buffer[10] ? 2 : (len>=15 ? 0 : 1);
            }
            /* Drop unrelated events without logging private payloads. */
            used-=(size_t)len; memmove(buffer,buffer+len,used);
        }
        if (used==sizeof(buffer)) return 1;
    }
    printf("No matching %s response; observed_frames=%u\n",
        verify ? "PIN verify" : (query_voice ? "voice registration" : (query_nets ? "available networks" : (query_op ? "operator" : (prefer_lte ? "preferred LTE" : (query_pref ? "preferred network" : (select_auto ? "selection" : (registration ? "registration" : (radio ? "radio" : "SIM")))))))), frames);
    close(fd); close(lock); return 3;
}
