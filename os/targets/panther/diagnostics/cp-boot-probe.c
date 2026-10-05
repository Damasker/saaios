/* Isolated diagnostic. Never dispatch the included production loader. */
#define SAAIOS_DIAGNOSTIC_WRAPPER 1
#if defined(PROBE_COMPLETE) && (!defined(PROBE_PREAMBLE) || !defined(PROBE_FULL_MAIN) || !defined(PROBE_FIRMWARE_ONLY))
#error "Complete probe requires all verified firmware stages and preamble"
#endif
#define main unused_original_main
#include "cp-boot-probe-support.inc"
#undef main
#include <poll.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <time.h>

/* Log a header. A 24-byte packet is six numeric words. Longer packets
 * stay at three words so a path is never printed. Command 7 with length
 * 4 and file id 3 is factory RFS_NV_UNPROTECT. The success reply is a
 * 16-byte status and does not chmod, open, or write any file. */
static unsigned rfs_u32(const unsigned char *p) {
    return (unsigned)p[0] | ((unsigned)p[1] << 8) |
           ((unsigned)p[2] << 16) | ((unsigned)p[3] << 24);
}

static void log_rfs_header(const char *what, const unsigned char *buf, ssize_t n) {
    unsigned w[6] = {0, 0, 0, 0, 0, 0};
    int words = (n == 24) ? 6 : 3;
    for (int i = 0; i < words; i++) {
        if (n >= (ssize_t)(i + 1) * 4)
            w[i] = rfs_u32(buf + i * 4);
    }
    if (words == 6)
        log_line("rfs %s bytes=%zd words=%08x %08x %08x %08x %08x %08x",
                 what, n, w[0], w[1], w[2], w[3], w[4], w[5]);
    else
        log_line("rfs %s bytes=%zd words=%08x %08x %08x", what, n, w[0], w[1], w[2]);
}

/* Command 6, file id 3, operation 1 is RFS_IO_REQUEST read. Factory reads
 * at most 2012 bytes and replies with command 1. The bytes come from the
 * userdata protected-NV copy. This does not open or write original EFS.
 * Any other operation gets no channel reply. */
static int reply_protected_read(int fd, const unsigned char *req) {
    unsigned id = rfs_u32(req + 8);
    unsigned off = rfs_u32(req + 12);
    unsigned len = rfs_u32(req + 16);
    unsigned op = rfs_u32(req + 20);
    log_line("rfs io id=%u off=%u len=%u op=%u", id, off, len, op);
    if (id != 3 || op != 1 || len == 0 || off >= 524288u || len > 524288u - off) {
        log_line("rfs io no reply");
        return 0;
    }
    unsigned chunk = len > 2012u ? 2012u : len;
    int nv = open(NV_PROT_PATH, O_RDONLY | O_CLOEXEC);
    if (nv < 0) {
        log_line("rfs io copy unavailable");
        return 0;
    }
    struct stat st;
    if (fstat(nv, &st) != 0 || st.st_size != 524288) {
        close(nv);
        log_line("rfs io copy size refused");
        return 0;
    }
    unsigned char reply[20 + 2012];
    memset(reply, 0, 20);
    ssize_t got = pread(nv, reply + 20, chunk, (off_t)off);
    close(nv);
    if (got <= 0) {
        log_line("rfs io copy read failed");
        return 0;
    }
    unsigned nread = (unsigned)got;
    unsigned pay = nread + 12u;
    reply[0] = 0x01;
    reply[2] = req[2];
    reply[3] = req[3];
    reply[4] = (unsigned char)pay;
    reply[5] = (unsigned char)(pay >> 8);
    reply[6] = (unsigned char)(pay >> 16);
    reply[7] = (unsigned char)(pay >> 24);
    reply[8] = 0x03;
    reply[12] = (unsigned char)off;
    reply[13] = (unsigned char)(off >> 8);
    reply[14] = (unsigned char)(off >> 16);
    reply[15] = (unsigned char)(off >> 24);
    reply[16] = (unsigned char)nread;
    reply[17] = (unsigned char)(nread >> 8);
    reply[18] = (unsigned char)(nread >> 16);
    reply[19] = (unsigned char)(nread >> 24);
    ssize_t wr = write(fd, reply, 20u + nread);
    log_line("rfs io read reply bytes=%zd", wr);
    return wr == (ssize_t)(20u + nread);
}

static void observe_rfs(int fd) {
    struct pollfd pfd = { .fd = fd, .events = POLLIN };
    int ready = poll(&pfd, 1, 2000);
    if (ready <= 0) {
        log_line("rfs observe: no request ready=%d", ready);
        return;
    }
    unsigned char buf[2112];
    ssize_t n = read(fd, buf, sizeof(buf));
    if (n < 0) {
        log_line("rfs observe read failed");
        return;
    }
    log_rfs_header("observe", buf, n);
    if (n != 12 || buf[0] != 0x07 || buf[1] || buf[2] || buf[3] ||
        buf[4] != 0x04 || buf[5] || buf[6] || buf[7] ||
        buf[8] != 0x03 || buf[9] || buf[10] || buf[11])
        return;
    unsigned char reply[16] = {
        0x03, 0x00, 0x00, 0x00,
        0x08, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
        0x03, 0x00, 0x00, 0x00
    };
    ssize_t wr = write(fd, reply, sizeof(reply));
    log_line("rfs unprotect status reply bytes=%zd", wr);
    /* Command 3 length 12 is RFS_OP_STATUS. After unprotect the factory
     * handler updates its own state and writes nothing back. */
    ready = poll(&pfd, 1, 3000);
    if (ready <= 0) {
        log_line("rfs after unprotect: no further request ready=%d", ready);
        return;
    }
    n = read(fd, buf, sizeof(buf));
    if (n < 0) {
        log_line("rfs next read failed");
        return;
    }
    log_rfs_header("next", buf, n);
    if (n == 20 && buf[0] == 0x03 && buf[1] == 0 && buf[2] == 0 && buf[3] == 0 &&
        buf[4] == 0x0c && buf[5] == 0 && buf[6] == 0 && buf[7] == 0) {
        unsigned id = buf[12] | (buf[13] << 8) | (buf[14] << 16) | (buf[15] << 24);
        unsigned extra = buf[16] | (buf[17] << 8) | (buf[18] << 16) | (buf[19] << 24);
        log_line("rfs op_status id=%08x extra=%08x no reply", id, extra);
        ready = poll(&pfd, 1, 3000);
        if (ready <= 0) {
            log_line("rfs after op_status: no further request ready=%d", ready);
            return;
        }
        n = read(fd, buf, sizeof(buf));
        if (n < 0) {
            log_line("rfs follow read failed");
            return;
        }
        log_rfs_header("follow", buf, n);
        /* A refused write must not close the channel. Factory returns to
         * its reader, and the CP may send another command. */
        log_line("rfs hold open");
        for (int i = 0; i < 16; i++) {
            int served = 0;
            if (n == 24 && buf[0] == 0x06 && buf[1] == 0 &&
                buf[4] == 0x10 && buf[5] == 0 && buf[6] == 0 && buf[7] == 0)
                served = reply_protected_read(fd, buf);
            ready = poll(&pfd, 1, served ? 1500 : 3000);
            if (ready <= 0) {
                log_line("rfs after io: no further request ready=%d", ready);
                return;
            }
            n = read(fd, buf, sizeof(buf));
            if (n < 0) {
                log_line("rfs after io read failed");
                return;
            }
            log_rfs_header("after io", buf, n);
        }
    }
}

static int64_t now_ms(void) {
    struct timespec t;
    if (clock_gettime(CLOCK_MONOTONIC, &t)) return 0;
    return (int64_t)t.tv_sec * 1000 + t.tv_nsec / 1000000;
}

/* Keep the fds opened before FIN. Poll SIM on that same IPC fd so the
 * channels are never closed between COMPLETE and the end of this wait. */
static int hold_frame_len(const unsigned char *p, size_t n) {
    if (n < 6 || p[0] > 2) return -1;
    unsigned min = p[0] == 2 ? 8u : 12u;
    unsigned len = (unsigned)p[4] | ((unsigned)p[5] << 8);
    if (len < min || len > 4096) return -1;
    return n < len ? 0 : (int)len;
}

static void hold_service_rfs(int fd) {
    unsigned char buf[256];
    ssize_t n = read(fd, buf, sizeof buf);
    if (n < 0 || n == 0) return;
    if (n == (ssize_t)sizeof buf) {
        unsigned char dump[256];
        while (read(fd, dump, sizeof dump) > 0) {}
        log_line("rfs large request drained no reply");
        return;
    }
    log_line("rfs bytes=%zd cmd=%u", n, buf[0]);
    if (n == 12 && buf[0] == 0x07 && !buf[1] && !buf[2] && !buf[3] &&
        buf[4] == 0x04 && !buf[5] && !buf[6] && !buf[7] &&
        buf[8] == 0x03 && !buf[9] && !buf[10] && !buf[11]) {
        static const unsigned char reply[16] = {
            0x03, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00
        };
        ssize_t wr = write(fd, reply, sizeof reply);
        log_line("rfs unprotect reply bytes=%zd", wr);
        return;
    }
    if (n == 20 && buf[0] == 0x03 && rfs_u32(buf + 4) == 12) {
        log_line("rfs op_status no reply");
        return;
    }
    if (n >= 24 && buf[0] == 0x06) {
        if (rfs_u32(buf + 20) == 2) log_line("rfs write op=2 refused no reply");
        else log_line("rfs io no reply");
        return;
    }
    log_line("rfs ignored no reply");
}

static int hold_send(int fd, unsigned id, unsigned len, uint32_t token, uint32_t word) {
    unsigned char req[18];
    if (len > sizeof req) return -1;
    memset(req, 0, sizeof req);
    req[2] = (unsigned char)id;
    req[3] = (unsigned char)(id >> 8);
    req[4] = (unsigned char)len;
    req[6] = (unsigned char)token;
    req[7] = (unsigned char)(token >> 8);
    if (len > 12) {
        req[12] = (unsigned char)word;
        if (len >= 16) {
            req[13] = (unsigned char)(word >> 8);
            req[14] = (unsigned char)(word >> 16);
            req[15] = (unsigned char)(word >> 24);
        }
    }
    ssize_t wr = write(fd, req, len);
    memset(req, 0, sizeof req);
    return wr == (ssize_t)len ? 0 : -1;
}

static void hold_note_sim(const unsigned char *b, int len, int *app, int *pin, int *ready, int *card) {
    if (len < 15 || b[10]) {
        log_line("SIM length=%d error_raw=%u", len, len > 10 ? b[10] : 0);
        return;
    }
    unsigned apps = b[14];
    int type = -1, state = -1, pin1 = -1;
    if (card) *card = (int)b[12];
    if (apps >= 1 && len >= 18) {
        type = b[15];
        state = b[17];
        *app = state;
        if (state == 5) *ready = 1;
    } else if (apps == 0) {
        *app = -1;
    }
    if (apps >= 1 && len >= 75) {
        pin1 = b[72];
        *pin = pin1;
    }
    if (pin1 >= 0)
        log_line("SIM length=%d error_raw=0 card_state_raw=%u applications=%u app0_type_raw=%d app0_state_raw=%d pin1_state_raw=%d",
                 len, b[12], apps, type, state, pin1);
    else
        log_line("SIM length=%d error_raw=0 card_state_raw=%u applications=%u app0_type_raw=%d app0_state_raw=%d",
                 len, b[12], apps, type, state);
}

static int hold_consume(unsigned char *ibuf, size_t *iused, int want, unsigned id,
                        uint32_t token, int *app, int *pin, int *ready, int *pref, int *card) {
    int found = 0;
    while (*iused) {
        int len = hold_frame_len(ibuf, *iused);
        if (len < 0) { *iused = 0; log_line("ipc framing drop"); return -1; }
        if (!len) break;
        unsigned fid = (unsigned)ibuf[2] | ((unsigned)ibuf[3] << 8);
        uint32_t ftok = (uint32_t)ibuf[6] | ((uint32_t)ibuf[7] << 8) |
                        ((uint32_t)ibuf[8] << 16) | ((uint32_t)ibuf[9] << 24);
        unsigned err = len > 10 ? ibuf[10] : 0;
        if (fid == 0x0200) hold_note_sim(ibuf, len, app, pin, ready, card);
        if (want && fid == id && ftok == token) {
            if (fid != 0x0200)
                log_line("response id=%u length=%d error_raw=%u", fid, len, err);
            if (fid == 0x0701 && !err && len >= 16)
                log_line("registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u",
                         ibuf[12], ibuf[13], ibuf[15]);
            if (fid == 0x0700 && !err && len >= 15)
                log_line("voice_registration_raw=%u reject_cause_raw=%u radio_tech_raw=%u",
                         ibuf[12], ibuf[13], ibuf[14]);
            if (fid == 0x070b && !err && len >= 16) {
                *pref = (int)((uint32_t)ibuf[12] | ((uint32_t)ibuf[13] << 8) |
                              ((uint32_t)ibuf[14] << 16) | ((uint32_t)ibuf[15] << 24));
                log_line("preferred_raw=%d", *pref);
            }
            found = 1;
        } else if (fid != 0x0200) {
            log_line("ipc id=%u length=%d", fid, len);
        }
        *iused -= (size_t)len;
        memmove(ibuf, ibuf + len, *iused);
    }
    return found;
}

static int hold_wait(int ipc, int rfs, unsigned char *ibuf, size_t *iused,
                     int want, unsigned id, uint32_t token, int ms,
                     int *app, int *pin, int *ready, int *pref, int *card) {
    int64_t end = now_ms() + ms;
    while (now_ms() < end) {
        int slice = (int)(end - now_ms());
        if (slice > 200) slice = 200;
        if (slice < 1) slice = 1;
        struct pollfd pfd[2] = { { ipc, POLLIN, 0 }, { rfs, POLLIN, 0 } };
        int pr = poll(pfd, 2, slice);
        if (pr < 0) { if (errno == EINTR) continue; return -1; }
        if (pfd[1].revents & POLLIN) hold_service_rfs(rfs);
        if (pfd[0].revents & POLLIN && *iused < 8192) {
            ssize_t n = read(ipc, ibuf + *iused, 8192 - *iused);
            if (n > 0) *iused += (size_t)n;
        }
        int got = hold_consume(ibuf, iused, want, id, token, app, pin, ready, pref, card);
        if (got < 0) return -1;
        if (got > 0) return 0;
    }
    return 1;
}

static void hold_rmnet(void) {
    unsigned long long rx = 0, tx = 0;
    FILE *f = fopen("/sys/class/net/rmnet0/statistics/rx_bytes", "r");
    if (f) { if (fscanf(f, "%llu", &rx) != 1) rx = 0; fclose(f); }
    f = fopen("/sys/class/net/rmnet0/statistics/tx_bytes", "r");
    if (f) { if (fscanf(f, "%llu", &tx) != 1) tx = 0; fclose(f); }
    log_line("rmnet_rx=%llu rmnet_tx=%llu", rx, tx);
}

static const char *hold_app_name(int app) {
    switch (app) {
    case -1: return "n/a";
    case 0: return "UNKNOWN";
    case 1: return "DETECTED";
    case 2: return "PIN";
    case 3: return "PUK";
    case 4: return "PERSO";
    case 5: return "READY";
    default: return "other";
    }
}

/* Cold-INSERT dense race: fast 0x0200 from first ONLINE; RadioPower ON ASAP;
   on DETECTED/READY keep radio ON, AllowData, poll voice/data reg + rmnet. */
static void hold_channels_poll_sim(int ipc, int rfs) {
    alarm(180);
    log_line("hold open from ONLINE; dense SIM 200ms for 45s (cold INSERT race)");
    unsigned char ibuf[8192];
    size_t iused = 0;
    int app = -1, pin = -1, ready = 0, pref = -1, card = -1;
    int last_app = -2;
    int saw_absent = 0, saw_unknown = 0, saw_detected = 0, saw_pin = 0, saw_ready = 0;
    int pin_disabled_identical = 0;
    int radio_on = 0, allow_sent = 0;
    int64_t start = now_ms();
    int64_t end = start + 45000;
    int64_t next = start;
    uint32_t token = 1;
    while (now_ms() < end && !ready) {
        if (now_ms() >= next) {
            if (hold_send(ipc, 0x0200, 12, token, 0)) log_line("SIM write failed");
            else if (hold_wait(ipc, rfs, ibuf, &iused, 1, 0x0200, token, 800,
                               &app, &pin, &ready, &pref, &card))
                log_line("SIM query timeout token=%u", token);
            if (card == 0) saw_absent = 1;
            if (app == 0) saw_unknown = 1;
            if (app == 1) saw_detected = 1;
            if (app == 2) {
                saw_pin = 1;
                if (pin == 3) pin_disabled_identical = 1;
            }
            if (app == 5) { saw_ready = 1; ready = 1; }
            if (app != last_app) {
                log_line("app_state transition token=%u card=%d app=%d(%s) pin1=%d",
                         token, card, app, hold_app_name(app), pin);
                last_app = app;
            }
            if (!radio_on) {
                log_line("RadioPower ON ASAP token=%u", token + 5000);
                if (hold_send(ipc, 0x0800, 18, token + 5000, 2))
                    log_line("RadioPower write failed");
                else
                    (void)hold_wait(ipc, rfs, ibuf, &iused, 1, 0x0800, token + 5000,
                                    1500, &app, &pin, &ready, &pref, &card);
                radio_on = 1;
            }
            if ((saw_detected || saw_ready) && !allow_sent) {
                log_line("AllowData=1 after DETECTED/READY");
                if (hold_send(ipc, 0x0710, 13, token + 6000, 1))
                    log_line("AllowData write failed");
                else
                    (void)hold_wait(ipc, rfs, ibuf, &iused, 1, 0x0710, token + 6000,
                                    1500, &app, &pin, &ready, &pref, &card);
                allow_sent = 1;
                if (!hold_send(ipc, 0x0700, 12, token + 7000, 0))
                    (void)hold_wait(ipc, rfs, ibuf, &iused, 1, 0x0700, token + 7000,
                                    2000, &app, &pin, &ready, &pref, &card);
                if (!hold_send(ipc, 0x0701, 12, token + 7001, 0))
                    (void)hold_wait(ipc, rfs, ibuf, &iused, 1, 0x0701, token + 7001,
                                    2000, &app, &pin, &ready, &pref, &card);
                hold_rmnet();
            }
            token++;
            next = now_ms() + 200;
        } else {
            int left = (int)(next - now_ms());
            if (left > 50) left = 50;
            if (left < 1) left = 1;
            hold_wait(ipc, rfs, ibuf, &iused, 0, 0, 0, left, &app, &pin, &ready, &pref, &card);
        }
    }
    if (ready) {
        log_line("preferred query");
        if (!hold_send(ipc, 0x070b, 12, 50, 0))
            hold_wait(ipc, rfs, ibuf, &iused, 1, 0x070b, 50, 10000, &app, &pin, &ready, &pref, &card);
        if (pref >= 0 && pref != 11) {
            log_line("preferred set LTE_ONLY");
            if (!hold_send(ipc, 0x070a, 16, 51, 11))
                hold_wait(ipc, rfs, ibuf, &iused, 1, 0x070a, 51, 10000, &app, &pin, &ready, &pref, &card);
        }
        log_line("network selection auto");
        if (!hold_send(ipc, 0x0704, 12, 52, 0))
            hold_wait(ipc, rfs, ibuf, &iused, 1, 0x0704, 52, 10000, &app, &pin, &ready, &pref, &card);
        for (int i = 0; i < 4; i++) {
            uint32_t t = 60 + (uint32_t)i;
            log_line("registration query");
            if (!hold_send(ipc, 0x0701, 12, t, 0))
                hold_wait(ipc, rfs, ibuf, &iused, 1, 0x0701, t, 10000, &app, &pin, &ready, &pref, &card);
            if (i != 3)
                hold_wait(ipc, rfs, ibuf, &iused, 0, 0, 0, 5000, &app, &pin, &ready, &pref, &card);
        }
    }
    hold_rmnet();
    log_line("HOLD RESULT app_state=%d(%s) pin1=%d ready=%d radio_on=%d "
             "saw_absent=%d saw_unknown=%d saw_detected=%d saw_pin=%d saw_ready=%d "
             "pin_disabled_identical=%d",
             app, hold_app_name(app), pin, ready, radio_on,
             saw_absent, saw_unknown, saw_detected, saw_pin, saw_ready,
             pin_disabled_identical);
}

#ifdef PROBE_HANDOVER
#ifndef PROBE_COMPLETE
#error "Handover comparison requires complete guarded probe"
#endif
#define SAAIOS_HANDOVER_INTERNAL 1
#include "handover-source-check.c"
#endif

#ifdef PROBE_PREAMBLE
#include "../src/sit-boot-preamble.h"
static int probe_exchange(void *ctx, const uint8_t *packet, size_t n, uint32_t ack) {
    (void)ctx;
    log_line("preamble packet len=%zu expected=%x", n, ack);
    if (sit_write_bytes(packet, n) < 0) return -1;
    return sit_req_resp(0, ack, SIT_ACK_DEADLINE_MS);
}
static int send_factory_preamble(const uint8_t *bin, size_t len) {
    return saaios_sit_boot_preamble(bin, len, probe_exchange, NULL);
}
#endif

int main(int argc, char **argv) {
#ifdef PROBE_COMPLETE
#ifdef PROBE_HANDOVER
    if (argc != 3 || strcmp(argv[1], "boot-b-with-verified-nv-handover") != 0) return 64;
    uint8_t handover[SAAIOS_HANDOVER_SIZE] = {0};
    char *source_args[] = {argv[0], "candidate-no-json", argv[2], NULL};
    if (check_handover_sources(3, source_args, handover)) die("handover sources refused");
#else
    if (argc != 2 || strcmp(argv[1], "boot-b-with-verified-nv") != 0) {
        fprintf(stderr, "Requires boot-b-with-verified-nv; never run automatically\n");
        return 64;
    }
#endif
    if (system("sh /tmp/saaios-verify-nv-copies.sh") != 0)
        die("NV verification failed; no hardware operations");
#else
    if (argc != 2 || strcmp(argv[1], "probe-b-ram-only") != 0) {
        fprintf(stderr, "Requires explicit probe-b-ram-only argument\n");
        return 64;
    }
#endif
    alarm(420);
    /* Accept stock B or MODEM-06 READY-patch COPY (MOVS #2→#5 @ 0x14fb404). */
    if (system("h=$(sha256sum /tmp/saaios-probe-b-modem.bin | cut -d ' ' -f 1); "
               "test \"$h\" = 449eeab3bf70fc4ed0793dce3a1f245447bf54a23b4e666df9881317bfc2344b "
               "-o \"$h\" = 193e4f48f46b2eed2023fbc1726cdb6e96f2f240ec1cc2b849d2a779dbc5cfd4") != 0)
        die("B firmware hash mismatch; no hardware operations");
    log_line("firmware sha accepted (stock or READY-patch COPY)");
    if (efs_is_mounted()) die("original EFS mounted: refusing");
    char state[64];
    read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
    if (strcmp(state, "OFFLINE") != 0) die("requires OFFLINE, got %s", state);
    if (dmesg_has_bad_cfg() != 0) die("BAD CFG or unavailable kernel log");
    size_t len = 0;
    uint8_t *bin = read_file("/tmp/saaios-probe-b-modem.bin", &len);
    if (!bin || len < sizeof(struct toc_entry)) die("missing verified B image");
    const struct toc_entry *toc = (const struct toc_entry *)bin;
    if (!toc[0].idx || toc[0].idx > 16 ||
        len < toc[0].idx * sizeof(*toc)) die("invalid TOC");
    const struct toc_entry *boot = find_toc(toc, toc[0].idx, "BOOT");
    const struct toc_entry *stage = find_toc(toc, toc[0].idx, "MAIN");
    if (!boot || !stage || boot->idx != 1 || stage->idx != 2 ||
        boot->size != 0x16800 || !boot->b_off || !stage->b_off ||
        stage->size <= 0x02400000u ||
        (uint64_t)boot->b_off + boot->size > len ||
        (uint64_t)stage->b_off + stage->size > len) die("unexpected TOC layout");
    log_line("B probe BOOT=%x MAIN=%x", boot->size, stage->size);
#ifdef PROBE_FIRMWARE_ONLY
    const char *names[] = { "VSS", "APM" };
    const struct toc_entry *extra[2];
    for (unsigned i = 0; i < 2; i++) {
        extra[i] = find_toc(toc, toc[0].idx, names[i]);
        if (!extra[i] || extra[i]->idx != i + 3 || !extra[i]->size ||
            !extra[i]->b_off || (uint64_t)extra[i]->b_off + extra[i]->size > len)
            die("invalid %s before POWER_ON", names[i]);
    }
#endif
    boot_fd = open(BOOT0_PATH, O_RDWR | O_CLOEXEC);
#ifdef PROBE_COMPLETE
    const char *nv_names[] = { "NV_NORM", "NV_PROT" };
    const char *nv_paths[] = { NV_NORM_PATH, NV_PROT_PATH };
    uint8_t *nv_data[2];
    for (unsigned i = 0; i < 2; i++) {
        const struct toc_entry *e = find_toc(toc, toc[0].idx, nv_names[i]);
        size_t n = 0;
        nv_data[i] = read_file(nv_paths[i], &n);
        if (!e || e->idx != i+5 || e->b_off != 0 || e->size != 0x80000 ||
            !nv_data[i] || n != e->size) die("invalid verified NV copy");
    }
#endif
    if (boot_fd < 0) die("boot node: %s", strerror(errno));
    if (do_ioctl("POWER_ON", IOCTL_POWER_ON, NULL) < 0) die("POWER_ON failed");
    sleep(3);
    read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
    if (strcmp(state, "OFFLINE") != 0 || dmesg_has_bad_cfg() != 0) die("not safe after POWER_ON");
    if (load_image("BOOT", bin + boot->b_off, boot->size, 0, 0) < 0) die("BOOT load failed");
    struct boot_mode mode = { .idx = CP_BOOT_MODE_NORMAL };
    if (do_ioctl("START", IOCTL_START_CP_BOOTLOADER, &mode) < 0) die("START failed");
    read_trimmed(MODEM_STATE_PATH, state, sizeof(state));
    if (strcmp(state, "BOOTING") != 0 || dmesg_has_bad_cfg() != 0) die("not safe after START");
#ifdef PROBE_HANDOVER
    int handover_rc = do_ioctl("HANDOVER_RAM_ONLY", 0x6f57, handover);
    wipe(handover, sizeof(handover));
    if (handover_rc < 0) die("handover failed; no firmware stages");
#endif
#ifdef PROBE_PREAMBLE
    log_line("FACTORY PREAMBLE: READY, TOC START/BIN/DONE");
    if (send_factory_preamble(bin, len) < 0) die("preamble failed; no MAIN");
#endif
    int result = sit_send_stage(stage->idx, "MAIN", bin + stage->b_off, stage->size, stage->crc);
#ifdef PROBE_FIRMWARE_ONLY
    for (unsigned i = 0; result == 0 && i < 2; i++) {
        log_line("Firmware stage %s; factory CRC disabled", names[i]);
        result = sit_send_stage(extra[i]->idx, names[i], bin + extra[i]->b_off,
                                extra[i]->size, extra[i]->crc);
    }
    log_line("FIRMWARE STAGES END");
#endif
#ifdef PROBE_COMPLETE
    for (unsigned i = 0; result == 0 && i < 2; i++) {
        result = sit_send_stage(i+5, nv_names[i], nv_data[i], 0x80000, 0);
    }
    if (result == 0) {
        int ipc_fd = open("/dev/umts_ipc0", O_RDWR | O_NONBLOCK | O_CLOEXEC);
        int rfs_fd = open("/dev/umts_rfs0", O_RDWR | O_NONBLOCK | O_CLOEXEC);
        if (ipc_fd < 0 || rfs_fd < 0) die("runtime endpoints unavailable; no FIN");
        result = sit_req_resp(SIT_FIN, SIT_FIN_ACK, SIT_ACK_DEADLINE_MS);
        if (result == 0) result = do_ioctl("COMPLETE", IOCTL_COMPLETE_NORMAL_BOOTUP, NULL);
#ifdef PROBE_QUERY_SIM
        if (result == 0) {
            log_line("SIM query with IPC/RFS held open; no RFS filesystem service");
            int query = system("/tmp/sit-sim-status query-sim-status");
            log_line("SIM query process status=%d (separate from boot result)", query);
        }
#endif
        print_modem_state("post-complete");
        if (result == 0) hold_channels_poll_sim(ipc_fd, rfs_fd);
        close(ipc_fd); close(rfs_fd);
    }
    for (unsigned i = 0; i < 2; i++) free(nv_data[i]);
#endif
    log_line("PROBE END result=%d (2=bound reached, -1=transfer failed)", result);
    print_modem_state("probe-end");
    free(bin);
    close(boot_fd);
#ifdef PROBE_FULL_MAIN
    return result == 0 ? 0 : 1;
#else
    return result == 2 ? 0 : 1;
#endif
}
