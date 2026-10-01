/* Synthetic host model only: no device paths, IPC writes, sleeps or logging. */
#include "sit-network-scan-host.h"

#include <limits.h>
#include <string.h>

enum { SCAN_ID = 0x0706, CANCEL_ID = 0x0707,
       SCAN_WAIT_MS = 300000, CANCEL_WAIT_MS = 5000,
       WRITE_CALLBACK_WAIT_MS = 5000, CANCEL_DISPATCH_WAIT_MS = 5000,
       MAX_NETWORK_COUNT = 64 };

static uint16_t read16(const uint8_t *p) {
    return (uint16_t)((uint16_t)p[0] | ((uint16_t)p[1] << 8));
}

static uint32_t read32(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static void write16(uint8_t *p, uint16_t value) {
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
}

static void write32(uint8_t *p, uint32_t value) {
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
    p[2] = (uint8_t)(value >> 16);
    p[3] = (uint8_t)(value >> 24);
}

static void request_header(uint8_t *request, size_t length,
                           uint16_t id, uint32_t token) {
    memset(request, 0, length);
    write16(request + 2, id);
    write16(request + 4, (uint16_t)length);
    write32(request + 6, token);
}

static int update_clock(struct scan_model *model, int64_t now_ms) {
    if (now_ms < model->last_ms || now_ms < 0) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CLOCK_ERROR;
        return 0;
    }
    model->last_ms = now_ms;
    return 1;
}

static void cancel_for(struct scan_model *model,
                       enum scan_cancel_cause cause) {
    int64_t prior_deadline = model->deadline_ms;
    model->cancel_cause = cause;
    if (model->last_ms > INT64_MAX - CANCEL_DISPATCH_WAIT_MS) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CLOCK_ERROR;
        return;
    }
    int64_t deadline = model->last_ms + CANCEL_DISPATCH_WAIT_MS;
    if (prior_deadline <= INT64_MAX - CANCEL_DISPATCH_WAIT_MS &&
        prior_deadline + CANCEL_DISPATCH_WAIT_MS < deadline)
        deadline = prior_deadline + CANCEL_DISPATCH_WAIT_MS;
    if (model->last_ms >= deadline) {
        /* A late event-loop tick cannot revive a long-expired scan. */
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CANCEL_NOT_DISPATCHED;
        return;
    }
    model->deadline_ms = deadline;
    model->phase = SCAN_CANCEL_READY;
}

static int gate_ok(const struct scan_gate *gate) {
    return gate && gate->explicit_opt_in && gate->exclusive_ipc_owner &&
        gate->local_requests_idle && gate->opposite_requests_idle &&
        gate->rf_policy_approved && gate->cp_online &&
        gate->same_boot_status_fresh && gate->sim_ready &&
        gate->pin_disabled && gate->radio_on &&
        gate->selection_automatic && gate->preferred_broad_verified &&
        gate->voice_unregistered && gate->data_unregistered;
}

void scan_model_init(struct scan_model *model) {
    if (model) memset(model, 0, sizeof *model);
}

int scan_model_begin(struct scan_model *model, const struct scan_gate *gate,
                     uint32_t scan_token, uint32_t cancel_token,
                     int64_t now_ms) {
    if (!model || !gate_ok(gate) || model->started ||
        model->phase != SCAN_IDLE || !scan_token || !cancel_token ||
        scan_token == cancel_token || now_ms < 0 ||
        now_ms > INT64_MAX - WRITE_CALLBACK_WAIT_MS) return 0;
    model->started = 1;
    model->scan_token = scan_token;
    model->cancel_token = cancel_token;
    model->last_ms = now_ms;
    model->deadline_ms = now_ms + WRITE_CALLBACK_WAIT_MS;
    model->phase = SCAN_DISPATCH_READY;
    return 1;
}

int scan_model_take_scan(struct scan_model *model, const struct scan_gate *gate,
                         int64_t now_ms, uint8_t request[16]) {
    if (!model || !request || model->phase != SCAN_DISPATCH_READY) return 0;
    scan_model_tick(model, now_ms);
    if (model->phase != SCAN_DISPATCH_READY) return 0;
    if (!gate_ok(gate)) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_GATE_LOST;
        return 0;
    }
    if (now_ms > INT64_MAX - WRITE_CALLBACK_WAIT_MS) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CLOCK_ERROR;
        return 0;
    }
    /* TD1A BuildQueryAvailableNetwork(0): 12-byte SIT header + LE32 zero. */
    request_header(request, 16, SCAN_ID, model->scan_token);
    model->deadline_ms = now_ms + WRITE_CALLBACK_WAIT_MS;
    model->phase = SCAN_WRITE_PENDING;
    return 1;
}

void scan_model_scan_write(struct scan_model *model,
                           enum scan_write_result result, int64_t now_ms) {
    if (!model || model->phase != SCAN_WRITE_PENDING ||
        !update_clock(model, now_ms)) return;
    scan_model_tick(model, now_ms);
    if (model->phase != SCAN_WRITE_PENDING) return;
    if (result != SCAN_WRITE_EXACT) {
        /* Even zero or short writes can be ambiguous after queue acceptance. */
        cancel_for(model, SCAN_CANCEL_SCAN_WRITE_AMBIGUOUS);
    } else if (now_ms > INT64_MAX - SCAN_WAIT_MS) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CLOCK_ERROR;
    } else {
        model->deadline_ms = now_ms + SCAN_WAIT_MS;
        model->phase = SCAN_WAITING;
    }
}

void scan_model_tick(struct scan_model *model, int64_t now_ms) {
    if (!model || model->phase == SCAN_IDLE || model->phase == SCAN_DONE ||
        !update_clock(model, now_ms)) return;
    if (model->phase == SCAN_DISPATCH_READY &&
        now_ms >= model->deadline_ms) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_SCAN_NOT_DISPATCHED;
    } else if (model->phase == SCAN_WRITE_PENDING &&
        now_ms >= model->deadline_ms) {
        cancel_for(model, SCAN_CANCEL_SCAN_WRITE_AMBIGUOUS);
    } else if (model->phase == SCAN_WAITING &&
               now_ms >= model->deadline_ms) {
        cancel_for(model, SCAN_CANCEL_SCAN_TIMEOUT);
    } else if (model->phase == SCAN_CANCEL_READY &&
               now_ms >= model->deadline_ms) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CANCEL_NOT_DISPATCHED;
    } else if (model->phase == SCAN_CANCEL_WRITE_PENDING &&
               now_ms >= model->deadline_ms) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CANCEL_WRITE_AMBIGUOUS;
    } else if (model->phase == SCAN_CANCEL_WAITING &&
               now_ms >= model->deadline_ms) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CANCEL_TIMEOUT;
    }
}

int scan_model_take_cancel(struct scan_model *model, int64_t now_ms,
                           uint8_t request[12]) {
    if (!model || !request || model->phase != SCAN_CANCEL_READY) return 0;
    scan_model_tick(model, now_ms);
    if (model->phase != SCAN_CANCEL_READY) return 0;
    if (model->last_ms > INT64_MAX - WRITE_CALLBACK_WAIT_MS) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CLOCK_ERROR;
        return 0;
    }
    request_header(request, 12, CANCEL_ID, model->cancel_token);
    model->deadline_ms = now_ms + WRITE_CALLBACK_WAIT_MS;
    model->phase = SCAN_CANCEL_WRITE_PENDING;
    return 1;
}

void scan_model_cancel_write(struct scan_model *model,
                             enum scan_write_result result, int64_t now_ms) {
    if (!model || model->phase != SCAN_CANCEL_WRITE_PENDING ||
        !update_clock(model, now_ms)) return;
    scan_model_tick(model, now_ms);
    if (model->phase != SCAN_CANCEL_WRITE_PENDING) return;
    if (result != SCAN_WRITE_EXACT) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CANCEL_WRITE_AMBIGUOUS;
    } else if (now_ms > INT64_MAX - CANCEL_WAIT_MS) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_CLOCK_ERROR;
    } else {
        model->deadline_ms = now_ms + CANCEL_WAIT_MS;
        model->phase = SCAN_CANCEL_WAITING;
    }
}

static int addressed_to(const uint8_t *frame, size_t length,
                        uint16_t id, uint32_t token) {
    return length >= 10 && read16(frame + 2) == id &&
           read32(frame + 6) == token;
}

void scan_model_frame(struct scan_model *model, const uint8_t *frame,
                      size_t length, int64_t now_ms) {
    if (!model || !frame || model->phase == SCAN_IDLE ||
        model->phase == SCAN_DONE) return;
    scan_model_tick(model, now_ms); /* deadline wins over a late frame */
    if (model->phase == SCAN_DONE) return;
    if (model->phase == SCAN_CANCEL_READY ||
        model->phase == SCAN_CANCEL_WRITE_PENDING ||
        model->phase == SCAN_CANCEL_WAITING) {
        if (addressed_to(frame, length, SCAN_ID, model->scan_token))
            model->late_scan_reply_seen = 1; /* never reinterpret as success */
        if ((model->phase != SCAN_CANCEL_WAITING &&
             model->phase != SCAN_CANCEL_WRITE_PENDING) ||
            !addressed_to(frame, length, CANCEL_ID, model->cancel_token))
            return;
        model->phase = SCAN_DONE;
        if (length != 12 || frame[0] != 1 || read16(frame + 4) != length)
            model->result = SCAN_RESULT_CANCEL_MALFORMED;
        else if (read16(frame + 10) != 0)
            model->result = SCAN_RESULT_CANCEL_ERROR;
        else
            model->result = SCAN_RESULT_CANCEL_ACKED;
        return;
    }
    if ((model->phase != SCAN_WAITING &&
         model->phase != SCAN_WRITE_PENDING) ||
        !addressed_to(frame, length, SCAN_ID, model->scan_token)) return;
    if (length < 12 || frame[0] != 1 || read16(frame + 4) != length) {
        cancel_for(model, SCAN_CANCEL_SCAN_MALFORMED);
    } else if (read16(frame + 10) != 0) {
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_REMOTE_ERROR;
    } else if (length < 16 || read32(frame + 12) > MAX_NETWORK_COUNT ||
               read32(frame + 12) > (length - 16) / 14) {
        /* Never read, copy or log the 14-byte entries (PLMN/name material). */
        cancel_for(model, SCAN_CANCEL_SCAN_MALFORMED);
    } else {
        model->network_count = read32(frame + 12);
        model->phase = SCAN_DONE;
        model->result = SCAN_RESULT_NETWORK_COUNT;
    }
}

void scan_model_cp_lost(struct scan_model *model) {
    if (!model || model->phase == SCAN_IDLE || model->phase == SCAN_DONE) return;
    model->phase = SCAN_DONE;
    model->result = SCAN_RESULT_CP_LOST;
}
