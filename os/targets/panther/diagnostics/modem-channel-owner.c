/*
 * Opt-in diagnostic owner for a CP boot-probe handoff, not a telephony service.
 * The probe opens IPC0/RFS0 before FIN, forks/execs this process with those
 * descriptors, and must wait for READY\n before sending FIN or COMPLETE.
 *
 * After ONLINE it sends four allowlisted, read-only SIT status GETs once.
 * SIM-status-change indications may then schedule up to three debounced,
 * read-only SIM status refreshes through the same IPC reader. One separate
 * settled pass of four read-only GETs is eligible after 60 seconds. Once
 * that pass finishes, four more factory read-only network GETs run once.
 * A separate, explicit guarded-boot opt-in may send one active RF network
 * scan after fresh same-boot status gates, with one bounded cancel on timeout.
 * The default remains passive. It never sends RFS replies or accesses NV/EFS.
 * It consumes unknown RFS requests without replying, so CP may still wait or
 * fail: this is observability only, not a substitute for the factory rfsd.
 * Logs contain frame-header metadata and allowlisted raw status fields,
 * plus the low seven signal-presence bits, never payload dumps or SIM
 * identifiers.
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
       RFS_HEADER_TRACE_LIMIT = 16, CP_IND_TRACE_LIMIT = 8,
       CP_IND_QUIET_COVERAGE_MS = 300000,
       SIM_REFRESH_MAX = 3, SIM_REFRESH_DEBOUNCE_MS = 500,
       SIM_REFRESH_COALESCE_MS = 2000, SIM_REFRESH_MIN_GAP_MS = 3000,
       SIM_REFRESH_REPLY_MS = 10000, SIM_SETTLED_DELAY_MS = 60000,
       SIM_WIRE_UNIVERSAL_PIN = 13,
       SIM_SLOT_MAX_SLOTS = 4, SIM_SLOT_LOG_SLOTS = 2,
       SIM_SLOT_RECORD_STRIDE = 105,
       SIM_SLOT_IND_LEN = 9 + SIM_SLOT_MAX_SLOTS * SIM_SLOT_RECORD_STRIDE,
       SIM_SLOT_PORT_COUNT_OFFSET = 52, SIM_SLOT_PORTS_OFFSET = 64,
       SIM_SLOT_PORT_STRIDE = 13, SIM_SLOT_MAX_PORTS = 3,
       SIM_SLOT_TRACE_LIMIT = 2,
       SCAN_WAIT_MS = 300000, SCAN_CANCEL_WAIT_MS = 5000,
       SCAN_DISPATCH_WAIT_MS = 5000, SCAN_GATE_WAIT_MS = 10000,
       SCAN_EVIDENCE_MAX_AGE_MS = 90000,
       SCAN_MAX_NETWORK_COUNT = 64 };
enum channel_kind { CHANNEL_IPC, CHANNEL_RFS };
enum sim_query_kind {
    SIM_QUERY_NONE, SIM_QUERY_CHANGE, SIM_QUERY_SETTLED, SIM_QUERY_FACTORY
};

enum live_scan_phase {
    LIVE_SCAN_IDLE, LIVE_SCAN_WAITING, LIVE_SCAN_CANCEL_READY,
    LIVE_SCAN_CANCEL_WAITING, LIVE_SCAN_DONE
};
enum live_scan_result {
    LIVE_SCAN_NONE, LIVE_SCAN_GATE_NOT_MET, LIVE_SCAN_COUNT, LIVE_SCAN_ERROR,
    LIVE_SCAN_MALFORMED_CANCEL_ACK, LIVE_SCAN_TIMEOUT_CANCEL_ACK,
    LIVE_SCAN_WRITE_CANCEL_ACK, LIVE_SCAN_GATE_LOST_CANCEL_ACK,
    LIVE_SCAN_CANCEL_ERROR,
    LIVE_SCAN_CANCEL_MALFORMED, LIVE_SCAN_CANCEL_TIMEOUT,
    LIVE_SCAN_CANCEL_NOT_SENT, LIVE_SCAN_CANCEL_WRITE_AMBIGUOUS,
    LIVE_SCAN_FRAMING_CANCEL_SENT, LIVE_SCAN_FRAMING_CANCEL_AMBIGUOUS,
    LIVE_SCAN_FRAMING_ALREADY_CANCELING, LIVE_SCAN_CP_LOST
};
enum live_scan_cancel_cause {
    LIVE_SCAN_CAUSE_NONE, LIVE_SCAN_CAUSE_MALFORMED,
    LIVE_SCAN_CAUSE_TIMEOUT, LIVE_SCAN_CAUSE_WRITE_AMBIGUOUS,
    LIVE_SCAN_CAUSE_GATE_LOST
};

struct scan_evidence {
    unsigned valid; /* settled SIM/radio/voice/data, then factory selection/RAT */
    int spoiled;
    int64_t observed_ms[6];
};
enum { SCAN_SIM = 0, SCAN_RADIO, SCAN_VOICE, SCAN_DATA,
       SCAN_SELECTION, SCAN_PREFERRED, SCAN_EVIDENCE_COUNT };

struct live_scan {
    enum live_scan_phase phase;
    enum live_scan_result result;
    enum live_scan_cancel_cause cancel_cause;
    uint32_t scan_token;
    uint32_t cancel_token;
    uint32_t count;
    unsigned remote_error;
    unsigned late_reply_seen;
    int64_t deadline_ms;
    int64_t gate_started_ms;
    int reported;
    int poisoned;
};

static const struct {
    unsigned id;
    const char *label;
} snapshot_gets[] = {
    {0x0200, "sim"}, {0x0801, "radio"},
    {SIT_NET_VOICE_REG, "voice"}, {SIT_NET_DATA_REG, "data"}
};
enum { SNAPSHOT_GET_COUNT = sizeof snapshot_gets / sizeof snapshot_gets[0] };

static const struct {
    unsigned id;
    const char *label;
} factory_gets[] = {
    {SIT_NET_SELECTION_MODE, "selection"},
    {SIT_NET_PREFERRED_GET, "preferred"},
    {SIT_NET_OPERATOR, "operator"},
    {0x0900, "signal"}
};
enum { FACTORY_GET_COUNT = sizeof factory_gets / sizeof factory_gets[0] };

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

struct slot_record_scalars {
    unsigned card_state;
    unsigned port0_logical;
    unsigned port0_state;
    int has_port0;
};

struct slot_status_scalars {
    unsigned slot_count;
    unsigned logged_slots;
    struct slot_record_scalars slot[SIM_SLOT_LOG_SLOTS];
};

struct slot_status_trace {
    unsigned seen;
};

/* Factory sit-stream.so: unsolicited 0x024e has four fixed 105-byte slot
 * records after byte +8 (total 429). Within each record: card +0,
 * port_count +52, port0 logical/state +64/+65. Never inspect or print
 * the ATR/ICCID/EID-bearing fields elsewhere in the record. */
static int slot_status_scalars(const uint8_t *p, size_t n,
                               struct slot_status_scalars *out) {
    if (n != SIM_SLOT_IND_LEN || p[0] != 2 ||
        le16(p + 2) != 0x024e || le16(p + 4) != n ||
        p[8] < 1 || p[8] > SIM_SLOT_MAX_SLOTS) return 0;
    memset(out, 0, sizeof *out);
    out->slot_count = p[8];
    out->logged_slots = out->slot_count < SIM_SLOT_LOG_SLOTS ?
                        out->slot_count : SIM_SLOT_LOG_SLOTS;
    for (unsigned i = 0; i < out->slot_count; ++i) {
        size_t record = 9U + (size_t)i * SIM_SLOT_RECORD_STRIDE;
        size_t record_end = record + SIM_SLOT_RECORD_STRIDE;
        size_t port_count_offset = record + SIM_SLOT_PORT_COUNT_OFFSET;
        if (record_end > n || port_count_offset >= record_end) return 0;
        unsigned port_count = p[port_count_offset];
        size_t ports_end = record + SIM_SLOT_PORTS_OFFSET +
                           SIM_SLOT_PORT_STRIDE * (size_t)port_count;
        if (port_count > SIM_SLOT_MAX_PORTS || ports_end > record_end) return 0;
        if (i >= SIM_SLOT_LOG_SLOTS) continue;
        struct slot_record_scalars *slot = &out->slot[i];
        slot->card_state = p[record];
        if (port_count) {
            size_t port0 = record + SIM_SLOT_PORTS_OFFSET;
            if (port0 + 1 >= record_end) return 0;
            slot->port0_logical = p[port0];
            slot->port0_state = p[port0 + 1];
            slot->has_port0 = 1;
        }
    }
    return 1;
}

/* Count the first two matching indications even if their body is malformed:
 * later traffic cannot cause an unbounded or selectively repeated log. */
static int slot_status_take(struct slot_status_trace *trace,
                            const uint8_t *p, size_t n,
                            struct slot_status_scalars *out) {
    if (n < 8 || p[0] != 2 || le16(p + 2) != 0x024e ||
        trace->seen >= SIM_SLOT_TRACE_LIMIT) return 0;
    trace->seen++;
    return slot_status_scalars(p, n, out);
}

struct sim_refresh {
    uint32_t token;
    unsigned sent;
    size_t settled_next;
    size_t factory_next;
    int queued;
    enum sim_query_kind pending_kind;
    int settled_started;
    int factory_stopped;
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

static enum sim_query_kind sim_refresh_expire(struct sim_refresh *refresh,
                                              int64_t now_ms) {
    if (refresh->pending_kind == SIM_QUERY_NONE ||
        now_ms < refresh->reply_deadline_ms) return SIM_QUERY_NONE;
    enum sim_query_kind expired = refresh->pending_kind;
    refresh->pending_kind = SIM_QUERY_NONE;
    if (expired == SIM_QUERY_SETTLED) refresh->settled_next++;
    if (expired == SIM_QUERY_FACTORY) refresh->factory_stopped = 1;
    return expired;
}

static int sim_refresh_ready(const struct sim_refresh *refresh,
                             int initial_done, int64_t now_ms) {
    return initial_done && !refresh->disabled &&
           refresh->pending_kind == SIM_QUERY_NONE &&
           !(refresh->settled_started &&
             refresh->settled_next < SNAPSHOT_GET_COUNT) &&
           refresh->queued && refresh->sent < SIM_REFRESH_MAX &&
           now_ms >= refresh->due_ms && now_ms >= refresh->next_allowed_ms;
}

static int sim_settled_ready(const struct sim_refresh *refresh,
                             int initial_done, int64_t now_ms,
                             int64_t owner_start_ms) {
    if (!initial_done || refresh->disabled ||
        refresh->pending_kind != SIM_QUERY_NONE ||
        refresh->settled_next >= SNAPSHOT_GET_COUNT) return 0;
    if (refresh->settled_started) return 1;
    return !refresh->queued &&
           now_ms - owner_start_ms >= SIM_SETTLED_DELAY_MS &&
           now_ms >= refresh->next_allowed_ms;
}

static int sim_factory_ready(const struct sim_refresh *refresh,
                             int initial_done) {
    /* Let a queued SIM refresh finish its debounce/gap before the next GET. */
    return initial_done && !refresh->disabled &&
           refresh->pending_kind == SIM_QUERY_NONE &&
           refresh->settled_started &&
           refresh->settled_next == SNAPSHOT_GET_COUNT &&
           !refresh->queued &&
           !refresh->factory_stopped &&
           refresh->factory_next < FACTORY_GET_COUNT;
}

static void sim_refresh_mark_sent(struct sim_refresh *refresh,
                                  uint32_t token, int64_t now_ms) {
    refresh->queued = 0;
    refresh->pending_kind = SIM_QUERY_CHANGE;
    refresh->token = token;
    refresh->sent++;
    refresh->reply_deadline_ms = now_ms + SIM_REFRESH_REPLY_MS;
    refresh->next_allowed_ms = now_ms + SIM_REFRESH_MIN_GAP_MS;
}

static void sim_settled_mark_sent(struct sim_refresh *refresh,
                                  uint32_t token, int64_t now_ms) {
    refresh->settled_started = 1;
    refresh->pending_kind = SIM_QUERY_SETTLED;
    refresh->token = token;
    refresh->reply_deadline_ms = now_ms + SIM_REFRESH_REPLY_MS;
    refresh->next_allowed_ms = now_ms + SIM_REFRESH_MIN_GAP_MS;
}

static void sim_factory_mark_sent(struct sim_refresh *refresh,
                                  uint32_t token, int64_t now_ms) {
    refresh->pending_kind = SIM_QUERY_FACTORY;
    refresh->token = token;
    refresh->reply_deadline_ms = now_ms + SIM_REFRESH_REPLY_MS;
}

static enum sim_query_kind sim_refresh_match_reply(struct sim_refresh *refresh,
                                                   const uint8_t *p, size_t n,
                                                   int64_t now_ms) {
    if (refresh->pending_kind == SIM_QUERY_NONE ||
        (refresh->pending_kind == SIM_QUERY_SETTLED &&
         refresh->settled_next >= SNAPSHOT_GET_COUNT) ||
        (refresh->pending_kind == SIM_QUERY_FACTORY &&
         refresh->factory_next >= FACTORY_GET_COUNT) ||
        now_ms >= refresh->reply_deadline_ms ||
        !is_reply(p, n, refresh->pending_kind == SIM_QUERY_CHANGE ?
                  0x0200 : refresh->pending_kind == SIM_QUERY_SETTLED ?
                  snapshot_gets[refresh->settled_next].id :
                  factory_gets[refresh->factory_next].id,
                  refresh->token)) return SIM_QUERY_NONE;
    enum sim_query_kind matched = refresh->pending_kind;
    refresh->pending_kind = SIM_QUERY_NONE;
    if (matched == SIM_QUERY_SETTLED) refresh->settled_next++;
    if (matched == SIM_QUERY_FACTORY) refresh->factory_next++;
    return matched;
}

/* Factory TD1A response offsets verified by ready-network-once. Only these
 * two scalar fields are read; the operator body is never parsed. */
static int factory_scalar(unsigned id, const uint8_t *p, size_t n,
                          uint32_t *value) {
    if (id == SIT_NET_SELECTION_MODE && n >= 13 && p[12] <= 1) {
        *value = p[12];
        return 1;
    }
    if (id == SIT_NET_PREFERRED_GET && n >= 16) {
        *value = le32(p + 12);
        return 1;
    }
    return 0;
}

/* sit-stream.so ProtocolSignalStrengthAdapter::GetSignalStrength reads a
 * 16-bit technology-presence mask at response +12. Its V4 parser needs at
 * least 196 bytes after the mask (12-byte response header + 2 + 196 = 210).
 * Do not inspect any measurements or identifiers in the remaining body. */
static int factory_signal_mask(const uint8_t *p, size_t n,
                               uint32_t token, uint32_t *value) {
    if (n < 210 || !is_reply(p, n, 0x0900, token) ||
        le16(p + 4) != n || le16(p + 10)) return 0;
    *value = le16(p + 12) & 0x7fU;
    return 1;
}

static int factory_reply_ok(struct sim_refresh *refresh, unsigned id,
                            const uint8_t *p, size_t n, uint32_t *value) {
    if (n < 12 || le16(p + 4) != n || le16(p + 10) ||
        ((id == SIT_NET_SELECTION_MODE || id == SIT_NET_PREFERRED_GET) &&
         !factory_scalar(id, p, n, value)) ||
        (id == 0x0900 &&
         !factory_signal_mask(p, n, refresh->token, value)) ||
        (id != SIT_NET_SELECTION_MODE && id != SIT_NET_PREFERRED_GET &&
         id != SIT_NET_OPERATOR && id != 0x0900)) {
        refresh->factory_stopped = 1;
        return 0;
    }
    return 1;
}

struct sim_status_scalars {
    unsigned card;
    unsigned apps;
    unsigned universal_pin;
    unsigned app_type;
    unsigned app_state;
    unsigned perso;
    unsigned pin1;
    unsigned pin1_remaining;
    int full_app_record;
};

/* Only scalar status bytes are extracted. Do not expose AID or identifiers.
 * The first app fields are valid only when every declared app record fits. */
static int sim_status_scalars(const uint8_t *p, size_t size,
                              struct sim_status_scalars *out) {
    memset(out, 0, sizeof *out);
    if (size < 15) return 0;
    out->card = p[SIT_SIM_CARD];
    out->apps = p[SIT_SIM_APPS];
    if (out->apps > 0 && out->apps <= 4 &&
        size >= 15U + SIT_SIM_APP_STRIDE * out->apps &&
        size > SIT_SIM_PIN1_REMAIN) {
        out->universal_pin = p[SIM_WIRE_UNIVERSAL_PIN];
        out->app_type = p[SIT_SIM_APP_TYPE];
        out->app_state = p[SIT_SIM_APP_STATE];
        out->perso = p[SIT_SIM_PERSO_STATE];
        out->pin1 = p[SIT_SIM_PIN1];
        out->pin1_remaining = p[SIT_SIM_PIN1_REMAIN];
        out->full_app_record = 1;
    }
    return 1;
}

/* Scan evidence comes only from successful, fully framed replies in this
 * owner's current boot. A timeout advancing a status sweep proves nothing. */
static int strict_success(const uint8_t *p, size_t n,
                          unsigned id, uint32_t token, size_t minimum) {
    return n >= minimum && n <= UINT16_MAX && is_reply(p, n, id, token) &&
           le16(p + 4) == n && le16(p + 10) == 0;
}

static void scan_evidence_note_status(struct scan_evidence *evidence,
                                      unsigned field, const uint8_t *p,
                                      size_t n, uint32_t token,
                                      int64_t now_ms) {
    if (field > SCAN_DATA || evidence->spoiled) return;
    unsigned id = snapshot_gets[field].id;
    size_t minimum = field == SCAN_SIM ? 15 + SIT_SIM_APP_STRIDE :
                     field == SCAN_RADIO ? 16 :
                     field == SCAN_VOICE ? 14 : 16;
    if (!strict_success(p, n, id, token, minimum)) return;
    int valid = 0;
    if (field == SCAN_SIM) {
        struct sim_status_scalars sim;
        valid = sim_status_scalars(p, n, &sim) && sim.full_app_record &&
                sim.card == 1 && sim.apps == 1 && sim.app_state == 5 &&
                sim.pin1 == 3;
    } else if (field == SCAN_RADIO)
        valid = le32(p + 12) == 10;
    else
        valid = p[SIT_NET_REG_STATE_OFFSET] == 0 &&
                p[SIT_NET_REJECT_OFFSET] == 0;
    if (!valid) return;
    evidence->valid |= 1U << field;
    evidence->observed_ms[field] = now_ms;
}

static void scan_evidence_note_factory(struct scan_evidence *evidence,
                                       unsigned id, const uint8_t *p,
                                       size_t n, uint32_t token,
                                       int64_t now_ms) {
    if (evidence->spoiled) return;
    unsigned field;
    uint32_t expected;
    if (id == SIT_NET_SELECTION_MODE) {
        field = SCAN_SELECTION;
        expected = 0;
        if (!strict_success(p, n, id, token, 13) || p[12] != expected) return;
    } else if (id == SIT_NET_PREFERRED_GET) {
        field = SCAN_PREFERRED;
        expected = 16; /* Samsung SIT NR/LTE/GSM/WCDMA, not Android enum 16. */
        if (!strict_success(p, n, id, token, 16) ||
            le32(p + 12) != expected) return;
    } else return;
    evidence->valid |= 1U << field;
    evidence->observed_ms[field] = now_ms;
}

static int scan_evidence_ready(const struct scan_evidence *evidence,
                                int64_t now_ms) {
    if (evidence->spoiled ||
        evidence->valid != (1U << SCAN_EVIDENCE_COUNT) - 1U) return 0;
    for (unsigned i = 0; i < SCAN_EVIDENCE_COUNT; ++i) {
        if (evidence->observed_ms[i] < 0 ||
            now_ms < evidence->observed_ms[i] ||
            now_ms - evidence->observed_ms[i] > SCAN_EVIDENCE_MAX_AGE_MS)
            return 0;
    }
    return 1;
}

/* Any later SIM, radio, or network unsolicited event makes the gathered
 * point-in-time status stale. Do not infer CP/eUICC RF idleness from it. */
static int scan_evidence_unsolicited(struct scan_evidence *evidence,
                                      const uint8_t *p, size_t n) {
    if (n < 8 || p[0] != 2 || !evidence->valid) return 0;
    unsigned id = le16(p + 2);
    if (id == 0x0210 || id == 0x024e ||
        (id >= 0x0700 && id <= 0x07ff) ||
        (id >= 0x0800 && id <= 0x08ff)) {
        evidence->spoiled = 1;
        return 1;
    }
    return 0;
}

struct cp_ind_trace {
    unsigned seen;
    unsigned logged;
    uint64_t overflow;
};

/* Header-only network/radio indications; the caller resets this each minute. */
static int cp_ind_trace_take(struct cp_ind_trace *trace,
                             const uint8_t *p, size_t n) {
    if (n < 8 || p[0] != 2 || le16(p + 4) != n) return 0;
    unsigned id = le16(p + 2);
    if (id < 0x0700 || id > 0x08ff) return 0;
    if (trace->seen != UINT_MAX) trace->seen++;
    if (trace->logged >= CP_IND_TRACE_LIMIT) {
        if (trace->overflow != UINT64_MAX) trace->overflow++;
        return 0;
    }
    trace->logged++;
    return 1;
}

static int cp_ind_quiet_window(int64_t owner_start_ms,
                               int64_t window_start_ms) {
    return window_start_ms >= owner_start_ms &&
           window_start_ms - owner_start_ms < CP_IND_QUIET_COVERAGE_MS;
}

static void make_scan_request(uint8_t request[16], uint32_t token) {
    memset(request, 0, 16);
    request[2] = 0x06;
    request[3] = 0x07;
    request[4] = 16;
    put32(request + 6, token);
    /* TD1A BuildQueryAvailableNetwork(0): LE32 argument at +12 is zero. */
}

static void make_cancel_request(uint8_t request[12], uint32_t token) {
    memset(request, 0, 12);
    request[2] = 0x07;
    request[3] = 0x07;
    request[4] = 12;
    put32(request + 6, token);
}

static void live_scan_request_cancel(struct live_scan *scan,
                                     enum live_scan_cancel_cause cause,
                                     int64_t now_ms) {
    if (scan->phase != LIVE_SCAN_WAITING) return;
    int64_t deadline = cause == LIVE_SCAN_CAUSE_TIMEOUT ?
                       scan->deadline_ms + SCAN_DISPATCH_WAIT_MS :
                       now_ms + SCAN_DISPATCH_WAIT_MS;
    scan->cancel_cause = cause;
    if (now_ms >= deadline) {
        scan->phase = LIVE_SCAN_DONE;
        scan->result = LIVE_SCAN_CANCEL_NOT_SENT;
        scan->poisoned = 1;
    } else {
        scan->phase = LIVE_SCAN_CANCEL_READY;
        scan->deadline_ms = deadline;
    }
}

static void live_scan_tick(struct live_scan *scan, int64_t now_ms) {
    if (scan->phase == LIVE_SCAN_WAITING && now_ms >= scan->deadline_ms)
        live_scan_request_cancel(scan, LIVE_SCAN_CAUSE_TIMEOUT, now_ms);
    else if (scan->phase == LIVE_SCAN_CANCEL_READY &&
             now_ms >= scan->deadline_ms) {
        scan->phase = LIVE_SCAN_DONE;
        scan->result = LIVE_SCAN_CANCEL_NOT_SENT;
        scan->poisoned = 1;
    } else if (scan->phase == LIVE_SCAN_CANCEL_WAITING &&
               now_ms >= scan->deadline_ms) {
        scan->phase = LIVE_SCAN_DONE;
        scan->result = LIVE_SCAN_CANCEL_TIMEOUT;
        scan->poisoned = 1;
    }
}

static void live_scan_scan_written(struct live_scan *scan,
                                   int exact, int64_t now_ms) {
    if (scan->phase != LIVE_SCAN_IDLE) return;
    scan->phase = LIVE_SCAN_WAITING;
    if (!exact)
        live_scan_request_cancel(scan,
                                 LIVE_SCAN_CAUSE_WRITE_AMBIGUOUS, now_ms);
    else
        scan->deadline_ms = now_ms + SCAN_WAIT_MS;
}

static void live_scan_cancel_written(struct live_scan *scan,
                                     int exact, int64_t now_ms) {
    if (scan->phase != LIVE_SCAN_CANCEL_READY) return;
    if (!exact) {
        scan->phase = LIVE_SCAN_DONE;
        scan->result = LIVE_SCAN_CANCEL_WRITE_AMBIGUOUS;
        scan->poisoned = 1;
    } else {
        scan->phase = LIVE_SCAN_CANCEL_WAITING;
        scan->deadline_ms = now_ms + SCAN_CANCEL_WAIT_MS;
    }
}

static int scan_addressed(const uint8_t *p, size_t n, unsigned id,
                          uint32_t token) {
    return n >= 10 && le16(p + 2) == id && le32(p + 6) == token;
}

static void live_scan_frame(struct live_scan *scan, const uint8_t *p,
                            size_t n, int64_t now_ms) {
    if (scan->phase == LIVE_SCAN_IDLE || scan->phase == LIVE_SCAN_DONE) return;
    live_scan_tick(scan, now_ms); /* deadline wins over late success */
    if (scan->phase == LIVE_SCAN_DONE) return;
    if (scan->phase == LIVE_SCAN_CANCEL_READY ||
        scan->phase == LIVE_SCAN_CANCEL_WAITING) {
        if (scan_addressed(p, n, 0x0706, scan->scan_token))
            scan->late_reply_seen = 1;
        if (scan->phase != LIVE_SCAN_CANCEL_WAITING ||
            !scan_addressed(p, n, 0x0707, scan->cancel_token)) return;
        scan->phase = LIVE_SCAN_DONE;
        if (n != 12 || p[0] != 1 || le16(p + 4) != n)
            scan->result = LIVE_SCAN_CANCEL_MALFORMED;
        else if (le16(p + 10) != 0)
            scan->result = LIVE_SCAN_CANCEL_ERROR;
        else
            scan->result = scan->cancel_cause == LIVE_SCAN_CAUSE_TIMEOUT ?
                LIVE_SCAN_TIMEOUT_CANCEL_ACK :
                scan->cancel_cause == LIVE_SCAN_CAUSE_MALFORMED ?
                LIVE_SCAN_MALFORMED_CANCEL_ACK :
                scan->cancel_cause == LIVE_SCAN_CAUSE_GATE_LOST ?
                LIVE_SCAN_GATE_LOST_CANCEL_ACK :
                LIVE_SCAN_WRITE_CANCEL_ACK;
        if (scan->result == LIVE_SCAN_CANCEL_MALFORMED ||
            scan->result == LIVE_SCAN_CANCEL_ERROR)
            scan->poisoned = 1;
        return;
    }
    if (scan->phase != LIVE_SCAN_WAITING ||
        !scan_addressed(p, n, 0x0706, scan->scan_token)) return;
    if (n < 12 || p[0] != 1 || le16(p + 4) != n) {
        live_scan_request_cancel(scan, LIVE_SCAN_CAUSE_MALFORMED, now_ms);
    } else if (le16(p + 10) != 0) {
        scan->remote_error = le16(p + 10);
        scan->phase = LIVE_SCAN_DONE;
        scan->result = LIVE_SCAN_ERROR;
    } else if (n < 16 || le32(p + 12) > SCAN_MAX_NETWORK_COUNT ||
               le32(p + 12) > (n - 16) / 14) {
        live_scan_request_cancel(scan, LIVE_SCAN_CAUSE_MALFORMED, now_ms);
    } else {
        scan->count = le32(p + 12);
        scan->phase = LIVE_SCAN_DONE;
        scan->result = LIVE_SCAN_COUNT;
    }
}

static int make_get_request(uint8_t request[12], unsigned id,
                            uint32_t token) {
    if (id != 0x0200 && id != 0x0801 &&
        id != SIT_NET_VOICE_REG && id != SIT_NET_DATA_REG &&
        id != SIT_NET_SELECTION_MODE && id != SIT_NET_PREFERRED_GET &&
        id != SIT_NET_OPERATOR && id != 0x0900) return -1;
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

static int fixture_slot_status(void) {
    if (SIM_SLOT_IND_LEN != 429 ||
        9 + SIM_SLOT_RECORD_STRIDE != 114 ||
        9 + SIM_SLOT_PORT_COUNT_OFFSET != 61 ||
        9 + SIM_SLOT_PORTS_OFFSET != 73 ||
        9 + SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORT_COUNT_OFFSET != 166 ||
        9 + SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORTS_OFFSET != 178)
        return 64;
    uint8_t indication[SIM_SLOT_IND_LEN] = {0};
    indication[0] = 2;
    indication[2] = 0x4e; indication[3] = 0x02;
    indication[4] = (uint8_t)sizeof indication;
    indication[5] = (uint8_t)(sizeof indication >> 8);
    indication[8] = 2;
    indication[9] = 1;
    indication[9 + SIM_SLOT_PORT_COUNT_OFFSET] = 2;
    indication[9 + SIM_SLOT_PORTS_OFFSET] = 0xff;
    indication[9 + SIM_SLOT_PORTS_OFFSET + 1] = 1;
    indication[9 + SIM_SLOT_RECORD_STRIDE] = 0;
    indication[9 + SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORT_COUNT_OFFSET] = 1;
    indication[9 + SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORTS_OFFSET] = 1;
    indication[9 + SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORTS_OFFSET + 1] = 2;
    struct slot_status_scalars status;
    if (!slot_status_scalars(indication, sizeof indication, &status) ||
        status.slot_count != 2 || status.logged_slots != 2 ||
        status.slot[0].card_state != 1 || !status.slot[0].has_port0 ||
        status.slot[0].port0_logical != 0xff ||
        status.slot[0].port0_state != 1 ||
        status.slot[1].card_state != 0 || !status.slot[1].has_port0 ||
        status.slot[1].port0_logical != 1 ||
        status.slot[1].port0_state != 2 ||
        slot_status_scalars(indication, sizeof indication - 1, &status))
        return 50;
    indication[8] = 1;
    if (!slot_status_scalars(indication, sizeof indication, &status) ||
        status.logged_slots != 1 || status.slot[1].has_port0) return 59;
    indication[8] = 2;
    indication[9 + SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORT_COUNT_OFFSET] = 0;
    if (!slot_status_scalars(indication, sizeof indication, &status) ||
        status.slot[1].has_port0) return 60;
    indication[9 + SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORT_COUNT_OFFSET] = 4;
    if (slot_status_scalars(indication, sizeof indication, &status)) return 61;
    indication[9 + SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORT_COUNT_OFFSET] = 1;
    indication[8] = 3;
    indication[9 + 2 * SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORT_COUNT_OFFSET] = 4;
    if (slot_status_scalars(indication, sizeof indication, &status)) return 62;
    indication[9 + 2 * SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORT_COUNT_OFFSET] = 0;
    if (!slot_status_scalars(indication, sizeof indication, &status) ||
        status.slot_count != 3 || status.logged_slots != 2) return 63;
    indication[8] = 4;
    indication[9 + 3 * SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORT_COUNT_OFFSET] = 3;
    if (!slot_status_scalars(indication, sizeof indication, &status) ||
        status.slot_count != 4 || status.logged_slots != 2) return 65;
    indication[9 + 3 * SIM_SLOT_RECORD_STRIDE + SIM_SLOT_PORT_COUNT_OFFSET] = 0;
    indication[8] = 2;
    uint8_t oversized[SIM_SLOT_IND_LEN + 1];
    memcpy(oversized, indication, sizeof indication);
    oversized[sizeof indication] = 0;
    oversized[4]++;
    if (slot_status_scalars(oversized, sizeof oversized, &status)) return 66;
    indication[8] = 0;
    if (slot_status_scalars(indication, sizeof indication, &status)) return 51;
    indication[8] = SIM_SLOT_MAX_SLOTS + 1;
    if (slot_status_scalars(indication, sizeof indication, &status)) return 52;
    indication[8] = 2;
    indication[4]--;
    if (slot_status_scalars(indication, sizeof indication, &status)) return 53;
    indication[4]++;
    indication[0] = 1;
    if (slot_status_scalars(indication, sizeof indication, &status)) return 54;
    indication[0] = 2; indication[2] = 0x4d;
    if (slot_status_scalars(indication, sizeof indication, &status)) return 55;
    indication[2] = 0x4e;
    struct slot_status_trace trace = {0};
    if (!slot_status_take(&trace, indication, sizeof indication, &status) ||
        !slot_status_take(&trace, indication, sizeof indication, &status) ||
        slot_status_take(&trace, indication, sizeof indication, &status) ||
        trace.seen != SIM_SLOT_TRACE_LIMIT) return 56;
    struct slot_status_trace malformed = {0};
    indication[8] = 0;
    if (slot_status_take(&malformed, indication, sizeof indication, &status) ||
        malformed.seen != 1) return 57;
    indication[8] = 2;
    if (!slot_status_take(&malformed, indication, sizeof indication, &status) ||
        slot_status_take(&malformed, indication, sizeof indication, &status))
        return 58;
    return 0;
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
    if (refresh.queued || refresh.pending_kind != SIM_QUERY_CHANGE ||
        refresh.sent != 1 ||
        sim_refresh_ready(&refresh, 1, 3500)) return 23;
    put32(reply + 6, 10);
    if (sim_refresh_match_reply(&refresh, reply, sizeof reply, 3200) ||
        refresh.pending_kind != SIM_QUERY_CHANGE) return 24;
    sim_refresh_note_indication(&refresh, indication, sizeof indication, 3100);
    if (sim_refresh_expire(&refresh, 12999) ||
        sim_refresh_match_reply(&refresh, reply, sizeof reply, 13000) ||
        !sim_refresh_expire(&refresh, 13000) ||
        !sim_refresh_ready(&refresh, 1, 13000)) return 25;
    sim_refresh_mark_sent(&refresh, 12, 13000);
    put32(reply + 6, 11);
    if (sim_refresh_match_reply(&refresh, reply, sizeof reply, 13001)) return 26;
    put32(reply + 6, 12);
    if (sim_refresh_match_reply(&refresh, reply, sizeof reply, 13001) !=
        SIM_QUERY_CHANGE || refresh.pending_kind != SIM_QUERY_NONE) return 27;
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

    struct sim_refresh settled = {0};
    if (sim_settled_ready(&settled, 0, 60000, 0) ||
        sim_settled_ready(&settled, 1, 59999, 0) ||
        !sim_settled_ready(&settled, 1, 60000, 0)) return 33;
    sim_refresh_note_indication(&settled, indication, sizeof indication, 59900);
    if (sim_settled_ready(&settled, 1, 60500, 0)) return 34;
    sim_refresh_mark_sent(&settled, 31, 60500);
    if (sim_settled_ready(&settled, 1, 63500, 0)) return 35;
    put32(reply + 6, 31);
    if (sim_refresh_match_reply(&settled, reply, sizeof reply, 60501) !=
        SIM_QUERY_CHANGE || sim_settled_ready(&settled, 1, 63499, 0) ||
        !sim_settled_ready(&settled, 1, 63500, 0)) return 36;
    sim_settled_mark_sent(&settled, 32, 63500);
    if (!settled.settled_started || settled.settled_next != 0 ||
        settled.pending_kind != SIM_QUERY_SETTLED ||
        sim_settled_ready(&settled, 1, 70000, 0)) return 37;
    put32(reply + 6, 31);
    if (sim_refresh_match_reply(&settled, reply, sizeof reply, 63501)) return 38;
    sim_refresh_note_indication(&settled, indication, sizeof indication, 63501);
    if (sim_refresh_ready(&settled, 1, 64001)) return 39;
    put32(reply + 6, 32);
    if (sim_refresh_match_reply(&settled, reply, sizeof reply, 63501) !=
        SIM_QUERY_SETTLED || settled.settled_next != 1 ||
        sim_refresh_ready(&settled, 1, 64001) ||
        !sim_settled_ready(&settled, 1, 63501, 0)) return 40;
    sim_settled_mark_sent(&settled, 33, 63502);
    put32(reply + 6, 33);
    if (sim_refresh_match_reply(&settled, reply, sizeof reply, 63503)) return 41;
    reply[2] = 0x01; reply[3] = 0x08;
    if (sim_refresh_match_reply(&settled, reply, sizeof reply, 63503) !=
        SIM_QUERY_SETTLED || settled.settled_next != 2) return 42;
    sim_settled_mark_sent(&settled, 34, 63504);
    reply[2] = (uint8_t)SIT_NET_VOICE_REG;
    reply[3] = (uint8_t)(SIT_NET_VOICE_REG >> 8);
    put32(reply + 6, 34);
    if (sim_refresh_match_reply(&settled, reply, sizeof reply, 73504) ||
        sim_refresh_expire(&settled, 73504) != SIM_QUERY_SETTLED ||
        settled.settled_next != 3 ||
        sim_refresh_match_reply(&settled, reply, sizeof reply, 73505)) return 46;
    if (!sim_settled_ready(&settled, 1, 73504, 0)) return 47;
    sim_settled_mark_sent(&settled, 35, 73504);
    reply[2] = (uint8_t)SIT_NET_DATA_REG;
    reply[3] = (uint8_t)(SIT_NET_DATA_REG >> 8);
    put32(reply + 6, 35);
    if (sim_refresh_match_reply(&settled, reply, sizeof reply, 73505) !=
        SIM_QUERY_SETTLED || settled.settled_next != SNAPSHOT_GET_COUNT ||
        sim_settled_ready(&settled, 1, 80000, 0) ||
        sim_refresh_ready(&settled, 1, 76503) ||
        !sim_refresh_ready(&settled, 1, 76504) ||
        sim_factory_ready(&settled, 1)) return 48;
    for (size_t i = 0; i < SNAPSHOT_GET_COUNT; ++i) {
        uint8_t request[12];
        if (make_get_request(request, snapshot_gets[i].id,
                             (uint32_t)(100 + i)) ||
            le16(request + 2) != snapshot_gets[i].id ||
            le32(request + 6) != (uint32_t)(100 + i)) return 49;
    }

    uint8_t sim[15 + SIT_SIM_APP_STRIDE] = {0};
    sim[SIT_SIM_CARD] = 1; sim[SIM_WIRE_UNIVERSAL_PIN] = 2;
    sim[SIT_SIM_APPS] = 1; sim[SIT_SIM_APP_TYPE] = 3;
    sim[SIT_SIM_APP_STATE] = 4; sim[SIT_SIM_PERSO_STATE] = 5;
    sim[SIT_SIM_PIN1] = 6; sim[SIT_SIM_PIN1_REMAIN] = 7;
    struct sim_status_scalars scalars;
    if (sim_status_scalars(sim, 14, &scalars) ||
        !sim_status_scalars(sim, sizeof sim, &scalars) ||
        !scalars.full_app_record || scalars.card != 1 || scalars.apps != 1 ||
        scalars.universal_pin != 2 || scalars.app_type != 3 ||
        scalars.app_state != 4 || scalars.perso != 5 ||
        scalars.pin1 != 6 || scalars.pin1_remaining != 7) return 43;
    if (!sim_status_scalars(sim, sizeof sim - 1, &scalars) ||
        scalars.full_app_record || scalars.universal_pin || scalars.app_type ||
        scalars.app_state || scalars.perso || scalars.pin1 ||
        scalars.pin1_remaining) return 44;
    sim[SIT_SIM_APPS] = 2;
    if (!sim_status_scalars(sim, sizeof sim, &scalars) ||
        scalars.full_app_record) return 45;
    return 0;
}

static int fixture_factory(void) {
    if (FACTORY_GET_COUNT != 4 ||
        factory_gets[0].id != SIT_NET_SELECTION_MODE ||
        factory_gets[1].id != SIT_NET_PREFERRED_GET ||
        factory_gets[2].id != SIT_NET_OPERATOR ||
        factory_gets[3].id != 0x0900) return 59;
    struct sim_refresh refresh = {
        .settled_started = 1, .settled_next = SNAPSHOT_GET_COUNT
    };
    if (sim_factory_ready(&refresh, 0) ||
        !sim_factory_ready(&refresh, 1) ||
        sim_refresh_ready(&refresh, 1, 70000)) return 60;
    uint8_t reply[16] = {1, 0, 3, 7, 13, 0};
    sim_factory_mark_sent(&refresh, 70, 70000);
    uint8_t indication[8] = {2, 0, 0x10, 0x02, 8, 0, 0, 0};
    sim_refresh_note_indication(&refresh, indication, sizeof indication, 70001);
    if (!refresh.queued || refresh.due_ms != 70501 ||
        sim_factory_ready(&refresh, 1)) return 71;
    put32(reply + 6, 69);
    if (sim_refresh_match_reply(&refresh, reply, 13, 70001) ||
        refresh.factory_next || sim_factory_ready(&refresh, 1)) return 61;
    put32(reply + 6, 70);
    reply[12] = 1;
    uint32_t scalar = 0;
    if (sim_refresh_match_reply(&refresh, reply, 13, 70002) !=
        SIM_QUERY_FACTORY || !factory_reply_ok(&refresh,
        SIT_NET_SELECTION_MODE, reply, 13, &scalar) ||
        scalar != 1 || refresh.factory_next != 1 ||
        sim_factory_ready(&refresh, 1) ||
        sim_refresh_ready(&refresh, 1, 70500) ||
        !sim_refresh_ready(&refresh, 1, 70501)) return 62;
    sim_refresh_mark_sent(&refresh, 90, 70501);
    uint8_t sim_reply[12] = {1, 0, 0, 2, 12, 0};
    put32(sim_reply + 6, 90);
    if (refresh.queued || sim_factory_ready(&refresh, 1) ||
        sim_refresh_match_reply(&refresh, sim_reply, sizeof sim_reply,
                                70502) != SIM_QUERY_CHANGE ||
        !sim_factory_ready(&refresh, 1) || refresh.factory_next != 1)
        return 72;
    sim_factory_mark_sent(&refresh, 71, 70503);
    reply[2] = 0x0b; reply[4] = 16;
    put32(reply + 6, 71);
    put32(reply + 12, 12);
    if (sim_refresh_match_reply(&refresh, reply, 16, 70504) !=
        SIM_QUERY_FACTORY || !factory_reply_ok(&refresh,
        SIT_NET_PREFERRED_GET, reply, 16, &scalar) ||
        scalar != 12 || refresh.factory_next != 2) return 63;
    sim_factory_mark_sent(&refresh, 72, 70505);
    reply[2] = 2; reply[4] = 16;
    put32(reply + 6, 72);
    scalar = 0xdeadbeefU;
    if (sim_refresh_match_reply(&refresh, reply, 16, 70506) !=
        SIM_QUERY_FACTORY || !factory_reply_ok(&refresh,
        SIT_NET_OPERATOR, reply, 16, &scalar) ||
        scalar != 0xdeadbeefU || refresh.factory_next != 3) return 64;
    sim_factory_mark_sent(&refresh, 73, 70507);
    uint8_t signal_reply[210] = {1, 0, 0, 9, 210, 0};
    put32(signal_reply + 6, 73);
    signal_reply[12] = 0x45;
    if (sim_refresh_match_reply(&refresh, signal_reply,
                                sizeof signal_reply, 70508) !=
        SIM_QUERY_FACTORY || !factory_reply_ok(&refresh,
        0x0900, signal_reply, sizeof signal_reply, &scalar) ||
        scalar != 0x45U || refresh.factory_next != FACTORY_GET_COUNT ||
        sim_factory_ready(&refresh, 1) ||
        sim_refresh_ready(&refresh, 1, 70508)) return 65;

    struct sim_refresh timeout = {
        .settled_started = 1, .settled_next = SNAPSHOT_GET_COUNT
    };
    sim_factory_mark_sent(&timeout, 80, 80000);
    if (sim_refresh_expire(&timeout, 89999) ||
        sim_refresh_expire(&timeout, 90000) != SIM_QUERY_FACTORY ||
        !timeout.factory_stopped || sim_factory_ready(&timeout, 1)) return 66;
    struct sim_refresh error = {
        .settled_started = 1, .settled_next = SNAPSHOT_GET_COUNT
    };
    sim_factory_mark_sent(&error, 81, 80000);
    reply[2] = 3; reply[3] = 7; reply[4] = 13;
    put32(reply + 6, 81);
    reply[10] = 1;
    if (sim_refresh_match_reply(&error, reply, 13, 80001) !=
        SIM_QUERY_FACTORY || factory_reply_ok(&error,
        SIT_NET_SELECTION_MODE, reply, 13, &scalar) ||
        !error.factory_stopped || sim_factory_ready(&error, 1)) return 67;
    reply[10] = 0;
    error.factory_stopped = 0;
    reply[12] = 2;
    if (factory_reply_ok(&error, SIT_NET_SELECTION_MODE, reply, 13, &scalar) ||
        !error.factory_stopped) return 68;
    error.factory_stopped = 0;
    reply[12] = 0;
    if (factory_reply_ok(&error, SIT_NET_SELECTION_MODE, reply, 12, &scalar) ||
        !error.factory_stopped) return 69;
    return 0;
}

static int fixture_signal_mask(void) {
    uint8_t reply[210] = {1, 0, 0, 9, 210, 0};
    put32(reply + 6, 123);
    reply[12] = 0x45;
    reply[13] = 0x01; /* Only the low seven bits may enter the log. */
    uint32_t mask = 0;
    if (!factory_signal_mask(reply, sizeof reply, 123, &mask) ||
        mask != 0x45U) return 73;
    if (factory_signal_mask(reply, sizeof reply - 1, 123, &mask)) return 74;
    reply[4]--;
    if (factory_signal_mask(reply, sizeof reply, 123, &mask)) return 75;
    reply[4]++;
    reply[0] = 2;
    if (factory_signal_mask(reply, sizeof reply, 123, &mask)) return 76;
    reply[0] = 1;
    reply[2] = 1;
    if (factory_signal_mask(reply, sizeof reply, 123, &mask)) return 77;
    reply[2] = 0;
    if (factory_signal_mask(reply, sizeof reply, 124, &mask)) return 78;
    reply[10] = 1;
    if (factory_signal_mask(reply, sizeof reply, 123, &mask)) return 79;
    reply[10] = 0;
    reply[12] = 0; reply[13] = 0;
    if (!factory_signal_mask(reply, sizeof reply, 123, &mask) || mask)
        return 80;
    return 0;
}

static void fixture_scan_reply(uint8_t *p, size_t n, unsigned id,
                               uint32_t token, unsigned error) {
    memset(p, 0, n);
    p[0] = 1;
    p[2] = (uint8_t)id;
    p[3] = (uint8_t)(id >> 8);
    p[4] = (uint8_t)n;
    p[5] = (uint8_t)(n >> 8);
    put32(p + 6, token);
    p[10] = (uint8_t)error;
    p[11] = (uint8_t)(error >> 8);
}

static int fixture_scan(void) {
    const uint32_t token = 0x12345678U;
    uint8_t request[16];
    make_scan_request(request, token);
    if (request[0] || le16(request + 2) != 0x0706 ||
        le16(request + 4) != 16 || le32(request + 6) != token ||
        le32(request + 12)) return 81;
    make_cancel_request(request, token + 1);
    if (request[0] || le16(request + 2) != 0x0707 ||
        le16(request + 4) != 12 || le32(request + 6) != token + 1 ||
        request[10] || request[11]) return 82;

    struct scan_evidence evidence = {0};
    uint8_t sim[15 + SIT_SIM_APP_STRIDE];
    fixture_scan_reply(sim, sizeof sim, 0x0200, token, 0x0100);
    sim[SIT_SIM_CARD] = 1; sim[SIT_SIM_APPS] = 1;
    sim[SIT_SIM_APP_STATE] = 5; sim[SIT_SIM_PIN1] = 3;
    scan_evidence_note_status(&evidence, SCAN_SIM, sim, sizeof sim,
                              token, 60000);
    if (evidence.valid) return 83; /* High error byte is not success. */
    sim[11] = 0;
    sim[SIT_SIM_PIN1] = 2; /* PIN is still enabled. */
    scan_evidence_note_status(&evidence, SCAN_SIM, sim, sizeof sim,
                              token, 60000);
    if (evidence.valid) return 103;
    sim[SIT_SIM_PIN1] = 3;
    scan_evidence_note_status(&evidence, SCAN_SIM, sim, sizeof sim,
                              token, 60000);
    if (evidence.valid != 1U) return 84;
    uint8_t radio[16];
    fixture_scan_reply(radio, sizeof radio, 0x0801, token, 0);
    put32(radio + 12, 10);
    scan_evidence_note_status(&evidence, SCAN_RADIO, radio,
                              sizeof radio, token, 60001);
    uint8_t voice[14];
    fixture_scan_reply(voice, sizeof voice, SIT_NET_VOICE_REG, token, 0);
    voice[SIT_NET_REJECT_OFFSET] = 1;
    scan_evidence_note_status(&evidence, SCAN_VOICE, voice,
                              sizeof voice, token, 60002);
    if (evidence.valid & (1U << SCAN_VOICE)) return 104;
    voice[SIT_NET_REJECT_OFFSET] = 0;
    scan_evidence_note_status(&evidence, SCAN_VOICE, voice,
                              sizeof voice, token, 60002);
    uint8_t data[16];
    fixture_scan_reply(data, sizeof data, SIT_NET_DATA_REG, token, 0);
    scan_evidence_note_status(&evidence, SCAN_DATA, data,
                              sizeof data, token, 60003);
    uint8_t selection[13];
    fixture_scan_reply(selection, sizeof selection,
                       SIT_NET_SELECTION_MODE, token, 0);
    scan_evidence_note_factory(&evidence, SIT_NET_SELECTION_MODE,
                                selection, sizeof selection, token, 60004);
    uint8_t preferred[16];
    fixture_scan_reply(preferred, sizeof preferred,
                       SIT_NET_PREFERRED_GET, token, 0);
    put32(preferred + 12, 12); /* Restrictive LTE/WCDMA fails this gate. */
    scan_evidence_note_factory(&evidence, SIT_NET_PREFERRED_GET,
                                preferred, sizeof preferred, token, 60005);
    if (evidence.valid & (1U << SCAN_PREFERRED)) return 105;
    put32(preferred + 12, 16);
    scan_evidence_note_factory(&evidence, SIT_NET_PREFERRED_GET,
                                preferred, sizeof preferred, token, 60005);
    if (!scan_evidence_ready(&evidence, 60006) ||
        scan_evidence_ready(&evidence, 150001)) return 85;
    uint8_t unsol[8] = {2, 0, 0x10, 0x02, 8, 0, 0, 0};
    if (!scan_evidence_unsolicited(&evidence, unsol, sizeof unsol) ||
        !evidence.spoiled || scan_evidence_ready(&evidence, 60006)) return 86;

    struct live_scan scan = {.phase = LIVE_SCAN_WAITING,
                             .scan_token = token,
                             .cancel_token = token + 1,
                             .deadline_ms = 300000};
    live_scan_tick(&scan, 299999);
    if (scan.phase != LIVE_SCAN_WAITING) return 87;
    live_scan_tick(&scan, 300000);
    if (scan.phase != LIVE_SCAN_CANCEL_READY ||
        scan.cancel_cause != LIVE_SCAN_CAUSE_TIMEOUT) return 88;
    scan.phase = LIVE_SCAN_CANCEL_WAITING;
    scan.deadline_ms = 305000;
    uint8_t reply[44];
    fixture_scan_reply(reply, sizeof reply, 0x0706, token, 0);
    put32(reply + 12, 2);
    live_scan_frame(&scan, reply, sizeof reply, 300001);
    if (!scan.late_reply_seen || scan.phase != LIVE_SCAN_CANCEL_WAITING ||
        scan.result != LIVE_SCAN_NONE) return 89;
    fixture_scan_reply(reply, 12, 0x0707, token + 1, 0);
    live_scan_frame(&scan, reply, 12, 305000);
    if (scan.result != LIVE_SCAN_CANCEL_TIMEOUT ||
        scan.phase != LIVE_SCAN_DONE) return 90; /* Late ACK loses. */

    scan = (struct live_scan){.phase = LIVE_SCAN_WAITING,
                               .scan_token = token,
                               .cancel_token = token + 1,
                               .deadline_ms = 300000};
    fixture_scan_reply(reply, sizeof reply, 0x0706, token, 0);
    put32(reply + 12, 2);
    live_scan_frame(&scan, reply, sizeof reply, 1000);
    if (scan.phase != LIVE_SCAN_DONE || scan.result != LIVE_SCAN_COUNT ||
        scan.count != 2) return 91;
    scan = (struct live_scan){.phase = LIVE_SCAN_WAITING,
                               .scan_token = token,
                               .cancel_token = token + 1,
                               .deadline_ms = 300000};
    fixture_scan_reply(reply, 16, 0x0706, token, 0);
    put32(reply + 12, 2); /* Count exceeds available 14-byte entries. */
    live_scan_frame(&scan, reply, 16, 1000);
    if (scan.phase != LIVE_SCAN_CANCEL_READY ||
        scan.cancel_cause != LIVE_SCAN_CAUSE_MALFORMED) return 92;
    scan = (struct live_scan){.phase = LIVE_SCAN_WAITING,
                               .scan_token = token,
                               .cancel_token = token + 1,
                               .deadline_ms = 300000};
    fixture_scan_reply(reply, 12, 0x0706, token, 0x0100);
    live_scan_frame(&scan, reply, 12, 1000);
    if (scan.phase != LIVE_SCAN_DONE || scan.result != LIVE_SCAN_ERROR ||
        scan.remote_error != 0x0100)
        return 93;
    scan = (struct live_scan){.phase = LIVE_SCAN_WAITING,
                               .scan_token = token,
                               .cancel_token = token + 1,
                               .deadline_ms = 300000};
    live_scan_request_cancel(&scan, LIVE_SCAN_CAUSE_WRITE_AMBIGUOUS, 1000);
    if (scan.phase != LIVE_SCAN_CANCEL_READY ||
        scan.cancel_cause != LIVE_SCAN_CAUSE_WRITE_AMBIGUOUS) return 94;
    scan.phase = LIVE_SCAN_CANCEL_WAITING;
    scan.deadline_ms = 6000;
    fixture_scan_reply(reply, 12, 0x0707, token + 1, 0);
    live_scan_frame(&scan, reply, 12, 1001);
    if (scan.phase != LIVE_SCAN_DONE ||
        scan.result != LIVE_SCAN_WRITE_CANCEL_ACK) return 95;
    scan = (struct live_scan){.phase = LIVE_SCAN_IDLE};
    live_scan_scan_written(&scan, 0, 1000); /* partial/EAGAIN: no retry */
    if (scan.phase != LIVE_SCAN_CANCEL_READY ||
        scan.cancel_cause != LIVE_SCAN_CAUSE_WRITE_AMBIGUOUS) return 96;
    live_scan_cancel_written(&scan, 0, 1001);
    if (scan.phase != LIVE_SCAN_DONE ||
        scan.result != LIVE_SCAN_CANCEL_WRITE_AMBIGUOUS ||
        !scan.poisoned) return 97;
    scan = (struct live_scan){.phase = LIVE_SCAN_WAITING,
                               .deadline_ms = 300000};
    live_scan_tick(&scan, 306000); /* no delayed cancel dispatch */
    if (scan.phase != LIVE_SCAN_DONE ||
        scan.result != LIVE_SCAN_CANCEL_NOT_SENT ||
        !scan.poisoned) return 98;
    scan = (struct live_scan){.phase = LIVE_SCAN_WAITING,
                               .scan_token = token,
                               .cancel_token = token + 1,
                               .deadline_ms = 300000};
    fixture_scan_reply(reply, 16, 0x0706, token, 0);
    reply[4] = 15; /* matching token, malformed declared length */
    live_scan_frame(&scan, reply, 16, 1000);
    if (scan.phase != LIVE_SCAN_CANCEL_READY ||
        scan.cancel_cause != LIVE_SCAN_CAUSE_MALFORMED) return 99;
    scan.phase = LIVE_SCAN_CANCEL_WAITING;
    scan.deadline_ms = 6000;
    fixture_scan_reply(reply, 12, 0x0707, token + 1, 0x0100);
    live_scan_frame(&scan, reply, 12, 1001);
    if (scan.phase != LIVE_SCAN_DONE ||
        scan.result != LIVE_SCAN_CANCEL_ERROR ||
        !scan.poisoned) return 100;
    evidence = (struct scan_evidence){.valid = 1U << SCAN_SIM};
    unsol[2] = 0x02; unsol[3] = 0x08; /* radio changed */
    if (!scan_evidence_unsolicited(&evidence, unsol, sizeof unsol) ||
        !evidence.spoiled) return 101;
    evidence = (struct scan_evidence){.valid = 1U << SCAN_SIM};
    unsol[2] = 0x08; unsol[3] = 0x07; /* network changed */
    if (!scan_evidence_unsolicited(&evidence, unsol, sizeof unsol) ||
        !evidence.spoiled) return 102;
    return 0;
}

static int fixture_cp_ind_trace(void) {
    struct cp_ind_trace trace = {0};
    uint8_t ind[8] = {2, 0, 0, 7, 8, 0, 0, 0};
    if (!cp_ind_trace_take(&trace, ind, sizeof ind) ||
        trace.seen != 1 || trace.logged != 1)
        return 103;
    ind[3] = 8;
    if (!cp_ind_trace_take(&trace, ind, sizeof ind) ||
        trace.seen != 2 || trace.logged != 2)
        return 104;
    ind[0] = 1;
    if (cp_ind_trace_take(&trace, ind, sizeof ind)) return 105;
    ind[0] = 2;
    ind[3] = 6;
    if (cp_ind_trace_take(&trace, ind, sizeof ind)) return 106;
    ind[3] = 9;
    if (cp_ind_trace_take(&trace, ind, sizeof ind)) return 107;
    ind[3] = 7;
    ind[4] = 7;
    if (cp_ind_trace_take(&trace, ind, sizeof ind) ||
        cp_ind_trace_take(&trace, ind, sizeof ind - 1)) return 108;
    if (trace.seen != 2 || trace.logged != 2 || trace.overflow) return 112;
    ind[4] = 8;
    for (unsigned i = trace.logged; i < CP_IND_TRACE_LIMIT; ++i)
        if (!cp_ind_trace_take(&trace, ind, sizeof ind)) return 109;
    if (cp_ind_trace_take(&trace, ind, sizeof ind) ||
        cp_ind_trace_take(&trace, ind, sizeof ind) ||
        trace.seen != CP_IND_TRACE_LIMIT + 2 ||
        trace.logged != CP_IND_TRACE_LIMIT || trace.overflow != 2) return 110;
    memset(&trace, 0, sizeof trace);
    if (trace.seen || trace.logged || trace.overflow ||
        !cp_ind_trace_take(&trace, ind, sizeof ind) ||
        trace.seen != 1 || trace.logged != 1) return 111;
    trace.seen = UINT_MAX;
    if (!cp_ind_trace_take(&trace, ind, sizeof ind) ||
        trace.seen != UINT_MAX || trace.logged != 2) return 113;
    if (!cp_ind_quiet_window(100, 100 + CP_IND_QUIET_COVERAGE_MS - 1) ||
        cp_ind_quiet_window(100, 100 + CP_IND_QUIET_COVERAGE_MS))
        return 114;
    return 0;
}

static int fixture(void) {
    int scan_check = fixture_scan();
    if (scan_check) return scan_check;
    int slot_check = fixture_slot_status();
    if (slot_check) return slot_check;
    int refresh_check = fixture_sim_refresh();
    if (refresh_check) return refresh_check;
    int factory_check = fixture_factory();
    if (factory_check) return factory_check;
    int signal_check = fixture_signal_mask();
    if (signal_check) return signal_check;
    int cp_ind_check = fixture_cp_ind_trace();
    if (cp_ind_check) return cp_ind_check;
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
        make_get_request(get, SIT_NET_SELECTION_AUTO, 1) == 0 ||
        make_get_request(get, SIT_NET_ALLOW_DATA, 1) == 0) return 14;
    for (size_t i = 0; i < FACTORY_GET_COUNT; ++i) {
        if (make_get_request(get, factory_gets[i].id, (uint32_t)i + 1) ||
            get[0] != 0 || le16(get + 2) != factory_gets[i].id ||
            le16(get + 4) != 12 || le32(get + 6) != i + 1 ||
            get[10] != 0 || get[11] != 0) return 70;
    }
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
    puts("PASS modem-channel-owner framing, slot-status, bounded status-query and CP-indication trace fixtures");
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
    int64_t last_rx_ms;
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
    struct cp_ind_trace cp_ind_trace;
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
static void report_metadata(struct metadata_counts *counts, int64_t now,
                            int quiet_coverage) {
    if (!counts->ipc_total && !counts->rfs_total && !quiet_coverage) {
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
    printf(" cp_ind_seen=%u cp_ind_logged=%u cp_ind_overflow=%llu"
           " cp_ind_limit=%u cp_ind_window_ms=%u",
           counts->cp_ind_trace.seen, counts->cp_ind_trace.logged,
           (unsigned long long)counts->cp_ind_trace.overflow,
           CP_IND_TRACE_LIMIT, HEARTBEAT_MS);
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

static void trace_cp_ind(struct cp_ind_trace *trace, const uint8_t *p,
                         size_t size, int64_t now_ms, int64_t owner_start_ms) {
    if (!cp_ind_trace_take(trace, p, size)) return;
    printf("cp_ind elapsed_ms=%lld id=0x%04x length=%zu\n",
           (long long)(now_ms - owner_start_ms), le16(p + 2), size);
}

static void print_sim_fields(const uint8_t *p, size_t size) {
    struct sim_status_scalars status;
    if (!sim_status_scalars(p, size, &status)) {
        printf(" status=short");
        return;
    }
    printf(" card_raw=%u apps=%u", status.card, status.apps);
    if (status.full_app_record)
        printf(" universal_pin_raw=%u app_type_raw=%u app_state_raw=%u"
               " perso_raw=%u pin1_raw=%u pin1_remaining_raw=%u",
               status.universal_pin, status.app_type, status.app_state,
               status.perso, status.pin1, status.pin1_remaining);
}

static void print_status_fields(unsigned id, const uint8_t *p, size_t size) {
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
    print_status_fields(id, p, size);
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

static void expire_sim_query(struct sim_refresh *refresh, int64_t now_ms) {
    size_t settled_index = refresh->settled_next;
    size_t factory_index = refresh->factory_next;
    enum sim_query_kind kind = sim_refresh_expire(refresh, now_ms);
    if (kind == SIM_QUERY_CHANGE)
        printf("sim_refresh response=no request=%u status=timeout\n",
               refresh->sent);
    else if (kind == SIM_QUERY_SETTLED)
        printf("sim_settled %s response=no status=timeout\n",
               snapshot_gets[settled_index].label);
    else if (kind == SIM_QUERY_FACTORY)
        printf("net_factory %s response=no status=timeout no-retry stopped=yes\n",
               factory_gets[factory_index].label);
}

static void sim_refresh_reply(struct sim_refresh *refresh,
                              struct scan_evidence *evidence,
                              const uint8_t *p, size_t size,
                              int64_t now_ms) {
    size_t settled_index = refresh->settled_next;
    size_t factory_index = refresh->factory_next;
    enum sim_query_kind kind = sim_refresh_match_reply(refresh, p, size, now_ms);
    if (kind == SIM_QUERY_NONE) return;
    if (kind == SIM_QUERY_FACTORY) {
        unsigned id = factory_gets[factory_index].id;
        uint32_t scalar = 0;
        int ok = factory_reply_ok(refresh, id, p, size, &scalar);
        if (ok) scan_evidence_note_factory(evidence, id, p, size,
                                           refresh->token, now_ms);
        printf("net_factory %s response=yes success=%s length=%zu error_raw=%u",
               factory_gets[factory_index].label, ok ? "yes" : "no",
               size, (unsigned)p[10]);
        if (ok && id == SIT_NET_SELECTION_MODE)
            printf(" mode_raw=%u", (unsigned)scalar);
        else if (ok && id == SIT_NET_PREFERRED_GET)
            printf(" preferred_raw=%u", (unsigned)scalar);
        else if (ok && id == 0x0900)
            printf(" mask_low7=%u", (unsigned)scalar);
        else if (!ok && !p[10])
            printf(" status=invalid-scalar stopped=yes");
        else if (!ok)
            printf(" stopped=yes");
        putchar('\n');
        return;
    }
    if (kind == SIM_QUERY_CHANGE)
        printf("sim_refresh response=yes request=%u error_raw=%u",
               refresh->sent, (unsigned)p[10]);
    else {
        scan_evidence_note_status(evidence, (unsigned)settled_index,
                                   p, size, refresh->token, now_ms);
        printf("sim_settled %s response=yes error_raw=%u",
               snapshot_gets[settled_index].label, (unsigned)p[10]);
    }
    if (!p[10])
        print_status_fields(kind == SIM_QUERY_CHANGE ? 0x0200 :
                            snapshot_gets[settled_index].id, p, size);
    putchar('\n');
}

static void sim_refresh_advance(struct sim_refresh *refresh,
                                struct snapshot *snapshot, int ipc_fd,
                                int64_t now_ms, int64_t owner_start_ms) {
    expire_sim_query(refresh, now_ms);
    int initial_done = snapshot_finished(snapshot);
    enum sim_query_kind kind;
    if (sim_refresh_ready(refresh, initial_done, now_ms))
        kind = SIM_QUERY_CHANGE;
    else if (sim_settled_ready(refresh, initial_done, now_ms,
                               owner_start_ms))
        kind = SIM_QUERY_SETTLED;
    else if (sim_factory_ready(refresh, initial_done))
        kind = SIM_QUERY_FACTORY;
    else return;
    unsigned id = kind == SIM_QUERY_CHANGE ? 0x0200 :
                  kind == SIM_QUERY_SETTLED ?
                  snapshot_gets[refresh->settled_next].id :
                  factory_gets[refresh->factory_next].id;
    uint8_t request[12];
    uint32_t token = ++snapshot->token;
    if (make_get_request(request, id, token) ||
        write(ipc_fd, request, sizeof request) != (ssize_t)sizeof request) {
        if (kind == SIM_QUERY_CHANGE)
            puts("sim_refresh request=failed no-retry disabled=yes");
        else
            printf("%s %s request=failed no-retry disabled=yes\n",
                   kind == SIM_QUERY_SETTLED ? "sim_settled" : "net_factory",
                   kind == SIM_QUERY_SETTLED ?
                   snapshot_gets[refresh->settled_next].label :
                   factory_gets[refresh->factory_next].label);
        refresh->disabled = 1;
        refresh->queued = 0;
        if (kind == SIM_QUERY_SETTLED) refresh->settled_started = 1;
        if (kind == SIM_QUERY_FACTORY) refresh->factory_stopped = 1;
        return;
    }
    if (kind == SIM_QUERY_CHANGE) {
        sim_refresh_mark_sent(refresh, token, now_ms);
        printf("sim_refresh request=sent request=%u\n", refresh->sent);
    } else if (kind == SIM_QUERY_SETTLED) {
        sim_settled_mark_sent(refresh, token, now_ms);
        printf("sim_settled %s request=sent\n",
               snapshot_gets[refresh->settled_next].label);
    } else {
        sim_factory_mark_sent(refresh, token, now_ms);
        printf("net_factory %s request=sent\n",
               factory_gets[refresh->factory_next].label);
    }
}

static int live_scan_active(const struct live_scan *scan) {
    return scan->phase == LIVE_SCAN_WAITING ||
           scan->phase == LIVE_SCAN_CANCEL_READY ||
           scan->phase == LIVE_SCAN_CANCEL_WAITING;
}

static const char *live_scan_result_name(enum live_scan_result result) {
    switch (result) {
    case LIVE_SCAN_GATE_NOT_MET: return "gate-not-met";
    case LIVE_SCAN_COUNT: return "network-count";
    case LIVE_SCAN_ERROR: return "remote-error";
    case LIVE_SCAN_MALFORMED_CANCEL_ACK: return "malformed-cancel-acked";
    case LIVE_SCAN_TIMEOUT_CANCEL_ACK: return "timeout-cancel-acked";
    case LIVE_SCAN_WRITE_CANCEL_ACK: return "write-ambiguous-cancel-acked";
    case LIVE_SCAN_GATE_LOST_CANCEL_ACK: return "gate-lost-cancel-acked";
    case LIVE_SCAN_CANCEL_ERROR: return "cancel-error";
    case LIVE_SCAN_CANCEL_MALFORMED: return "cancel-malformed";
    case LIVE_SCAN_CANCEL_TIMEOUT: return "cancel-timeout";
    case LIVE_SCAN_CANCEL_NOT_SENT: return "cancel-not-sent";
    case LIVE_SCAN_CANCEL_WRITE_AMBIGUOUS: return "cancel-write-ambiguous";
    case LIVE_SCAN_FRAMING_CANCEL_SENT: return "framing-cancel-sent";
    case LIVE_SCAN_FRAMING_CANCEL_AMBIGUOUS:
        return "framing-cancel-ambiguous";
    case LIVE_SCAN_FRAMING_ALREADY_CANCELING:
        return "framing-already-canceling";
    case LIVE_SCAN_CP_LOST: return "cp-lost";
    default: return "none";
    }
}

/* This is the existing owner, not a second IPC reader. An exclusive-open
 * observation and empty AP queue prove no competing AP requests right now;
 * they do not prove the CP/eUICC RF is idle. The explicit scan variant is a
 * reviewed risk policy for one active diagnostic attempt only. */
static void live_scan_advance(struct live_scan *scan, int armed,
                              const struct scan_evidence *evidence,
                              struct snapshot *snapshot,
                              const struct sim_refresh *refresh,
                              int ipc_fd, int rfs_fd,
                              size_t ipc_buffered, size_t rfs_buffered,
                              int64_t ipc_backoff_ms, int64_t rfs_backoff_ms,
                              int64_t last_rx_ms,
                              int64_t now_ms) {
    if (!armed) return;
    live_scan_tick(scan, now_ms);
    if (scan->phase == LIVE_SCAN_IDLE) {
        if (snapshot->aborted || refresh->disabled ||
            refresh->factory_stopped) {
            scan->phase = LIVE_SCAN_DONE;
            scan->result = LIVE_SCAN_GATE_NOT_MET;
        }
    }
    if (scan->phase == LIVE_SCAN_IDLE) {
        if (!snapshot_finished(snapshot) || !refresh->settled_started ||
            refresh->settled_next != SNAPSHOT_GET_COUNT ||
            refresh->factory_next != FACTORY_GET_COUNT ||
            refresh->pending_kind != SIM_QUERY_NONE || refresh->queued)
            return;
        if (!scan->gate_started_ms) scan->gate_started_ms = now_ms;
        if (now_ms - scan->gate_started_ms >= SCAN_GATE_WAIT_MS) {
            scan->phase = LIVE_SCAN_DONE;
            scan->result = LIVE_SCAN_GATE_NOT_MET;
            goto report_scan;
        }
        if (!scan_evidence_ready(evidence, now_ms)) {
            scan->phase = LIVE_SCAN_DONE;
            scan->result = LIVE_SCAN_GATE_NOT_MET;
        } else if (ipc_buffered || rfs_buffered ||
                   now_ms < ipc_backoff_ms || now_ms < rfs_backoff_ms ||
                   now_ms - last_rx_ms < 500) {
            /* First drain the reply/indication stream, then observe a quiet
             * interval before checking ownership and dispatching active RF. */
            return;
        } else {
            struct pollfd pending[2] = {
                {.fd = ipc_fd, .events = POLLIN},
                {.fd = rfs_fd, .events = POLLIN}
            };
            if (poll(pending, 2, 0) != 0) return;
            if (cp_state() != CP_ONLINE ||
                opened_once(ipc_fd) || opened_once(rfs_fd)) {
                scan->phase = LIVE_SCAN_DONE;
                scan->result = LIVE_SCAN_GATE_NOT_MET;
                goto report_scan;
            }
            uint8_t request[16];
            /* The snapshot token is session-specific; no other request is
             * outstanding, and the cancel token is reserved immediately. */
            scan->scan_token = snapshot->token + 1;
            scan->cancel_token = snapshot->token + 2;
            if (!scan->scan_token || !scan->cancel_token) {
                scan->phase = LIVE_SCAN_DONE;
                scan->result = LIVE_SCAN_GATE_NOT_MET;
            } else {
                /* Recheck after the ownership ioctls; any newly queued frame
                 * must be drained before the one active RF request. */
                if (poll(pending, 2, 0) != 0 || cp_state() != CP_ONLINE)
                    return;
                snapshot->token += 2; /* Never reuse scan/cancel tokens later. */
                make_scan_request(request, scan->scan_token);
                /* A short/EAGAIN write is ambiguous after queue acceptance.
                 * Never resend 0x0706; attempt one 0x0707 cancellation. */
                ssize_t written = write(ipc_fd, request, sizeof request);
                live_scan_scan_written(scan,
                                       written == (ssize_t)sizeof request,
                                       now_ms);
                if (written != (ssize_t)sizeof request) {
                    puts("network_scan request=ambiguous no-retry cancel=required");
                } else {
                    puts("network_scan request=sent active-rf=yes payload=redacted");
                }
            }
        }
    }
    if (scan->phase == LIVE_SCAN_CANCEL_READY) {
        uint8_t request[12];
        make_cancel_request(request, scan->cancel_token);
        /* No second attempt even when the driver returns an ambiguous write. */
        ssize_t written = write(ipc_fd, request, sizeof request);
        live_scan_cancel_written(scan,
                                 written == (ssize_t)sizeof request, now_ms);
        if (written != (ssize_t)sizeof request) {
            puts("network_scan cancel=ambiguous no-retry owner=retained");
        } else {
            puts("network_scan cancel=sent once");
        }
    }
report_scan:
    if (scan->phase == LIVE_SCAN_DONE && !scan->reported) {
        printf("network_scan result=%s", live_scan_result_name(scan->result));
        if (scan->result == LIVE_SCAN_COUNT)
            printf(" count=%u", scan->count);
        if (scan->result == LIVE_SCAN_ERROR)
            printf(" error_raw=%u", scan->remote_error);
        printf(" late_scan_reply=%u", scan->late_reply_seen);
        if (scan->poisoned) printf(" owner=quarantine-until-cp-offline");
        putchar('\n');
        scan->reported = 1;
    }
}

/* Once framing is lost, no response can be trusted or resynchronized. Send
 * at most one cancel if none was dispatched, then retain both device FDs
 * without further parsing or writes until CP goes OFFLINE. */
static void live_scan_framing_poison(struct live_scan *scan, int ipc_fd) {
    if (!live_scan_active(scan)) return;
    int64_t now_ms = monotonic_ms();
    if (now_ms < 0) {
        scan->phase = LIVE_SCAN_DONE;
        scan->result = LIVE_SCAN_CANCEL_NOT_SENT;
        scan->poisoned = 1;
        puts("network_scan result=clock-error owner=quarantine-until-cp-offline");
        scan->reported = 1;
        return;
    }
    live_scan_tick(scan, now_ms);
    if (scan->phase == LIVE_SCAN_DONE) {
        /* A delayed poll failure cannot extend the cancel window. */
    } else if (scan->phase == LIVE_SCAN_CANCEL_WAITING) {
        scan->result = LIVE_SCAN_FRAMING_ALREADY_CANCELING;
    } else if (cp_state() != CP_ONLINE) {
        scan->result = LIVE_SCAN_CP_LOST;
    } else {
        uint8_t cancel[12];
        make_cancel_request(cancel, scan->cancel_token);
        ssize_t written = write(ipc_fd, cancel, sizeof cancel);
        scan->result = written == (ssize_t)sizeof cancel ?
            LIVE_SCAN_FRAMING_CANCEL_SENT :
            LIVE_SCAN_FRAMING_CANCEL_AMBIGUOUS;
    }
    scan->phase = LIVE_SCAN_DONE;
    scan->poisoned = 1;
    printf("network_scan result=%s owner=quarantine-until-cp-offline\n",
           live_scan_result_name(scan->result));
    scan->reported = 1;
}

static void trace_slot_status(struct slot_status_trace *trace,
                              const uint8_t *p, size_t size,
                              int64_t now_ms, int64_t owner_start_ms) {
    struct slot_status_scalars status;
    if (!slot_status_take(trace, p, size, &status)) return;
    printf("sim_slot_ind elapsed_ms=%lld slot_count=%u",
           (long long)(now_ms - owner_start_ms), status.slot_count);
    for (unsigned i = 0; i < status.logged_slots; ++i) {
        const struct slot_record_scalars *slot = &status.slot[i];
        printf(" slot%u_card_state_raw=%u", i, slot->card_state);
        if (slot->has_port0)
            printf(" slot%u_port0_logical_raw=%u"
                   " slot%u_port0_state_raw=%u",
                   i, slot->port0_logical, i, slot->port0_state);
    }
    putchar('\n');
}

struct live_frame_context {
    struct channel *channel;
    struct metadata_counts *counts;
    struct snapshot *snapshot;
    struct sim_refresh *refresh;
    struct scan_evidence *evidence;
    struct live_scan *scan;
    struct slot_status_trace *slot_trace;
    struct rfs_header_trace *trace;
    int64_t now_ms;
    int64_t owner_start_ms;
};

static void live_frame(void *opaque, enum channel_kind kind,
                       const uint8_t *p, int size) {
    struct live_frame_context *context = opaque;
    count_frame(context->channel, p, size, context->counts);
    if (kind == CHANNEL_IPC) {
        trace_cp_ind(&context->counts->cp_ind_trace, p, (size_t)size,
                     context->now_ms, context->owner_start_ms);
        trace_slot_status(context->slot_trace, p, (size_t)size,
                          context->now_ms, context->owner_start_ms);
        expire_sim_query(context->refresh, context->now_ms);
        snapshot_reply(context->snapshot, p, (size_t)size);
        sim_refresh_reply(context->refresh, context->evidence,
                          p, (size_t)size, context->now_ms);
        live_scan_frame(context->scan, p, (size_t)size, context->now_ms);
        if (scan_evidence_unsolicited(context->evidence, p, (size_t)size) &&
            context->scan->phase == LIVE_SCAN_WAITING)
            live_scan_request_cancel(context->scan,
                                     LIVE_SCAN_CAUSE_GATE_LOST,
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
                         struct scan_evidence *evidence,
                         struct live_scan *scan,
                         struct slot_status_trace *slot_trace,
                         struct rfs_header_trace *trace,
                         int64_t owner_start_ms) {
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
    channel->last_rx_ms = now;
    struct live_frame_context context = {
        .channel = channel, .counts = counts, .snapshot = snapshot,
        .refresh = refresh, .evidence = evidence, .scan = scan,
        .slot_trace = slot_trace, .trace = trace,
        .now_ms = now, .owner_start_ms = owner_start_ms
    };
    return parse_available(channel->kind, channel->rx, &channel->used,
                           live_frame, &context);
}

static int run_owner(int ipc, int rfs, int ready, int lock, int attached) {
#ifdef SAAIOS_SCAN_ONCE
    const int scan_armed = !attached;
#else
    const int scan_armed = 0;
#endif
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
    struct scan_evidence evidence = {0};
    struct live_scan scan = {0};
    struct slot_status_trace slot_trace = {0};
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
    if (scan_armed)
        puts("network_scan=armed_once policy=reviewed-rf-risk cp_rf_idle=unproven");
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
            report_metadata(&counts, now,
                            cp_ind_quiet_window(start,
                                                counts.window_start_ms));
            report_rfs_overflow(&trace);
        }
        if (scan.poisoned) {
            /* A malformed stream cannot be resynchronized safely. Keep the
             * sole IPC/RFS owner alive without more GETs or writes. */
            (void)poll(NULL, 0, POLL_SLICE_MS);
            continue;
        }
        if (state == CP_ONLINE) {
            snapshot_advance(&snapshot, ipc, now);
            if (!live_scan_active(&scan))
                sim_refresh_advance(&refresh, &snapshot, ipc, now, start);
            live_scan_advance(&scan, scan_armed, &evidence, &snapshot,
                              &refresh, ipc, rfs,
                              channels[0].used, channels[1].used,
                              channels[0].backoff_until_ms,
                              channels[1].backoff_until_ms,
                              channels[0].last_rx_ms > channels[1].last_rx_ms ?
                              channels[0].last_rx_ms : channels[1].last_rx_ms,
                              now);
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
        if (events < 0) {
            perror("owner poll");
            if (live_scan_active(&scan)) {
                live_scan_framing_poison(&scan, ipc);
                continue;
            }
            rc = 1; break;
        }
        for (int i = 0; i < 2; ++i) {
            if (fds[i].revents & (POLLERR | POLLHUP | POLLNVAL)) {
                fprintf(stderr, "%s poll failure event=0x%x\n",
                        i ? "RFS" : "IPC", fds[i].revents);
                if (live_scan_active(&scan)) {
                    live_scan_framing_poison(&scan, ipc);
                    break;
                }
                rc = 1; break;
            }
            if ((fds[i].revents & POLLIN) &&
                drain_channel(&channels[i], &counts, &snapshot, &refresh,
                              &evidence, &scan, &slot_trace, &trace, start)) {
                fprintf(stderr, "%s read/framing failure or buffer cap\n",
                        i ? "RFS" : "IPC");
                if (live_scan_active(&scan)) {
                    live_scan_framing_poison(&scan, ipc);
                    break;
                }
                rc = 1; break;
            }
        }
        if (scan.poisoned) continue;
        if (rc) break;
    }
    if (counts.ipc_total || counts.rfs_total) {
        int64_t now = monotonic_ms();
        report_metadata(&counts, now >= 0 ? now : counts.window_start_ms, 1);
    }
    report_rfs_overflow(&trace);
    if (live_scan_active(&scan)) {
        scan.phase = LIVE_SCAN_DONE;
        scan.result = LIVE_SCAN_CP_LOST;
    }
    if (scan.phase == LIVE_SCAN_DONE && !scan.reported) {
        printf("network_scan result=%s late_scan_reply=%u\n",
               live_scan_result_name(scan.result), scan.late_reply_seen);
        scan.reported = 1;
    }
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

#ifndef SAAIOS_SCAN_ONCE
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
#endif

int main(int argc, char **argv) {
    if (argc == 2 && !strcmp(argv[1], "self-test")) return fixture();
    if (argc == 2 && !strcmp(argv[1], "--mode")) {
#ifdef SAAIOS_SCAN_ONCE
        puts("scan-once");
#else
        puts("passive");
#endif
        return 0;
    }
#ifdef _WIN32
    (void)argc;
    (void)argv;
    fputs("Live channel ownership requires Linux.\n", stderr);
    return 69;
#else
    if (argc == 2 && !strcmp(argv[1], "--attach-online")) {
#ifdef SAAIOS_SCAN_ONCE
        fputs("ABORT scan-once requires a fresh guarded pre-FIN handoff\n", stderr);
        return 64;
#else
        return attach_online();
#endif
    }
    int ipc, rfs, ready;
    if (parse_args(argc, argv, &ipc, &rfs, &ready)) {
        fputs("usage: modem-channel-owner self-test | "
              "--attach-online | --ipc-fd N --rfs-fd N --ready-fd N\n", stderr);
        return 64;
    }
    return run_owner(ipc, rfs, ready, -1, 0);
#endif
}
