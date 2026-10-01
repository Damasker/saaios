/*
 * Opt-in diagnostic owner for a CP boot-probe handoff, not a telephony service.
 * The probe opens IPC0/RFS0 before FIN, forks/execs this process with those
 * descriptors, and must wait for READY\n before sending FIN or COMPLETE.
 *
 * After ONLINE it sends four allowlisted, read-only SIT status GETs once.
 * SIM-status-change indications may then schedule up to three debounced,
 * read-only SIM status refreshes through the same IPC reader.
 * It never sends RFS replies or accesses NV/EFS.
 * It consumes unknown RFS requests without replying, so CP may still wait or
 * fail: this is observability only, not a substitute for the factory rfsd.
 * Logs contain frame-header metadata and allowlisted raw status fields,
 * never payload dumps or SIM identifiers.
 */
#define _GNU_SOURCE
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "sit-network-layout.h"
#include "sit-sim-layout.h"

#ifndef _WIN32
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <sys/file.h>
#include <sys/ioctl.h>
#include <sys/prctl.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <time.h>
#include <unistd.h>
#endif

enum { RX_CAP = 65536, EMPTY_READ_BACKOFF_MS = 100,
       BOOTING_LIMIT_MS = 60000, POLL_SLICE_MS = 250,
       HEARTBEAT_MS = 60000, METADATA_KEYS = 16,
       RFS_HEADER_TRACE_LIMIT = 16,
       SIM_REFRESH_MAX = 3, SIM_REFRESH_DEBOUNCE_MS = 500,
       SIM_REFRESH_COALESCE_MS = 2000, SIM_REFRESH_MIN_GAP_MS = 3000,
       SIM_REFRESH_REPLY_MS = 10000 };
enum channel_kind { CHANNEL_IPC, CHANNEL_RFS };

static unsigned le16(const uint8_t *p) {
    return (unsigned)p[0] | ((unsigned)p[1] << 8);
}

static uint32_t le32(const uint8_t *p) {
    return (uint32_t)le16(p) | ((uint32_t)le16(p + 2) << 16);
}

struct rfs_header_fields {
    unsigned command;
    unsigned sequence;
    uint32_t payload_len;
};

/* Caller supplies a complete RFS frame (minimum eight-byte header). */
static struct rfs_header_fields rfs_header_fields(const uint8_t *p) {
    uint32_t word = le32(p);
    return (struct rfs_header_fields){
        .command = word & 0xffffU,
        .sequence = word >> 16,
        .payload_len = le32(p + 4)
    };
}

static void put32(uint8_t *p, uint32_t value) {
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
    p[2] = (uint8_t)(value >> 16);
    p[3] = (uint8_t)(value >> 24);
}

static int is_reply(const uint8_t *p, size_t n, unsigned id, uint32_t token) {
    return n >= 12 && p[0] == 1 && le16(p + 2) == id &&
           le32(p + 6) == token;
}

/* Factory TD1A: 0x0210 is SIT_IND_SIM_STATUS_CHANGED, not a reply. */
static int is_sim_status_changed(const uint8_t *p, size_t n) {
    return n >= 8 && p[0] == 2 && le16(p + 2) == 0x0210;
}

struct sim_refresh {
    uint32_t token;
    unsigned sent;
    int queued;
    int pending;
    int disabled;
    int64_t first_queued_ms;
    int64_t due_ms;
    int64_t next_allowed_ms;
    int64_t reply_deadline_ms;
};

/* Repeated indications form one bounded queue entry; continuous traffic
 * cannot postpone that entry beyond the first indication +2 seconds. */
static void sim_refresh_note_indication(struct sim_refresh *refresh,
                                        const uint8_t *p, size_t n,
                                        int64_t now_ms) {
    if (!is_sim_status_changed(p, n) || refresh->disabled ||
        refresh->sent >= SIM_REFRESH_MAX) return;
    if (!refresh->queued) {
        refresh->queued = 1;
        refresh->first_queued_ms = now_ms;
    }
    int64_t latest = now_ms + SIM_REFRESH_DEBOUNCE_MS;
    int64_t cap = refresh->first_queued_ms + SIM_REFRESH_COALESCE_MS;
    refresh->due_ms = latest < cap ? latest : cap;
}

static int sim_refresh_expire(struct sim_refresh *refresh, int64_t now_ms) {
    if (!refresh->pending || now_ms < refresh->reply_deadline_ms) return 0;
    refresh->pending = 0;
    return 1;
}

static int sim_refresh_ready(const struct sim_refresh *refresh,
                             int initial_done, int64_t now_ms) {
    return initial_done && !refresh->disabled && !refresh->pending &&
           refresh->queued && refresh->sent < SIM_REFRESH_MAX &&
           now_ms >= refresh->due_ms && now_ms >= refresh->next_allowed_ms;
}

static void sim_refresh_mark_sent(struct sim_refresh *refresh,
                                  uint32_t token, int64_t now_ms) {
    refresh->queued = 0;
    refresh->pending = 1;
    refresh->token = token;
    refresh->sent++;
    refresh->reply_deadline_ms = now_ms + SIM_REFRESH_REPLY_MS;
    refresh->next_allowed_ms = now_ms + SIM_REFRESH_MIN_GAP_MS;
}

static int sim_refresh_match_reply(struct sim_refresh *refresh,
                                   const uint8_t *p, size_t n,
                                   int64_t now_ms) {
    if (!refresh->pending || now_ms >= refresh->reply_deadline_ms ||
        !is_reply(p, n, 0x0200, refresh->token)) return 0;
    refresh->pending = 0;
    return 1;
}

static int make_get_request(uint8_t request[12], unsigned id,
                            uint32_t token) {
    if (id != 0x0200 && id != 0x0801 &&
        id != SIT_NET_VOICE_REG && id != SIT_NET_DATA_REG) return -1;
    memset(request, 0, 12);
    request[2] = (uint8_t)id;
    request[3] = (uint8_t)(id >> 8);
    request[4] = 12;
    put32(request + 6, token);
    return 0;
}

/* Zero means incomplete, -1 invalid/over cap, positive a complete frame. */
static int frame_size(enum channel_kind kind, const uint8_t *p, size_t n) {
    if (kind == CHANNEL_IPC) {
        if (n < 6) return 0;
        if (p[0] > 2) return -1;
        unsigned size = le16(p + 4);
        unsigned minimum = p[0] == 2 ? 8U : 12U;
        if (size < minimum || size > RX_CAP) return -1;
        return n < size ? 0 : (int)size;
    }
    if (n < 8) return 0;
    /* RFS command frames observed in the factory diagnostic: payload length
     * at +4, following an eight-byte header. No command is interpreted. */
    uint64_t size = (uint64_t)le32(p + 4) + 8U;
    if (size > RX_CAP) return -1;
    return n < size ? 0 : (int)size;
}

typedef void (*frame_callback)(void *, enum channel_kind,
                               const uint8_t *, int);

/* Common streaming parser used by both the live reader and host fixtures. */
static int parse_available(enum channel_kind kind, uint8_t rx[RX_CAP],
                           size_t *used, frame_callback on_frame, void *context) {
    if (*used > RX_CAP) return -1;
    size_t offset = 0;
    while (offset < *used) {
        int size = frame_size(kind, rx + offset, *used - offset);
        if (size < 0) return -1;
        if (!size) break;
        on_frame(context, kind, rx + offset, size);
        offset += (size_t)size;
    }
    if (offset) {
        size_t remaining = *used - offset;
        memmove(rx, rx + offset, remaining);
        memset(rx + remaining, 0, offset);
        *used = remaining;
    }
    return *used == RX_CAP ? -1 : 0;
}

static void fixture_frame(void *opaque, enum channel_kind kind,
                          const uint8_t *p, int size) {
    unsigned *count = opaque;
    (void)kind;
    (void)p;
    (void)size;
    (*count)++;
}

static int fixture_sim_refresh(void) {
    uint8_t indication[8] = {2, 0, 0x10, 0x02, 8, 0, 0, 0};
    uint8_t reply[12] = {1, 0, 0, 2, 12, 0};
    if (!is_sim_status_changed(indication, sizeof indication) ||
        is_sim_status_changed(indication, 7)) return 17;
    indication[0] = 1;
    if (is_sim_status_changed(indication, sizeof indication)) return 18;
    indication[0] = 2; indication[2] = 0x11;
    if (is_sim_status_changed(indication, sizeof indication)) return 19;
    indication[2] = 0x10;

    struct sim_refresh refresh = {0};
    sim_refresh_note_indication(&refresh, indication, sizeof indication, 1000);
    if (!refresh.queued || refresh.due_ms != 1500 ||
        sim_refresh_ready(&refresh, 0, 1500)) return 20;
    sim_refresh_note_indication(&refresh, indication, sizeof indication, 1300);
    if (refresh.due_ms != 1800) return 21;
    sim_refresh_note_indication(&refresh, indication, sizeof indication, 2900);
    if (refresh.due_ms != 3000 ||
        sim_refresh_ready(&refresh, 1, 2999) ||
        !sim_refresh_ready(&refresh, 1, 3000)) return 22;
    sim_refresh_mark_sent(&refresh, 11, 3000);
    if (refresh.queued || !refresh.pending || refresh.sent != 1 ||
        sim_refresh_ready(&refresh, 1, 3500)) return 23;
    put32(reply + 6, 10);
    if (sim_refresh_match_reply(&refresh, reply, sizeof reply, 3200) ||
        !refresh.pending) return 24;
    sim_refresh_note_indication(&refresh, indication, sizeof indication, 3100);
    if (sim_refresh_expire(&refresh, 12999) ||
        sim_refresh_match_reply(&refresh, reply, sizeof reply, 13000) ||
        !sim_refresh_expire(&refresh, 13000) ||
        !sim_refresh_ready(&refresh, 1, 13000)) return 25;
    sim_refresh_mark_sent(&refresh, 12, 13000);
    put32(reply + 6, 11);
    if (sim_refresh_match_reply(&refresh, reply, sizeof reply, 13001)) return 26;
    put32(reply + 6, 12);
    if (!sim_refresh_match_reply(&refresh, reply, sizeof reply, 13001) ||
        refresh.pending) return 27;
    sim_refresh_note_indication(&refresh, indication, sizeof indication, 13010);
    if (sim_refresh_ready(&refresh, 1, 15999) ||
        !sim_refresh_ready(&refresh, 1, 16000)) return 28;
    sim_refresh_mark_sent(&refresh, 13, 16000);
    sim_refresh_note_indication(&refresh, indication, sizeof indication, 16010);
    if (refresh.queued || refresh.sent != SIM_REFRESH_MAX) return 29;
    put32(reply + 6, 13);
    if (!sim_refresh_match_reply(&refresh, reply, sizeof reply, 16001) ||
        sim_refresh_ready(&refresh, 1, 17000)) return 30;

    struct sim_refresh no_retry = {0};
    sim_refresh_note_indication(&no_retry, indication, sizeof indication, 0);
    sim_refresh_mark_sent(&no_retry, 21, 500);
    if (!sim_refresh_expire(&no_retry, 10500) || no_retry.queued ||
        sim_refresh_ready(&no_retry, 1, 20000)) return 31;
    put32(reply + 6, 21);
    if (sim_refresh_match_reply(&no_retry, reply, sizeof reply, 10501)) return 32;
    return 0;
}

static int fixture(void) {
    int refresh_check = fixture_sim_refresh();
    if (refresh_check) return refresh_check;
    const uint8_t ipc[] = {2, 0, 0x34, 0x12, 8, 0, 0, 0};
    const uint8_t rfs[] = {7, 0, 0, 0, 4, 0, 0, 0, 3, 0, 0, 0};
    for (size_t n = 0; n < sizeof ipc; ++n)
        if (frame_size(CHANNEL_IPC, ipc, n) != 0) return 1;
    if (frame_size(CHANNEL_IPC, ipc, sizeof ipc) != 8 ||
        le16(ipc + 2) != 0x1234) return 2;
    for (size_t n = 0; n < sizeof rfs; ++n)
        if (frame_size(CHANNEL_RFS, rfs, n) != 0) return 3;
    if (frame_size(CHANNEL_RFS, rfs, sizeof rfs) != 12 ||
        le32(rfs) != 7) return 4;
    struct rfs_header_fields header = rfs_header_fields(rfs);
    if (header.command != 7 || header.sequence != 0 ||
        header.payload_len != 4) return 15;
    const uint8_t rfs_next[] = {6, 0, 1, 0, 16, 0, 0, 0};
    header = rfs_header_fields(rfs_next);
    if (header.command != 6 || header.sequence != 1 ||
        header.payload_len != 16) return 16;
    uint8_t bad[12];
    memcpy(bad, rfs, sizeof bad);
    bad[7] = 1;
    if (frame_size(CHANNEL_RFS, bad, sizeof bad) != -1) return 5;
    memcpy(bad, ipc, sizeof ipc);
    bad[0] = 3;
    if (frame_size(CHANNEL_IPC, bad, sizeof ipc) != -1) return 6;
    bad[0] = 2; bad[4] = 7;
    if (frame_size(CHANNEL_IPC, bad, sizeof ipc) != -1) return 7;
    uint8_t response[16] = {1, 0, 0, 2, 16, 0, 0, 0, 0, 0, 0};
    put32(response + 6, 0xabcdef12U);
    if (!is_reply(response, sizeof response, 0x0200, 0xabcdef12U) ||
        is_reply(response, sizeof response, 0x0200, 0xabcdef13U) ||
        is_reply(response, sizeof response, 0x0801, 0xabcdef12U)) return 8;
    response[0] = 2;
    if (is_reply(response, sizeof response, 0x0200, 0xabcdef12U)) return 9;
    uint8_t get[12];
    if (make_get_request(get, SIT_NET_DATA_REG, 0xabcdef12U) ||
        get[0] != 0 || le16(get + 2) != SIT_NET_DATA_REG ||
        le16(get + 4) != 12 || le32(get + 6) != 0xabcdef12U ||
        make_get_request(get, SIT_NET_OPERATOR, 1) == 0 ||
        make_get_request(get, SIT_NET_ALLOW_DATA, 1) == 0) return 14;
    uint8_t stream[RX_CAP] = {0};
    size_t used = 0;
    unsigned frames = 0;
    memcpy(stream, ipc, 3); used = 3;
    if (parse_available(CHANNEL_IPC, stream, &used, fixture_frame, &frames) ||
        used != 3 || frames != 0) return 10;
    memcpy(stream + used, ipc + 3, 5); used += 5;
    memcpy(stream + used, ipc, sizeof ipc); used += sizeof ipc;
    if (parse_available(CHANNEL_IPC, stream, &used, fixture_frame, &frames) ||
        used != 0 || frames != 2) return 11;
    memcpy(stream, rfs, sizeof rfs);
    memcpy(stream + sizeof rfs, rfs, 6);
    used = sizeof rfs + 6; frames = 0;
    if (parse_available(CHANNEL_RFS, stream, &used, fixture_frame, &frames) ||
        used != 6 || frames != 1) return 12;
    memcpy(stream + used, rfs + 6, 6); used += 6;
    if (parse_available(CHANNEL_RFS, stream, &used, fixture_frame, &frames) ||
        used != 0 || frames != 2) return 13;
    puts("PASS modem-channel-owner framing, response-token and SIM-refresh fixtures");
    return 0;
}

#ifndef _WIN32
enum cp_state { CP_INVALID, CP_OFFLINE, CP_BOOTING, CP_ONLINE };
/* Google/Samsung modem_prj.h: _IOR(IOCTL_MAGIC='o', 0x59, int). */
#define IOCTL_GET_OPENED_STATUS _IOR('o', 0x59, int)

struct channel {
    int fd;
    enum channel_kind kind;
    uint8_t rx[RX_CAP];
    size_t used;
    uint64_t frames;
    int64_t backoff_until_ms;
};

struct metadata_entry {
    uint32_t key;
    uint64_t count;
    unsigned min_len;
    unsigned max_len;
};

struct metadata_counts {
    struct metadata_entry ipc[METADATA_KEYS];
    struct metadata_entry rfs[METADATA_KEYS];
    uint64_t ipc_other;
    uint64_t rfs_other;
    uint64_t ipc_total;
    uint64_t rfs_total;
    int64_t window_start_ms;
};

struct snapshot {
    uint32_t token;
    size_t next;
    int started;
    int pending;
    int aborted;
    int64_t deadline_ms;
};

struct rfs_header_trace {
    int enabled;
    unsigned logged;
    uint64_t overflow;
    uint64_t overflow_reported;
    int64_t start_ms;
};

static const struct {
    unsigned id;
    const char *label;
} snapshot_gets[] = {
    {0x0200, "sim"}, {0x0801, "radio"},
    {SIT_NET_VOICE_REG, "voice"}, {SIT_NET_DATA_REG, "data"}
};

static volatile sig_atomic_t stop_requested;

static void request_stop(int sig) {
    (void)sig;
    stop_requested = 1;
}

static int64_t monotonic_ms(void) {
    struct timespec ts;
    if (clock_gettime(CLOCK_MONOTONIC, &ts)) return -1;
    return (int64_t)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

static enum cp_state cp_state(void) {
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    char state[32] = {0};
    if (!f) return CP_INVALID;
    int got = fscanf(f, "%31s", state);
    fclose(f);
    if (got != 1) return CP_INVALID;
    if (!strcmp(state, "BOOTING")) return CP_BOOTING;
    if (!strcmp(state, "ONLINE")) return CP_ONLINE;
    if (!strcmp(state, "OFFLINE")) return CP_OFFLINE;
    return CP_INVALID;
}

static int parse_fd(const char *text) {
    char *end = NULL;
    errno = 0;
    long n = strtol(text, &end, 10);
    if (errno || end == text || *end || n < 3 || n > INT_MAX) return -1;
    return (int)n;
}

static int parse_args(int argc, char **argv, int *ipc, int *rfs, int *ready) {
    *ipc = *rfs = *ready = -1;
    if (argc != 7) return -1;
    for (int i = 1; i < argc; i += 2) {
        int fd = parse_fd(argv[i + 1]);
        if (fd < 0) return -1;
        if (!strcmp(argv[i], "--ipc-fd") && *ipc < 0) *ipc = fd;
        else if (!strcmp(argv[i], "--rfs-fd") && *rfs < 0) *rfs = fd;
        else if (!strcmp(argv[i], "--ready-fd") && *ready < 0) *ready = fd;
        else return -1;
    }
    return *ipc >= 0 && *rfs >= 0 && *ready >= 0 &&
           *ipc != *rfs && *ipc != *ready && *rfs != *ready ? 0 : -1;
}

static int verify_node(int fd, const char *sysdev) {
    unsigned expected_major = 0, expected_minor = 0;
    FILE *f = fopen(sysdev, "r");
    if (!f) return -1;
    int got = fscanf(f, "%u:%u", &expected_major, &expected_minor);
    fclose(f);
    if (got != 2) return -1;
    struct stat st;
    if (fstat(fd, &st) || !S_ISCHR(st.st_mode) ||
        major(st.st_rdev) != expected_major ||
        minor(st.st_rdev) != expected_minor) return -1;
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || (flags & O_ACCMODE) != O_RDWR) return -1;
    if (!(flags & O_NONBLOCK) && fcntl(fd, F_SETFL, flags | O_NONBLOCK)) return -1;
    return 0;
}

static int verify_ready_pipe(int fd) {
    struct stat st;
    int flags = fcntl(fd, F_GETFL);
    return flags >= 0 && (flags & O_ACCMODE) == O_WRONLY &&
           fstat(fd, &st) == 0 && S_ISFIFO(st.st_mode) ? 0 : -1;
}

static int opened_once(int fd) {
    int opened = -1;
    return ioctl(fd, IOCTL_GET_OPENED_STATUS, &opened) == 0 &&
           opened == 1 ? 0 : -1;
}

static int acquire_lock(void) {
    const char *path = "/run/saaios-sit-status.lock";
    int fd = open(path, O_CREAT | O_RDWR | O_CLOEXEC | O_NOFOLLOW, 0600);
    if (fd < 0) return -1;
    struct stat held, named;
    if (fstat(fd, &held) || !S_ISREG(held.st_mode) || held.st_nlink != 1 ||
        flock(fd, LOCK_EX | LOCK_NB) || lstat(path, &named) ||
        named.st_dev != held.st_dev || named.st_ino != held.st_ino) {
        close(fd);
        return -1;
    }
    return fd;
}

static int acknowledge_ready(int fd) {
    static const char message[] = "READY\n";
    size_t written = 0;
    while (written < sizeof message - 1) {
        ssize_t n = write(fd, message + written,
                          sizeof message - 1 - written);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) return -1;
        written += (size_t)n;
    }
    /* Deliberately leave fd open: the probe can detect an immediate exit. */
    return 0;
}

static void count_frame(struct channel *channel, const uint8_t *p,
                        int size, struct metadata_counts *counts) {
    struct metadata_entry *entries;
    uint64_t *other;
    uint32_t key;
    if (channel->kind == CHANNEL_IPC) {
        counts->ipc_total++;
        entries = counts->ipc;
        other = &counts->ipc_other;
        key = ((uint32_t)p[0] << 16) | le16(p + 2);
    } else {
        counts->rfs_total++;
        entries = counts->rfs;
        other = &counts->rfs_other;
        key = le32(p);
    }
    for (size_t i = 0; i < (size_t)METADATA_KEYS; ++i) {
        if (!entries[i].count || entries[i].key == key) {
            entries[i].key = key;
            if (!entries[i].count) entries[i].min_len = (unsigned)size;
            if ((unsigned)size < entries[i].min_len)
                entries[i].min_len = (unsigned)size;
            if ((unsigned)size > entries[i].max_len)
                entries[i].max_len = (unsigned)size;
            entries[i].count++;
            return;
        }
    }
    (*other)++;
}

/* One bounded redacted line per minute, including per-header counts. */
static void report_metadata(struct metadata_counts *counts, int64_t now) {
    if (!counts->ipc_total && !counts->rfs_total) {
        counts->window_start_ms = now;
        return;
    }
    printf("heartbeat_ms=%lld ipc_total=%llu rfs_total=%llu",
           (long long)(now - counts->window_start_ms),
           (unsigned long long)counts->ipc_total,
           (unsigned long long)counts->rfs_total);
    for (size_t i = 0; i < (size_t)METADATA_KEYS; ++i) {
        if (counts->ipc[i].count)
            printf(" ipc_%u:0x%04x=%llu,len=%u-%u", counts->ipc[i].key >> 16,
                   counts->ipc[i].key & 0xffff,
                   (unsigned long long)counts->ipc[i].count,
                   counts->ipc[i].min_len, counts->ipc[i].max_len);
        if (counts->rfs[i].count)
            printf(" rfs_0x%08x=%llu,len=%u-%u", counts->rfs[i].key,
                   (unsigned long long)counts->rfs[i].count,
                   counts->rfs[i].min_len, counts->rfs[i].max_len);
    }
    if (counts->ipc_other)
        printf(" ipc_other=%llu", (unsigned long long)counts->ipc_other);
    if (counts->rfs_other)
        printf(" rfs_other=%llu", (unsigned long long)counts->rfs_other);
    putchar('\n');
    memset(counts, 0, sizeof *counts);
    counts->window_start_ms = now;
}

static void trace_rfs_header(struct rfs_header_trace *trace,
                             const uint8_t *p, int64_t now_ms) {
    if (!trace->enabled) return;
    if (trace->logged >= RFS_HEADER_TRACE_LIMIT) {
        if (trace->overflow != UINT64_MAX) trace->overflow++;
        return;
    }
    struct rfs_header_fields header = rfs_header_fields(p);
    printf("rfs_header_ms=%lld cmd=%u seq=%u payload_len=%u\n",
           (long long)(now_ms - trace->start_ms),
           header.command, header.sequence, (unsigned)header.payload_len);
    trace->logged++;
}

static void report_rfs_overflow(struct rfs_header_trace *trace) {
    if (!trace->enabled || trace->overflow == trace->overflow_reported) return;
    printf("rfs_header_overflow=%llu\n",
           (unsigned long long)trace->overflow);
    trace->overflow_reported = trace->overflow;
}

static void print_sim_fields(const uint8_t *p, size_t size) {
    if (size < 15) { printf(" status=short"); return; }
    unsigned apps = p[SIT_SIM_APPS];
    printf(" card_raw=%u apps=%u", p[SIT_SIM_CARD], apps);
    if (apps > 0 && apps <= 4 &&
        size >= 15U + SIT_SIM_APP_STRIDE * apps &&
        size > SIT_SIM_PIN1)
        printf(" app_state_raw=%u pin1_raw=%u",
               p[SIT_SIM_APP_STATE], p[SIT_SIM_PIN1]);
}

static void snapshot_reply(struct snapshot *snapshot, const uint8_t *p,
                           size_t size) {
    if (!snapshot->pending || snapshot->next >=
        sizeof snapshot_gets / sizeof snapshot_gets[0] ||
        !is_reply(p, size, snapshot_gets[snapshot->next].id,
                  snapshot->token)) return;
    unsigned id = snapshot_gets[snapshot->next].id;
    const char *label = snapshot_gets[snapshot->next].label;
    snapshot->pending = 0;
    snapshot->next++;
    printf("snapshot %s response=yes error_raw=%u", label, (unsigned)p[10]);
    if (p[10]) { putchar('\n'); return; }
    if (id == 0x0200) {
        print_sim_fields(p, size);
    } else if (id == 0x0801 && size >= 16) {
        printf(" radio_raw=%u", le32(p + 12));
    } else if ((id == SIT_NET_VOICE_REG && size >= 14) ||
               (id == SIT_NET_DATA_REG && size >= 16)) {
        printf(" registration_raw=%u reject_raw=%u",
               p[SIT_NET_REG_STATE_OFFSET], p[SIT_NET_REJECT_OFFSET]);
        if (id == SIT_NET_DATA_REG)
            printf(" tech_raw=%u", p[SIT_NET_DATA_TECH_OFFSET]);
    } else {
        printf(" status=short");
    }
    putchar('\n');
}

static void snapshot_advance(struct snapshot *snapshot, int ipc_fd,
                             int64_t now_ms) {
    if (snapshot->aborted) return;
    if (!snapshot->started) {
        snapshot->started = 1;
        /* Session-specific tokens distinguish these GETs from earlier tools. */
        snapshot->token = (uint32_t)now_ms ^ (uint32_t)getpid() ^ 0x5a170000U;
        puts("snapshot=started commands=read-only-sim-radio-voice-data");
    }
    if (snapshot->pending) {
        if (now_ms < snapshot->deadline_ms) return;
        printf("snapshot %s response=no status=timeout\n",
               snapshot_gets[snapshot->next].label);
        snapshot->pending = 0;
        snapshot->next++;
    }
    if (snapshot->next >= sizeof snapshot_gets / sizeof snapshot_gets[0])
        return;
    unsigned id = snapshot_gets[snapshot->next].id;
    uint8_t request[12];
    snapshot->token++;
    if (make_get_request(request, id, snapshot->token)) {
        snapshot->aborted = 1;
        return;
    }
    /* A partial/EAGAIN write is never resent: avoid a guessed second frame. */
    if (write(ipc_fd, request, sizeof request) != (ssize_t)sizeof request) {
        printf("snapshot %s request=failed no-retry\n",
               snapshot_gets[snapshot->next].label);
        snapshot->aborted = 1;
        return;
    }
    printf("snapshot %s request=sent token=%u\n",
           snapshot_gets[snapshot->next].label, snapshot->token);
    snapshot->pending = 1;
    snapshot->deadline_ms = now_ms + 10000;
}

static int snapshot_finished(const struct snapshot *snapshot) {
    return snapshot->started && !snapshot->aborted && !snapshot->pending &&
           snapshot->next >= sizeof snapshot_gets / sizeof snapshot_gets[0];
}

static void sim_refresh_reply(struct sim_refresh *refresh, const uint8_t *p,
                              size_t size, int64_t now_ms) {
    if (!sim_refresh_match_reply(refresh, p, size, now_ms)) return;
    printf("sim_refresh response=yes request=%u error_raw=%u",
           refresh->sent, (unsigned)p[10]);
    if (!p[10]) print_sim_fields(p, size);
    putchar('\n');
}

static void sim_refresh_advance(struct sim_refresh *refresh,
                                struct snapshot *snapshot, int ipc_fd,
                                int64_t now_ms) {
    if (sim_refresh_expire(refresh, now_ms))
        printf("sim_refresh response=no request=%u status=timeout\n",
               refresh->sent);
    if (!sim_refresh_ready(refresh, snapshot_finished(snapshot), now_ms))
        return;
    uint8_t request[12];
    uint32_t token = ++snapshot->token;
    if (make_get_request(request, 0x0200, token) ||
        write(ipc_fd, request, sizeof request) != (ssize_t)sizeof request) {
        puts("sim_refresh request=failed no-retry disabled=yes");
        refresh->disabled = 1;
        refresh->queued = 0;
        return;
    }
    sim_refresh_mark_sent(refresh, token, now_ms);
    printf("sim_refresh request=sent request=%u\n", refresh->sent);
}

struct live_frame_context {
    struct channel *channel;
    struct metadata_counts *counts;
    struct snapshot *snapshot;
    struct sim_refresh *refresh;
    struct rfs_header_trace *trace;
    int64_t now_ms;
};

static void live_frame(void *opaque, enum channel_kind kind,
                       const uint8_t *p, int size) {
    struct live_frame_context *context = opaque;
    count_frame(context->channel, p, size, context->counts);
    if (kind == CHANNEL_IPC) {
        if (sim_refresh_expire(context->refresh, context->now_ms))
            printf("sim_refresh response=no request=%u status=timeout\n",
                   context->refresh->sent);
        snapshot_reply(context->snapshot, p, (size_t)size);
        sim_refresh_reply(context->refresh, p, (size_t)size,
                          context->now_ms);
        sim_refresh_note_indication(context->refresh, p, (size_t)size,
                                    context->now_ms);
    } else
        trace_rfs_header(context->trace, p, context->now_ms);
    context->channel->frames++;
}

/* Return -1 on malformed/over-cap framing; never retain payload after parse. */
static int drain_channel(struct channel *channel, struct metadata_counts *counts,
                         struct snapshot *snapshot,
                         struct sim_refresh *refresh,
                         struct rfs_header_trace *trace) {
    if (channel->used == sizeof channel->rx) return -1;
    ssize_t n = read(channel->fd, channel->rx + channel->used,
                     sizeof channel->rx - channel->used);
    if (n < 0 && errno == EINTR) return 0;
    if (n == 0 || (n < 0 && (errno == EAGAIN || errno == EWOULDBLOCK))) {
        /* This CP character driver may return zero after a 100 ms empty
         * read. EAGAIN or a zero read is not EOF; avoid a POLLIN spin. */
        int64_t now = monotonic_ms();
        if (now < 0) return -1;
        channel->backoff_until_ms = now + EMPTY_READ_BACKOFF_MS;
        return 0;
    }
    if (n < 0) return -1;
    channel->used += (size_t)n;
    int64_t now = monotonic_ms();
    if (now < 0) return -1;
    struct live_frame_context context = {
        channel, counts, snapshot, refresh, trace, now
    };
    return parse_available(channel->kind, channel->rx, &channel->used,
                           live_frame, &context);
}

static int run_owner(int ipc, int rfs, int ready, int lock, int attached) {
    struct rlimit no_core = {0, 0};
    if (setrlimit(RLIMIT_CORE, &no_core) || prctl(PR_SET_DUMPABLE, 0)) {
        fputs("ABORT could not disable core dumps\n", stderr);
        if (lock >= 0) close(lock);
        return 1;
    }
    if (verify_node(ipc, "/sys/class/cpif/umts_ipc0/dev") ||
        verify_node(rfs, "/sys/class/cpif/umts_rfs0/dev") ||
        (!attached && verify_ready_pipe(ready))) {
        fputs("ABORT IPC/RFS/READY descriptors failed validation\n", stderr);
        if (lock >= 0) close(lock);
        return 1;
    }
    enum cp_state state = cp_state();
    if (attached ? state != CP_ONLINE : state != CP_BOOTING) {
        fputs("ABORT CP state changed before owner startup\n", stderr);
        if (lock >= 0) close(lock);
        return 1;
    }
    if (lock < 0) lock = acquire_lock();
    if (lock < 0) {
        fputs("ABORT common SIT lock busy or unstable\n", stderr);
        return 1;
    }
    struct sigaction action = {0};
    action.sa_handler = request_stop;
    sigemptyset(&action.sa_mask);
    if (sigaction(SIGTERM, &action, NULL) ||
        sigaction(SIGINT, &action, NULL) ||
        signal(SIGHUP, SIG_IGN) == SIG_ERR ||
        signal(SIGPIPE, SIG_IGN) == SIG_ERR) {
        fputs("ABORT signal setup failed\n", stderr);
        close(lock);
        return 1;
    }
    setvbuf(stdout, NULL, _IOLBF, 0);
    struct channel channels[2] = {
        {.fd = ipc, .kind = CHANNEL_IPC},
        {.fd = rfs, .kind = CHANNEL_RFS}
    };
    struct metadata_counts counts = {0};
    struct snapshot snapshot = {0};
    struct sim_refresh refresh = {0};
    int64_t start = monotonic_ms();
    struct rfs_header_trace trace = {.enabled = !attached, .start_ms = start};
    counts.window_start_ms = start;
    if (!attached && (cp_state() != CP_BOOTING ||
                      opened_once(ipc) || opened_once(rfs))) {
        fputs("ABORT pre-FIN BOOTING or exclusive endpoint check failed\n", stderr);
        close(lock);
        return 1;
    }
    if (start < 0 || (!attached && acknowledge_ready(ready))) {
        fputs("ABORT READY pipe unavailable\n", stderr);
        close(lock);
        return 1;
    }
    puts(attached ?
         "owner=attach-online history=unknown channels=ipc0,rfs0 payload=redacted rfs_responses=none" :
         "owner=ready channels=ipc0,rfs0 payload=redacted rfs_responses=none");
    int rc = 0;
    int deferred_stop_logged = 0;
    for (;;) {
        int64_t now = monotonic_ms();
        state = cp_state();
        if (now < 0 || state == CP_INVALID) { rc = 1; break; }
        if (state == CP_OFFLINE) break;
        if (state == CP_BOOTING && now - start > BOOTING_LIMIT_MS) {
            fputs("ABORT CP stayed BOOTING after READY\n", stderr);
            rc = 1; break;
        }
        if (stop_requested) {
            if (state == CP_BOOTING) break;
            if (!deferred_stop_logged) {
                fputs("SIGTERM deferred until CP OFFLINE to avoid RX purge\n", stderr);
                deferred_stop_logged = 1;
            }
        }
        if (now - counts.window_start_ms >= HEARTBEAT_MS) {
            report_metadata(&counts, now);
            report_rfs_overflow(&trace);
        }
        if (state == CP_ONLINE) {
            snapshot_advance(&snapshot, ipc, now);
            sim_refresh_advance(&refresh, &snapshot, ipc, now);
        }
        struct pollfd fds[2];
        int timeout = POLL_SLICE_MS;
        for (int i = 0; i < 2; ++i) {
            fds[i].fd = channels[i].backoff_until_ms > now ? -1 : channels[i].fd;
            fds[i].events = POLLIN;
            fds[i].revents = 0;
            if (channels[i].backoff_until_ms > now) {
                int64_t remaining = channels[i].backoff_until_ms - now;
                if (remaining < timeout) timeout = (int)remaining;
            }
        }
        int events = poll(fds, 2, timeout);
        if (events < 0 && errno == EINTR) continue;
        if (events < 0) { perror("owner poll"); rc = 1; break; }
        for (int i = 0; i < 2; ++i) {
            if (fds[i].revents & (POLLERR | POLLHUP | POLLNVAL)) {
                fprintf(stderr, "%s poll failure event=0x%x\n",
                        i ? "RFS" : "IPC", fds[i].revents);
                rc = 1; break;
            }
            if ((fds[i].revents & POLLIN) &&
                drain_channel(&channels[i], &counts, &snapshot, &refresh,
                              &trace)) {
                fprintf(stderr, "%s read/framing failure or buffer cap\n",
                        i ? "RFS" : "IPC");
                rc = 1; break;
            }
        }
        if (rc) break;
    }
    if (counts.ipc_total || counts.rfs_total) {
        int64_t now = monotonic_ms();
        report_metadata(&counts, now >= 0 ? now : counts.window_start_ms);
    }
    report_rfs_overflow(&trace);
    if (trace.enabled)
        printf("rfs_header_trace_logged=%u overflow=%llu\n", trace.logged,
               (unsigned long long)trace.overflow);
    printf("owner=stopped ipc_frames=%llu rfs_frames=%llu result=%d\n",
           (unsigned long long)channels[0].frames,
           (unsigned long long)channels[1].frames, rc);
    if (state != CP_OFFLINE)
        fputs("WARNING: closing a last endpoint may purge queued CP requests\n", stderr);
    if (!attached) close(ready);
    close(lock);
    /* The inherited device descriptors close on process exit. */
    return rc;
}

static int open_verified_node(const char *node, const char *sysdev) {
    int fd = open(node, O_RDWR | O_NONBLOCK | O_CLOEXEC | O_NOFOLLOW);
    if (fd < 0) return -1;
    if (verify_node(fd, sysdev)) { close(fd); return -1; }
    return fd;
}

static int attach_online(void) {
    if (cp_state() != CP_ONLINE) {
        fputs("ABORT attach requires already ONLINE CP\n", stderr);
        return 1;
    }
    int lock = acquire_lock();
    if (lock < 0) {
        fputs("ABORT common SIT lock busy or unstable\n", stderr);
        return 1;
    }
    int ipc = open_verified_node("/dev/umts_ipc0", "/sys/class/cpif/umts_ipc0/dev");
    int rfs = open_verified_node("/dev/umts_rfs0", "/sys/class/cpif/umts_rfs0/dev");
    int ipc_opened = 0, rfs_opened = 0;
    if (ipc < 0 || rfs < 0 || cp_state() != CP_ONLINE ||
        ioctl(ipc, IOCTL_GET_OPENED_STATUS, &ipc_opened) || ipc_opened != 1 ||
        ioctl(rfs, IOCTL_GET_OPENED_STATUS, &rfs_opened) || rfs_opened != 1) {
        fputs("ABORT attach cannot prove exclusive IPC/RFS ownership\n", stderr);
        if (ipc >= 0) close(ipc);
        if (rfs >= 0) close(rfs);
        close(lock);
        return 1;
    }
    int rc = run_owner(ipc, rfs, -1, lock, 1);
    close(rfs);
    close(ipc);
    return rc;
}
#endif

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "self-test")) return fixture();
#ifdef _WIN32
    (void)argc;
    (void)argv;
    fputs("Live channel ownership requires Linux.\n", stderr);
    return 69;
#else
    if (argc == 2 && !strcmp(argv[1], "--attach-online"))
        return attach_online();
    int ipc, rfs, ready;
    if (parse_args(argc, argv, &ipc, &rfs, &ready)) {
        fputs("usage: modem-channel-owner self-test | "
              "--attach-online | --ipc-fd N --rfs-fd N --ready-fd N\n", stderr);
        return 64;
    }
    return run_owner(ipc, rfs, ready, -1, 0);
#endif
}
