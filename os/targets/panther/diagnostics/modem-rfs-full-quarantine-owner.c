/*
 * Separate, opt-in S5300 RFS quarantine diagnostic. The passive owner and
 * one-grant owner are unchanged. The original EFS is never opened for writing.
 * CP bytes may only replace a bounded prefix of a private candidate copy.
 * A final success response is gated on durable readback and a binary SHA-256
 * sidecar; neither response nor sidecar authorizes promotion to live NV.
 * Never logs NV bytes, digests, or identifiers.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/file.h>
#include <sys/ioctl.h>
#include <sys/prctl.h>
#include <sys/random.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <sys/vfs.h>
#include <linux/magic.h>
#include <time.h>
#include <unistd.h>
#include "sit-network-layout.h"
#include "sit-sim-layout.h"

/* The combined owner (SAAIOS_RFS_CAMP) adds an active IPC camp dispatcher on
 * top of the unchanged RFS quarantine serving: on the early radio edge it
 * issues the stock stage-1 0x093f -> 0x0404 -> 0x0800 while the RFS machinery
 * serves the protected-NV cmd7/cmd3/cmd6 sequence into the quarantine copy.
 * The SGC compile modes belong to the separate modem-channel-owner.c program. */
#if defined(SAAIOS_RFS_CAMP) && (defined(SAAIOS_SCAN_ONCE) || \
    defined(SAAIOS_SGC_ONCE) || defined(SAAIOS_SGC_EARLY_ONCE) || \
    defined(SAAIOS_SGC_SEQ_ONCE) || defined(SAAIOS_SGC_CAMP_ONCE))
#error "SAAIOS_RFS_CAMP is mutually exclusive with the SGC owner modes"
#endif

enum { BASELINE_BYTES = 524288, FIRST_CHUNK = 2012,
       RFS_TRANSFER_BYTES = 189446, RFS_GRANTS_MAX = 95,
       RFS_FRAME_MAX = 20 + FIRST_CHUNK, RX_CAP = 4096,
       POLL_MS = 250, EMPTY_BACKOFF_MS = 100,
       FIRST_DEADLINE_MS = 60000, STEP_DEADLINE_MS = 30000,
       TOTAL_DEADLINE_MS = 300000, BOOTING_LIMIT_MS = 60000,
       SIT_RX_CAP = 65536, SIT_READ_SLICE = 1024,
       SIT_REPLY_MS = 10000, SIT_SETTLED_MS = 60000,
       SIT_RFS_GUARD_MS = 1000, SIT_GET_COUNT = 4,
       SIT_EVENT_WINDOW_MS = 60000, SIT_EVENT_TRACE_LIMIT = 8,
       SIT_EVENT_QUIET_COVERAGE_MS = 300000, RFS_TRACE_MAX = 512 };
#define IOCTL_GET_OPENED_STATUS _IOR('o', 0x59, int)
#define SOURCE_NAME "nv_protected.bin"
#define PIN_NAME "expected.sha256"
#define USED_PIN_NAME "consumed.sha256"
#define MARKER_NAME "NO_PROMOTION"
#define CANDIDATE_NAME "candidate.bin"
#define SIDECAR_NAME "candidate.sha256"

enum phase { WAIT_7, WAIT_3, WAIT_6, WAIT_DATA, TERMINAL };
enum action { BAD_FRAME, NO_REPLY, STATUS_7, GRANT_1, STORE_CHUNK };
enum cp_state { CP_UNKNOWN, CP_OFFLINE, CP_BOOTING, CP_ONLINE, CP_CRASH };
enum failure_stage {
    STAGE_NONE, STAGE_GRANT, STAGE_FINAL_GRANT, STAGE_RFS_FRAME,
    STAGE_FINAL_FRAME, STAGE_STORE_CHUNK, STAGE_FINALIZE, STAGE_FINAL_ACK,
    STAGE_DEADLINE
};
enum failure_reason {
    REASON_NONE, REASON_STATE, REASON_GATE, REASON_SEND, REASON_NO_READ,
    REASON_PARTIAL_READ, REASON_LENGTH, REASON_MALFORMED, REASON_TRAILING,
    REASON_IO, REASON_READBACK, REASON_TIMEOUT, REASON_RX_OVERFLOW
};
enum padding_zero { PADDING_UNKNOWN, PADDING_YES, PADDING_NO };
enum {
    MISMATCH_LENGTH = 1u << 0, MISMATCH_COMMAND = 1u << 1,
    MISMATCH_SEQUENCE = 1u << 2, MISMATCH_PAYLOAD_SIZE = 1u << 3,
    MISMATCH_STATUS = 1u << 4, MISMATCH_FILE = 1u << 5,
    MISMATCH_CHUNK_SIZE = 1u << 6
};

static const char *const failure_stage_name[] = {
    "none", "grant", "final_grant", "rfs_frame", "final_rfs_frame",
    "store_chunk", "finalize", "final_ack", "deadline"
};
static const char *const failure_reason_name[] = {
    "none", "state", "gate", "send_ambiguous", "no_read", "partial_read",
    "length", "malformed", "trailing", "io", "readback", "timeout",
    "rx_overflow"
};

struct sha256 {
    uint32_t h[8];
    uint64_t bits;
    uint8_t block[64];
    size_t used;
};

struct sit_observer {
    uint8_t rx[SIT_RX_CAP];
    size_t used;
    uint32_t token;
    unsigned pass, next;
    int started, pending, poisoned, endpoint_failed;
    int64_t ready_ms, deadline_ms, backoff_until_ms;
    int64_t event_window_ms;
    unsigned event_seen, event_logged;
    uint64_t event_overflow;
};

#ifdef SAAIOS_RFS_CAMP
enum { SEQ_CONFIG_COMMAND = 0x093f, SEQ_CONFIG_LEN = 13,
       SGC_COMMAND = 0x0404, SGC_LEN = 24,
       CAMP_POWER_COMMAND = 0x0800, CAMP_POWER_LEN = 18, CAMP_POWER_ON = 2,
       CAMP_RX_CAP = 4096, CAMP_WINDOW_MS = 30000,
       CAMP_PAIR_GAP_MS = 1000, CAMP_DISPATCH_MS = 1000,
       PROBE_START_MS = 2000, PROBE_REPLY_MS = 3000, PROBE_GAP_MS = 1500,
       PROBE_SIGNAL_MIN = 210,
       REG_RADIO_GET = 0x0801, REG_SEL_GET = 0x0703, REG_SEL_AUTO_SET = 0x0704,
       REG_PREF_GET = 0x070b, REG_PREF_SET = 0x070a, REG_ALLOW_DATA = 0x0710,
       REG_PREF_LEN = 16, REG_ALLOW_LEN = 13, REG_GET_MAX = 3,
       /* SIT_SET_INITIAL_ATTACH_APN (0x0603): a 250-byte request, recovered
        * from ProtocolPsBuilder::BuildSetInitialAttachApn + FillApnInfo
        * <sit_pdp_set_initial_attach_apn_req> in the factory libsitril.so
        * (efcca0d5). The modem treats this as the PS-attach precondition: the
        * network-attach/default-bearer APN. All body bytes are recovered, none
        * invented. Frame-relative offsets: [12]=attach pdp cid, [13]=0x0e
        * (fixed in the builder), [14]=dataProfileId, [15]=apnType,
        * [16..115]=APN string (strlcpy, 100), [117..165]=username (49),
        * [167..215]=password (49), [217]=authType, [218]=pdpType,
        * [219]=pcscfReqType. For a plain IP APN with no auth every enum byte
        * resolves to a recovered constant (GetPdpType("IP")=1,
        * ConvertAuthTypeToProtocolAuthType(0)=0, default profile/apnType=0,
        * pcscfReqType=0); the attach cid is RetrieveAttachPdpContext's
        * profile-base+1 (base 0 on the default config => 1). This is not an
        * NV/EFS write. */
       REG_INIT_ATTACH_APN = 0x0603, REG_IA_LEN = 250,
       REG_IA_APN_OFF = 16, REG_IA_APN_MAX = 100,
       REG_IA_CID = 1, REG_IA_CONST13 = 0x0e,
       REG_IA_AUTH_OFF = 217, REG_IA_PDPTYPE_OFF = 218, REG_IA_PCSCF_OFF = 219,
       REG_IA_PDPTYPE_IP = 1,
       RAT_LTE_ONLY = 11, RAT_LTE_WCDMA = 12, RADIO_STATE_ON = 10,
       /* Untried operational-state SETs/GETs (open sitdef.h, same SIT family as
        * the proven 0x0700/0x0800/0x0710/0x093f wire IDs; 0x091a body also
        * cross-checked against the factory libsitril BuildSetVoiceOperation,
        * payload int32=3, len16). None is an NV/EFS write. One SET per boot,
        * selected by /data/saaios/etc/opx-step; GETs always read for the log. */
       OPX_STACK_GET = 0x0810, OPX_STACK_SET = 0x080f, OPX_STACK_LEN = 13,
       OPX_VOICE_GET = 0x091b, OPX_VOICE_SET = 0x091a, OPX_VOICE_LEN = 16,
       OPX_INTPS_SET = 0x0933, OPX_INTPS_LEN = 16,
       OPX_DEVSVC_GET = 0x0957, OPX_DEVSVC_SET = 0x0956, OPX_DEVSVC_LEN = 16,
       OPX_DUAL_SET = 0x072b, OPX_DUAL_LEN = 28,
       OPX_VOICE_MODE = 3, OPX_INTPS_MODE = 1, OPX_STACK_MODE_ENABLE = 1,
       OPX_DEVSVC_MODE_DATA = 2,
       OPX_DUAL_NET = 12, OPX_DUAL_ALLOW = 1,
       OPX_STEP_NONE = 0, OPX_STEP_VOICE = 1, OPX_STEP_INTPS = 2,
       OPX_STEP_STACK = 3, OPX_STEP_DEVSVC = 4, OPX_STEP_DUAL = 5 };

/* Active camp dispatcher. Isolated from the passive SIT observer: it keeps its
 * own streaming framer and token, never a SET on the RFS channel. It arms on
 * the exact 0x0803 -> 0x0802-raw0 radio edge and sends the stock stage-1 trio
 * once. A malformed/oversized IPC stream only disables the dispatcher. */
struct camp_driver {
    uint8_t rx[CAMP_RX_CAP];
    size_t used;
    int poisoned;
    int64_t owner_start_ms;
    int radio_stage;              /* 0 none, 1 saw exact 0x0803 */
    int radio_invalidated;
    int64_t radio_unavail_ms, radio_ready0_ms;
    uint32_t token;
    int dispatched;
    uint32_t cfg_token, sgc_token, camp_token;
    int cfg_sent, sgc_sent, camp_sent;
    int cfg_acked, sgc_acked, camp_acked;
    unsigned cfg_error, sgc_error, camp_error;
    /* Active prober: drives the SIM to READY and sustains signal/registration
     * tracing. It owns a private token namespace and a single outstanding GET.
     * A reply timeout only backs this prober off; it never self-poisons and
     * never stops the dispatcher or RFS quarantine. */
    uint32_t probe_token;
    int probe_pending;
    unsigned probe_idx, probe_id;
    const char *probe_name;
    int64_t probe_deadline_ms, probe_next_ms;
    int sim_change_pending, sim_ready;
    unsigned probe_sent, probe_replies, probe_timeouts;
    unsigned last_mask_low7;
    int mask_seen;
    /* One-shot registration-trigger sequence, run once the SIM is READY: confirm
     * radio ON, read selection mode (set auto if manual), read preferred RAT
     * (broaden LTE_ONLY->LTE_WCDMA so the present UMTS signal can be used), then
     * AllowData(1). Recovered/self-tested factory builders; each step sent at
     * most once, matched by id+token, non-poisoning. Then observation resumes. */
    int radio_on;
    int radio_get_tries, sel_get_tries, pref_get_tries;
    int sel_known, sel_mode, sel_auto_sent;
    int pref_known; unsigned preferred_raw; int pref_set_sent;
    int allow_data_sent, reg_complete;
    /* Initial-attach APN (stock SET_INITIAL_ATTACH_APN 0x0603) sent once before
     * allow-data, only when an APN is configured. apn[] is loaded at startup from
     * /data/saaios/etc/apn; empty => the step is skipped (proven boot unchanged). */
    int ia_apn_sent;
    char apn[REG_IA_APN_MAX];
    /* Post-registration one-shot operational-SET experiment. After reg_complete
     * the three GETs below are read once for the log, then the single SET named
     * by opx_step is sent once (matched by id+token, non-poisoning). */
    int opx_step;                 /* OPX_STEP_* selected at startup from file */
    int opx_stack_get_sent, opx_voice_get_sent, opx_devsvc_get_sent;
    int opx_stack_known; unsigned opx_stack_mode;
    int opx_set_sent, opx_done;
};
#endif

struct owner {
    int ipc, rfs, ready, lock;
    int source, pin_fd, source_dir, pin_dir;
    int quarantine_parent, quarantine_dir, candidate, marker_fd, sidecar_fd;
    struct stat source_stat, pin_stat, candidate_stat, marker_stat, sidecar_stat;
    struct stat source_dir_stat, pin_dir_stat;
    struct stat quarantine_parent_stat, quarantine_dir_stat;
    char quarantine_leaf[48];
    uint8_t pin_digest[32];
    uint8_t rx[RX_CAP];
    size_t used;
    enum phase phase;
    int64_t deadline_ms;
    int64_t total_deadline_ms;
    int grant_attempted;
    int chunks_stored;
    uint32_t received_bytes, expected_chunk;
    struct sha256 received_hash;
    uint8_t candidate_digest[32];
    int final_ack_attempted;
    int final_ack_sent;
    int pin_consumed;
    enum failure_stage failure_stage;
    enum failure_reason failure_reason;
    unsigned frame_mismatch_mask;
    unsigned final_parsed_len, final_outer_payload_len, final_trailing;
    enum padding_zero final_padding_zero;
    struct sit_observer sit;
#ifdef SAAIOS_RFS_CAMP
    struct camp_driver camp;
#endif
};

static volatile sig_atomic_t stop_requested;
#ifdef RFS_HOST_TEST
static ssize_t (*host_write_override)(int, const void *, size_t);
static unsigned host_write_calls;
static int host_gate_override;
static int host_store_override;
static int host_finish_override;
static int host_sidecar_close_error;
static int host_sidecar_sync_error;
static ssize_t (*host_sit_write_override)(int, const void *, size_t);
static unsigned host_sit_write_calls;
#endif
static const uint8_t request_7[12] =
    {7,0,0,0, 4,0,0,0, 3,0,0,0};
static const uint8_t request_3[20] =
    {3,0,0,0, 12,0,0,0, 0,0,0,0, 3,0,0,0, 0,0,0,0};
static const uint8_t request_6[24] =
    {6,0,1,0, 16,0,0,0, 3,0,0,0, 0,0,0,0,
     0x06,0xe4,0x02,0, 2,0,0,0};
static const uint8_t status_7[16] =
    {3,0,0,0, 8,0,0,0, 0,0,0,0, 3,0,0,0};
static const uint8_t final_status[16] =
    {3,0,1,0, 8,0,0,0, 0,0,0,0, 3,0,0,0};

static void zero_bytes(void *pointer, size_t count)
{
    volatile uint8_t *p = (volatile uint8_t *)pointer;
    while (count--) *p++ = 0;
}

/* Only fixed stage/reason labels reach the terminal log; never frame data. */
static void diagnose(struct owner *o, enum failure_stage stage,
                     enum failure_reason reason)
{
    if (o->failure_stage == STAGE_NONE) {
        o->failure_stage = stage;
        o->failure_reason = reason;
    }
}

static enum failure_stage frame_stage(const struct owner *o)
{
    return o->grant_attempted == RFS_GRANTS_MAX &&
           o->chunks_stored == RFS_GRANTS_MAX - 1 ?
           STAGE_FINAL_FRAME : STAGE_RFS_FRAME;
}

static uint32_t rotate_right(uint32_t n, unsigned bits)
{
    return (n >> bits) | (n << (32u - bits));
}

static uint32_t big32(const uint8_t *p)
{
    return ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
           ((uint32_t)p[2] << 8) | p[3];
}

static uint16_t little16(const uint8_t *p)
{
    return (uint16_t)((uint16_t)p[0] | ((uint16_t)p[1] << 8));
}

static uint32_t little32(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static int same_bytes(const uint8_t *a, const uint8_t *b, size_t n)
{
    uint8_t difference = 0;
    for (size_t i = 0; i < n; ++i) difference |= (uint8_t)(a[i] ^ b[i]);
    return difference == 0;
}

/* SHA-256 is local so the ARM diagnostic needs no Android crypto service. */
static void sha_transform(struct sha256 *ctx, const uint8_t block[64])
{
    static const uint32_t k[64] = {
        0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,
        0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
        0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,
        0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
        0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,
        0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
        0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,
        0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
        0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,
        0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
        0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,
        0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
        0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,
        0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
        0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,
        0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2
    };
    uint32_t w[64];
    uint32_t a, b, c, d, e, f, g, h;
    for (unsigned i = 0; i < 16; ++i) w[i] = big32(block + 4u * i);
    for (unsigned i = 16; i < 64; ++i) {
        uint32_t s0 = rotate_right(w[i-15], 7) ^
                      rotate_right(w[i-15], 18) ^ (w[i-15] >> 3);
        uint32_t s1 = rotate_right(w[i-2], 17) ^
                      rotate_right(w[i-2], 19) ^ (w[i-2] >> 10);
        w[i] = w[i-16] + s0 + w[i-7] + s1;
    }
    a=ctx->h[0]; b=ctx->h[1]; c=ctx->h[2]; d=ctx->h[3];
    e=ctx->h[4]; f=ctx->h[5]; g=ctx->h[6]; h=ctx->h[7];
    for (unsigned i = 0; i < 64; ++i) {
        uint32_t s1 = rotate_right(e,6) ^ rotate_right(e,11) ^ rotate_right(e,25);
        uint32_t ch = (e & f) ^ (~e & g);
        uint32_t t1 = h + s1 + ch + k[i] + w[i];
        uint32_t s0 = rotate_right(a,2) ^ rotate_right(a,13) ^ rotate_right(a,22);
        uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
        uint32_t t2 = s0 + maj;
        h=g; g=f; f=e; e=d+t1; d=c; c=b; b=a; a=t1+t2;
    }
    ctx->h[0]+=a; ctx->h[1]+=b; ctx->h[2]+=c; ctx->h[3]+=d;
    ctx->h[4]+=e; ctx->h[5]+=f; ctx->h[6]+=g; ctx->h[7]+=h;
    zero_bytes(w, sizeof w);
}

static void sha_init(struct sha256 *ctx)
{
    static const uint32_t initial[8] = {
        0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,
        0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19
    };
    memset(ctx, 0, sizeof *ctx);
    memcpy(ctx->h, initial, sizeof initial);
}

static void sha_update(struct sha256 *ctx, const uint8_t *data, size_t len)
{
    ctx->bits += (uint64_t)len * 8u;
    while (len) {
        size_t space = 64u - ctx->used;
        size_t take = len < space ? len : space;
        memcpy(ctx->block + ctx->used, data, take);
        ctx->used += take;
        data += take;
        len -= take;
        if (ctx->used == 64u) {
            sha_transform(ctx, ctx->block);
            ctx->used = 0;
        }
    }
}

static void sha_final(struct sha256 *ctx, uint8_t out[32])
{
    uint64_t bits = ctx->bits;
    ctx->block[ctx->used++] = 0x80;
    if (ctx->used > 56u) {
        memset(ctx->block + ctx->used, 0, 64u - ctx->used);
        sha_transform(ctx, ctx->block);
        ctx->used = 0;
    }
    memset(ctx->block + ctx->used, 0, 56u - ctx->used);
    for (unsigned i = 0; i < 8; ++i)
        ctx->block[56u+i] = (uint8_t)(bits >> (56u - 8u*i));
    sha_transform(ctx, ctx->block);
    for (unsigned i = 0; i < 8; ++i) {
        out[4u*i] = (uint8_t)(ctx->h[i] >> 24);
        out[4u*i+1] = (uint8_t)(ctx->h[i] >> 16);
        out[4u*i+2] = (uint8_t)(ctx->h[i] >> 8);
        out[4u*i+3] = (uint8_t)ctx->h[i];
    }
    zero_bytes(ctx, sizeof *ctx);
}

static int read_all_at(int fd, void *out, size_t len, off_t offset)
{
    size_t done = 0;
    while (done < len) {
        ssize_t got = pread(fd, (uint8_t *)out + done, len - done,
                            offset + (off_t)done);
        if (got < 0 && errno == EINTR) continue;
        if (got <= 0) return -1;
        done += (size_t)got;
    }
    return 0;
}

static int write_all_at(int fd, const void *bytes, size_t len, off_t offset)
{
    size_t done = 0;
    while (done < len) {
        ssize_t count = pwrite(fd, (const uint8_t *)bytes + done, len - done,
                               offset + (off_t)done);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) return -1;
        done += (size_t)count;
    }
    return 0;
}

static int same_file(const struct stat *a, const struct stat *b)
{
    return a->st_dev == b->st_dev && a->st_ino == b->st_ino &&
           a->st_mode == b->st_mode && a->st_uid == b->st_uid &&
           a->st_nlink == b->st_nlink && a->st_size == b->st_size &&
           a->st_mtim.tv_sec == b->st_mtim.tv_sec &&
           a->st_mtim.tv_nsec == b->st_mtim.tv_nsec &&
           a->st_ctim.tv_sec == b->st_ctim.tv_sec &&
           a->st_ctim.tv_nsec == b->st_ctim.tv_nsec;
}

static int same_directory_identity(const struct stat *a,
                                   const struct stat *b)
{
    return a->st_dev == b->st_dev && a->st_ino == b->st_ino &&
           a->st_mode == b->st_mode && a->st_uid == b->st_uid &&
           a->st_gid == b->st_gid && a->st_nlink == b->st_nlink &&
           S_ISDIR(a->st_mode);
}

static int host_fixture_mode(void)
{
#ifdef RFS_HOST_TEST
    return host_gate_override;
#else
    return 0;
#endif
}

static int regular_exact(int fd, off_t size, mode_t mode, struct stat *out)
{
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || (flags & O_ACCMODE) != O_RDONLY ||
        fstat(fd, out) || !S_ISREG(out->st_mode) ||
        out->st_uid != 0 || out->st_nlink != 1 ||
        (out->st_mode & 07777) != mode || out->st_size != size)
        return -1;
    return 0;
}

static int directory_exact(int fd, mode_t mode)
{
    struct stat st;
    if (fstat(fd, &st) || !S_ISDIR(st.st_mode) || st.st_uid != 0 ||
        (st.st_mode & 07777) != mode || st.st_nlink < 2) return -1;
    return 0;
}

/* O_NOFOLLOW on every component, including trusted fixed-path ancestors. */
static int walk_directory(const char *const *parts, size_t count)
{
    int fd = open("/", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (fd < 0) return -1;
    for (size_t i = 0; i < count; ++i) {
        int next = openat(fd, parts[i],
                          O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
        close(fd);
        if (next < 0) return -1;
        fd = next;
    }
    return fd;
}

static int named_regular(int dirfd, const char *name, int fd,
                         const struct stat *expected)
{
    struct stat named, held;
    return fstat(fd, &held) == 0 && same_file(&held, expected) &&
           fstatat(dirfd, name, &named, AT_SYMLINK_NOFOLLOW) == 0 &&
           same_file(&named, expected) && S_ISREG(named.st_mode) ? 0 : -1;
}

static int named_directory(const char *path, int fd,
                           const struct stat *expected)
{
    struct stat held, named;
    return fstat(fd, &held) == 0 && same_file(&held, expected) &&
           lstat(path, &named) == 0 && same_file(&named, expected) &&
           S_ISDIR(named.st_mode) ? 0 : -1;
}

static int decode_lower_hex(const uint8_t chars[64], uint8_t digest[32])
{
    for (size_t i = 0; i < 32; ++i) {
        unsigned high, low;
        uint8_t a = chars[2*i], b = chars[2*i+1];
        if (a >= '0' && a <= '9') high = a - '0';
        else if (a >= 'a' && a <= 'f') high = a - 'a' + 10u;
        else return -1;
        if (b >= '0' && b <= '9') low = b - '0';
        else if (b >= 'a' && b <= 'f') low = b - 'a' + 10u;
        else return -1;
        digest[i] = (uint8_t)((high << 4) | low);
    }
    return 0;
}

static int original_efs_unmounted(void)
{
    FILE *f = fopen("/sys/block/sda/sda5/uevent", "r");
    char line[4096];
    unsigned partname = 0, devname = 0;
    unsigned major_seen = 0, minor_seen = 0;
    unsigned major = 0, minor = 0;
    if (!f) return -1;
    while (fgets(line, sizeof line, f)) {
        if (!strchr(line, '\n') && !feof(f)) { fclose(f); return -1; }
        line[strcspn(line, "\r\n")] = 0;
        if (!strcmp(line, "PARTNAME=efs")) ++partname;
        else if (!strcmp(line, "DEVNAME=sda5")) ++devname;
        else if (!strncmp(line, "MAJOR=", 6)) {
            char extra;
            if (sscanf(line + 6, "%u%c", &major, &extra) != 1) {
                fclose(f); return -1;
            }
            ++major_seen;
        } else if (!strncmp(line, "MINOR=", 6)) {
            char extra;
            if (sscanf(line + 6, "%u%c", &minor, &extra) != 1) {
                fclose(f); return -1;
            }
            ++minor_seen;
        } else if (!strncmp(line, "PARTNAME=", 9) ||
                   !strncmp(line, "DEVNAME=", 8)) {
            fclose(f); return -1;
        }
    }
    int io_error = ferror(f);
    int close_error = fclose(f);
    if (io_error || close_error != 0 || partname != 1 || devname != 1 ||
        major_seen != 1 || minor_seen != 1) return -1;
    f = fopen("/sys/block/sda/sda5/dev", "r");
    if (!f) return -1;
    unsigned actual_major = 0, actual_minor = 0;
    int got = fscanf(f, "%u:%u", &actual_major, &actual_minor);
    if (fclose(f) != 0 || got != 2 || actual_major != major ||
        actual_minor != minor) return -1;
    f = fopen("/proc/self/mountinfo", "r");
    if (!f) return -1;
    while (fgets(line, sizeof line, f)) {
        unsigned mounted_major = 0, mounted_minor = 0;
        if (!strchr(line, '\n') && !feof(f)) { fclose(f); return -1; }
        if (sscanf(line, "%*u %*u %u:%u", &mounted_major,
                   &mounted_minor) != 2) { fclose(f); return -1; }
        if (mounted_major == major && mounted_minor == minor) {
            fclose(f); return -1;
        }
    }
    io_error = ferror(f);
    close_error = fclose(f);
    int ok = !io_error && close_error == 0;
    return ok ? 0 : -1;
}

/* A dedicated /run tmpfs makes this pin boot-local, not persisted userdata. */
static int run_tmpfs(int pin_dir)
{
    int run_fd = open("/run", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    struct stat held, named, pin_stat;
    struct statfs fs;
    char line[4096], mountpoint[256], fstype[64];
    unsigned matches = 0;
    if (run_fd < 0) return -1;
    int valid = fstat(run_fd, &held) == 0 &&
                lstat("/run", &named) == 0 &&
                S_ISDIR(held.st_mode) && held.st_uid == 0 &&
                (held.st_mode & 07777) == 0755 &&
                held.st_dev == named.st_dev && held.st_ino == named.st_ino &&
                held.st_mode == named.st_mode &&
                fstatfs(run_fd, &fs) == 0 &&
                (unsigned long)fs.f_type == TMPFS_MAGIC &&
                fstat(pin_dir, &pin_stat) == 0 &&
                pin_stat.st_dev == held.st_dev;
    close(run_fd);
    if (!valid) return -1;
    FILE *f = fopen("/proc/self/mountinfo", "r");
    if (!f) return -1;
    while (fgets(line, sizeof line, f)) {
        if (!strchr(line, '\n') && !feof(f)) {
            fclose(f); return -1;
        }
        char *split = strstr(line, " - ");
        if (!split) { fclose(f); return -1; }
        *split = 0;
        if (sscanf(line, "%*s %*s %*s %*s %255s", mountpoint) != 1 ||
            sscanf(split + 3, "%63s", fstype) != 1) {
            fclose(f); return -1;
        }
        if (!strcmp(mountpoint, "/run")) {
            ++matches;
            if (strcmp(fstype, "tmpfs")) { fclose(f); return -1; }
        }
    }
    int io_error = ferror(f);
    int close_error = fclose(f);
    return !io_error && close_error == 0 && matches == 1 ? 0 : -1;
}

static int load_source_and_pin(struct owner *o)
{
    static const char *const source_path[] =
        {"data", "saaios", "var", "efs-copy"};
    static const char *const pin_path[] =
        {"run", "saaios-rfs-one-grant"};
    uint8_t chars[64];
    o->source_dir = walk_directory(source_path, 4);
    o->pin_dir = walk_directory(pin_path, 2);
    if (o->source_dir < 0 || o->pin_dir < 0) return -1;
    if (fstat(o->source_dir, &o->source_dir_stat) ||
        !S_ISDIR(o->source_dir_stat.st_mode) ||
        o->source_dir_stat.st_uid != 0 ||
        ((o->source_dir_stat.st_mode & 07777) != 0700 &&
         (o->source_dir_stat.st_mode & 07777) != 0755) ||
        fstat(o->pin_dir, &o->pin_dir_stat) ||
        directory_exact(o->pin_dir, 0700) || run_tmpfs(o->pin_dir) ||
        named_directory("/data/saaios/var/efs-copy", o->source_dir,
                        &o->source_dir_stat) ||
        named_directory("/run/saaios-rfs-one-grant", o->pin_dir,
                        &o->pin_dir_stat))
        return -1;
    struct stat used;
    if (fstatat(o->pin_dir, USED_PIN_NAME, &used,
                AT_SYMLINK_NOFOLLOW) == 0 || errno != ENOENT) return -1;
    o->source = openat(o->source_dir, SOURCE_NAME,
                       O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    o->pin_fd = openat(o->pin_dir, PIN_NAME,
                       O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    if (o->source < 0 || o->pin_fd < 0 ||
        regular_exact(o->source, BASELINE_BYTES, 0600, &o->source_stat) ||
        regular_exact(o->pin_fd, 64, 0600, &o->pin_stat) ||
        named_regular(o->source_dir, SOURCE_NAME, o->source,
                      &o->source_stat) ||
        named_regular(o->pin_dir, PIN_NAME, o->pin_fd, &o->pin_stat) ||
        read_all_at(o->pin_fd, chars, sizeof chars, 0) ||
        decode_lower_hex(chars, o->pin_digest)) {
        zero_bytes(chars, sizeof chars);
        return -1;
    }
    zero_bytes(chars, sizeof chars);
    return named_regular(o->pin_dir, PIN_NAME, o->pin_fd,
                         &o->pin_stat);
}

static int stable_sources(const struct owner *o)
{
    if (named_directory("/data/saaios/var/efs-copy", o->source_dir,
                        &o->source_dir_stat) ||
        named_directory("/run/saaios-rfs-one-grant", o->pin_dir,
                        &o->pin_dir_stat) ||
        named_regular(o->source_dir, SOURCE_NAME, o->source,
                      &o->source_stat) ||
        directory_exact(o->pin_dir, 0700) || run_tmpfs(o->pin_dir))
        return 0;
    if (!o->pin_consumed)
        return named_regular(o->pin_dir, PIN_NAME, o->pin_fd,
                             &o->pin_stat) == 0;
    struct stat unexpected;
    if (named_regular(o->pin_dir, USED_PIN_NAME, o->pin_fd,
                      &o->pin_stat) ||
        fstatat(o->pin_dir, PIN_NAME, &unexpected,
                AT_SYMLINK_NOFOLLOW) == 0 || errno != ENOENT)
        return 0;
    return 1;
}

/* Once linked under the no-overwrite consumed name, no retry may use it. */
static int consume_pin(struct owner *o)
{
    struct stat used;
    if (!stable_sources(o) || o->pin_consumed ||
        fstatat(o->pin_dir, USED_PIN_NAME, &used,
                AT_SYMLINK_NOFOLLOW) == 0 || errno != ENOENT ||
        linkat(o->pin_dir, PIN_NAME, o->pin_dir, USED_PIN_NAME, 0) ||
        unlinkat(o->pin_dir, PIN_NAME, 0) ||
        regular_exact(o->pin_fd, 64, 0600, &o->pin_stat) ||
        named_regular(o->pin_dir, USED_PIN_NAME, o->pin_fd,
                      &o->pin_stat) || fsync(o->pin_dir) ||
        fstat(o->pin_dir, &o->pin_dir_stat)) return -1;
    o->pin_consumed = 1;
    return stable_sources(o) ? 0 : -1;
}

static int fresh_leaf(char leaf[48])
{
    static const char hex[] = "0123456789abcdef";
    uint8_t random_bytes[16];
    size_t done = 0;
    memset(leaf, 0, 48);
    memcpy(leaf, "full-rfs-", 9);
    while (done < sizeof random_bytes) {
        ssize_t got = getrandom(random_bytes + done,
                                sizeof random_bytes - done, 0);
        if (got < 0 && errno == EINTR) continue;
        if (got <= 0) return -1;
        done += (size_t)got;
    }
    for (size_t i = 0; i < sizeof random_bytes; ++i) {
        leaf[9 + 2*i] = hex[random_bytes[i] >> 4];
        leaf[10 + 2*i] = hex[random_bytes[i] & 15];
    }
    leaf[41] = 0;
    zero_bytes(random_bytes, sizeof random_bytes);
    return 0;
}

static int quarantine_location_stable(const struct owner *o)
{
    static const uint8_t marker[] = "QUARANTINE_NO_PROMOTION\n";
    struct stat held_parent, named_parent, held_dir, named_dir;
    uint8_t content[sizeof marker - 1];
    int valid = o->candidate >= 0 && o->marker_fd >= 0 &&
           o->quarantine_dir >= 0 &&
           directory_exact(o->quarantine_parent, 0700) == 0 &&
           fstat(o->quarantine_parent, &held_parent) == 0 &&
           same_file(&held_parent, &o->quarantine_parent_stat) &&
           lstat("/data/saaios/var/rfs-quarantine", &named_parent) == 0 &&
           same_file(&named_parent, &o->quarantine_parent_stat) &&
           directory_exact(o->quarantine_dir, 0700) == 0 &&
           fstat(o->quarantine_dir, &held_dir) == 0 &&
           same_file(&held_dir, &o->quarantine_dir_stat) &&
           fstatat(o->quarantine_parent, o->quarantine_leaf, &named_dir,
                   AT_SYMLINK_NOFOLLOW) == 0 &&
           same_file(&named_dir, &o->quarantine_dir_stat) &&
           named_regular(o->quarantine_dir, MARKER_NAME,
                         o->marker_fd, &o->marker_stat) == 0 &&
           read_all_at(o->marker_fd, content, sizeof content, 0) == 0 &&
           same_bytes(content, marker, sizeof content) &&
           named_regular(o->quarantine_dir, MARKER_NAME,
                         o->marker_fd, &o->marker_stat) == 0;
    zero_bytes(content, sizeof content);
    return valid ? 0 : -1;
}

static int candidate_stable(const struct owner *o)
{
    struct stat st;
    return quarantine_location_stable(o) == 0 &&
           fstat(o->candidate, &st) == 0 &&
           same_file(&st, &o->candidate_stat) &&
           named_regular(o->quarantine_dir, CANDIDATE_NAME,
                         o->candidate, &o->candidate_stat) == 0 ? 0 : -1;
}

static int prepare_quarantine(struct owner *o)
{
    static const char *const quarantine_path[] =
        {"data", "saaios", "var", "rfs-quarantine"};
    static const uint8_t marker[] = "QUARANTINE_NO_PROMOTION\n";
    uint8_t buffer[4096], digest[32];
    struct sha256 source_hash, candidate_hash;
    char leaf[48];
    o->quarantine_parent = walk_directory(quarantine_path, 4);
    if (o->quarantine_parent < 0 ||
        directory_exact(o->quarantine_parent, 0700) ||
        fstat(o->quarantine_parent, &o->quarantine_parent_stat)) return -1;
    for (unsigned attempt = 0; attempt < 16; ++attempt) {
        if (fresh_leaf(leaf)) return -1;
        if (mkdirat(o->quarantine_parent, leaf, 0700) == 0) break;
        if (errno != EEXIST || attempt == 15) return -1;
    }
    o->quarantine_dir = openat(o->quarantine_parent, leaf,
                               O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (o->quarantine_dir < 0 ||
        directory_exact(o->quarantine_dir, 0700) ||
        fstat(o->quarantine_dir, &o->quarantine_dir_stat)) return -1;
    memcpy(o->quarantine_leaf, leaf, sizeof leaf);
    o->marker_fd = openat(o->quarantine_dir, MARKER_NAME,
                          O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                          0600);
    if (o->marker_fd < 0 || fstat(o->marker_fd, &o->marker_stat) ||
        !S_ISREG(o->marker_stat.st_mode) || o->marker_stat.st_uid != 0 ||
        o->marker_stat.st_nlink != 1 ||
        (o->marker_stat.st_mode & 07777) != 0600 ||
        write_all_at(o->marker_fd, marker, sizeof marker - 1, 0) ||
        fsync(o->marker_fd) ||
        fstat(o->marker_fd, &o->marker_stat) ||
        o->marker_stat.st_size != (off_t)(sizeof marker - 1)) goto failure;
    o->candidate = openat(o->quarantine_dir, CANDIDATE_NAME,
                          O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                          0600);
    if (o->candidate < 0) return -1;
    sha_init(&source_hash);
    for (off_t off = 0; off < BASELINE_BYTES; off += sizeof buffer) {
        if (read_all_at(o->source, buffer, sizeof buffer, off) ||
            write_all_at(o->candidate, buffer, sizeof buffer, off))
            goto failure;
        sha_update(&source_hash, buffer, sizeof buffer);
    }
    sha_final(&source_hash, digest);
    if (!same_bytes(digest, o->pin_digest, sizeof digest) ||
        !stable_sources(o) || fsync(o->candidate)) goto failure;
    sha_init(&candidate_hash);
    for (off_t off = 0; off < BASELINE_BYTES; off += sizeof buffer) {
        if (read_all_at(o->candidate, buffer, sizeof buffer, off)) goto failure;
        sha_update(&candidate_hash, buffer, sizeof buffer);
    }
    sha_final(&candidate_hash, digest);
    if (!same_bytes(digest, o->pin_digest, sizeof digest) ||
        fstat(o->candidate, &o->candidate_stat) ||
        !S_ISREG(o->candidate_stat.st_mode) ||
        o->candidate_stat.st_uid != 0 || o->candidate_stat.st_nlink != 1 ||
        (o->candidate_stat.st_mode & 07777) != 0600 ||
        o->candidate_stat.st_size != BASELINE_BYTES ||
        fstat(o->quarantine_parent, &o->quarantine_parent_stat) ||
        fstat(o->quarantine_dir, &o->quarantine_dir_stat) ||
        candidate_stable(o) || fsync(o->quarantine_dir) ||
        fsync(o->quarantine_parent) || !stable_sources(o)) goto failure;
    zero_bytes(buffer, sizeof buffer);
    zero_bytes(digest, sizeof digest);
    return 0;
failure:
    zero_bytes(buffer, sizeof buffer);
    zero_bytes(digest, sizeof digest);
    zero_bytes(&source_hash, sizeof source_hash);
    zero_bytes(&candidate_hash, sizeof candidate_hash);
    return -1;
}

static int64_t monotonic_ms(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now)) return -1;
    return (int64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000;
}

static enum cp_state cp_state(void)
{
    char state[32] = {0};
    FILE *f = fopen("/sys/devices/platform/cpif/modem_state", "r");
    if (!f) return CP_UNKNOWN;
    int got = fscanf(f, "%31s", state);
    if (fclose(f) != 0 || got != 1) return CP_UNKNOWN;
    if (!strcmp(state, "OFFLINE")) return CP_OFFLINE;
    if (!strcmp(state, "BOOTING")) return CP_BOOTING;
    if (!strcmp(state, "ONLINE")) return CP_ONLINE;
    if (!strcmp(state, "CRASH_EXIT")) return CP_CRASH;
    return CP_UNKNOWN;
}

static int verify_device(int fd, const char *sysdev)
{
    FILE *f = fopen(sysdev, "r");
    unsigned expected_major = 0, expected_minor = 0;
    struct stat st;
    if (!f) return -1;
    int got = fscanf(f, "%u:%u", &expected_major, &expected_minor);
    if (fclose(f) != 0 || got != 2 || fstat(fd, &st) ||
        !S_ISCHR(st.st_mode) || major(st.st_rdev) != expected_major ||
        minor(st.st_rdev) != expected_minor) return -1;
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || (flags & O_ACCMODE) != O_RDWR) return -1;
    if (!(flags & O_NONBLOCK) && fcntl(fd, F_SETFL, flags | O_NONBLOCK))
        return -1;
    return 0;
}

static int opened_once(int fd)
{
    int opened = -1;
    return ioctl(fd, IOCTL_GET_OPENED_STATUS, &opened) == 0 &&
           opened == 1 ? 0 : -1;
}

static int endpoint_fault_free(int fd)
{
    struct pollfd p = {.fd = fd, .events = 0};
    int checked = poll(&p, 1, 0);
    return checked >= 0 && !(p.revents & (POLLERR | POLLHUP | POLLNVAL)) ?
           0 : -1;
}

static int ready_pipe(int fd)
{
    struct stat st;
    int flags = fcntl(fd, F_GETFL);
    return flags >= 0 && (flags & O_ACCMODE) == O_WRONLY &&
           fstat(fd, &st) == 0 && S_ISFIFO(st.st_mode) ? 0 : -1;
}

static int acquire_lock(void)
{
    static const char path[] = "/run/saaios-sit-status.lock";
    int fd = open(path, O_RDWR | O_CREAT | O_NOFOLLOW | O_CLOEXEC, 0600);
    struct stat held, named;
    if (fd < 0) return -1;
    if (fstat(fd, &held) || !S_ISREG(held.st_mode) ||
        held.st_uid != 0 || held.st_nlink != 1 ||
        (held.st_mode & 07777) != 0600 ||
        flock(fd, LOCK_EX | LOCK_NB) ||
        lstat(path, &named) || !same_file(&held, &named)) {
        close(fd);
        return -1;
    }
    return fd;
}

static int send_ready(int fd)
{
    static const char ready[] = "READY\n";
    /* This pipe is not a modem endpoint; handling a short write is safe. */
    size_t done = 0;
    while (done < sizeof ready - 1) {
        ssize_t n = write(fd, ready + done, sizeof ready - 1 - done);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) return -1;
        done += (size_t)n;
    }
    return 0;
}

static enum action classify(const struct owner *o, const uint8_t *frame,
                            size_t len)
{
    if (!frame) return BAD_FRAME;
    switch (o->phase) {
    case WAIT_7:
        return len == sizeof request_7 &&
               same_bytes(frame, request_7, len) ? STATUS_7 : BAD_FRAME;
    case WAIT_3:
        return len == sizeof request_3 &&
               same_bytes(frame, request_3, len) ? NO_REPLY : BAD_FRAME;
    case WAIT_6:
        return len == sizeof request_6 &&
               same_bytes(frame, request_6, len) ? GRANT_1 : BAD_FRAME;
    case WAIT_DATA:
        return o->grant_attempted > 0 &&
               o->grant_attempted <= RFS_GRANTS_MAX &&
               o->chunks_stored + 1 == o->grant_attempted &&
               o->expected_chunk > 0 && o->expected_chunk <= FIRST_CHUNK &&
               /* The observed final CP frame has two zero bytes after the
                * 318 granted bytes. Accept only that exact final shape. */
               ((len == 20u + o->expected_chunk &&
                 little32(frame + 4) == 12u + o->expected_chunk) ||
                (o->grant_attempted == RFS_GRANTS_MAX &&
                 o->chunks_stored == RFS_GRANTS_MAX - 1 &&
                 o->expected_chunk == 318 && len == 340 &&
                 little32(frame + 4) == 332 &&
                 frame[338] == 0 && frame[339] == 0)) &&
               little16(frame) == 2 &&
               little16(frame + 2) == 1 &&
               little32(frame + 8) == 0 &&
               little32(frame + 12) == 3 &&
               little32(frame + 16) == o->expected_chunk ?
               STORE_CHUNK : BAD_FRAME;
    case TERMINAL:
        return BAD_FRAME;
    }
    return BAD_FRAME;
}

/* Compare only against public protocol constants; retain no observed fields. */
static unsigned data_mismatch_mask(const struct owner *o,
                                   const uint8_t *frame, size_t len)
{
    unsigned mask = 0;
    if (len != 20u + o->expected_chunk) mask |= MISMATCH_LENGTH;
    if (!frame || len < 2 || little16(frame) != 2) mask |= MISMATCH_COMMAND;
    if (!frame || len < 4 || little16(frame + 2) != 1)
        mask |= MISMATCH_SEQUENCE;
    if (!frame || len < 8 ||
        little32(frame + 4) != 12u + o->expected_chunk)
        mask |= MISMATCH_PAYLOAD_SIZE;
    if (!frame || len < 12 || little32(frame + 8) != 0)
        mask |= MISMATCH_STATUS;
    if (!frame || len < 16 || little32(frame + 12) != 3)
        mask |= MISMATCH_FILE;
    if (!frame || len < 20 || little32(frame + 16) != o->expected_chunk)
        mask |= MISMATCH_CHUNK_SIZE;
    return mask;
}

/* Lengths are bounded by the parser; inspect only bytes already in this frame. */
static void diagnose_final_frame_shape(struct owner *o, const uint8_t *frame,
                                       size_t len, size_t trailing)
{
    if (!frame || len < 8 || len > RFS_FRAME_MAX || trailing > RX_CAP)
        return;
    uint32_t outer_payload_len = little32(frame + 4);
    if (outer_payload_len > RFS_FRAME_MAX - 8u) return;
    o->final_parsed_len = (unsigned)len;
    o->final_outer_payload_len = outer_payload_len;
    o->final_trailing = (unsigned)trailing;
    o->final_padding_zero = PADDING_UNKNOWN;
    if (o->expected_chunk > FIRST_CHUNK ||
        len < 20u + o->expected_chunk) return;
    o->final_padding_zero = PADDING_YES;
    for (size_t i = 20u + o->expected_chunk; i < len; ++i) {
        if (frame[i]) {
            o->final_padding_zero = PADDING_NO;
            break;
        }
    }
}

static void put_little32(uint8_t *p, uint32_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
    p[2] = (uint8_t)(value >> 16);
    p[3] = (uint8_t)(value >> 24);
}

static int reply_gate(const struct owner *o);
static int send_modem_once(const struct owner *o, const uint8_t *reply,
                           size_t len);

static uint32_t next_chunk_length(const struct owner *o)
{
    if (o->received_bytes >= RFS_TRANSFER_BYTES) return 0;
    uint32_t remain = RFS_TRANSFER_BYTES - o->received_bytes;
    return remain < FIRST_CHUNK ? remain : FIRST_CHUNK;
}

static int send_next_grant(struct owner *o)
{
    uint32_t length = next_chunk_length(o);
    enum failure_stage stage = o->grant_attempted == RFS_GRANTS_MAX - 1 ?
                               STAGE_FINAL_GRANT : STAGE_GRANT;
    if (!length || o->grant_attempted >= RFS_GRANTS_MAX ||
        o->chunks_stored != o->grant_attempted) {
        diagnose(o, stage, REASON_STATE);
        return -1;
    }
    if (reply_gate(o)) {
        diagnose(o, stage, REASON_GATE);
        return -1;
    }
    uint8_t grant[20] = {2,0,1,0, 12,0,0,0, 3,0,0,0};
    put_little32(grant + 12, o->received_bytes);
    put_little32(grant + 16, length);
    /* Attempt is consumed even if the endpoint reports a short write. */
    o->grant_attempted++;
    o->expected_chunk = length;
    int rc = send_modem_once(o, grant, sizeof grant);
    zero_bytes(grant, sizeof grant);
    if (rc) {
        diagnose(o, stage, REASON_SEND);
        return -1;
    }
    o->phase = WAIT_DATA;
    return 0;
}

static int valid_frame_length(const uint8_t *rx, size_t used)
{
    if (used < 8) return 0;
    uint32_t payload = little32(rx + 4);
    if (payload < 4 || payload > RFS_FRAME_MAX - 8u) return -1;
    size_t total = 8u + payload;
    return used < total ? 0 : (int)total;
}

static int reply_gate(const struct owner *o)
{
    int64_t now = monotonic_ms();
    if (stop_requested || now < 0 || now >= o->deadline_ms ||
        now >= o->total_deadline_ms) return -1;
#ifdef RFS_HOST_TEST
    if (host_gate_override) return host_gate_override > 0 ? 0 : -1;
#endif
    return cp_state() == CP_ONLINE && opened_once(o->ipc) == 0 &&
           endpoint_fault_free(o->ipc) == 0 &&
           opened_once(o->rfs) == 0 && original_efs_unmounted() == 0 &&
           stable_sources(o) && candidate_stable(o) == 0 ? 0 : -1;
}

/* An unexpected result may already have reached the CP. Never retry it. */
static int send_modem_once(const struct owner *o, const uint8_t *reply,
                           size_t len)
{
    sigset_t blocked, old;
    sigemptyset(&blocked);
    sigaddset(&blocked, SIGTERM);
    sigaddset(&blocked, SIGINT);
    if (sigprocmask(SIG_BLOCK, &blocked, &old)) return -1;
    int64_t now = monotonic_ms();
    ssize_t written = -1;
    if (!stop_requested && now >= 0 && now < o->deadline_ms &&
        now < o->total_deadline_ms) {
#ifdef RFS_HOST_TEST
        if (host_write_override) {
            ++host_write_calls;
            written = host_write_override(o->rfs, reply, len);
        } else
#endif
        written = write(o->rfs, reply, len);
    }
    int restore = sigprocmask(SIG_SETMASK, &old, NULL);
    return restore == 0 && written == (ssize_t)len ? 0 : -1;
}

struct chunk_io {
    int (*sync)(int);
    int (*read)(int, void *, size_t, off_t);
};

/* A grant is never advanced until this exact chunk is durable and read back. */
static int apply_chunk(int candidate, int source, uint32_t offset,
                       const uint8_t *bytes, uint32_t length,
                       const struct chunk_io *io,
                       enum failure_reason *failure_reason)
{
    uint8_t check[4096];
    enum failure_reason reason = REASON_STATE;
    if (!length || length > FIRST_CHUNK ||
        offset > RFS_TRANSFER_BYTES - length) goto failure;
    reason = REASON_IO;
    if (write_all_at(candidate, bytes, length, offset) ||
        io->sync(candidate)) goto failure;
    reason = REASON_READBACK;
    if (io->read(candidate, check, length, offset) ||
        !same_bytes(check, bytes, length)) goto failure;
    for (off_t off = (off_t)offset + length; off < BASELINE_BYTES;) {
        size_t amount = (size_t)(BASELINE_BYTES - off);
        if (amount > sizeof check / 2) amount = sizeof check / 2;
        if (io->read(source, check, amount, off) ||
            io->read(candidate, check + sizeof check / 2, amount, off) ||
            !same_bytes(check, check + sizeof check / 2, amount))
            goto failure;
        off += (off_t)amount;
    }
    zero_bytes(check, sizeof check);
    return 0;
failure:
    if (failure_reason) *failure_reason = reason;
    zero_bytes(check, sizeof check);
    return -1;
}

static int store_chunk(struct owner *o, const uint8_t *bytes)
{
    static const struct chunk_io io = {fsync, read_all_at};
    enum failure_reason reason = REASON_NONE;
    if (reply_gate(o)) {
        diagnose(o, STAGE_STORE_CHUNK, REASON_GATE);
        return -1;
    }
    if (o->received_bytes > RFS_TRANSFER_BYTES - o->expected_chunk) {
        diagnose(o, STAGE_STORE_CHUNK, REASON_STATE);
        return -1;
    }
    if (apply_chunk(o->candidate, o->source, o->received_bytes, bytes,
                    o->expected_chunk, &io, &reason)) {
        diagnose(o, STAGE_STORE_CHUNK, reason);
        return -1;
    }
    struct stat current;
    if (fstat(o->candidate, &current)) {
        diagnose(o, STAGE_STORE_CHUNK, REASON_IO);
        return -1;
    }
    if (current.st_dev != o->candidate_stat.st_dev ||
        current.st_ino != o->candidate_stat.st_ino ||
        current.st_uid != 0 || current.st_nlink != 1 ||
        (current.st_mode & 07777) != 0600 ||
        current.st_size != BASELINE_BYTES ||
        quarantine_location_stable(o) ||
        named_regular(o->quarantine_dir, CANDIDATE_NAME,
                      o->candidate, &current) ||
        !stable_sources(o) || original_efs_unmounted() || stop_requested) {
        diagnose(o, STAGE_STORE_CHUNK, REASON_GATE);
        return -1;
    }
    o->candidate_stat = current;
    sha_update(&o->received_hash, bytes, o->expected_chunk);
    o->received_bytes += o->expected_chunk;
    o->chunks_stored++;
    return 0;
}

/* Verify the source pin, all received bytes, and the unchanged baseline tail. */
static int verify_full_candidate(struct owner *o, uint8_t digest[32])
{
    uint8_t source[4096], candidate[4096], source_digest[32];
    uint8_t prefix_digest[32], received_digest[32];
    struct sha256 source_hash, prefix_hash, candidate_hash, received_hash;
    int result = -1;
    if (stop_requested ||
        (!host_fixture_mode() &&
         (original_efs_unmounted() || !stable_sources(o) ||
          candidate_stable(o))) ||
        o->received_bytes != RFS_TRANSFER_BYTES ||
        o->chunks_stored != RFS_GRANTS_MAX ||
        o->grant_attempted != RFS_GRANTS_MAX) return -1;
    sha_init(&source_hash);
    sha_init(&prefix_hash);
    sha_init(&candidate_hash);
    received_hash = o->received_hash;
    sha_final(&received_hash, received_digest);
    for (off_t off = 0; off < BASELINE_BYTES; off += sizeof source) {
        size_t amount = (size_t)(BASELINE_BYTES - off);
        if (amount > sizeof source) amount = sizeof source;
        if (read_all_at(o->source, source, amount, off) ||
            read_all_at(o->candidate, candidate, amount, off)) goto done;
        sha_update(&source_hash, source, amount);
        sha_update(&candidate_hash, candidate, amount);
        size_t prefix = off >= RFS_TRANSFER_BYTES ? 0 :
                        (size_t)(RFS_TRANSFER_BYTES - off);
        if (prefix > amount) prefix = amount;
        if (prefix) sha_update(&prefix_hash, candidate, prefix);
        if (prefix < amount &&
            !same_bytes(source + prefix, candidate + prefix,
                        amount - prefix)) goto done;
    }
    sha_final(&source_hash, source_digest);
    sha_final(&prefix_hash, prefix_digest);
    sha_final(&candidate_hash, digest);
    if (same_bytes(source_digest, o->pin_digest, 32) &&
        same_bytes(prefix_digest, received_digest, 32) &&
        !stop_requested &&
        (host_fixture_mode() ||
         (!original_efs_unmounted() && stable_sources(o) &&
          candidate_stable(o) == 0)))
        result = 0;
done:
    zero_bytes(source, sizeof source);
    zero_bytes(candidate, sizeof candidate);
    zero_bytes(source_digest, sizeof source_digest);
    zero_bytes(prefix_digest, sizeof prefix_digest);
    zero_bytes(received_digest, sizeof received_digest);
    zero_bytes(&source_hash, sizeof source_hash);
    zero_bytes(&prefix_hash, sizeof prefix_hash);
    zero_bytes(&candidate_hash, sizeof candidate_hash);
    zero_bytes(&received_hash, sizeof received_hash);
    if (result) zero_bytes(digest, 32);
    return result;
}

static int sidecar_stable(const struct owner *o)
{
    struct stat held;
    uint8_t digest[32];
    uid_t owner_uid = host_fixture_mode() ? geteuid() : 0;
    int valid = o->sidecar_fd >= 0 &&
        (fcntl(o->sidecar_fd, F_GETFL) & O_ACCMODE) == O_RDONLY &&
        fstat(o->sidecar_fd, &held) == 0 &&
        same_file(&held, &o->sidecar_stat) &&
        S_ISREG(held.st_mode) && held.st_uid == owner_uid &&
        held.st_nlink == 1 && (held.st_mode & 07777) == 0600 &&
        held.st_size == 32 &&
        named_regular(o->quarantine_dir, SIDECAR_NAME, o->sidecar_fd,
                      &o->sidecar_stat) == 0 &&
        read_all_at(o->sidecar_fd, digest, sizeof digest, 0) == 0 &&
        same_bytes(digest, o->candidate_digest, sizeof digest) &&
        (host_fixture_mode() || quarantine_location_stable(o) == 0);
    zero_bytes(digest, sizeof digest);
    return valid ? 0 : -1;
}

static int close_sidecar_once(struct owner *o)
{
    int fd = o->sidecar_fd;
    o->sidecar_fd = -1;
    int result = close(fd);
#ifdef RFS_HOST_TEST
    if (host_sidecar_close_error) return -1;
#endif
    return result;
}

static int sync_sidecar_once(int fd)
{
#ifdef RFS_HOST_TEST
    if (host_sidecar_sync_error) {
        errno = EIO;
        return -1;
    }
#endif
    return fsync(fd);
}

/* The new sidecar changes directory timestamps; only its identity may carry. */
static int refresh_quarantine_dir_stat(struct owner *o)
{
    struct stat parent, held, named;
    uid_t owner_uid = host_fixture_mode() ? geteuid() : 0;
    if (fstat(o->quarantine_parent, &parent) ||
        !same_file(&parent, &o->quarantine_parent_stat) ||
        fstat(o->quarantine_dir, &held) ||
        !same_directory_identity(&held, &o->quarantine_dir_stat) ||
        held.st_uid != owner_uid || (held.st_mode & 07777) != 0700 ||
        fstatat(o->quarantine_parent, o->quarantine_leaf, &named,
                AT_SYMLINK_NOFOLLOW) ||
        !same_file(&held, &named)) return -1;
    if (!host_fixture_mode() &&
        (directory_exact(o->quarantine_parent, 0700) ||
         named_directory("/data/saaios/var/rfs-quarantine",
                         o->quarantine_parent, &o->quarantine_parent_stat) ||
         named_regular(o->quarantine_dir, MARKER_NAME, o->marker_fd,
                       &o->marker_stat) ||
         named_regular(o->quarantine_dir, CANDIDATE_NAME, o->candidate,
                       &o->candidate_stat))) return -1;
    o->quarantine_dir_stat = held;
    return host_fixture_mode() || quarantine_location_stable(o) == 0 ? 0 : -1;
}

/* The sidecar is a quarantine integrity record, never a promotion signal. */
static int finish_candidate(struct owner *o)
{
    uint8_t digest[32], again[32];
    int rc = -1;
    uid_t owner_uid = host_fixture_mode() ? geteuid() : 0;
    if (reply_gate(o) || fsync(o->candidate) ||
        verify_full_candidate(o, digest)) goto done;
    o->sidecar_fd = openat(o->quarantine_dir, SIDECAR_NAME,
                           O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                           0600);
    if (o->sidecar_fd < 0 ||
        write_all_at(o->sidecar_fd, digest, sizeof digest, 0) ||
        sync_sidecar_once(o->sidecar_fd) ||
        fstat(o->sidecar_fd, &o->sidecar_stat) ||
        !S_ISREG(o->sidecar_stat.st_mode) ||
        o->sidecar_stat.st_uid != owner_uid ||
        o->sidecar_stat.st_nlink != 1 ||
        (o->sidecar_stat.st_mode & 07777) != 0600 ||
        o->sidecar_stat.st_size != 32 ||
        fsync(o->quarantine_dir) || fsync(o->quarantine_parent) ||
        refresh_quarantine_dir_stat(o))
        goto done;
    /* A close failure is ambiguous: never retry and never ACK. */
    if (close_sidecar_once(o)) goto done;
    o->sidecar_fd = openat(o->quarantine_dir, SIDECAR_NAME,
                            O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    if (o->sidecar_fd < 0 ||
        named_regular(o->quarantine_dir, SIDECAR_NAME, o->sidecar_fd,
                      &o->sidecar_stat)) goto done;
    memcpy(o->candidate_digest, digest, sizeof digest);
    if (sidecar_stable(o) || verify_full_candidate(o, again) ||
        !same_bytes(digest, again, sizeof digest) ||
        sidecar_stable(o) || reply_gate(o))
        goto done;
    rc = 0;
done:
    zero_bytes(digest, sizeof digest);
    zero_bytes(again, sizeof again);
    return rc;
}

static void log_terminal(FILE *out, const struct owner *o, const char *reason)
{
    char final_shape[160] = {0};
    if (o->final_parsed_len)
        (void)snprintf(final_shape, sizeof final_shape,
                       " final_parsed_len=%u final_outer_payload_len=%u "
                       "final_trailing=%u final_padding_zero=%s",
                       o->final_parsed_len, o->final_outer_payload_len,
                       o->final_trailing,
                       o->final_padding_zero == PADDING_YES ? "1" :
                       o->final_padding_zero == PADDING_NO ? "0" : "unknown");
    fprintf(out, "rfs_full_quarantine=%s grants_attempted=%d chunks_stored=%d "
            "bytes_stored=%u final_ack_attempted=%d final_ack_sent=%d "
            "failure_stage=%s failure_reason=%s frame_mismatch_mask=0x%02x%s\n",
            reason, o->grant_attempted, o->chunks_stored,
            o->received_bytes, o->final_ack_attempted, o->final_ack_sent,
            failure_stage_name[o->failure_stage],
            failure_reason_name[o->failure_reason], o->frame_mismatch_mask,
            final_shape);
}

static void terminal(struct owner *o, const char *reason)
{
    if (o->phase == TERMINAL) return;
    o->phase = TERMINAL;
    o->used = 0;
    zero_bytes(o->rx, sizeof o->rx);
    log_terminal(stdout, o, reason);
}

static void diagnose_waiting(struct owner *o)
{
    if (o->phase == WAIT_DATA && frame_stage(o) == STAGE_FINAL_FRAME)
        diagnose(o, STAGE_FINAL_FRAME,
                 o->used ? REASON_PARTIAL_READ : REASON_NO_READ);
    else
        diagnose(o, STAGE_DEADLINE, REASON_TIMEOUT);
}

/* Passive early-boot RFS trace. Logs ONLY numeric protocol header fields
 * (command, numeric file handle, offset/size counters) and the byte length;
 * never any payload content (protected-NV bytes are secret). This RFS variant
 * addresses files by a numeric handle, not an ASCII path, so no path string is
 * present to leak. Bounded so a misbehaving endpoint cannot flood the log. */
static unsigned rfs_trace_count;
static void rfs_trace(const char *note, enum phase phase,
                      const uint8_t *frame, size_t len, int64_t now)
{
    if (!frame || len < 4 || rfs_trace_count >= RFS_TRACE_MAX) return;
    rfs_trace_count++;
    uint32_t cmd = little32(frame);
    uint32_t f1 = len >= 8 ? little32(frame + 4) : 0;
    uint32_t handle = len >= 12 ? little32(frame + 8) : 0;
    uint32_t w3 = len >= 16 ? little32(frame + 12) : 0;
    uint32_t w4 = len >= 20 ? little32(frame + 16) : 0;
    /* Bytes at offset 20+ are request-header fields only on short control
     * frames (<=24 bytes); on data-carrying frames offset 20 is the start of
     * the quarantined NV payload, which must never be logged. */
    uint32_t w5 = len == 24 ? little32(frame + 20) : 0;
    printf("RFS_TRACE note=%s t=%lld phase=%d len=%zu cmd=%u paylen=%u "
           "handle=%u off=%u w4=%u w5=%u\n",
           note, (long long)now, (int)phase, len,
           cmd, f1, handle, w3, w4, w5);
    fflush(stdout);
}

/* Post-completion RFS observer. After the protected-NV write-out quarantine is
 * done (TERMINAL + final_ack_sent) the main loop normally stops reading RFS.
 * This drain keeps reading so any RFS request the CP issues during the later
 * RadioPower-ON / MM registration-gate window is captured. It LOGS headers
 * only and, to keep the CP's handshake alive so subsequent requests keep
 * flowing, replies to an exact unprotect request_7 with the known status_7.
 * It NEVER re-runs the write sequence, serves data, stores payload, or writes
 * any file. It invents no reply bytes (status_7 is the already-observed ack). */
static void post_terminal_rfs_drain(struct owner *o, const uint8_t *bytes,
                                    size_t got, int64_t now)
{
    static uint8_t pt_rx[RX_CAP];
    static size_t pt_used;
#ifdef SAAIOS_RFS_NORMAL_CAPTURE
    /* Read-only diagnostic (opt-in): capture the CP's handle-1 normal-NV
     * write-OUT into a SEPARATE quarantine file so it can be diffed offline
     * against the fed-in nv_normal.bin. The CP repeatedly issues a handle-1
     * grant-request (cmd=6, total size in w4) that the proven drain never
     * answers, so the data never flows. Here we answer it with the SAME grant
     * byte layout already proven on the handle-3 path, carrying the file id
     * (1) the CP itself declared -- recovered protocol, no invented opcode --
     * and write the returned chunks to the quarantine copy only. Never touches
     * real EFS/nv_normal; never logs payload bytes. */
    static int normal_fd = -1;
    static uint32_t normal_total, normal_received;
    static unsigned normal_grants;
    static uint16_t normal_seq;
    static int normal_done;
#endif
    if (!bytes || got > sizeof pt_rx - pt_used) {
        pt_used = 0; /* resynchronize rather than retain ambiguous bytes */
        return;
    }
    memcpy(pt_rx + pt_used, bytes, got);
    pt_used += got;
    while (pt_used) {
        int size = valid_frame_length(pt_rx, pt_used);
        if (size <= 0) break;
        rfs_trace("post_term", TERMINAL, pt_rx, (size_t)size, now);
        if ((size_t)size == sizeof request_7 &&
            same_bytes(pt_rx, request_7, sizeof request_7)) {
            size_t done = 0;
            while (done < sizeof status_7) {
                ssize_t n = write(o->rfs, status_7 + done,
                                  sizeof status_7 - done);
                if (n < 0 && errno == EINTR) continue;
                if (n <= 0) break;
                done += (size_t)n;
            }
        }
#ifdef SAAIOS_RFS_NORMAL_CAPTURE
        if (!normal_done && (size_t)size >= 20) {
            uint16_t cmd = little16(pt_rx);
            if (cmd == 7 && (size_t)size == 12 && little32(pt_rx + 8) == 1) {
                /* handle-1 open: ack with the proven status layout, file id 1 */
                static const uint8_t st1[16] =
                    {3,0,0,0, 8,0,0,0, 0,0,0,0, 1,0,0,0};
                size_t done = 0;
                while (done < sizeof st1) {
                    ssize_t n = write(o->rfs, st1 + done, sizeof st1 - done);
                    if (n < 0 && errno == EINTR) continue;
                    if (n <= 0) break;
                    done += (size_t)n;
                }
            } else if (cmd == 6 && (size_t)size == 24 &&
                       little32(pt_rx + 8) == 1) {
                /* handle-1 grant-request: w4 = total normal-NV size; echo the
                 * CP's sequence (w0 high 16) in our grant so it is accepted. */
                uint32_t total = little32(pt_rx + 16);
                normal_seq = little16(pt_rx + 2);
                if (normal_fd < 0 && total > 0 && total <= BASELINE_BYTES &&
                    o->quarantine_dir >= 0) {
                    normal_fd = openat(o->quarantine_dir, "normal-candidate.bin",
                                       O_CREAT | O_RDWR | O_EXCL | O_CLOEXEC,
                                       0600);
                    normal_total = total;
                    normal_received = 0;
                    printf("NORMAL_CAPTURE open total=%u seq=%u fd=%d\n",
                           total, normal_seq, normal_fd);
                    fflush(stdout);
                }
                if (normal_fd >= 0 && normal_received < normal_total) {
                    uint32_t remain = normal_total - normal_received;
                    uint32_t len = remain < FIRST_CHUNK ? remain : FIRST_CHUNK;
                    uint8_t grant[20] = {2,0,0,0, 12,0,0,0, 1,0,0,0};
                    put_little32(grant, 2u | ((uint32_t)normal_seq << 16));
                    put_little32(grant + 12, normal_received);
                    put_little32(grant + 16, len);
                    size_t done = 0;
                    while (done < sizeof grant) {
                        ssize_t n = write(o->rfs, grant + done,
                                          sizeof grant - done);
                        if (n < 0 && errno == EINTR) continue;
                        if (n <= 0) break;
                        done += (size_t)n;
                    }
                    normal_grants++;
                }
            } else if (cmd == 2 && normal_fd >= 0 &&
                       little32(pt_rx + 12) == 1) {
                /* handle-1 CP response: data chunk (len>0) or status (len==0) */
                uint32_t clen = little32(pt_rx + 16);
                if (clen == 0) {
                    printf("NORMAL_CAPTURE status=%u seq=%u received=%u\n",
                           little32(pt_rx + 8), little16(pt_rx + 2),
                           normal_received);
                    fflush(stdout);
                }
                if (clen > 0 && clen <= FIRST_CHUNK &&
                    (size_t)size >= 20u + clen &&
                    normal_received + clen <= normal_total &&
                    write_all_at(normal_fd, pt_rx + 20, clen,
                                 (off_t)normal_received) == 0)
                    normal_received += clen;
                if (normal_received >= normal_total) {
                    (void)fsync(normal_fd);
                    normal_done = 1;
                    printf("NORMAL_CAPTURE done received=%u grants=%u\n",
                           normal_received, normal_grants);
                    fflush(stdout);
                } else if (clen > 0) {
                    uint32_t remain = normal_total - normal_received;
                    uint32_t len = remain < FIRST_CHUNK ? remain : FIRST_CHUNK;
                    uint8_t grant[20] = {2,0,0,0, 12,0,0,0, 1,0,0,0};
                    put_little32(grant, 2u | ((uint32_t)normal_seq << 16));
                    put_little32(grant + 12, normal_received);
                    put_little32(grant + 16, len);
                    size_t done = 0;
                    while (done < sizeof grant) {
                        ssize_t n = write(o->rfs, grant + done,
                                          sizeof grant - done);
                        if (n < 0 && errno == EINTR) continue;
                        if (n <= 0) break;
                        done += (size_t)n;
                    }
                    normal_grants++;
                }
            }
        }
#endif
        memmove(pt_rx, pt_rx + size, pt_used - (size_t)size);
        pt_used -= (size_t)size;
    }
    if (pt_used == sizeof pt_rx) pt_used = 0;
}

static int complete_frame(struct owner *o, const uint8_t *frame,
                          size_t len, size_t trailing, int64_t now)
{
    rfs_trace("serve", o->phase, frame, len, now);
    if (stop_requested || now < 0 || now >= o->deadline_ms ||
        now >= o->total_deadline_ms) {
        diagnose(o, frame_stage(o), REASON_TIMEOUT);
        return -1;
    }
    enum action action = classify(o, frame, len);
    if (action == BAD_FRAME) {
        if (o->phase == WAIT_DATA) {
            o->frame_mismatch_mask = data_mismatch_mask(o, frame, len);
            if (frame_stage(o) == STAGE_FINAL_FRAME)
                diagnose_final_frame_shape(o, frame, len, trailing);
        }
        diagnose(o, frame_stage(o), REASON_MALFORMED);
        return -1;
    }
    if (trailing && action != NO_REPLY) {
        diagnose(o, frame_stage(o), REASON_TRAILING);
        return -1;
    }
    if (action == STATUS_7) {
        if (reply_gate(o) ||
            send_modem_once(o, status_7, sizeof status_7)) return -1;
        o->phase = WAIT_3;
    } else if (action == NO_REPLY) {
        o->phase = WAIT_6;
    } else if (action == GRANT_1) {
        if (send_next_grant(o)) return -1;
    } else if (action == STORE_CHUNK) {
        int stored;
#ifdef RFS_HOST_TEST
        if (host_store_override) stored = 0;
        else
#endif
        stored = store_chunk(o, frame + 20);
        if (stored) return -1;
#ifdef RFS_HOST_TEST
        if (host_store_override) {
            sha_update(&o->received_hash, frame + 20, o->expected_chunk);
            o->received_bytes += o->expected_chunk;
            o->chunks_stored++;
        }
#endif
        if (o->received_bytes < RFS_TRANSFER_BYTES) {
            if (send_next_grant(o)) return -1;
        } else {
            if (o->received_bytes != RFS_TRANSFER_BYTES ||
                o->chunks_stored != RFS_GRANTS_MAX ||
                o->grant_attempted != RFS_GRANTS_MAX) {
                diagnose(o, STAGE_FINALIZE, REASON_STATE);
                return -1;
            }
            int finished;
#ifdef RFS_HOST_TEST
            if (host_finish_override) finished = 0;
            else
#endif
            finished = finish_candidate(o);
            if (finished) {
                diagnose(o, STAGE_FINALIZE, REASON_IO);
                return -1;
            }
            if (reply_gate(o)) {
                diagnose(o, STAGE_FINAL_ACK, REASON_GATE);
                return -1;
            }
#ifndef RFS_HOST_TEST
            if (sidecar_stable(o)) {
                diagnose(o, STAGE_FINAL_ACK, REASON_GATE);
                return -1;
            }
#else
            if (!host_finish_override && sidecar_stable(o)) {
                diagnose(o, STAGE_FINAL_ACK, REASON_GATE);
                return -1;
            }
#endif
            o->final_ack_attempted = 1;
            if (send_modem_once(o, final_status, sizeof final_status)) {
                diagnose(o, STAGE_FINAL_ACK, REASON_SEND);
                return -1;
            }
            o->final_ack_sent = 1;
            int64_t ack_returned_at = monotonic_ms();
            if (ack_returned_at >= o->sit.ready_ms)
                printf("rfs_final_ack_local_write_returned owner_elapsed_ms=%lld\n",
                       (long long)(ack_returned_at - o->sit.ready_ms));
            else
                puts("rfs_final_ack_local_write_returned owner_elapsed_ms=unknown");
            terminal(o, "complete_quarantined_ack");
            return 0;
        }
    } else {
        diagnose(o, frame_stage(o), REASON_STATE);
        return -1;
    }
    int64_t progressed_at = monotonic_ms();
    if (progressed_at < 0 || progressed_at >= o->total_deadline_ms) {
        diagnose(o, STAGE_DEADLINE, REASON_TIMEOUT);
        return -1;
    }
    o->deadline_ms = progressed_at + STEP_DEADLINE_MS;
    if (o->deadline_ms > o->total_deadline_ms)
        o->deadline_ms = o->total_deadline_ms;
    return 0;
}

static int feed_rfs(struct owner *o, const uint8_t *bytes,
                    size_t len, int64_t now)
{
    if (o->phase == TERMINAL || !bytes || len > sizeof o->rx - o->used) {
        diagnose(o, frame_stage(o), REASON_RX_OVERFLOW);
        return -1;
    }
    memcpy(o->rx + o->used, bytes, len);
    o->used += len;
    while (o->used) {
        int size = valid_frame_length(o->rx, o->used);
        if (size < 0) {
            diagnose(o, frame_stage(o), REASON_LENGTH);
            return -1;
        }
        if (size == 0) break;
        size_t trailing = o->used - (size_t)size;
        if (complete_frame(o, o->rx, (size_t)size, trailing, now)) return -1;
        if (o->phase == TERMINAL) return 0;
        memmove(o->rx, o->rx + size, trailing);
        memset(o->rx + trailing, 0, (size_t)size);
        o->used = trailing;
    }
    if (o->used == sizeof o->rx) {
        diagnose(o, frame_stage(o), REASON_RX_OVERFLOW);
        return -1;
    }
    return 0;
}

/* This observer shares the existing exclusive IPC owner. It never opens a
 * second endpoint, issues a SET, or gives RFS work to the SIT parser. */
static const struct {
    uint16_t id;
    const char *name;
} sit_gets[SIT_GET_COUNT] = {
    {0x0200, "sim"}, {0x0801, "radio"},
    {SIT_NET_VOICE_REG, "voice"}, {SIT_NET_DATA_REG, "data"}
};

static const char *sit_pass_name(unsigned pass)
{
    return pass == 0 ? "online" : "settled";
}

static void sit_unknown(struct sit_observer *s, const char *status)
{
    printf("sit_snapshot pass=%s field=%s status=%s\n",
           sit_pass_name(s->pass), sit_gets[s->next].name, status);
}

static void sit_disable(struct sit_observer *s, const char *reason)
{
    if (!s->poisoned)
        printf("sit_observer=%s no_more_gets=1\n", reason);
    s->poisoned = 1;
    s->pending = 0;
    s->used = 0;
    zero_bytes(s->rx, sizeof s->rx);
}

static void sit_poison(struct sit_observer *s)
{
    sit_disable(s, "framing_unknown");
}

static void sit_endpoint_fault(struct owner *o)
{
    o->sit.endpoint_failed = 1;
    sit_disable(&o->sit, "endpoint_lost");
    /* The RFS reply gate requires both channel descriptors. Losing IPC
     * before the final ACK disarms quarantine, as in the prior owner. */
    if (o->phase != TERMINAL) terminal(o, "ipc_endpoint_lost");
}

static void poll_faults_before_rfs(struct owner *o,
                                   short rfs_revents, short ipc_revents)
{
    const short faults = POLLERR | POLLHUP | POLLNVAL;
    /* A co-reported final RFS chunk must not be acknowledged while IPC is
     * already in fault state. Inspect both endpoints before either read. */
    if (ipc_revents & faults) sit_endpoint_fault(o);
    if (rfs_revents & faults) terminal(o, "rfs_endpoint_lost");
}

/* Response size is the exact SIT header length, not the read() boundary. */
static int sit_frame_length(const uint8_t *p, size_t used)
{
    if (used < 6) return 0;
    if (p[0] > 2) return -1;
    uint16_t length = little16(p + 4);
    unsigned minimum = p[0] == 2 ? 8u : 12u;
    /* The 16-bit wire length cannot exceed the 65536-byte parser cap. */
    if (length < minimum) return -1;
    return used < length ? 0 : (int)length;
}

static int sit_sim_status_complete(const uint8_t *p, size_t len)
{
    if (len < 15) return 0;
    unsigned apps = p[SIT_SIM_APPS];
    return apps <= 4 && len >= 15u + (size_t)apps * SIT_SIM_APP_STRIDE;
}

/* Record only framed unsolicited network/radio headers. The payload may
 * contain network or subscriber data and must not be inspected or logged. */
static int sit_event_trace_take(struct sit_observer *s, const uint8_t *p,
                                size_t len, unsigned *id)
{
    if (len < 8 || p[0] != 2 || little16(p + 4) != len) return 0;
    unsigned value = little16(p + 2);
    if ((value & 0xff00u) != 0x0700u &&
        (value & 0xff00u) != 0x0800u) return 0;
    if (s->event_seen != UINT_MAX) s->event_seen++;
    if (s->event_logged >= SIT_EVENT_TRACE_LIMIT) {
        if (s->event_overflow != UINT64_MAX) s->event_overflow++;
        return 0;
    }
    s->event_logged++;
    *id = value;
    return 1;
}

static int sit_event_quiet_window(const struct sit_observer *s)
{
    return s->event_window_ms >= s->ready_ms &&
           s->event_window_ms - s->ready_ms <
           SIT_EVENT_QUIET_COVERAGE_MS;
}

static void sit_event_trace_window(struct sit_observer *s, int64_t now)
{
    if (now < s->event_window_ms ||
        now - s->event_window_ms < SIT_EVENT_WINDOW_MS) return;
    if (s->event_seen || sit_event_quiet_window(s))
        printf("sit_event_window owner_elapsed_ms=%lld window_ms=%lld seen=%u logged=%u overflow=%llu trace_active=%u\n",
               (long long)(now - s->ready_ms),
               (long long)(now - s->event_window_ms), s->event_seen,
               s->event_logged, (unsigned long long)s->event_overflow,
               !s->poisoned && !s->endpoint_failed);
    s->event_window_ms = now;
    s->event_seen = s->event_logged = 0;
    s->event_overflow = 0;
}

static void sit_on_frame(struct sit_observer *s, const uint8_t *p, size_t len,
                         int64_t received_at)
{
    unsigned event_id;
    if (sit_event_trace_take(s, p, len, &event_id))
        printf("sit_event elapsed_ms=%lld id=0x%04x len=%zu\n",
               (long long)(received_at - s->ready_ms), event_id, len);
    if (!s->pending || s->pass >= 2 || s->next >= SIT_GET_COUNT ||
        (s->deadline_ms > 0 && received_at >= s->deadline_ms) ||
        len < 12 || p[0] != 1 ||
        little16(p + 2) != sit_gets[s->next].id ||
        little16(p + 4) != len || little32(p + 6) != s->token)
        return; /* Unsolicited and stale responses are never evidence. */
    unsigned id = sit_gets[s->next].id;
    unsigned error = little16(p + 10);
    const char *name = sit_gets[s->next].name;
    printf("sit_snapshot pass=%s field=%s response=yes error_raw=%u",
           sit_pass_name(s->pass), name, error);
    if (error) {
        puts(" status=unknown_remote_error");
    } else if (id == 0x0200) {
        if (!sit_sim_status_complete(p, len))
            puts(" status=unknown_short");
        else {
            unsigned apps = p[SIT_SIM_APPS];
            printf(" card_raw=%u apps=%u", p[SIT_SIM_CARD], apps);
            if (apps)
                printf(" app_state_raw=%u pin1_raw=%u",
                       p[SIT_SIM_APP_STATE], p[SIT_SIM_PIN1]);
            putchar('\n');
        }
    } else if (id == 0x0801) {
        if (len < 16) puts(" status=unknown_short");
        else printf(" radio_raw=%u\n", little32(p + 12));
    } else if (len < (id == SIT_NET_DATA_REG ? 16u : 14u)) {
        puts(" status=unknown_short");
    } else {
        printf(" registration_raw=%u reject_raw=%u",
               p[SIT_NET_REG_STATE_OFFSET], p[SIT_NET_REJECT_OFFSET]);
        if (id == SIT_NET_DATA_REG)
            printf(" tech_raw=%u", p[SIT_NET_DATA_TECH_OFFSET]);
        putchar('\n');
    }
    s->pending = 0;
    s->next++;
}

/* A separate, bounded streaming buffer handles fragmented and coalesced IPC.
 * Malformed/oversized IPC disables only this observer, never RFS quarantine. */
static void sit_feed(struct sit_observer *s, const uint8_t *bytes, size_t len,
                     int64_t received_at)
{
    if (s->poisoned) return;
    while (len && !s->poisoned) {
        size_t space = sizeof s->rx - s->used;
        if (!space) { sit_poison(s); break; }
        size_t take = len < space ? len : space;
        memcpy(s->rx + s->used, bytes, take);
        s->used += take;
        bytes += take;
        len -= take;
        size_t offset = 0;
        while (offset < s->used) {
            int length = sit_frame_length(s->rx + offset, s->used - offset);
            if (length < 0) { sit_poison(s); break; }
            if (!length) break;
            sit_on_frame(s, s->rx + offset, (size_t)length, received_at);
            offset += (size_t)length;
        }
        if (s->poisoned) break;
        if (offset) {
            size_t remaining = s->used - offset;
            memmove(s->rx, s->rx + offset, remaining);
            zero_bytes(s->rx + remaining, offset);
            s->used = remaining;
        }
    }
}

/* A short or ambiguous IPC write consumes the request; never resend it. */
static int sit_send_get_once(int ipc, uint16_t id, uint32_t token)
{
    uint8_t request[12] = {0};
    request[2] = (uint8_t)id;
    request[3] = (uint8_t)(id >> 8);
    request[4] = sizeof request;
    put_little32(request + 6, token);
    sigset_t blocked, old;
    sigemptyset(&blocked);
    sigaddset(&blocked, SIGTERM);
    sigaddset(&blocked, SIGINT);
    if (sigprocmask(SIG_BLOCK, &blocked, &old)) return -1;
    ssize_t written = -1;
    if (!stop_requested) {
#ifdef RFS_HOST_TEST
        if (host_sit_write_override) {
            host_sit_write_calls++;
            written = host_sit_write_override(ipc, request, sizeof request);
        } else
#endif
        written = write(ipc, request, sizeof request);
    }
    int restored = sigprocmask(SIG_SETMASK, &old, NULL);
    zero_bytes(request, sizeof request);
    return restored == 0 && written == 12 ? 0 : -1;
}

static void sit_advance(struct owner *o, int64_t now)
{
    struct sit_observer *s = &o->sit;
    if (s->poisoned || s->pass >= 2 || stop_requested) return;
    /* RFS failure or ambiguous ACK forbids any further modem writes, even
     * read-only GETs. A completed, acknowledged quarantine may be observed. */
    if (o->phase == TERMINAL && !o->final_ack_sent) return;
    if (s->pending) {
        if (now < s->deadline_ms) return;
        sit_unknown(s, "unknown_timeout");
        /* The CP may still be completing this request. Do not overlap it
         * with another GET, even if its late reply is ignored by token. */
        sit_disable(s, "reply_timeout");
        return;
    }
    if (s->next == SIT_GET_COUNT) {
        s->pass++;
        s->next = 0;
        s->started = 0;
        if (s->pass >= 2) return;
    }
    if (s->pass == 1 && now < s->ready_ms + SIT_SETTLED_MS) return;
    /* The loop services RFS before IPC and reaches here only afterward.
     * Keep away from partial RFS frames and its deadline; do not poll the
     * CPIF fd again here, because it may report POLLIN for an empty read. */
    if (o->phase != TERMINAL) {
        if (now + SIT_RFS_GUARD_MS >= o->deadline_ms ||
            o->used != 0) return;
    }
    if (!s->started) {
        s->started = 1;
        printf("sit_snapshot=%s_started elapsed_ms=%lld read_only=1\n",
               sit_pass_name(s->pass), (long long)(now - s->ready_ms));
    }
    if (!s->token)
        s->token = (uint32_t)now ^ (uint32_t)getpid() ^ 0x5a170000u;
    ++s->token;
    if (sit_send_get_once(o->ipc, sit_gets[s->next].id, s->token)) {
        sit_unknown(s, "unknown_write_no_retry");
        s->poisoned = 1; /* Do not start a later pass after ambiguous write. */
        return;
    }
    s->pending = 1;
    s->deadline_ms = now + SIT_REPLY_MS;
}

#ifdef SAAIOS_RFS_CAMP
/* Exact stock stage-1 request builders, recovered from the vendor RIL and
 * matching modem-channel-owner.c byte-for-byte. No CLI value or NV access. */
static void make_setmodemsconfig_request(uint8_t request[SEQ_CONFIG_LEN],
                                         uint32_t token)
{
    memset(request, 0, SEQ_CONFIG_LEN);
    request[2] = (uint8_t)SEQ_CONFIG_COMMAND;        /* 0x3f */
    request[3] = (uint8_t)(SEQ_CONFIG_COMMAND >> 8); /* 0x09 */
    request[4] = SEQ_CONFIG_LEN;
    put_little32(request + 6, token);
    request[12] = 0; /* one modem */
}

static void make_sgc_request(uint8_t request[SGC_LEN], uint32_t token)
{
    memset(request, 0, SGC_LEN);
    request[2] = 0x04;
    request[3] = 0x04;
    request[4] = SGC_LEN;
    put_little32(request + 6, token);
    put_little32(request + 12, 0x0101);
}

static void make_radiopower_request(uint8_t request[CAMP_POWER_LEN],
                                    uint32_t token)
{
    memset(request, 0, CAMP_POWER_LEN);
    request[2] = (uint8_t)CAMP_POWER_COMMAND;        /* 0x00 */
    request[3] = (uint8_t)(CAMP_POWER_COMMAND >> 8); /* 0x08 */
    request[4] = CAMP_POWER_LEN;
    put_little32(request + 6, token);
    put_little32(request + 12, CAMP_POWER_ON); /* arg1!=0 -> power word 2 */
    request[16] = 0; /* arg2 */
    request[17] = 0; /* arg3 */
}

/* Fed every framed IPC indication. Tracks the factory-callback trigger pair:
 * an exact 8-byte type-2 0x0803 (UNAVAILABLE) then an exact 12-byte type-2
 * 0x0802 with raw scalar 0. Any duplicate, reversed, malformed or superseding
 * radio event invalidates eligibility; no later event rearms it. */
static void camp_observe(struct camp_driver *c, const uint8_t *p, size_t n,
                         int64_t now)
{
    if (c->radio_invalidated || c->radio_ready0_ms) {
        if (n >= 8 && p[0] == 2 &&
            (little16(p + 2) == 0x0803 || little16(p + 2) == 0x0802))
            c->radio_invalidated = 1;
        return;
    }
    if (n < 8 || p[0] != 2) return;
    unsigned id = little16(p + 2);
    if (id == 0x0803) {
        if (n != 8 || little16(p + 4) != n) { c->radio_invalidated = 1; return; }
        if (now - c->owner_start_ms > CAMP_WINDOW_MS) return;
        if (c->radio_stage != 0) { c->radio_invalidated = 1; return; }
        c->radio_stage = 1;
        c->radio_unavail_ms = now;
    } else if (id == 0x0802) {
        int parsed = (n == 12 && little16(p + 4) == n) ? 1 : -1;
        uint32_t raw = parsed == 1 ? little32(p + 8) : UINT32_MAX;
        if (c->radio_stage != 1 || parsed != 1 || raw != 0 ||
            now - c->radio_unavail_ms > CAMP_PAIR_GAP_MS ||
            now - c->owner_start_ms > CAMP_WINDOW_MS) {
            c->radio_invalidated = 1;
            return;
        }
        c->radio_ready0_ms = now;
    }
}

/* Match the three camp ACKs by id and echoed token. Only fixed opcodes and the
 * public error word reach the log; never any payload. */
static void camp_ack(struct camp_driver *c, const uint8_t *p, size_t n)
{
    if (n < 12 || p[0] != 1 || little16(p + 4) != n) return;
    unsigned id = little16(p + 2);
    uint32_t token = little32(p + 6);
    unsigned error = little16(p + 10);
    if (c->cfg_sent && !c->cfg_acked &&
        id == SEQ_CONFIG_COMMAND && token == c->cfg_token) {
        c->cfg_acked = 1;
        c->cfg_error = error;
        printf("camp_ack cmd=0x093f response=yes error_raw=%u\n", error);
    } else if (c->sgc_sent && !c->sgc_acked &&
               id == SGC_COMMAND && token == c->sgc_token) {
        c->sgc_acked = 1;
        c->sgc_error = error;
        printf("camp_ack cmd=0x0404 response=yes error_raw=%u\n", error);
    } else if (c->camp_sent && !c->camp_acked &&
               id == CAMP_POWER_COMMAND && token == c->camp_token) {
        c->camp_acked = 1;
        c->camp_error = error;
        printf("camp_ack cmd=0x0800 response=yes error_raw=%u\n", error);
    }
}

/* sit-stream.so GetSignalStrength reads a 16-bit technology-presence mask at
 * response +12; its V4 parser needs 196 bytes after the mask (210 total).
 * Only the low-seven presence bits are extracted; never any measurement. */
static int camp_signal_mask(const uint8_t *p, size_t n, uint32_t token,
                            uint32_t *mask)
{
    if (n < PROBE_SIGNAL_MIN || p[0] != 1 || little16(p + 2) != 0x0900 ||
        little32(p + 6) != token || little16(p + 4) != n ||
        little16(p + 10)) return 0;
    *mask = little16(p + 12) & 0x7fu;
    return 1;
}

/* Prober GET rotation: SIM status, signal presence, and voice/data
 * registration. SIM is also re-queried immediately on each 0x0210. */
static const struct { unsigned id; const char *name; } camp_probe_gets[] = {
    {0x0200, "sim"}, {0x0900, "signal"},
    {SIT_NET_VOICE_REG, "voice"}, {SIT_NET_DATA_REG, "data"}
};
enum { CAMP_PROBE_COUNT =
           (int)(sizeof camp_probe_gets / sizeof camp_probe_gets[0]) };

/* Factory BuildSetPreferredNetworkType (0x070a): 16-byte request, the RAT value
 * as a little-endian word at +12. Recovered and self-tested in
 * ready-network-once.c; no invented bytes. */
static void make_setpref_request(uint8_t request[REG_PREF_LEN], uint32_t value,
                                 uint32_t token)
{
    memset(request, 0, REG_PREF_LEN);
    request[2] = (uint8_t)REG_PREF_SET;        /* 0x0a */
    request[3] = (uint8_t)(REG_PREF_SET >> 8); /* 0x07 */
    request[4] = REG_PREF_LEN;
    put_little32(request + 6, token);
    put_little32(request + 12, value);
}

/* Factory BuildAllowData (0x0710): 13-byte request, allow flag 1 at +12. */
static void make_allowdata_request(uint8_t request[REG_ALLOW_LEN],
                                   uint32_t token)
{
    memset(request, 0, REG_ALLOW_LEN);
    request[2] = (uint8_t)REG_ALLOW_DATA;        /* 0x10 */
    request[3] = (uint8_t)(REG_ALLOW_DATA >> 8); /* 0x07 */
    request[4] = REG_ALLOW_LEN;
    put_little32(request + 6, token);
    request[12] = 1;
}

_Static_assert(REG_IA_LEN == 250, "SET_INITIAL_ATTACH_APN frame must be 250 bytes");
_Static_assert(REG_IA_PCSCF_OFF < REG_IA_LEN, "pcscf offset within frame");

/* Factory BuildSetInitialAttachApn (0x0603): 250-byte request. Layout and every
 * body byte are recovered from ProtocolPsBuilder::BuildSetInitialAttachApn +
 * FillApnInfo<sit_pdp_set_initial_attach_apn_req> in the factory libsitril.so;
 * see the REG_IA_* enum comment. For a plain IP APN with no username/password
 * and no auth, the enum-converted bytes are constants (pdpType=IP=1, authType=0,
 * dataProfileId/apnType/pcscfReqType=0); the APN string is copied verbatim. No
 * invented bytes; nothing here is an NV/EFS write. */
static void make_initial_attach_apn_request(uint8_t request[REG_IA_LEN],
                                            const char *apn, uint32_t token)
{
    memset(request, 0, REG_IA_LEN);
    request[2] = (uint8_t)REG_INIT_ATTACH_APN;        /* 0x03 */
    request[3] = (uint8_t)(REG_INIT_ATTACH_APN >> 8); /* 0x06 */
    request[4] = (uint8_t)REG_IA_LEN;                 /* 250, [5]=0 => LE 250 */
    put_little32(request + 6, token);
    request[12] = REG_IA_CID;        /* attach pdp cid (RetrieveAttachPdpContext) */
    request[13] = REG_IA_CONST13;    /* fixed 0x0e in BuildSetInitialAttachApn */
    /* request[14]=dataProfileId=0, request[15]=apnType=0 (default APN) */
    if (apn) {
        size_t n = 0;
        while (n < (size_t)(REG_IA_APN_MAX - 1) && apn[n]) n++;
        memcpy(request + REG_IA_APN_OFF, apn, n);   /* NUL already from memset */
    }
    /* username (+117) / password (+167) left empty (zeroed) */
    request[REG_IA_AUTH_OFF] = 0;                 /* ConvertAuthType(0)=0 */
    request[REG_IA_PDPTYPE_OFF] = REG_IA_PDPTYPE_IP; /* GetPdpType("IP")=1 */
    request[REG_IA_PCSCF_OFF] = 0;                /* pcscfReqType */
}

/* Operational-SET request with a 4-byte little-endian payload at +12 (matches
 * the factory sitril builders, e.g. BuildSetVoiceOperation stores int32 mode at
 * body+0 of a len-16 request). Used for 0x091a (mode 3) and 0x0933 (mode 1). */
static void make_opx_u32_request(uint8_t *request, uint16_t id, uint8_t len,
                                 uint32_t token, uint32_t value)
{
    memset(request, 0, len);
    request[2] = (uint8_t)id;
    request[3] = (uint8_t)(id >> 8);
    request[4] = len;
    put_little32(request + 6, token);
    put_little32(request + 12, value);
}

/* Operational-SET request with a single payload byte at +12 (SIT_SET_STACK_
 * STATUS 0x080f, len 13, mode byte 1 = ENABLE). */
static void make_opx_byte_request(uint8_t *request, uint16_t id, uint8_t len,
                                  uint32_t token, uint8_t value)
{
    memset(request, 0, len);
    request[2] = (uint8_t)id;
    request[3] = (uint8_t)(id >> 8);
    request[4] = len;
    put_little32(request + 6, token);
    request[12] = value;
}

/* SIT_SET_DUAL_NETWORK_AND_ALLOW_DATA (0x072b, len 28): 4 x int32 payload,
 * recovered from ProtocolNetworkBuilder::BuildSetDualNetworkAndAllowData in the
 * full libsitril.so. Layout (log "Primary(net,allow), Secondary(net,allow)"):
 *   +12 translate(primaryNet)   +16 translate(secondaryNet)
 *   +20 primaryAllowData        +24 secondaryAllowData
 * translateNetworktype is the SAME table BuildSetPreferredNetworkType (0x070a)
 * uses, and translate(12)=12, so the already-proven wire RAT 12 (LTE/WCDMA) and
 * allow-data 1 are reused for all four fields -- no invented bytes. */
static void make_opx_dual_request(uint8_t *request, uint32_t token)
{
    memset(request, 0, OPX_DUAL_LEN);
    request[2] = (uint8_t)OPX_DUAL_SET;
    request[3] = (uint8_t)(OPX_DUAL_SET >> 8);
    request[4] = OPX_DUAL_LEN;
    put_little32(request + 6, token);
    put_little32(request + 12, OPX_DUAL_NET);   /* primary net (wire 12) */
    put_little32(request + 16, OPX_DUAL_NET);   /* secondary net (wire 12) */
    put_little32(request + 20, OPX_DUAL_ALLOW); /* primary allow data */
    put_little32(request + 24, OPX_DUAL_ALLOW); /* secondary allow data */
}

/* Next post-registration operational step, or 0 when the experiment is idle or
 * complete. Reads the three operational GETs for the log, then issues the one
 * selected SET. GETs and the SET are each dispatched at most once. */
static unsigned camp_opx_next(const struct camp_driver *c)
{
    if (!c->reg_complete || c->opx_done) return 0;
    if (!c->opx_stack_get_sent) return OPX_STACK_GET;
    if (!c->opx_voice_get_sent) return OPX_VOICE_GET;
    if (!c->opx_devsvc_get_sent) return OPX_DEVSVC_GET;
    if (!c->opx_set_sent) {
        if (c->opx_step == OPX_STEP_VOICE) return OPX_VOICE_SET;
        if (c->opx_step == OPX_STEP_INTPS) return OPX_INTPS_SET;
        if (c->opx_step == OPX_STEP_STACK) return OPX_STACK_SET;
        if (c->opx_step == OPX_STEP_DEVSVC) return OPX_DEVSVC_SET;
        if (c->opx_step == OPX_STEP_DUAL) return OPX_DUAL_SET;
    }
    return 0;
}

/* Next one-shot registration step given current known state, or 0 when the
 * sequence has nothing to do this tick (waiting, blocked, or complete). */
static unsigned camp_reg_next(const struct camp_driver *c)
{
    if (!c->sim_ready || c->reg_complete) return 0;

    /* 1. Confirm radio ON (idempotent GET, bounded retries). If the modem
     *    never answers, stop: the SETs below are meaningless with radio off. */
    if (!c->radio_on)
        return c->radio_get_tries < REG_GET_MAX ? REG_RADIO_GET : 0;

    /* 2. Selection mode: read (bounded), then force automatic unless the card
     *    already reads automatic. A slow/absent 0x0703 reply must not block the
     *    experiment, so after REG_GET_MAX reads we send 0x0704 once regardless. */
    if (!c->sel_auto_sent) {
        if (!c->sel_known && c->sel_get_tries < REG_GET_MAX) return REG_SEL_GET;
        if (!(c->sel_known && c->sel_mode == 0)) return REG_SEL_AUTO_SET;
    }

    /* 3. Preferred RAT: read (bounded), then set LTE_WCDMA once so the present
     *    UMTS signal is usable. Any value other than an explicit LTE_WCDMA
     *    readback (including unknown-after-retries and the observed raw 16) is
     *    broadened — PS sitting NOT_SEARCHING on UMTS means WCDMA is excluded. */
    if (!c->pref_set_sent) {
        if (!c->pref_known && c->pref_get_tries < REG_GET_MAX) return REG_PREF_GET;
        if (!(c->pref_known && c->preferred_raw == RAT_LTE_WCDMA)) return REG_PREF_SET;
    }

    /* 4. Initial-attach APN: stock issues SET_INITIAL_ATTACH_APN (0x0603) as the
     *    PS-attach precondition (the default-bearer/attach APN) before allowing
     *    data. Sent once, only when an APN is configured. */
    if (c->apn[0] && !c->ia_apn_sent) return REG_INIT_ATTACH_APN;

    /* 5. Allow PS data once. */
    if (!c->allow_data_sent) return REG_ALLOW_DATA;
    return 0;
}

/* Match a prober reply or note a SIM-status-changed indication. Only scalar
 * status fields and the public error word ever reach the log. */
static void camp_probe_match(struct camp_driver *c, const uint8_t *p, size_t n,
                             int64_t now)
{
    (void)now;
    if (n >= 8 && p[0] == 2 && little16(p + 2) == 0x0210) {
        if (!c->sim_ready) c->sim_change_pending = 1;
        return;
    }
    if (!c->probe_pending || n < 12 || p[0] != 1 ||
        little16(p + 4) != n || little32(p + 6) != c->probe_token ||
        little16(p + 2) != c->probe_id)
        return;
    c->probe_pending = 0;
    c->probe_replies++;
    unsigned id = c->probe_id;
    unsigned error = little16(p + 10);
    if (error) {
        printf("camp_probe field=%s response=yes error_raw=%u\n",
               c->probe_name, error);
        return;
    }
    if (id == 0x0200) {
        if (!sit_sim_status_complete(p, n)) {
            printf("camp_probe field=sim status=unknown_short\n");
            return;
        }
        unsigned apps = p[SIT_SIM_APPS];
        printf("camp_probe field=sim card_raw=%u apps=%u", p[SIT_SIM_CARD],
               apps);
        if (apps) {
            unsigned app_state = p[SIT_SIM_APP_STATE];
            printf(" app_state_raw=%u pin1_raw=%u", app_state,
                   p[SIT_SIM_PIN1]);
            if (app_state == 5 && !c->sim_ready) {
                c->sim_ready = 1;
                putchar('\n');
                printf("camp_sim=ready app_state_raw=5\n");
                return;
            }
        }
        putchar('\n');
    } else if (id == 0x0900) {
        uint32_t mask;
        if (camp_signal_mask(p, n, c->probe_token, &mask)) {
            c->mask_seen = 1;
            c->last_mask_low7 = (unsigned)mask;
            printf("camp_probe field=signal mask_low7=%u\n", (unsigned)mask);
        } else {
            printf("camp_probe field=signal status=unknown_short\n");
        }
    } else if (id == OPX_STACK_GET) {
        if (n >= 16) {
            c->opx_stack_known = 1;
            c->opx_stack_mode = little32(p + 12);
            printf("camp_opx get=stack_status mode_raw=%u\n", little32(p + 12));
        } else if (n >= 13) {
            c->opx_stack_known = 1;
            c->opx_stack_mode = p[12];
            printf("camp_opx get=stack_status mode_raw=%u\n", p[12]);
        } else {
            printf("camp_opx get=stack_status status=unknown_short len=%zu\n", n);
        }
    } else if (id == OPX_VOICE_GET) {
        if (n >= 16)
            printf("camp_opx get=voice_operation mode_raw=%u\n", little32(p + 12));
        else
            printf("camp_opx get=voice_operation status=unknown_short\n");
    } else if (id == OPX_DEVSVC_GET) {
        if (n >= 16)
            printf("camp_opx get=device_service mode_raw=%u\n", little32(p + 12));
        else
            printf("camp_opx get=device_service status=unknown_short\n");
    } else if (id == OPX_VOICE_SET || id == OPX_INTPS_SET ||
               id == OPX_STACK_SET || id == OPX_DEVSVC_SET ||
               id == OPX_DUAL_SET) {
        printf("camp_opx set=%s response=yes error_raw=%u\n", c->probe_name,
               error);
    } else if (id == REG_ALLOW_DATA) {
        /* SET acks are short; handle before the generic length guard so the
         * reg_complete gate (and the opx experiment) can actually fire. */
        c->reg_complete = 1;
        printf("camp_reg set=allow_data response=yes error_raw=%u\n", error);
    } else if (id == REG_INIT_ATTACH_APN) {
        printf("camp_reg set=initial_attach_apn response=yes error_raw=%u\n",
               error);
    } else if (id == REG_SEL_AUTO_SET) {
        printf("camp_reg set=selection_auto response=yes error_raw=%u\n", error);
    } else if (id == REG_PREF_SET) {
        printf("camp_reg set=preferred_lte_wcdma response=yes error_raw=%u\n",
               error);
    } else if (n < (id == SIT_NET_DATA_REG ? 16u : 14u)) {
        printf("camp_probe field=%s status=unknown_short\n", c->probe_name);
    } else if (id == SIT_NET_VOICE_REG || id == SIT_NET_DATA_REG) {
        printf("camp_probe field=%s registration_raw=%u reject_raw=%u",
               c->probe_name, p[SIT_NET_REG_STATE_OFFSET],
               p[SIT_NET_REJECT_OFFSET]);
        if (id == SIT_NET_DATA_REG)
            printf(" tech_raw=%u", p[SIT_NET_DATA_TECH_OFFSET]);
        putchar('\n');
    } else if (id == REG_RADIO_GET) {
        if (n >= 16) {
            c->radio_on = little32(p + 12) == RADIO_STATE_ON;
            printf("camp_reg field=radio radio_raw=%u\n", little32(p + 12));
        } else {
            printf("camp_reg field=radio status=unknown_short\n");
        }
    } else if (id == REG_SEL_GET) {
        if (n >= 13 && p[12] <= 1) {
            c->sel_known = 1;
            c->sel_mode = p[12];
            printf("camp_reg field=selection mode_raw=%u\n", p[12]);
        } else {
            c->sel_known = 1; /* unparseable: treat as auto, do not re-GET */
            c->sel_mode = 0;
            printf("camp_reg field=selection status=unknown_short\n");
        }
    } else if (id == REG_PREF_GET) {
        if (n >= 16) {
            c->pref_known = 1;
            c->preferred_raw = little32(p + 12);
            printf("camp_reg field=preferred preferred_raw=%u\n",
                   little32(p + 12));
        } else {
            c->pref_known = 1;
            c->preferred_raw = 0;
            printf("camp_reg field=preferred status=unknown_short\n");
        }
    } else if (id == REG_SEL_AUTO_SET || id == REG_PREF_SET ||
               id == REG_ALLOW_DATA) {
        /* Handled above, before the generic length guard. */
    }
}

/* Independent bounded framer, separate from the SIT observer's buffer. */
static void camp_feed(struct camp_driver *c, const uint8_t *bytes, size_t len,
                      int64_t now)
{
    if (c->poisoned) return;
    while (len) {
        size_t space = sizeof c->rx - c->used;
        if (!space) { c->poisoned = 1; break; }
        size_t take = len < space ? len : space;
        memcpy(c->rx + c->used, bytes, take);
        c->used += take;
        bytes += take;
        len -= take;
        size_t offset = 0;
        while (offset < c->used) {
            int length = sit_frame_length(c->rx + offset, c->used - offset);
            if (length < 0) { c->poisoned = 1; break; }
            if (!length) break;
            camp_observe(c, c->rx + offset, (size_t)length, now);
            camp_ack(c, c->rx + offset, (size_t)length);
            camp_probe_match(c, c->rx + offset, (size_t)length, now);
            offset += (size_t)length;
        }
        if (c->poisoned) break;
        if (offset) {
            size_t remaining = c->used - offset;
            memmove(c->rx, c->rx + offset, remaining);
            zero_bytes(c->rx + remaining, offset);
            c->used = remaining;
        }
    }
    if (c->poisoned) { c->used = 0; zero_bytes(c->rx, sizeof c->rx); }
}

/* A short or ambiguous IPC write consumes the request; never resend it. */
static int camp_send_once(int ipc, const uint8_t *frame, size_t len)
{
    sigset_t blocked, old;
    sigemptyset(&blocked);
    sigaddset(&blocked, SIGTERM);
    sigaddset(&blocked, SIGINT);
    if (sigprocmask(SIG_BLOCK, &blocked, &old)) return -1;
    ssize_t written = -1;
    if (!stop_requested) {
#ifdef RFS_HOST_TEST
        if (host_sit_write_override) {
            host_sit_write_calls++;
            written = host_sit_write_override(ipc, frame, len);
        } else
#endif
        written = write(ipc, frame, len);
    }
    int restored = sigprocmask(SIG_SETMASK, &old, NULL);
    return restored == 0 && written == (ssize_t)len ? 0 : -1;
}

/* Dispatch the stock stage-1 trio once, back-to-back on the radio edge. */
static void camp_advance(struct owner *o, int64_t now)
{
    struct camp_driver *c = &o->camp;
    if (c->dispatched || c->poisoned || c->radio_invalidated || stop_requested)
        return;
    /* A failed RFS terminal forbids further modem writes; a successful,
     * acknowledged quarantine (final ACK sent) does not block the IPC camp. */
    if (o->phase == TERMINAL && !o->final_ack_sent) return;
    if (!c->radio_ready0_ms) return;       /* trigger pair not yet armed */
    if (now - c->radio_ready0_ms > CAMP_DISPATCH_MS) {
        c->radio_invalidated = 1;          /* missed the bounded window */
        return;
    }
    if (o->used || o->sit.used || c->used || o->sit.pending)
        return;                            /* mid-frame or GET in flight */
#ifndef RFS_HOST_TEST
    if (cp_state() != CP_ONLINE) return;
#endif
    if (!c->token)
        c->token = (uint32_t)now ^ (uint32_t)getpid() ^ 0x0ca70000u;
    uint32_t cfg_token = ++c->token;
    uint8_t cfg[SEQ_CONFIG_LEN];
    make_setmodemsconfig_request(cfg, cfg_token);
    if (camp_send_once(o->ipc, cfg, sizeof cfg)) {
        c->poisoned = 1;
        puts("camp_dispatch=write_failed step=cfg");
        return;
    }
    c->cfg_token = cfg_token;
    c->cfg_sent = 1;
    uint32_t sgc_token = ++c->token;
    uint8_t sgc[SGC_LEN];
    make_sgc_request(sgc, sgc_token);
    if (camp_send_once(o->ipc, sgc, sizeof sgc)) {
        c->dispatched = 1;
        puts("camp_dispatch=write_failed step=sgc");
        return;
    }
    c->sgc_token = sgc_token;
    c->sgc_sent = 1;
    uint32_t camp_token = ++c->token;
    uint8_t power[CAMP_POWER_LEN];
    make_radiopower_request(power, camp_token);
    if (camp_send_once(o->ipc, power, sizeof power)) {
        c->dispatched = 1;
        puts("camp_dispatch=write_failed step=power");
        return;
    }
    c->camp_token = camp_token;
    c->camp_sent = 1;
    c->dispatched = 1;
    printf("camp_dispatch=sent elapsed_ms=%lld trigger=0x0803-0x0802-raw0"
           " seq=0x093f,0x0404,0x0800 power=on\n",
           (long long)(now - c->owner_start_ms));
}

/* Sustained, non-self-poisoning observer. Drives the SIM to READY by re-GETting
 * 0x0200 on every 0x0210, and keeps polling signal and voice/data registration
 * across the whole settle window. A reply timeout or ambiguous write only backs
 * the prober off; it never disables itself, the dispatcher, or RFS quarantine. */
static void camp_probe_advance(struct owner *o, int64_t now)
{
    struct camp_driver *c = &o->camp;
    if (c->poisoned || stop_requested) return;
    /* A failed RFS terminal forbids further modem writes; a completed,
     * acknowledged quarantine does not block read-only GETs. */
    if (o->phase == TERMINAL && !o->final_ack_sent) return;
#ifndef RFS_HOST_TEST
    if (cp_state() != CP_ONLINE) return;
#endif
    if (now - c->owner_start_ms < PROBE_START_MS) return;
    /* Let the one-shot stage-1 dispatch win the armed radio edge. */
    if (c->radio_ready0_ms && !c->dispatched && !c->radio_invalidated) return;
    if (c->probe_pending) {
        if (now < c->probe_deadline_ms) return;
        c->probe_pending = 0;
        c->probe_timeouts++;
        printf("camp_probe field=%s status=timeout\n", c->probe_name);
        c->probe_next_ms = now + PROBE_GAP_MS;
        return;
    }
    if (now < c->probe_next_ms) return;
    if (o->used || o->sit.used || c->used) return; /* avoid mid-frame writes */
    if (!c->probe_token)
        c->probe_token = (uint32_t)now ^ (uint32_t)getpid() ^ 0x0b0b0000u;
    /* Priority: once the SIM is READY, run the one-shot registration-trigger
     * sequence before resuming round-robin observation. GETs are idempotent and
     * may retry on timeout; the three SETs are marked sent and never resent. */
    unsigned reg = camp_reg_next(c);
    if (reg) {
        ++c->probe_token;
        int wrote;
        const char *rname;
        if (reg == REG_PREF_SET) {
            uint8_t f[REG_PREF_LEN];
            make_setpref_request(f, RAT_LTE_WCDMA, c->probe_token);
            c->pref_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_preferred_lte_wcdma";
        } else if (reg == REG_ALLOW_DATA) {
            uint8_t f[REG_ALLOW_LEN];
            make_allowdata_request(f, c->probe_token);
            c->allow_data_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "allow_data";
        } else if (reg == REG_INIT_ATTACH_APN) {
            uint8_t f[REG_IA_LEN];
            make_initial_attach_apn_request(f, c->apn, c->probe_token);
            c->ia_apn_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_initial_attach_apn";
        } else {
            if (reg == REG_SEL_AUTO_SET) c->sel_auto_sent = 1;
            else if (reg == REG_RADIO_GET) c->radio_get_tries++;
            else if (reg == REG_SEL_GET) c->sel_get_tries++;
            else if (reg == REG_PREF_GET) c->pref_get_tries++;
            wrote = sit_send_get_once(o->ipc, (uint16_t)reg, c->probe_token);
            rname = reg == REG_RADIO_GET ? "get_radio" :
                    reg == REG_SEL_GET ? "get_selection" :
                    reg == REG_PREF_GET ? "get_preferred" : "set_selection_auto";
        }
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = reg;
        c->probe_name = rname;
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_reg=sent step=%s elapsed_ms=%lld\n", rname,
               (long long)(now - c->owner_start_ms));
        return;
    }
    unsigned opx = camp_opx_next(c);
    if (opx) {
        ++c->probe_token;
        int wrote;
        const char *rname;
        if (opx == OPX_VOICE_SET) {
            uint8_t f[OPX_VOICE_LEN];
            make_opx_u32_request(f, OPX_VOICE_SET, OPX_VOICE_LEN,
                                 c->probe_token, OPX_VOICE_MODE);
            c->opx_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_voice_operation";
        } else if (opx == OPX_INTPS_SET) {
            uint8_t f[OPX_INTPS_LEN];
            make_opx_u32_request(f, OPX_INTPS_SET, OPX_INTPS_LEN,
                                 c->probe_token, OPX_INTPS_MODE);
            c->opx_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_intps_service";
        } else if (opx == OPX_STACK_SET) {
            uint8_t f[OPX_STACK_LEN];
            make_opx_byte_request(f, OPX_STACK_SET, OPX_STACK_LEN,
                                  c->probe_token, OPX_STACK_MODE_ENABLE);
            c->opx_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_stack_status";
        } else if (opx == OPX_DEVSVC_SET) {
            uint8_t f[OPX_DEVSVC_LEN];
            make_opx_u32_request(f, OPX_DEVSVC_SET, OPX_DEVSVC_LEN,
                                 c->probe_token, OPX_DEVSVC_MODE_DATA);
            c->opx_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_device_service";
        } else if (opx == OPX_DUAL_SET) {
            uint8_t f[OPX_DUAL_LEN];
            make_opx_dual_request(f, c->probe_token);
            c->opx_set_sent = 1;
            wrote = camp_send_once(o->ipc, f, sizeof f);
            rname = "set_dual_network_allow_data";
        } else {
            if (opx == OPX_STACK_GET) c->opx_stack_get_sent = 1;
            else if (opx == OPX_VOICE_GET) c->opx_voice_get_sent = 1;
            else if (opx == OPX_DEVSVC_GET) c->opx_devsvc_get_sent = 1;
            wrote = sit_send_get_once(o->ipc, (uint16_t)opx, c->probe_token);
            rname = opx == OPX_STACK_GET ? "get_stack_status" :
                    opx == OPX_VOICE_GET ? "get_voice_operation" :
                                           "get_device_service";
        }
        /* After the single SET is dispatched the experiment is done; the normal
         * round-robin then keeps snapshotting voice/data registration. */
        if (c->opx_set_sent) c->opx_done = 1;
        if (wrote) { c->probe_next_ms = now + PROBE_GAP_MS; return; }
        c->probe_pending = 1;
        c->probe_id = opx;
        c->probe_name = rname;
        c->probe_deadline_ms = now + PROBE_REPLY_MS;
        c->probe_next_ms = now + PROBE_GAP_MS;
        c->probe_sent++;
        printf("camp_opx=sent step=%s elapsed_ms=%lld\n", rname,
               (long long)(now - c->owner_start_ms));
        return;
    }
    unsigned pick;
    const char *name;
    if (c->sim_change_pending && !c->sim_ready) {
        pick = 0x0200;
        name = "sim";
        c->sim_change_pending = 0;
    } else {
        pick = camp_probe_gets[c->probe_idx].id;
        name = camp_probe_gets[c->probe_idx].name;
        c->probe_idx = (c->probe_idx + 1u) % (unsigned)CAMP_PROBE_COUNT;
    }
    ++c->probe_token;
    if (sit_send_get_once(o->ipc, (uint16_t)pick, c->probe_token)) {
        c->probe_next_ms = now + PROBE_GAP_MS; /* ambiguous write: back off */
        return;
    }
    c->probe_pending = 1;
    c->probe_id = pick;
    c->probe_name = name;
    c->probe_deadline_ms = now + PROBE_REPLY_MS;
    c->probe_next_ms = now + PROBE_GAP_MS;
    c->probe_sent++;
    printf("camp_probe=sent field=%s elapsed_ms=%lld\n", name,
           (long long)(now - c->owner_start_ms));
}

/* One operational SET is run per boot; the operator selects which by writing a
 * single token to /data/saaios/etc/opx-step. Absent/empty/unknown => GET-only
 * (no SET). Read once at startup; never a secret and never an NV/EFS path. */
static int read_opx_step(void)
{
    int fd = open("/data/saaios/etc/opx-step", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return OPX_STEP_NONE;
    char buf[16] = {0};
    ssize_t r = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (r <= 0) return OPX_STEP_NONE;
    if (!strncmp(buf, "voice", 5)) return OPX_STEP_VOICE;
    if (!strncmp(buf, "intps", 5)) return OPX_STEP_INTPS;
    if (!strncmp(buf, "stack", 5)) return OPX_STEP_STACK;
    if (!strncmp(buf, "devsvc", 6)) return OPX_STEP_DEVSVC;
    if (!strncmp(buf, "dual", 4)) return OPX_STEP_DUAL;
    return OPX_STEP_NONE;
}

/* Load the configured attach APN from /data/saaios/etc/apn (e.g. "internet").
 * Trailing whitespace/newlines are stripped. Returns 1 and fills out[] on
 * success; 0 (and out[0]=0) when absent/empty. The APN is network config, not a
 * secret, and this is never an NV/EFS path. */
static int read_apn(char *out, size_t cap)
{
    if (cap) out[0] = 0;
    int fd = open("/data/saaios/etc/apn", O_RDONLY | O_CLOEXEC);
    if (fd < 0) return 0;
    char buf[REG_IA_APN_MAX] = {0};
    ssize_t r = read(fd, buf, sizeof buf - 1);
    close(fd);
    if (r <= 0) return 0;
    size_t n = (size_t)r;
    while (n && (buf[n - 1] == '\n' || buf[n - 1] == '\r' ||
                 buf[n - 1] == ' ' || buf[n - 1] == '\t'))
        n--;
    if (!n || n >= cap) return 0;
    memcpy(out, buf, n);
    out[n] = 0;
    return 1;
}
#endif /* SAAIOS_RFS_CAMP */

static void request_stop(int signal_number)
{
    (void)signal_number;
    stop_requested = 1;
}

static int parse_fd(const char *text)
{
    char *end = NULL;
    errno = 0;
    long value = strtol(text, &end, 10);
    return errno || end == text || *end || value < 3 || value > INT_MAX ?
           -1 : (int)value;
}

static int parse_args(int argc, char **argv, int *ipc, int *rfs, int *ready)
{
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

static void close_if_open(int fd)
{
    if (fd >= 0) (void)close(fd);
}

static int run_owner(int ipc, int rfs, int ready)
{
    struct owner o = {
        .ipc = ipc, .rfs = rfs, .ready = ready,
        .lock = -1, .source = -1, .pin_fd = -1,
        .source_dir = -1, .pin_dir = -1,
        .quarantine_parent = -1, .quarantine_dir = -1,
        .candidate = -1, .marker_fd = -1, .sidecar_fd = -1,
        .phase = WAIT_7
    };
    struct rlimit no_core = {0, 0};
    struct sigaction action = {0};
    enum cp_state state = CP_UNKNOWN;
    int rc = 1;
    if (geteuid() != 0 || setrlimit(RLIMIT_CORE, &no_core) ||
        prctl(PR_SET_DUMPABLE, 0, 0, 0, 0)) {
        fputs("ABORT root or dump protection unavailable\n", stderr);
        goto done;
    }
    (void)umask(0077);
    action.sa_handler = request_stop;
    sigemptyset(&action.sa_mask);
    if (sigaction(SIGTERM, &action, NULL) ||
        sigaction(SIGINT, &action, NULL) ||
        signal(SIGHUP, SIG_IGN) == SIG_ERR ||
        signal(SIGPIPE, SIG_IGN) == SIG_ERR ||
        verify_device(ipc, "/sys/class/cpif/umts_ipc0/dev") ||
        verify_device(rfs, "/sys/class/cpif/umts_rfs0/dev") ||
        ready_pipe(ready) || cp_state() != CP_BOOTING) {
        fputs("ABORT owner handoff descriptors or CP state refused\n", stderr);
        goto done;
    }
    if (original_efs_unmounted()) {
        fputs("ABORT original EFS mount gate refused\n", stderr);
        goto done;
    }
    o.lock = acquire_lock();
    if (o.lock < 0 || opened_once(ipc) || opened_once(rfs) ||
        original_efs_unmounted() || load_source_and_pin(&o) ||
        consume_pin(&o) ||
        prepare_quarantine(&o) || !stable_sources(&o) ||
        candidate_stable(&o) || original_efs_unmounted() ||
        cp_state() != CP_BOOTING || opened_once(ipc) || opened_once(rfs) ||
        stop_requested) {
        fputs("ABORT source, quarantine, EFS, or exclusive owner gate refused\n",
              stderr);
        goto done;
    }
    int64_t started = monotonic_ms();
    if (started < 0 || send_ready(ready)) {
        fputs("ABORT READY unavailable\n", stderr);
        goto done;
    }
    setvbuf(stdout, NULL, _IOLBF, 0);
    puts("owner=rfs-full-quarantine-ready payload=redacted "
         "responses_max=97 grants_max=95 no_promotion=1");
#ifdef SAAIOS_RFS_CAMP
    puts("owner=rfs-camp-combined camp=armed "
         "trigger=0x0803-0x0802-raw0 seq=0x093f,0x0404,0x0800 "
         "probe=sim,signal,voice,data sim_drive=0x0210->0x0200 "
         "reg=sel_auto,pref_lte_wcdma,allow_data");
#endif
    sha_init(&o.received_hash);
    o.deadline_ms = started + FIRST_DEADLINE_MS;
    o.total_deadline_ms = started + TOTAL_DEADLINE_MS;
    o.sit.ready_ms = started;
    o.sit.event_window_ms = started;
#ifdef SAAIOS_RFS_CAMP
    o.camp.owner_start_ms = started;
    o.camp.opx_step = read_opx_step();
    printf("camp_opx_step=%d\n", o.camp.opx_step);
    if (read_apn(o.camp.apn, sizeof o.camp.apn))
        printf("camp_apn=loaded len=%zu\n", strlen(o.camp.apn));
    else
        puts("camp_apn=none");
#endif
    for (;;) {
        int64_t now = monotonic_ms();
        if (now >= 0) sit_event_trace_window(&o.sit, now);
        state = cp_state();
        if (state == CP_OFFLINE) {
            if (o.phase != TERMINAL) {
                if (o.phase == WAIT_DATA &&
                    frame_stage(&o) == STAGE_FINAL_FRAME)
                    diagnose_waiting(&o);
                terminal(&o, "cp_offline");
            }
            break;
        }
        if (now < 0) {
            terminal(&o, "clock_refused");
        } else if (state == CP_CRASH || state == CP_UNKNOWN) {
            terminal(&o, "cp_state_lost");
        } else if (state == CP_BOOTING &&
                   now - started > BOOTING_LIMIT_MS) {
            terminal(&o, "booting_deadline");
        } else if (o.phase != TERMINAL && now >= o.total_deadline_ms) {
            diagnose_waiting(&o);
            terminal(&o, "total_deadline");
        } else if (o.phase != TERMINAL && now >= o.deadline_ms) {
            diagnose_waiting(&o);
            terminal(&o, "step_deadline");
        }
        if (stop_requested) {
            terminal(&o, "requested_stop");
        }
        if (o.phase == TERMINAL && !o.final_ack_sent && !o.sit.poisoned)
            sit_disable(&o.sit, "rfs_terminal");
        if (state != CP_ONLINE) {
            (void)poll(NULL, 0, POLL_MS);
            continue;
        }
        struct pollfd fds[2] = {
            {.fd = (o.phase == TERMINAL && !o.final_ack_sent) ? -1 : rfs,
             .events = POLLIN},
            {.fd = o.sit.endpoint_failed ||
                   now < o.sit.backoff_until_ms ? -1 : ipc,
             .events = POLLIN}
        };
        int events = poll(fds, 2, POLL_MS);
        if (events < 0 && errno == EINTR) continue;
        if (events < 0) {
            terminal(&o, "poll_refused");
            (void)poll(NULL, 0, POLL_MS);
            continue;
        }
        poll_faults_before_rfs(&o, fds[0].revents, fds[1].revents);
        if (o.phase == TERMINAL && !o.final_ack_sent && !o.sit.poisoned)
            sit_disable(&o.sit, "rfs_terminal");
        for (size_t i = 0; i < 2; ++i) {
            if (fds[i].revents & (POLLERR | POLLHUP | POLLNVAL)) continue;
            if (i == 0 && o.phase == TERMINAL && !o.final_ack_sent) continue;
            if (!(fds[i].revents & POLLIN)) continue;
            uint8_t bytes[RX_CAP];
            size_t cap = i == 0 ? sizeof bytes : SIT_READ_SLICE;
            ssize_t got = read(fds[i].fd, bytes, cap);
            if (got < 0 && errno == EINTR) continue;
            if (got == 0 || (got < 0 &&
                (errno == EAGAIN || errno == EWOULDBLOCK))) {
                if (i == 0) (void)poll(NULL, 0, EMPTY_BACKOFF_MS);
                else o.sit.backoff_until_ms = now + EMPTY_BACKOFF_MS;
                continue;
            }
            if (got < 0) {
                if (i == 0) {
                    diagnose(&o, frame_stage(&o), REASON_IO);
                    terminal(&o, "rfs_read_refused");
                }
                else sit_endpoint_fault(&o);
            } else if (i == 0) {
                int64_t received_at = monotonic_ms();
                if (o.phase == TERMINAL) {
                    if (received_at >= 0)
                        post_terminal_rfs_drain(&o, bytes, (size_t)got,
                                                received_at);
                    zero_bytes(bytes, sizeof bytes);
                    continue;
                }
                if (received_at < 0 || received_at >= o.deadline_ms)
                    diagnose(&o, frame_stage(&o), REASON_TIMEOUT);
                if (received_at < 0 || received_at >= o.deadline_ms ||
                    feed_rfs(&o, bytes, (size_t)got, received_at))
                    terminal(&o, "rfs_frame_or_io_refused");
                if (o.phase == TERMINAL && !o.final_ack_sent &&
                    !o.sit.poisoned)
                    sit_disable(&o.sit, "rfs_terminal");
            } else {
                int64_t received_at = monotonic_ms();
                if (received_at < 0) sit_poison(&o.sit);
                else sit_feed(&o.sit, bytes, (size_t)got, received_at);
#ifdef SAAIOS_RFS_CAMP
                if (received_at >= 0)
                    camp_feed(&o.camp, bytes, (size_t)got, received_at);
#endif
            }
            zero_bytes(bytes, sizeof bytes);
        }
        int64_t observed_at = monotonic_ms();
#ifndef SAAIOS_RFS_CAMP
        if (observed_at >= 0) sit_advance(&o, observed_at);
#else
        /* Camp mode drives IPC via the active prober, not the passive SIT
         * observer, so it never self-poisons on a settled-GET timeout. The
         * observer definition is retained for the host self-test. */
        (void)sit_advance;
        if (observed_at >= 0) camp_advance(&o, observed_at);
        if (observed_at >= 0) camp_probe_advance(&o, observed_at);
#endif
    }
    if (state != CP_OFFLINE)
        fputs("WARNING owner exit before CP OFFLINE may purge RX\n", stderr);
    if (o.chunks_stored &&
        (candidate_stable(&o) || !stable_sources(&o) ||
         original_efs_unmounted())) {
        fputs("ABORT post-run quarantine or source identity changed\n", stderr);
        rc = 1;
    } else {
        rc = o.final_ack_sent && sidecar_stable(&o) == 0 ? 0 : 1;
    }
done:
    close_if_open(o.candidate);
    close_if_open(o.sidecar_fd);
    close_if_open(o.marker_fd);
    close_if_open(o.quarantine_dir);
    close_if_open(o.quarantine_parent);
    close_if_open(o.source);
    close_if_open(o.pin_fd);
    close_if_open(o.source_dir);
    close_if_open(o.pin_dir);
    close_if_open(o.lock);
    close_if_open(ready);
    close_if_open(rfs);
    close_if_open(ipc);
    zero_bytes(o.pin_digest, sizeof o.pin_digest);
    zero_bytes(o.candidate_digest, sizeof o.candidate_digest);
    zero_bytes(&o.received_hash, sizeof o.received_hash);
    zero_bytes(o.rx, sizeof o.rx);
    zero_bytes(o.sit.rx, sizeof o.sit.rx);
#ifdef SAAIOS_RFS_CAMP
    zero_bytes(o.camp.rx, sizeof o.camp.rx);
#endif
    return rc;
}

static int test_framing(void)
{
    int64_t now = monotonic_ms();
    if (now < 0) return -1;
    struct owner o = {.phase = WAIT_3, .deadline_ms = now + 10000,
                      .total_deadline_ms = now + 10000};
    uint8_t coalesced[sizeof request_3 + 12];
    if (feed_rfs(&o, request_3, 7, now) || o.phase != WAIT_3 || o.used != 7 ||
        feed_rfs(&o, request_3 + 7, sizeof request_3 - 7, now) ||
        o.phase != WAIT_6 || o.used != 0)
        return -1;
    memset(&o, 0, sizeof o);
    o.phase = WAIT_3;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    memcpy(coalesced, request_3, sizeof request_3);
    memcpy(coalesced + sizeof request_3, request_6, 12);
    if (feed_rfs(&o, coalesced, sizeof coalesced, now) ||
        o.phase != WAIT_6 || o.used != 12)
        return -1;
    o.phase = TERMINAL;
    if (feed_rfs(&o, request_6, sizeof request_6, now) == 0)
        return -1;
    zero_bytes(coalesced, sizeof coalesced);
    zero_bytes(&o, sizeof o);
    return 0;
}

#ifdef RFS_HOST_TEST
static uint8_t host_last_reply[20];
static size_t host_last_reply_len;
static uint8_t host_last_sit_request[12];

static ssize_t host_short_write(int fd, const void *bytes, size_t len)
{
    (void)fd; (void)bytes;
    return (ssize_t)len - 1;
}

static ssize_t host_full_write(int fd, const void *bytes, size_t len)
{
    (void)fd;
    if (len > sizeof host_last_reply) return -1;
    memcpy(host_last_reply, bytes, len);
    host_last_reply_len = len;
    return (ssize_t)len;
}

static ssize_t host_sit_full_write(int fd, const void *bytes, size_t len)
{
    (void)fd;
    if (len != sizeof host_last_sit_request) return -1;
    memcpy(host_last_sit_request, bytes, len);
    return (ssize_t)len;
}

static ssize_t host_sit_short_write(int fd, const void *bytes, size_t len)
{
    (void)fd; (void)bytes;
    return (ssize_t)len - 1;
}

static void host_sit_reply(uint8_t *frame, size_t length, uint16_t id,
                           uint32_t token, uint16_t error)
{
    memset(frame, 0, length);
    frame[0] = 1;
    frame[2] = (uint8_t)id;
    frame[3] = (uint8_t)(id >> 8);
    frame[4] = (uint8_t)length;
    frame[5] = (uint8_t)(length >> 8);
    put_little32(frame + 6, token);
    frame[10] = (uint8_t)error;
    frame[11] = (uint8_t)(error >> 8);
}

static int test_sit_event_trace(void)
{
    struct sit_observer s = {.ready_ms = 100,
                             .event_window_ms = 100};
    uint8_t frame[8] = {2, 0, 0x10, 0x07, 8, 0, 0, 0};
    unsigned id = 0;
    for (unsigned i = 0; i < SIT_EVENT_TRACE_LIMIT; ++i)
        if (!sit_event_trace_take(&s, frame, sizeof frame, &id) ||
            id != 0x0710) return -1;
    frame[3] = 0x08;
    if (sit_event_trace_take(&s, frame, sizeof frame, &id) ||
        s.event_seen != SIT_EVENT_TRACE_LIMIT + 1 ||
        s.event_logged != SIT_EVENT_TRACE_LIMIT ||
        s.event_overflow != 1) return -1;
    frame[0] = 1;
    if (sit_event_trace_take(&s, frame, sizeof frame, &id) ||
        s.event_seen != SIT_EVENT_TRACE_LIMIT + 1) return -1;
    frame[0] = 2;
    frame[4] = 7; /* A malformed header is not a traced indication. */
    if (sit_event_trace_take(&s, frame, sizeof frame, &id) ||
        s.event_seen != SIT_EVENT_TRACE_LIMIT + 1) return -1;
    sit_event_trace_window(&s, 100 + SIT_EVENT_WINDOW_MS);
    if (s.event_seen || s.event_logged || s.event_overflow ||
        s.event_window_ms != 100 + SIT_EVENT_WINDOW_MS) return -1;
    frame[4] = 8;
    if (!sit_event_trace_take(&s, frame, sizeof frame, &id) ||
        id != 0x0810) return -1;
    s.event_window_ms = s.ready_ms + SIT_EVENT_QUIET_COVERAGE_MS - 1;
    if (!sit_event_quiet_window(&s)) return -1;
    s.event_window_ms++;
    if (sit_event_quiet_window(&s)) return -1;
    return 0;
}

static int test_sit_observer(void)
{
    struct sit_observer s = {.pending = 1, .token = 0xabcdef12u};
    struct owner o = {.phase = TERMINAL, .final_ack_sent = 1,
                      .ipc = -1, .rfs = -1};
    uint8_t sim[15 + SIT_SIM_APP_STRIDE], sim_short[15];
    uint8_t radio[16], voice[14], data[16];
    uint8_t combined[32], invalid[6] = {3,0,0,0,12,0};
    uint8_t indication[8] = {2,0,0x10,0x02,8,0,0,0};
    uint8_t final_frame[20 + 318] = {0};
    int64_t now = monotonic_ms();
    int rc = -1;
    if (now < 0) return -1;
    host_sit_reply(sim, sizeof sim, 0x0200, s.token, 0);
    sim[SIT_SIM_CARD] = 1;
    sim[SIT_SIM_APPS] = 1;
    sim[SIT_SIM_APP_STATE] = 5;
    sim[SIT_SIM_PIN1] = 3;
    host_sit_reply(sim_short, sizeof sim_short, 0x0200, s.token, 0);
    sim_short[SIT_SIM_APPS] = 1;
    if (!sit_sim_status_complete(sim, sizeof sim) ||
        sit_sim_status_complete(sim_short, sizeof sim_short)) goto done;
    sit_feed(&s, sim, 5, now);
    if (s.used != 5 || !s.pending || s.next) goto done;
    sit_feed(&s, sim + 5, sizeof sim - 5, now);
    if (s.used || s.pending || s.next != 1) goto done;
    sit_feed(&s, sim, sizeof sim, now); /* Old reply cannot match next field. */
    if (s.next != 1) goto done;
    s.pending = 1;
    ++s.token;
    host_sit_reply(radio, sizeof radio, 0x0801, s.token, 0);
    put_little32(radio + 12, 10);
    memcpy(combined, radio, sizeof radio);
    memcpy(combined + sizeof radio, radio, sizeof radio);
    sit_feed(&s, combined, sizeof combined, now);
    if (s.used || s.pending || s.next != 2) goto done;
    s.pending = 1;
    ++s.token;
    host_sit_reply(voice, sizeof voice, SIT_NET_VOICE_REG, s.token + 1, 0);
    sit_feed(&s, voice, sizeof voice, now); /* Stale token. */
    if (!s.pending || s.next != 2) goto done;
    sit_feed(&s, indication, sizeof indication, now); /* Not a reply. */
    if (!s.pending || s.next != 2) goto done;
    host_sit_reply(data, sizeof data, SIT_NET_DATA_REG, s.token, 0);
    sit_feed(&s, data, sizeof data, now); /* Wrong ID. */
    if (!s.pending || s.next != 2) goto done;
    host_sit_reply(voice, sizeof voice, SIT_NET_VOICE_REG, s.token, 0);
    sit_feed(&s, voice, sizeof voice, now);
    if (s.pending || s.next != 3) goto done;
    s.pending = 1;
    ++s.token;
    host_sit_reply(data, sizeof data, SIT_NET_DATA_REG, s.token, 5);
    sit_feed(&s, data, sizeof data, now); /* Error is unknown, not a scalar. */
    if (s.pending || s.next != 4) goto done;
    s = (struct sit_observer){.pending = 1, .token = 42};
    host_sit_reply(sim_short, sizeof sim_short, 0x0200, s.token, 0);
    sim_short[SIT_SIM_APPS] = 1;
    sit_feed(&s, sim_short, sizeof sim_short, now);
    if (s.pending || s.next != 1 || s.poisoned) goto done;
    s = (struct sit_observer){.pending = 1, .token = 10,
                              .deadline_ms = now - 1, .ready_ms = now};
    host_sit_reply(sim, sizeof sim, 0x0200, s.token, 0);
    sit_feed(&s, sim, sizeof sim, now); /* Late reply is not evidence. */
    if (!s.pending || s.next) goto done;
    o.sit = s;
    host_sit_write_override = host_sit_full_write;
    host_sit_write_calls = 0;
    o.final_ack_sent = 0;
    sit_advance(&o, now);
    if (host_sit_write_calls || o.sit.next != 0 || !o.sit.pending)
        goto done; /* A failed RFS terminal cannot dispatch new GETs. */
    o.final_ack_sent = 1;
    sit_advance(&o, now);
    if (!o.sit.poisoned || o.sit.pending || host_sit_write_calls)
        goto done; /* Timed-out GET cannot be followed by another. */
    o.sit = (struct sit_observer){.ready_ms = now};
    sit_advance(&o, now);
    if (!o.sit.pending || host_sit_write_calls != 1 ||
        little16(host_last_sit_request + 2) != 0x0200 ||
        little16(host_last_sit_request + 4) != 12 ||
        host_last_sit_request[0] != 0) goto done;
    o.sit.deadline_ms = now + 1;
    sit_advance(&o, now + 1);
    sit_advance(&o, now + SIT_SETTLED_MS);
    if (!o.sit.poisoned || o.sit.pending || host_sit_write_calls != 1)
        goto done; /* One send total, even after the settled threshold. */
    o.sit = (struct sit_observer){.ready_ms = now,
                                  .started = 1, .next = SIT_GET_COUNT};
    host_sit_write_calls = 0;
    sit_advance(&o, now + SIT_SETTLED_MS - 1);
    if (o.sit.pass != 1 || o.sit.pending || host_sit_write_calls)
        goto done;
    sit_advance(&o, now + SIT_SETTLED_MS);
    if (o.sit.pass != 1 || !o.sit.pending ||
        host_sit_write_calls != 1 ||
        little16(host_last_sit_request + 2) != 0x0200) goto done;
    o.sit = (struct sit_observer){.ready_ms = now};
    host_sit_write_override = host_sit_short_write;
    host_sit_write_calls = 0;
    sit_advance(&o, now);
    if (!o.sit.poisoned || o.sit.pending ||
        host_sit_write_calls != 1) goto done;
    host_sit_write_override = host_sit_full_write;
    sit_advance(&o, now + SIT_SETTLED_MS);
    if (host_sit_write_calls != 1) goto done; /* No retry or later pass. */
    s = (struct sit_observer){0};
    sit_feed(&s, invalid, sizeof invalid, now);
    if (!s.poisoned || s.used) goto done;
    o.phase = WAIT_6;
    o.final_ack_sent = 0;
    o.sit = (struct sit_observer){0};
    sit_endpoint_fault(&o);
    if (o.phase != TERMINAL || !o.sit.poisoned ||
        !o.sit.endpoint_failed) goto done;
    o.phase = TERMINAL;
    o.final_ack_sent = 1;
    o.sit = (struct sit_observer){0};
    sit_endpoint_fault(&o);
    if (o.phase != TERMINAL || !o.final_ack_sent || !o.sit.poisoned ||
        !o.sit.endpoint_failed)
        goto done;
    final_frame[0] = 2;
    final_frame[2] = 1;
    put_little32(final_frame + 4, 12 + 318);
    put_little32(final_frame + 12, 3);
    put_little32(final_frame + 16, 318);
    o.phase = WAIT_DATA;
    o.final_ack_sent = 0;
    o.final_ack_attempted = 0;
    o.sit = (struct sit_observer){0};
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    host_write_override = host_full_write;
    host_write_calls = 0;
    poll_faults_before_rfs(&o, POLLIN, POLLERR);
    if (o.phase != TERMINAL || !o.sit.poisoned ||
        o.final_ack_attempted || host_write_calls ||
        feed_rfs(&o, final_frame, sizeof final_frame, now) == 0)
        goto done; /* Simultaneous final RFS data + IPC fault: no ACK. */
    rc = 0;
done:
    host_write_override = NULL;
    host_write_calls = 0;
    host_sit_write_override = NULL;
    host_sit_write_calls = 0;
    zero_bytes(host_last_sit_request, sizeof host_last_sit_request);
    zero_bytes(&s, sizeof s);
    zero_bytes(&o.sit, sizeof o.sit);
    zero_bytes(sim, sizeof sim);
    zero_bytes(sim_short, sizeof sim_short);
    zero_bytes(radio, sizeof radio);
    zero_bytes(voice, sizeof voice);
    zero_bytes(data, sizeof data);
    zero_bytes(combined, sizeof combined);
    zero_bytes(final_frame, sizeof final_frame);
    return rc;
}

static int host_sync_fail(int fd)
{
    (void)fd;
    return -1;
}

static int host_readback_corrupt(int fd, void *out, size_t len, off_t offset)
{
    if (read_all_at(fd, out, len, offset)) return -1;
    if (offset == 0 && len == FIRST_CHUNK)
        ((uint8_t *)out)[0] ^= 1;
    return 0;
}

static int test_host_storage_faults(void)
{
    static const struct chunk_io actual = {fsync, read_all_at};
    static const struct chunk_io failed_sync = {host_sync_fail, read_all_at};
    static const struct chunk_io failed_readback = {fsync, host_readback_corrupt};
    enum failure_reason reason = REASON_NONE;
    uint8_t zero[4096] = {0}, chunk[FIRST_CHUNK], verify[FIRST_CHUNK];
    FILE *source = tmpfile(), *candidate = tmpfile();
    int rc = -1;
    if (!source || !candidate) goto done;
    memset(chunk, 0x5a, sizeof chunk);
    int source_fd = fileno(source), candidate_fd = fileno(candidate);
    if (source_fd < 0 || candidate_fd < 0) goto done;
    for (off_t off = 0; off < BASELINE_BYTES; off += sizeof zero) {
        if (write_all_at(source_fd, zero, sizeof zero, off) ||
            write_all_at(candidate_fd, zero, sizeof zero, off)) goto done;
    }
    if (apply_chunk(candidate_fd, source_fd, 0, chunk, FIRST_CHUNK,
                    &failed_sync, &reason) == 0 || reason != REASON_IO ||
        apply_chunk(candidate_fd, source_fd, 0, chunk, FIRST_CHUNK,
                    &failed_readback, &reason) == 0 ||
        reason != REASON_READBACK ||
        apply_chunk(candidate_fd, source_fd, 0, chunk, FIRST_CHUNK,
                    &actual, NULL) ||
        read_all_at(candidate_fd, verify, FIRST_CHUNK, 0) ||
        !same_bytes(verify, chunk, FIRST_CHUNK) ||
        apply_chunk(candidate_fd, source_fd, RFS_TRANSFER_BYTES - 318,
                    chunk, 318, &actual, NULL) ||
        read_all_at(candidate_fd, verify, 318,
                    RFS_TRANSFER_BYTES - 318) ||
        !same_bytes(verify, chunk, 318) ||
        apply_chunk(candidate_fd, source_fd, RFS_TRANSFER_BYTES - 317,
                    chunk, 318, &actual, NULL) == 0)
        goto done;
    rc = 0;
done:
    if (source) fclose(source);
    if (candidate) fclose(candidate);
    zero_bytes(zero, sizeof zero);
    zero_bytes(chunk, sizeof chunk);
    zero_bytes(verify, sizeof verify);
    return rc;
}

static int test_host_failure_diagnostics(void)
{
    struct owner o = {0};
    uint8_t frame[20 + 318 + 1] = {0};
    int64_t now = monotonic_ms();
    int rc = -1;
    if (now < 0) return -1;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    o.phase = WAIT_DATA;
    o.grant_attempted = o.chunks_stored = RFS_GRANTS_MAX - 1;
    o.received_bytes = (RFS_GRANTS_MAX - 1) * FIRST_CHUNK;
    host_gate_override = -1;
    host_write_override = host_short_write;
    host_write_calls = 0;
    if (send_next_grant(&o) == 0 ||
        o.failure_stage != STAGE_FINAL_GRANT ||
        o.failure_reason != REASON_GATE ||
        o.grant_attempted != RFS_GRANTS_MAX - 1 || host_write_calls)
        goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    host_gate_override = 1;
    if (send_next_grant(&o) == 0 ||
        o.failure_stage != STAGE_FINAL_GRANT ||
        o.failure_reason != REASON_SEND ||
        o.grant_attempted != RFS_GRANTS_MAX || host_write_calls != 1)
        goto done;

    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    o.expected_chunk = 318;
    put_little32(frame + 4, 12 + 318);
    put_little32(frame + 12, 3);
    put_little32(frame + 16, 318);
    frame[0] = 2;
    frame[2] = 2;
    if (feed_rfs(&o, frame, 20 + 318, now) == 0 ||
        o.failure_stage != STAGE_FINAL_FRAME ||
        o.failure_reason != REASON_MALFORMED ||
        o.frame_mismatch_mask != MISMATCH_SEQUENCE) goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    o.frame_mismatch_mask = 0;
    o.used = 0;
    frame[2] = 1;
    if (feed_rfs(&o, frame, sizeof frame, now) == 0 ||
        o.failure_stage != STAGE_FINAL_FRAME ||
        o.failure_reason != REASON_TRAILING ||
        o.frame_mismatch_mask) goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    o.used = 0;
    diagnose_waiting(&o);
    if (o.failure_stage != STAGE_FINAL_FRAME ||
        o.failure_reason != REASON_NO_READ) goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    o.used = 7;
    diagnose_waiting(&o);
    if (o.failure_stage != STAGE_FINAL_FRAME ||
        o.failure_reason != REASON_PARTIAL_READ) goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    host_gate_override = -1;
    if (store_chunk(&o, frame + 20) == 0 ||
        o.failure_stage != STAGE_STORE_CHUNK ||
        o.failure_reason != REASON_GATE) goto done;
    o.failure_stage = STAGE_NONE;
    o.failure_reason = REASON_NONE;
    o.used = 0;
    host_gate_override = 1;
    o.candidate = -1;
    if (store_chunk(&o, frame + 20) == 0 ||
        o.failure_stage != STAGE_STORE_CHUNK ||
        o.failure_reason != REASON_IO) goto done;
    rc = 0;
done:
    host_gate_override = 0;
    host_write_override = NULL;
    host_write_calls = 0;
    zero_bytes(frame, sizeof frame);
    zero_bytes(&o, sizeof o);
    return rc;
}

static int test_host_final_frame_shape(void)
{
    struct owner o = {0};
    uint8_t frame[RFS_FRAME_MAX + 1] = {0};
    char line[512];
    FILE *log = tmpfile();
    int64_t now = monotonic_ms();
    int rc = -1;
    if (!log || now < 0) goto done;
    o.phase = WAIT_DATA;
    o.grant_attempted = RFS_GRANTS_MAX;
    o.chunks_stored = RFS_GRANTS_MAX - 1;
    o.expected_chunk = 318;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    frame[0] = 2;
    frame[2] = 1;
    put_little32(frame + 4, RFS_FRAME_MAX - 8);
    put_little32(frame + 12, 3);
    put_little32(frame + 16, 318);
    if (feed_rfs(&o, frame, RFS_FRAME_MAX, now) == 0 ||
        o.failure_stage != STAGE_FINAL_FRAME ||
        o.failure_reason != REASON_MALFORMED ||
        o.frame_mismatch_mask !=
            (MISMATCH_LENGTH | MISMATCH_PAYLOAD_SIZE) ||
        o.final_parsed_len != 2032 || o.final_outer_payload_len != 2024 ||
        o.final_trailing != 0 || o.final_padding_zero != PADDING_YES)
        goto done;
    log_terminal(log, &o, "rfs_frame_or_io_refused");
    if (fflush(log) || fseek(log, 0, SEEK_SET) ||
        !fgets(line, sizeof line, log) ||
        !strstr(line, "frame_mismatch_mask=0x09") ||
        !strstr(line, "final_parsed_len=2032 final_outer_payload_len=2024 "
                      "final_trailing=0 final_padding_zero=1"))
        goto done;

    memset(&o, 0, sizeof o);
    o.phase = WAIT_DATA;
    o.grant_attempted = RFS_GRANTS_MAX;
    o.chunks_stored = RFS_GRANTS_MAX - 1;
    o.expected_chunk = 318;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    frame[20 + 318] = 1;
    if (feed_rfs(&o, frame, sizeof frame, now) == 0 ||
        o.final_parsed_len != 2032 || o.final_outer_payload_len != 2024 ||
        o.final_trailing != 1 || o.final_padding_zero != PADDING_NO)
        goto done;

    memset(&o, 0, sizeof o);
    o.phase = WAIT_DATA;
    o.grant_attempted = RFS_GRANTS_MAX;
    o.chunks_stored = RFS_GRANTS_MAX - 1;
    o.expected_chunk = 318;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    put_little32(frame + 4, 12);
    if (feed_rfs(&o, frame, 20, now) == 0 ||
        o.final_parsed_len != 20 || o.final_outer_payload_len != 12 ||
        o.final_trailing != 0 || o.final_padding_zero != PADDING_UNKNOWN)
        goto done;
    rc = 0;
done:
    if (log) fclose(log);
    zero_bytes(frame, sizeof frame);
    zero_bytes(line, sizeof line);
    zero_bytes(&o, sizeof o);
    return rc;
}

static int host_reject_data_frame(const uint8_t *frame, size_t len,
                                  int final, int64_t now)
{
    struct owner o = {.phase = WAIT_DATA, .rfs = -1,
                      .deadline_ms = now + 10000,
                      .total_deadline_ms = now + 10000,
                      .grant_attempted = final ? RFS_GRANTS_MAX : 1,
                      .chunks_stored = final ? RFS_GRANTS_MAX - 1 : 0,
                      .received_bytes = final ? 94 * FIRST_CHUNK : 0,
                      .expected_chunk = final ? 318 : FIRST_CHUNK};
    sha_init(&o.received_hash);
    host_gate_override = 1;
    host_store_override = 1;
    host_finish_override = 1;
    host_write_override = host_full_write;
    host_write_calls = 0;
    int refused = feed_rfs(&o, frame, len, now);
    int valid = refused != 0 && o.phase == WAIT_DATA &&
                o.grant_attempted == (final ? RFS_GRANTS_MAX : 1) &&
                o.chunks_stored == (final ? RFS_GRANTS_MAX - 1 : 0) &&
                o.received_bytes == (final ? 94 * FIRST_CHUNK : 0) &&
                !o.final_ack_attempted && !o.final_ack_sent &&
                host_write_calls == 0;
    host_gate_override = 0;
    host_store_override = 0;
    host_finish_override = 0;
    host_write_override = NULL;
    host_write_calls = 0;
    zero_bytes(&o, sizeof o);
    return valid ? 0 : -1;
}

static int test_host_padded_final_refusals(void)
{
    uint8_t frame[RFS_FRAME_MAX + 2] = {0};
    int64_t now = monotonic_ms();
    int rc = -1;
    if (now < 0) return -1;
    frame[0] = 2;
    frame[2] = 1;
    put_little32(frame + 4, 332);
    put_little32(frame + 12, 3);
    put_little32(frame + 16, 318);
    frame[338] = 1;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[338] = 0;
    frame[339] = 1;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[339] = 0;
    put_little32(frame + 4, 331);
    if (host_reject_data_frame(frame, 340, 1, now) ||
        host_reject_data_frame(frame, 339, 1, now)) goto done;
    put_little32(frame + 4, 333);
    if (host_reject_data_frame(frame, 341, 1, now)) goto done;
    put_little32(frame + 4, 332);
    if (host_reject_data_frame(frame, 341, 1, now)) goto done;
    frame[0] = 3;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[0] = 2;
    frame[2] = 2;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[2] = 1;
    frame[8] = 1;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[8] = 0;
    frame[12] = 4;
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    frame[12] = 3;
    put_little32(frame + 16, 319);
    if (host_reject_data_frame(frame, 340, 1, now)) goto done;
    put_little32(frame + 16, FIRST_CHUNK);
    put_little32(frame + 4, 12 + FIRST_CHUNK + 2);
    if (host_reject_data_frame(frame, RFS_FRAME_MAX + 2, 0, now)) goto done;
    rc = 0;
done:
    zero_bytes(frame, sizeof frame);
    return rc;
}

/* Test the real finish/sidecar durability path with only device gates mocked. */
static int test_host_finish_candidate(int fault, int padded_final)
{
    char parent_path[] = "/tmp/saaios-rfs-full-test-XXXXXX";
    static const char leaf[] = "run";
    uint8_t zero[4096] = {0}, chunk[FIRST_CHUNK], digest[32], again[32];
    uint8_t frame[20 + 318 + 2] = {0};
    struct sha256 source_hash;
    struct owner o = {.source = -1, .candidate = -1, .sidecar_fd = -1,
                      .quarantine_parent = -1, .quarantine_dir = -1};
    FILE *source = NULL;
    int parent_created = 0, child_created = 0, rc = -1;
    if (!mkdtemp(parent_path)) goto done;
    parent_created = 1;
    o.quarantine_parent = open(parent_path,
                                O_RDONLY | O_DIRECTORY | O_NOFOLLOW |
                                O_CLOEXEC);
    if (o.quarantine_parent < 0 ||
        mkdirat(o.quarantine_parent, leaf, 0700)) goto done;
    child_created = 1;
    o.quarantine_dir = openat(o.quarantine_parent, leaf,
                              O_RDONLY | O_DIRECTORY | O_NOFOLLOW |
                              O_CLOEXEC);
    if (o.quarantine_dir < 0) goto done;
    memcpy(o.quarantine_leaf, leaf, sizeof leaf);
    o.candidate = openat(o.quarantine_dir, CANDIDATE_NAME,
                         O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                         0600);
    source = tmpfile();
    if (o.candidate < 0 || !source) goto done;
    o.source = fileno(source);
    if (o.source < 0) goto done;
    sha_init(&source_hash);
    for (off_t off = 0; off < BASELINE_BYTES; off += sizeof zero) {
        if (write_all_at(o.source, zero, sizeof zero, off) ||
            write_all_at(o.candidate, zero, sizeof zero, off)) goto done;
        sha_update(&source_hash, zero, sizeof zero);
    }
    sha_final(&source_hash, o.pin_digest);
    sha_init(&o.received_hash);
    for (unsigned i = 0; i < RFS_GRANTS_MAX; ++i) {
        uint32_t length = i == RFS_GRANTS_MAX - 1 ? 318 : FIRST_CHUNK;
        uint32_t offset = i * FIRST_CHUNK;
        memset(chunk, (int)(i + 1), length);
        if (write_all_at(o.candidate, chunk, length, offset)) goto done;
        sha_update(&o.received_hash, chunk, length);
    }
    o.received_bytes = RFS_TRANSFER_BYTES;
    o.chunks_stored = o.grant_attempted = RFS_GRANTS_MAX;
    int64_t now = monotonic_ms();
    if (now < 0 || fsync(o.candidate) ||
        fstat(o.candidate, &o.candidate_stat) ||
        fstat(o.quarantine_parent, &o.quarantine_parent_stat) ||
        fstat(o.quarantine_dir, &o.quarantine_dir_stat)) goto done;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    host_gate_override = 1; /* Non-root test fixture only. */
    if (fault) {
        sha_init(&o.received_hash);
        for (unsigned i = 0; i < RFS_GRANTS_MAX - 1; ++i) {
            memset(chunk, (int)(i + 1), FIRST_CHUNK);
            sha_update(&o.received_hash, chunk, FIRST_CHUNK);
        }
        o.received_bytes = 94 * FIRST_CHUNK;
        o.chunks_stored = RFS_GRANTS_MAX - 1;
        o.grant_attempted = RFS_GRANTS_MAX;
        o.expected_chunk = 318;
        o.phase = WAIT_DATA;
        host_store_override = 1;
        host_write_override = host_short_write;
        host_write_calls = 0;
        host_sidecar_close_error = fault == 1;
        host_sidecar_sync_error = fault == 2;
        memset(frame, RFS_GRANTS_MAX, sizeof frame);
        frame[0] = 2; frame[1] = 0;
        frame[2] = 1; frame[3] = 0;
        put_little32(frame + 4, 12 + 318);
        put_little32(frame + 8, 0);
        put_little32(frame + 12, 3);
        put_little32(frame + 16, 318);
        size_t frame_len = 20 + 318;
        if (padded_final) {
            frame_len += 2;
            put_little32(frame + 4, 332);
            frame[338] = frame[339] = 0;
        }
        struct stat sidecar;
        if (feed_rfs(&o, frame, frame_len, now) == 0 ||
            o.chunks_stored != RFS_GRANTS_MAX ||
            o.received_bytes != RFS_TRANSFER_BYTES ||
            o.final_ack_attempted || o.final_ack_sent ||
            host_write_calls != 0 ||
            fstatat(o.quarantine_dir, SIDECAR_NAME, &sidecar,
                    AT_SYMLINK_NOFOLLOW))
            goto done;
        if (fault == 1 && o.sidecar_fd != -1) goto done;
        rc = 0;
        goto done;
    }
    int finish_result = finish_candidate(&o);
    if (finish_result || sidecar_stable(&o) ||
        verify_full_candidate(&o, digest) ||
        !same_bytes(digest, o.candidate_digest, sizeof digest) ||
        read_all_at(o.sidecar_fd, again, sizeof again, 0) ||
        !same_bytes(digest, again, sizeof digest))
        goto done;
    /* Corrupting the unchanged tail must make a second content gate fail. */
    chunk[0] = 0x7f;
    if (write_all_at(o.candidate, chunk, 1, RFS_TRANSFER_BYTES) ||
        fsync(o.candidate) ||
        verify_full_candidate(&o, again) == 0)
        goto done;
    rc = 0;
done:
    host_gate_override = 0;
    host_store_override = 0;
    host_sidecar_close_error = 0;
    host_sidecar_sync_error = 0;
    host_write_override = NULL;
    host_write_calls = 0;
    close_if_open(o.sidecar_fd);
    close_if_open(o.candidate);
    if (source) fclose(source);
    if (o.quarantine_dir >= 0) {
        (void)unlinkat(o.quarantine_dir, SIDECAR_NAME, 0);
        (void)unlinkat(o.quarantine_dir, CANDIDATE_NAME, 0);
    }
    close_if_open(o.quarantine_dir);
    if (o.quarantine_parent >= 0 && child_created)
        (void)unlinkat(o.quarantine_parent, leaf, AT_REMOVEDIR);
    close_if_open(o.quarantine_parent);
    if (parent_created) (void)rmdir(parent_path);
    zero_bytes(zero, sizeof zero);
    zero_bytes(chunk, sizeof chunk);
    zero_bytes(digest, sizeof digest);
    zero_bytes(again, sizeof again);
    zero_bytes(frame, sizeof frame);
    zero_bytes(&o, sizeof o);
    return rc;
}

static int test_host_transcript(int padded_final)
{
    struct owner o = {.rfs = -1, .phase = WAIT_6};
    uint8_t frame[RFS_FRAME_MAX] = {0};
    int rc = -1;
    int64_t now = monotonic_ms();
    if (now < 0) return -1;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    sha_init(&o.received_hash);
    host_gate_override = 1;
    host_write_override = host_short_write;
    host_write_calls = 0;
    if (complete_frame(&o, request_6, sizeof request_6, 0, now) == 0 ||
        host_write_calls != 1 || o.grant_attempted != 1)
        goto done;
    memset(&o, 0, sizeof o);
    o.rfs = -1; o.phase = WAIT_6;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    stop_requested = 1;
    host_write_calls = 0;
    if (complete_frame(&o, request_6, sizeof request_6, 0, now) == 0 ||
        host_write_calls != 0 || o.grant_attempted != 0)
        goto done;
    stop_requested = 0;
    memset(&o, 0, sizeof o);
    o.rfs = -1; o.phase = WAIT_7;
    o.deadline_ms = o.total_deadline_ms = now + 10000;
    sha_init(&o.received_hash);
    host_store_override = 1;
    host_finish_override = 1;
    host_write_override = host_full_write;
    host_write_calls = 0;
    if (feed_rfs(&o, request_7, sizeof request_7, now) ||
        o.phase != WAIT_3 || host_write_calls != 1 ||
        host_last_reply_len != sizeof status_7 ||
        !same_bytes(host_last_reply, status_7, sizeof status_7) ||
        feed_rfs(&o, request_3, sizeof request_3, now) ||
        o.phase != WAIT_6 ||
        feed_rfs(&o, request_6, sizeof request_6, now) ||
        o.phase != WAIT_DATA || host_write_calls != 2)
        goto done;
    for (unsigned i = 0; i < RFS_GRANTS_MAX; ++i) {
        uint32_t expected_offset = i * FIRST_CHUNK;
        uint32_t expected_length = i == RFS_GRANTS_MAX - 1 ?
                                   318 : FIRST_CHUNK;
        if (o.received_bytes != expected_offset ||
            o.expected_chunk != expected_length ||
            host_last_reply_len != 20 ||
            little16(host_last_reply) != 2 ||
            little16(host_last_reply + 2) != 1 ||
            little32(host_last_reply + 12) != expected_offset ||
            little32(host_last_reply + 16) != expected_length)
            goto done;
        memset(frame, (int)(i & 0xff), sizeof frame);
        frame[0] = 2; frame[1] = 0;
        frame[2] = 1; frame[3] = 0;
        put_little32(frame + 4, 12 + expected_length);
        put_little32(frame + 8, 0);
        put_little32(frame + 12, 3);
        put_little32(frame + 16, expected_length);
        size_t frame_len = 20 + expected_length;
        if (padded_final && i == RFS_GRANTS_MAX - 1) {
            frame_len += 2;
            put_little32(frame + 4, 332);
            frame[338] = frame[339] = 0;
        }
        if (feed_rfs(&o, frame, frame_len, monotonic_ms()))
            goto done;
        if (i == 19) {
            uint8_t sim[15 + SIT_SIM_APP_STRIDE];
            unsigned grants = (unsigned)o.grant_attempted;
            o.sit.pending = 1;
            o.sit.token = 0x15302026u;
            host_sit_reply(sim, sizeof sim, 0x0200, o.sit.token, 0);
            sim[SIT_SIM_CARD] = 1;
            sim[SIT_SIM_APPS] = 1;
            sit_feed(&o.sit, sim, 7, now);
            sit_feed(&o.sit, sim + 7, sizeof sim - 7, now);
            zero_bytes(sim, sizeof sim);
            if (o.sit.next != 1 || o.sit.pending ||
                o.grant_attempted != (int)grants || o.phase != WAIT_DATA)
                goto done;
        }
    }
    if (o.phase != TERMINAL || !o.final_ack_sent ||
        !o.final_ack_attempted || o.grant_attempted != RFS_GRANTS_MAX ||
        o.chunks_stored != RFS_GRANTS_MAX ||
        o.received_bytes != RFS_TRANSFER_BYTES ||
        host_write_calls != 97 ||
        host_last_reply_len != sizeof final_status ||
        !same_bytes(host_last_reply, final_status, sizeof final_status) ||
        feed_rfs(&o, frame, 20 + 318, monotonic_ms()) == 0)
        goto done;
    rc = 0;
done:
    host_gate_override = 0;
    host_store_override = 0;
    host_finish_override = 0;
    host_write_override = NULL;
    host_write_calls = 0;
    host_last_reply_len = 0;
    stop_requested = 0;
    zero_bytes(host_last_reply, sizeof host_last_reply);
    zero_bytes(frame, sizeof frame);
    zero_bytes(&o, sizeof o);
    return rc;
}
#endif

#ifdef SAAIOS_RFS_CAMP
static void camp_build_ind(uint8_t *buf, unsigned id, size_t len, uint32_t raw)
{
    memset(buf, 0, len);
    buf[0] = 2;
    buf[2] = (uint8_t)id;
    buf[3] = (uint8_t)(id >> 8);
    buf[4] = (uint8_t)len;
    buf[5] = (uint8_t)(len >> 8);
    if (id == 0x0802 && len >= 12) put_little32(buf + 8, raw);
}

static int test_camp_builders(void)
{
    uint8_t cfg[SEQ_CONFIG_LEN];
    static const uint8_t cfg_e[SEQ_CONFIG_LEN] =
        {0,0,0x3f,0x09,13,0,0x78,0x56,0x34,0x12,0,0,0};
    make_setmodemsconfig_request(cfg, 0x12345678);
    if (!same_bytes(cfg, cfg_e, sizeof cfg)) return 101;
    uint8_t sgc[SGC_LEN];
    static const uint8_t sgc_e[SGC_LEN] =
        {0,0,0x04,0x04,24,0,0x78,0x56,0x34,0x12,0,0,
         0x01,0x01,0,0,0,0,0,0,0,0,0,0};
    make_sgc_request(sgc, 0x12345678);
    if (!same_bytes(sgc, sgc_e, sizeof sgc)) return 102;
    uint8_t power[CAMP_POWER_LEN];
    static const uint8_t power_e[CAMP_POWER_LEN] =
        {0,0,0x00,0x08,18,0,0x78,0x56,0x34,0x12,0,0,2,0,0,0,0,0};
    make_radiopower_request(power, 0x12345678);
    if (!same_bytes(power, power_e, sizeof power)) return 103;
    return 0;
}

static int test_camp_observer(void)
{
    struct camp_driver c;
    uint8_t e3[8], e2[12], e2n[12];
    camp_build_ind(e3, 0x0803, 8, 0);
    camp_build_ind(e2, 0x0802, 12, 0);
    camp_build_ind(e2n, 0x0802, 12, 2);
    /* A clean 0x0803 -> 0x0802 raw 0 pair arms the dispatcher. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e3, 8, 100);
    camp_feed(&c, e2, 12, 200);
    if (!c.radio_ready0_ms || c.radio_invalidated) return 104;
    /* Reversed order invalidates. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e2, 12, 100);
    if (!c.radio_invalidated || c.radio_ready0_ms) return 105;
    /* Duplicate 0x0803 invalidates. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e3, 8, 100);
    camp_feed(&c, e3, 8, 150);
    if (!c.radio_invalidated || c.radio_ready0_ms) return 106;
    /* Non-zero scalar after 0x0803 invalidates. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e3, 8, 100);
    camp_feed(&c, e2n, 12, 200);
    if (!c.radio_invalidated || c.radio_ready0_ms) return 107;
    /* Second event beyond the inter-event gap invalidates. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e3, 8, 100);
    camp_feed(&c, e2, 12, 100 + CAMP_PAIR_GAP_MS + 1);
    if (!c.radio_invalidated || c.radio_ready0_ms) return 108;
    /* A superseding radio event after a settled pair invalidates. */
    memset(&c, 0, sizeof c);
    camp_feed(&c, e3, 8, 100);
    camp_feed(&c, e2, 12, 200);
    camp_feed(&c, e3, 8, 300);
    if (!c.radio_invalidated) return 109;
    return 0;
}

#ifdef RFS_HOST_TEST
static uint8_t camp_cap[3][32];
static size_t camp_cap_len[3];
static unsigned camp_cap_n;
static ssize_t camp_capture_write(int fd, const void *buf, size_t len)
{
    (void)fd;
    if (camp_cap_n < 3 && len <= sizeof camp_cap[0]) {
        memcpy(camp_cap[camp_cap_n], buf, len);
        camp_cap_len[camp_cap_n] = len;
    }
    camp_cap_n++;
    return (ssize_t)len;
}

static int test_camp_dispatch(void)
{
    struct owner o;
    memset(&o, 0, sizeof o);
    o.phase = WAIT_7;
    o.ipc = 5;
    o.camp.owner_start_ms = 0;
    o.camp.radio_ready0_ms = 1000;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_advance(&o, 1000);
    host_sit_write_override = NULL;
    if (!o.camp.dispatched || camp_cap_n != 3) return 110;
    if (camp_cap_len[0] != SEQ_CONFIG_LEN ||
        little16(camp_cap[0] + 2) != SEQ_CONFIG_COMMAND) return 111;
    if (camp_cap_len[1] != SGC_LEN ||
        little16(camp_cap[1] + 2) != SGC_COMMAND) return 112;
    if (camp_cap_len[2] != CAMP_POWER_LEN ||
        little16(camp_cap[2] + 2) != CAMP_POWER_COMMAND) return 113;
    /* A second advance must not re-dispatch. */
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_advance(&o, 1000);
    host_sit_write_override = NULL;
    if (camp_cap_n != 0) return 114;
    /* The matching ACK records success without re-sending. */
    uint8_t ack[12];
    memset(ack, 0, sizeof ack);
    ack[0] = 1;
    ack[2] = (uint8_t)CAMP_POWER_COMMAND;
    ack[3] = (uint8_t)(CAMP_POWER_COMMAND >> 8);
    ack[4] = 12;
    put_little32(ack + 6, o.camp.camp_token);
    camp_feed(&o.camp, ack, sizeof ack, 1100);
    if (!o.camp.camp_acked || o.camp.camp_error) return 115;
    /* An armed pair that misses the dispatch window invalidates, no send. */
    struct owner o2;
    memset(&o2, 0, sizeof o2);
    o2.phase = WAIT_7;
    o2.ipc = 5;
    o2.camp.radio_ready0_ms = 1000;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_advance(&o2, 1000 + CAMP_DISPATCH_MS + 1);
    host_sit_write_override = NULL;
    if (o2.camp.dispatched || camp_cap_n != 0 ||
        !o2.camp.radio_invalidated) return 116;
    /* A successful RFS terminal (final ACK sent) still permits camp dispatch;
     * a failed terminal (no final ACK) blocks it. */
    struct owner o3;
    memset(&o3, 0, sizeof o3);
    o3.phase = TERMINAL;
    o3.final_ack_sent = 1;
    o3.ipc = 5;
    o3.camp.radio_ready0_ms = 1000;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_advance(&o3, 1000);
    host_sit_write_override = NULL;
    if (!o3.camp.dispatched || camp_cap_n != 3) return 117;
    struct owner o4;
    memset(&o4, 0, sizeof o4);
    o4.phase = TERMINAL;
    o4.final_ack_sent = 0;
    o4.ipc = 5;
    o4.camp.radio_ready0_ms = 1000;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_advance(&o4, 1000);
    host_sit_write_override = NULL;
    if (o4.camp.dispatched || camp_cap_n != 0) return 118;
    return 0;
}

static int test_camp_prober(void)
{
    /* A signal reply shorter than the V4 minimum yields no mask. */
    uint8_t sig_short[PROBE_SIGNAL_MIN - 1];
    uint8_t sig[PROBE_SIGNAL_MIN];
    uint32_t mask = 0xffffffffu;
    host_sit_reply(sig_short, sizeof sig_short, 0x0900, 0x55u, 0);
    if (camp_signal_mask(sig_short, sizeof sig_short, 0x55u, &mask)) return 120;
    host_sit_reply(sig, sizeof sig, 0x0900, 0x55u, 0);
    sig[12] = 0x45; /* low-seven presence bits 0x45; high bits ignored */
    sig[13] = 0x80;
    if (!camp_signal_mask(sig, sizeof sig, 0x55u, &mask) || mask != 0x45u)
        return 121;
    if (camp_signal_mask(sig, sizeof sig, 0x56u, &mask)) return 122; /* token */
    host_sit_reply(sig, sizeof sig, 0x0900, 0x55u, 6);
    if (camp_signal_mask(sig, sizeof sig, 0x55u, &mask)) return 123;  /* error */

    /* The prober sends a GET once started and clears on the matching reply. */
    struct owner o;
    memset(&o, 0, sizeof o);
    o.phase = TERMINAL;
    o.final_ack_sent = 1;
    o.ipc = 5;
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, PROBE_START_MS - 1); /* too early */
    if (camp_cap_n != 0 || o.camp.probe_pending) { host_sit_write_override = NULL; return 124; }
    camp_probe_advance(&o, PROBE_START_MS);
    host_sit_write_override = NULL;
    if (camp_cap_n != 1 || !o.camp.probe_pending ||
        o.camp.probe_id != 0x0200 || camp_cap_len[0] != 12) return 125;

    /* A SIM reply with app_state READY(5) marks the SIM ready, no poison. */
    uint8_t sim[15 + SIT_SIM_APP_STRIDE];
    host_sit_reply(sim, sizeof sim, 0x0200, o.camp.probe_token, 0);
    sim[SIT_SIM_CARD] = 1;
    sim[SIT_SIM_APPS] = 1;
    sim[SIT_SIM_APP_STATE] = 5;
    sim[SIT_SIM_PIN1] = 3;
    camp_feed(&o.camp, sim, sizeof sim, PROBE_START_MS + 10);
    if (o.camp.probe_pending || !o.camp.sim_ready || o.camp.poisoned ||
        o.camp.probe_replies != 1) return 126;

    /* A reply timeout backs the prober off without poisoning. */
    camp_cap_n = 0;
    host_sit_write_override = camp_capture_write;
    camp_probe_advance(&o, PROBE_START_MS + 2000);     /* next GET */
    if (camp_cap_n != 1 || !o.camp.probe_pending) { host_sit_write_override = NULL; return 127; }
    camp_probe_advance(&o, PROBE_START_MS + 2000 + PROBE_REPLY_MS + 1);
    host_sit_write_override = NULL;
    if (o.camp.probe_pending || o.camp.poisoned ||
        o.camp.probe_timeouts != 1) return 128;

    /* A 0x0210 indication re-arms a SIM GET only while the SIM is not ready. */
    struct camp_driver fresh;
    memset(&fresh, 0, sizeof fresh);
    uint8_t ind[8] = {2, 0, 0x10, 0x02, 8, 0, 0, 0};
    camp_feed(&fresh, ind, sizeof ind, 100);
    if (!fresh.sim_change_pending) return 129;
    fresh.sim_ready = 1;
    fresh.sim_change_pending = 0;
    camp_feed(&fresh, ind, sizeof ind, 200);
    if (fresh.sim_change_pending) return 130;
    return 0;
}
#endif

static int test_camp_reg(void)
{
    /* Builders: exact factory byte shapes, no invented bytes. */
    uint8_t pref[REG_PREF_LEN];
    static const uint8_t pref_e[REG_PREF_LEN] =
        {0,0,0x0a,0x07,16,0,0x78,0x56,0x34,0x12,0,0,12,0,0,0};
    make_setpref_request(pref, RAT_LTE_WCDMA, 0x12345678);
    if (!same_bytes(pref, pref_e, sizeof pref)) return 131;
    uint8_t allow[REG_ALLOW_LEN];
    static const uint8_t allow_e[REG_ALLOW_LEN] =
        {0,0,0x10,0x07,13,0,0x78,0x56,0x34,0x12,0,0,1};
    make_allowdata_request(allow, 0x12345678);
    if (!same_bytes(allow, allow_e, sizeof allow)) return 132;

    /* Step machine: radio -> selection (read, bounded) -> auto SET ->
     * preferred (read, bounded) -> broaden SET -> allow_data -> done. */
    struct camp_driver c;
    memset(&c, 0, sizeof c);
    if (camp_reg_next(&c) != 0) return 133;          /* SIM not ready yet */
    c.sim_ready = 1;
    if (camp_reg_next(&c) != REG_RADIO_GET) return 134;
    c.radio_on = 1;
    if (camp_reg_next(&c) != REG_SEL_GET) return 135;
    c.sel_known = 1; c.sel_mode = 1;                 /* manual -> force auto */
    if (camp_reg_next(&c) != REG_SEL_AUTO_SET) return 136;
    c.sel_auto_sent = 1;
    if (camp_reg_next(&c) != REG_PREF_GET) return 137;
    c.pref_known = 1; c.preferred_raw = RAT_LTE_ONLY;
    if (camp_reg_next(&c) != REG_PREF_SET) return 138;
    c.pref_set_sent = 1;
    if (camp_reg_next(&c) != REG_ALLOW_DATA) return 139;
    c.allow_data_sent = 1;
    if (camp_reg_next(&c) != 0) return 140;

    /* With an APN configured, SET_INITIAL_ATTACH_APN precedes allow_data. */
    struct camp_driver iac;
    memset(&iac, 0, sizeof iac);
    iac.sim_ready = 1; iac.radio_on = 1;
    iac.sel_auto_sent = 1; iac.pref_set_sent = 1;
    memcpy(iac.apn, "internet", 9);
    if (camp_reg_next(&iac) != REG_INIT_ATTACH_APN) return 170;
    iac.ia_apn_sent = 1;
    if (camp_reg_next(&iac) != REG_ALLOW_DATA) return 171;
    iac.allow_data_sent = 1;
    if (camp_reg_next(&iac) != 0) return 172;

    /* Already-auto selection skips 0x0704; already-LTE_WCDMA preferred skips
     * 0x070a; any other preferred value (e.g. raw 16) is broadened. */
    struct camp_driver d;
    memset(&d, 0, sizeof d);
    d.sim_ready = 1; d.radio_on = 1; d.sel_known = 1; d.sel_mode = 0;
    if (camp_reg_next(&d) != REG_PREF_GET) return 141;
    d.pref_known = 1; d.preferred_raw = RAT_LTE_WCDMA;
    if (camp_reg_next(&d) != REG_ALLOW_DATA) return 142;
    d.allow_data_sent = 1;
    if (camp_reg_next(&d) != 0) return 143;

    struct camp_driver d16;
    memset(&d16, 0, sizeof d16);
    d16.sim_ready = 1; d16.radio_on = 1; d16.sel_auto_sent = 1;
    d16.pref_known = 1; d16.preferred_raw = 16;      /* observed settled value */
    if (camp_reg_next(&d16) != REG_PREF_SET) return 144;

    /* Bounded GET retries: unanswered 0x0703/0x070b still advance to the SETs. */
    struct camp_driver e;
    memset(&e, 0, sizeof e);
    e.sim_ready = 1; e.radio_on = 1;
    e.sel_get_tries = REG_GET_MAX;                   /* selection never answered */
    if (camp_reg_next(&e) != REG_SEL_AUTO_SET) return 145;
    e.sel_auto_sent = 1;
    e.pref_get_tries = REG_GET_MAX;                  /* preferred never answered */
    if (camp_reg_next(&e) != REG_PREF_SET) return 146;
    e.pref_set_sent = 1;
    if (camp_reg_next(&e) != REG_ALLOW_DATA) return 147;

    /* Unanswered radio GET gives up rather than looping forever. */
    struct camp_driver f;
    memset(&f, 0, sizeof f);
    f.sim_ready = 1; f.radio_get_tries = REG_GET_MAX;
    if (camp_reg_next(&f) != 0) return 148;

    /* reg_complete gate: ACK of allow_data stops all further steps. */
    struct camp_driver g;
    memset(&g, 0, sizeof g);
    g.sim_ready = 1; g.reg_complete = 1;
    if (camp_reg_next(&g) != 0) return 149;

    /* OPX experiment: after reg_complete the three GETs are read, then one SET
     * per the selected step. GET-only when no step is selected. */
    if (camp_opx_next(&g) != OPX_STACK_GET) return 150;
    g.opx_stack_get_sent = 1;
    if (camp_opx_next(&g) != OPX_VOICE_GET) return 151;
    g.opx_voice_get_sent = 1;
    if (camp_opx_next(&g) != OPX_DEVSVC_GET) return 152;
    g.opx_devsvc_get_sent = 1;
    if (camp_opx_next(&g) != 0) return 153;           /* no SET step selected */
    g.opx_step = OPX_STEP_VOICE;
    if (camp_opx_next(&g) != OPX_VOICE_SET) return 154;
    g.opx_set_sent = 1; g.opx_done = 1;
    if (camp_opx_next(&g) != 0) return 155;           /* done after one SET */

    struct camp_driver h;
    memset(&h, 0, sizeof h);
    h.sim_ready = 1; h.reg_complete = 1; h.opx_step = OPX_STEP_INTPS;
    h.opx_stack_get_sent = h.opx_voice_get_sent = h.opx_devsvc_get_sent = 1;
    if (camp_opx_next(&h) != OPX_INTPS_SET) return 156;
    struct camp_driver k;
    memset(&k, 0, sizeof k);
    k.sim_ready = 1; k.reg_complete = 1; k.opx_step = OPX_STEP_STACK;
    k.opx_stack_get_sent = k.opx_voice_get_sent = k.opx_devsvc_get_sent = 1;
    if (camp_opx_next(&k) != OPX_STACK_SET) return 157;
    struct camp_driver dv;
    memset(&dv, 0, sizeof dv);
    dv.sim_ready = 1; dv.reg_complete = 1; dv.opx_step = OPX_STEP_DEVSVC;
    dv.opx_stack_get_sent = dv.opx_voice_get_sent = dv.opx_devsvc_get_sent = 1;
    if (camp_opx_next(&dv) != OPX_DEVSVC_SET) return 161;
    struct camp_driver du;
    memset(&du, 0, sizeof du);
    du.sim_ready = 1; du.reg_complete = 1; du.opx_step = OPX_STEP_DUAL;
    du.opx_stack_get_sent = du.opx_voice_get_sent = du.opx_devsvc_get_sent = 1;
    if (camp_opx_next(&du) != OPX_DUAL_SET) return 163;

    /* Frame byte-exactness: id@+2, len@+4, token@+6, body@+12. */
    uint8_t vs[OPX_VOICE_LEN];
    static const uint8_t vs_e[OPX_VOICE_LEN] =
        {0,0,0x1a,0x09,16,0,0x78,0x56,0x34,0x12,0,0,3,0,0,0};
    make_opx_u32_request(vs, OPX_VOICE_SET, OPX_VOICE_LEN, 0x12345678,
                         OPX_VOICE_MODE);
    if (!same_bytes(vs, vs_e, sizeof vs)) return 158;
    uint8_t is[OPX_INTPS_LEN];
    static const uint8_t is_e[OPX_INTPS_LEN] =
        {0,0,0x33,0x09,16,0,0x78,0x56,0x34,0x12,0,0,1,0,0,0};
    make_opx_u32_request(is, OPX_INTPS_SET, OPX_INTPS_LEN, 0x12345678,
                         OPX_INTPS_MODE);
    if (!same_bytes(is, is_e, sizeof is)) return 159;
    uint8_t ss[OPX_STACK_LEN];
    static const uint8_t ss_e[OPX_STACK_LEN] =
        {0,0,0x0f,0x08,13,0,0x78,0x56,0x34,0x12,0,0,1};
    make_opx_byte_request(ss, OPX_STACK_SET, OPX_STACK_LEN, 0x12345678,
                          OPX_STACK_MODE_ENABLE);
    if (!same_bytes(ss, ss_e, sizeof ss)) return 160;
    uint8_t ds[OPX_DEVSVC_LEN];
    static const uint8_t ds_e[OPX_DEVSVC_LEN] =
        {0,0,0x56,0x09,16,0,0x78,0x56,0x34,0x12,0,0,2,0,0,0};
    make_opx_u32_request(ds, OPX_DEVSVC_SET, OPX_DEVSVC_LEN, 0x12345678,
                         OPX_DEVSVC_MODE_DATA);
    if (!same_bytes(ds, ds_e, sizeof ds)) return 162;
    uint8_t du2[OPX_DUAL_LEN];
    static const uint8_t du_e[OPX_DUAL_LEN] =
        {0,0,0x2b,0x07,28,0,0x78,0x56,0x34,0x12,0,0,
         12,0,0,0, 12,0,0,0, 1,0,0,0, 1,0,0,0};
    make_opx_dual_request(du2, 0x12345678);
    if (!same_bytes(du2, du_e, sizeof du2)) return 164;
    /* SET_INITIAL_ATTACH_APN (0x0603): header + recovered body bytes for a plain
     * IP APN "internet" (no user/pass/auth). */
    uint8_t ia[REG_IA_LEN];
    make_initial_attach_apn_request(ia, "internet", 0x12345678);
    if (ia[0] || ia[1] || ia[2] != 0x03 || ia[3] != 0x06 ||
        ia[4] != 250 || ia[5]) return 165;
    if (little32(ia + 6) != 0x12345678 || ia[10] || ia[11]) return 166;
    if (ia[12] != REG_IA_CID || ia[13] != REG_IA_CONST13 ||
        ia[14] || ia[15]) return 167;
    if (memcmp(ia + REG_IA_APN_OFF, "internet", 9)) return 168; /* incl NUL */
    if (ia[REG_IA_AUTH_OFF] || ia[REG_IA_PDPTYPE_OFF] != REG_IA_PDPTYPE_IP ||
        ia[REG_IA_PCSCF_OFF] || ia[117] || ia[167]) return 169;
    return 0;
}
#endif

static int self_test(void)
{
    static const uint8_t abc[] = {'a', 'b', 'c'};
    static const uint8_t abc_sha256[32] = {
        0xba,0x78,0x16,0xbf,0x8f,0x01,0xcf,0xea,
        0x41,0x41,0x40,0xde,0x5d,0xae,0x22,0x23,
        0xb0,0x03,0x61,0xa3,0x96,0x17,0x7a,0x9c,
        0xb4,0x10,0xff,0x61,0xf2,0x00,0x15,0xad
    };
    struct sha256 hash;
    struct owner o = {.phase = WAIT_7};
    uint8_t digest[32], data[RFS_FRAME_MAX] = {0};
    sha_init(&hash);
    sha_update(&hash, abc, sizeof abc);
    sha_final(&hash, digest);
    if (!same_bytes(digest, abc_sha256, sizeof digest)) return 11;
    if (classify(&o, request_7, sizeof request_7) != STATUS_7) return 12;
    o.phase = WAIT_3;
    if (classify(&o, request_3, sizeof request_3) != NO_REPLY)
        return 2;
    o.phase = WAIT_6;
    if (classify(&o, request_6, sizeof request_6) != GRANT_1 ||
        RFS_TRANSFER_BYTES != 94 * FIRST_CHUNK + 318 ||
        test_framing())
        return 3;
    o.phase = WAIT_DATA;
    o.grant_attempted = RFS_GRANTS_MAX;
    o.chunks_stored = RFS_GRANTS_MAX - 1;
    o.received_bytes = 94 * FIRST_CHUNK;
    o.expected_chunk = 318;
    data[0] = 2; data[2] = 1;
    put_little32(data + 4, 12 + 318);
    put_little32(data + 12, 3);
    put_little32(data + 16, 318);
    if (valid_frame_length(data, 338) != 338 ||
        classify(&o, data, 338) != STORE_CHUNK)
        return 4;
    data[2] = 2;
    if (classify(&o, data, 338) != BAD_FRAME)
        return 5;
    data[2] = 1;
    put_little32(data + 16, 319);
    if (classify(&o, data, 338) != BAD_FRAME)
        return 6;
    o.phase = TERMINAL;
    if (classify(&o, data, 338) != BAD_FRAME)
        return 7;
#ifdef RFS_HOST_TEST
    if (test_host_storage_faults() || test_host_finish_candidate(0, 0) ||
        test_host_finish_candidate(1, 0) ||
        test_host_finish_candidate(2, 0) ||
        test_host_finish_candidate(1, 1) ||
        test_host_finish_candidate(2, 1) ||
        test_host_transcript(0) || test_host_transcript(1) ||
        test_sit_observer() || test_sit_event_trace() ||
        test_host_failure_diagnostics() || test_host_final_frame_shape() ||
        test_host_padded_final_refusals())
        return 8;
#endif
#ifdef SAAIOS_RFS_CAMP
    if (test_camp_builders() || test_camp_observer() || test_camp_reg()
#ifdef RFS_HOST_TEST
        || test_camp_dispatch() || test_camp_prober()
#endif
       )
        return 9;
#endif
    zero_bytes(data, sizeof data);
    zero_bytes(digest, sizeof digest);
#ifdef SAAIOS_RFS_CAMP
    puts("PASS combined RFS quarantine + camp dispatch self-test");
#else
    puts("PASS full-RFS quarantine, SIT observer, and SHA-256 self-test");
#endif
    return 0;
}

int main(int argc, char **argv)
{
    if (argc == 2 && (!strcmp(argv[1], "--mode") ||
                      !strcmp(argv[1], "--mode=rfs-full-quarantine"))) {
#ifdef SAAIOS_RFS_CAMP
        puts("rfs-camp-combined");
#else
        puts("rfs-full-quarantine");
#endif
        return 0;
    }
    if (argc == 2 && !strcmp(argv[1], "self-test"))
        return self_test();
    int ipc, rfs, ready;
    if (parse_args(argc, argv, &ipc, &rfs, &ready)) {
        fputs("usage: modem-rfs-full-quarantine-owner self-test | --mode | "
              "--ipc-fd N --rfs-fd N --ready-fd N\n", stderr);
        return 64;
    }
    return run_owner(ipc, rfs, ready);
}
