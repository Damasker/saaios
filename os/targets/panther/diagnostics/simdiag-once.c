/* Self-contained one-shot post-READY diagnostic on a live, stable link.
 *
 * Confirms the registration-trigger outcome and runs the read-only USIM
 * forbidden-PLMN / administrative-data checks the owner cannot do:
 *   - GET preferred RAT (0x070b): confirm the owner's LTE_WCDMA(12) SET took.
 *   - GET voice (0x0700) and data (0x0701) registration: confirm settled state.
 *   - SIM_IO (0x0208) READ_BINARY of EFfplmn (0x6F7B) and EFad (0x6FAD) under
 *     ADF_USIM: report forbidden-PLMN entries (public MCC/MNC only) and the
 *     admin-data MNC-length/flags. Never reads or prints the IMSI/ICCID/AID.
 *
 * Serves the RFS cmd7 unprotect reply itself (no separate broker), takes the
 * sit-status flock, requires CP ONLINE. No POWER_OFF / crash / cbd / rild /
 * EFS write. No invented reply bytes.
 */
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
static uint32_t le32(const uint8_t *p)
{
    return le16(p) | ((uint32_t)le16(p + 2) << 16);
}
static int64_t now_ms(void)
{
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}
static int frame_size(const uint8_t *p, size_t n)
{
    if (n < 6) return 0;
    if (p[0] > 2) return -1;
    unsigned min = p[0] == 2 ? 8 : 12;
    unsigned len = le16(p + 4);
    if (len < min || len > 65536) return -1;
    return n < len ? 0 : (int)len;
}
static int open_node(const char *node, const char *sysdev)
{
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
static void service_rfs(int fd)
{
    uint8_t buf[256];
    ssize_t n = read(fd, buf, sizeof buf);
    if (n <= 0) return;
    if (n == 12 && buf[0] == 0x07 && !buf[1] && !buf[2] && !buf[3] &&
        buf[4] == 0x04 && !buf[5] && !buf[6] && !buf[7] &&
        buf[8] == 0x03 && !buf[9] && !buf[10] && !buf[11]) {
        static const uint8_t reply[16] = {
            0x03, 0, 0, 0, 0x08, 0, 0, 0, 0, 0, 0, 0, 0x03, 0, 0, 0};
        ssize_t w = write(fd, reply, sizeof reply);
        (void)w;
    }
}

/* Exchange one request, return the matching response (by type/id/token). */
static int exchange(int ipc, int rfs, const uint8_t *req, size_t req_len,
                    unsigned id, uint32_t tok, int timeout_ms, uint8_t *resp,
                    int *resp_len)
{
    if (write(ipc, req, req_len) != (ssize_t)req_len) return -1;
    static uint8_t buf[16384];
    size_t used = 0;
    int64_t end = now_ms() + timeout_ms;
    while (now_ms() < end) {
        struct pollfd pfd[2] = {{ipc, POLLIN, 0}, {rfs, POLLIN, 0}};
        if (poll(pfd, 2, 250) < 0) {
            if (errno == EINTR) continue;
            return -1;
        }
        if (pfd[1].revents & POLLIN) service_rfs(rfs);
        if ((pfd[0].revents & POLLIN) && used < sizeof buf) {
            ssize_t n = read(ipc, buf + used, sizeof buf - used);
            if (n > 0) used += (size_t)n;
        }
        size_t off = 0;
        while (off < used) {
            int len = frame_size(buf + off, used - off);
            if (len < 0) return -1;
            if (!len) break;
            if (buf[off] == 1 && le16(buf + off + 2) == id &&
                le32(buf + off + 6) == tok) {
                int c = len < 512 ? len : 512;
                if (resp && resp_len) {
                    memcpy(resp, buf + off, (size_t)c);
                    *resp_len = c;
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
    return -2;
}

/* 12-byte GET; print the scalar at +12 (le32) when the reply is long enough.
 * When dump!=0, also hexdump the payload (public network info: PLMN, reg
 * state, reject cause, RAT) for offline decode. */
static void get_scalar2(int ipc, int rfs, unsigned id, uint32_t tok,
                        const char *name, int dump)
{
    uint8_t req[12] = {0};
    req[2] = (uint8_t)id;
    req[3] = (uint8_t)(id >> 8);
    req[4] = 12;
    req[6] = (uint8_t)tok;
    req[7] = (uint8_t)(tok >> 8);
    uint8_t resp[512];
    int n = 0;
    int rc = exchange(ipc, rfs, req, 12, id, tok, 8000, resp, &n);
    if (rc != 0) {
        printf("GET %s id=0x%04x rc=%d (timeout/err)\n", name, id, rc);
        return;
    }
    unsigned err = n >= 12 ? le16(resp + 10) : 0xffff;
    printf("GET %s id=0x%04x len=%d error_raw=%u", name, id, n, err);
    if (n >= 16) printf(" scalar_raw=%u", le32(resp + 12));
    if (n >= 13) printf(" byte12=%u", resp[12]);
    if (n >= 14) printf(" byte13=%u", resp[13]);
    putchar('\n');
    if (dump && n > 12) {
        printf("  %s payload[12..]:", name);
        for (int i = 12; i < n; i++) printf(" %02x", resp[i]);
        putchar('\n');
    }
}
static void get_scalar(int ipc, int rfs, unsigned id, uint32_t tok,
                       const char *name)
{
    get_scalar2(ipc, rfs, id, tok, name, 0);
}

/* BuildSimIO 0x0208 len 0x23c: +12 cmd, +13 fileid_lo, +14 u16(fileid LE),
 * +16 path_len, +17 path, +29 p1, +30 p2, +31 p3, +32 data_len,
 * +555 aid_len, +556 aid. Empty fields = zero. */
static int sim_io(int ipc, int rfs, uint32_t tok, uint8_t cmd, unsigned fileid,
                  const uint8_t *path, unsigned path_len, uint8_t p1, uint8_t p2,
                  uint8_t p3, const uint8_t *aid, unsigned aid_len,
                  uint8_t *resp, int *resp_len)
{
    uint8_t req[0x23c];
    memset(req, 0, sizeof req);
    req[2] = 0x08;
    req[3] = 0x02;
    req[4] = 0x3c;
    req[5] = 0x02;
    req[6] = (uint8_t)tok;
    req[7] = (uint8_t)(tok >> 8);
    req[12] = cmd;
    req[13] = (uint8_t)(fileid & 0xff);
    req[14] = (uint8_t)(fileid & 0xff);
    req[15] = (uint8_t)(fileid >> 8);
    if (path_len && path_len <= 12) {
        req[16] = (uint8_t)path_len;
        memcpy(req + 17, path, path_len);
    }
    req[29] = p1;
    req[30] = p2;
    req[31] = p3;
    if (aid && aid_len && aid_len <= 16) {
        req[555] = (uint8_t)aid_len;
        memcpy(req + 556, aid, aid_len);
    }
    return exchange(ipc, rfs, req, sizeof req, 0x0208, tok, 15000, resp,
                    resp_len);
}

/* Decode the SIM_IO response: SW1/SW2 at +12/+13, response body follows. The
 * exact body offset is reported so a wrong selection is visible, not guessed. */
static void simio_report(const char *name, const uint8_t *r, int n)
{
    unsigned err = n >= 12 ? le16(r + 10) : 0xffff;
    if (n < 14) {
        printf("SIMIO %s short len=%d error_raw=%u\n", name, n, err);
        return;
    }
    unsigned sw1 = r[12], sw2 = r[13];
    printf("SIMIO %s len=%d error_raw=%u sw=%02x%02x body_len=%d\n", name, n,
           err, sw1, sw2, n - 14);
}

/* Read EFdir (0x2F00) record 1 under MF and extract the application AID
 * (template tag 0x61 -> AID tag 0x4F). The AID is a secret handle: it is
 * stored but NEVER printed. Returns AID length (0 on failure). */
static unsigned get_aid(int ipc, int rfs, uint8_t *aid, unsigned aid_cap)
{
    static const uint8_t mf[2] = {0x3f, 0x00};
    const uint8_t try_len[] = {0x26, 0x38, 0x20, 0x00};
    for (unsigned t = 0; t < sizeof try_len; t++) {
        uint8_t resp[512];
        int n = 0;
        if (sim_io(ipc, rfs, 30 + t, 0xB2, 0x2F00, mf, 2, 1, 0x04, try_len[t],
                   NULL, 0, resp, &n) != 0)
            continue;
        if (n < 14) continue;
        const uint8_t *b = resp + 14;
        int blen = n - 14;
        for (int i = 0; i + 1 < blen; i++) {
            if (b[i] == 0x4f) {
                unsigned l = b[i + 1];
                if (l >= 1 && l <= aid_cap && i + 2 + (int)l <= blen) {
                    memcpy(aid, b + i + 2, l);
                    printf("EFdir: AID obtained len=%u rec_try=0x%02x "
                           "(not logged)\n",
                           l, try_len[t]);
                    return l;
                }
            }
        }
    }
    puts("EFdir: AID not found (ADF_USIM selection unavailable)");
    return 0;
}

/* EFfplmn: list of 3-byte PLMN entries (MCC/MNC, public codes, not secret).
 * Decode each to MCC-MNC; 0xFF FF FF = empty slot. Body assumed to start at
 * +14 after SW; also scan from +12 in case SW trails the body. */
static void decode_fplmn(const uint8_t *r, int n)
{
    if (n < 14) return;
    const uint8_t *b = r + 14;
    int blen = n - 14;
    printf("EFfplmn entries (public MCC-MNC only):\n");
    int any = 0;
    for (int i = 0; i + 3 <= blen; i += 3) {
        const uint8_t *e = b + i;
        if (e[0] == 0xff && e[1] == 0xff && e[2] == 0xff) continue;
        unsigned mcc1 = e[0] & 0x0f, mcc2 = e[0] >> 4, mcc3 = e[1] & 0x0f;
        unsigned mnc3 = e[1] >> 4, mnc1 = e[2] & 0x0f, mnc2 = e[2] >> 4;
        if (mnc3 == 0xf)
            printf("  forbidden MCC=%u%u%u MNC=%u%u\n", mcc1, mcc2, mcc3, mnc1,
                   mnc2);
        else
            printf("  forbidden MCC=%u%u%u MNC=%u%u%u\n", mcc1, mcc2, mcc3,
                   mnc1, mnc2, mnc3);
        any = 1;
    }
    if (!any) printf("  (no forbidden PLMN entries / all slots empty)\n");
}

int main(void)
{
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f || fscanf(f, "%31s", state) != 1) return 1;
    fclose(f);
    if (strcmp(state, "ONLINE")) {
        printf("requires ONLINE got=%s\n", state);
        return 1;
    }
    int lock = open("/run/saaios-sit-status.lock", O_CREAT | O_RDWR | O_CLOEXEC,
                    0600);
    if (lock < 0 || flock(lock, LOCK_EX | LOCK_NB)) {
        puts("lock busy (owner still holding channel?)");
        return 1;
    }
    int ipc = open_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    if (ipc < 0 || rfs < 0) {
        printf("open fail ipc=%d rfs=%d\n", ipc, rfs);
        return 1;
    }

    puts("=== registration readback ===");
    get_scalar(ipc, rfs, 0x070b, 11, "preferred_rat");
    get_scalar(ipc, rfs, 0x0703, 12, "selection_mode");
    get_scalar(ipc, rfs, 0x0801, 13, "radio_state");
    get_scalar2(ipc, rfs, 0x0700, 14, "voice_reg", 1);
    get_scalar2(ipc, rfs, 0x0701, 15, "data_reg", 1);

    puts("=== USIM EF read-only diagnosis (no IMSI/ICCID/AID) ===");
    uint8_t resp[512];
    int n = 0;
    uint8_t aid[16];
    unsigned aid_len = get_aid(ipc, rfs, aid, sizeof aid);
    const uint8_t *pa = aid_len ? aid : NULL;
    static const uint8_t adf[4] = {0x3f, 0x00, 0x7f, 0xff};
    /* Try ADF_USIM selection variants for EFad (0x6FAD), then on the variant
     * that returns SW present, read EFfplmn (0x6F7B) and decode. */
    struct { const char *tag; const uint8_t *path; unsigned plen;
             const uint8_t *aidp; unsigned alen; } v[] = {
        {"bare", NULL, 0, NULL, 0},
        {"aid", NULL, 0, pa, aid_len},
        {"adfpath", adf, 4, NULL, 0},
        {"aid+adfpath", adf, 4, pa, aid_len},
    };
    int ok_variant = -1;
    for (int i = 0; i < 4; i++) {
        n = 0;
        int rc = sim_io(ipc, rfs, 20 + (unsigned)i, 0xB0, 0x6FAD, v[i].path,
                        v[i].plen, 0, 0, 0x04, v[i].aidp, v[i].alen, resp, &n);
        if (rc != 0) {
            printf("SIMIO EFad[%s] no response rc=%d\n", v[i].tag, rc);
            continue;
        }
        char label[32];
        snprintf(label, sizeof label, "EFad[%s]", v[i].tag);
        simio_report(label, resp, n);
        if (n >= 14 && (resp[12] == 0x90 || resp[12] == 0x91 ||
                        resp[12] == 0x61 || resp[12] == 0x62 ||
                        resp[12] == 0x63))
            ok_variant = i;
    }
    if (ok_variant >= 0) {
        n = 0;
        if (sim_io(ipc, rfs, 40, 0xB0, 0x6F7B, v[ok_variant].path,
                   v[ok_variant].plen, 0, 0, 0x3c, v[ok_variant].aidp,
                   v[ok_variant].alen, resp, &n) == 0) {
            simio_report("EFfplmn", resp, n);
            decode_fplmn(resp, n);
        } else {
            puts("SIMIO EFfplmn no response");
        }
    } else {
        puts("EFfplmn skipped: no EFad selection variant accepted (ADF_USIM "
             "selection mechanism unresolved)");
    }

    close(ipc);
    close(rfs);
    close(lock);
    return 0;
}
