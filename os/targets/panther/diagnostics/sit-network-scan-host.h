/* Pure, host-only TD1A available-network scan model. Never link into a device build. */
#ifndef SAAIOS_SIT_NETWORK_SCAN_HOST_H
#define SAAIOS_SIT_NETWORK_SCAN_HOST_H
#ifndef SAAIOS_NETWORK_SCAN_HOST_ONLY
#error "SIT network scan model is host-only; no device transport is implemented"
#endif
#if defined(__arm__) || defined(__aarch64__)
#error "SIT network scan model must not be compiled for the phone"
#endif

#include <stddef.h>
#include <stdint.h>

enum scan_phase {
    SCAN_IDLE, SCAN_DISPATCH_READY, SCAN_WRITE_PENDING, SCAN_WAITING,
    SCAN_CANCEL_READY, SCAN_CANCEL_WRITE_PENDING,
    SCAN_CANCEL_WAITING, SCAN_DONE
};

enum scan_result {
    SCAN_RESULT_NONE, SCAN_RESULT_NETWORK_COUNT,
    SCAN_RESULT_GATE_LOST, SCAN_RESULT_SCAN_NOT_DISPATCHED,
    SCAN_RESULT_REMOTE_ERROR, SCAN_RESULT_CANCEL_ACKED,
    SCAN_RESULT_CANCEL_ERROR, SCAN_RESULT_CANCEL_MALFORMED,
    SCAN_RESULT_CANCEL_TIMEOUT, SCAN_RESULT_CANCEL_NOT_DISPATCHED,
    SCAN_RESULT_CANCEL_WRITE_AMBIGUOUS,
    SCAN_RESULT_CP_LOST, SCAN_RESULT_CLOCK_ERROR
};

enum scan_cancel_cause {
    SCAN_CANCEL_NONE, SCAN_CANCEL_SCAN_TIMEOUT,
    SCAN_CANCEL_SCAN_MALFORMED, SCAN_CANCEL_SCAN_WRITE_AMBIGUOUS
};

enum scan_write_result { SCAN_WRITE_EXACT, SCAN_WRITE_AMBIGUOUS };

/* All fields are external conditions for a future reviewed owner.
 * rf_policy_approved is a human-reviewed risk gate, NOT an RF measurement.
 * CP/eSIM RF idleness cannot be inferred from reg=0 or slot metadata. */
struct scan_gate {
    int explicit_opt_in;
    int exclusive_ipc_owner;
    int local_requests_idle;
    int opposite_requests_idle;
    int rf_policy_approved;
    int cp_online;
    int same_boot_status_fresh;
    int sim_ready;
    int pin_disabled;
    int radio_on;
    int selection_automatic;
    int preferred_broad_verified;
    int voice_unregistered;
    int data_unregistered;
};

struct scan_model {
    enum scan_phase phase;
    enum scan_result result;
    enum scan_cancel_cause cancel_cause;
    uint32_t scan_token;
    uint32_t cancel_token;
    uint32_t network_count; /* count only; no PLMN or name bytes retained */
    int64_t last_ms;
    int64_t deadline_ms;
    unsigned late_scan_reply_seen;
    unsigned started;
};

void scan_model_init(struct scan_model *model);
int scan_model_begin(struct scan_model *model, const struct scan_gate *gate,
                     uint32_t scan_token, uint32_t cancel_token,
                     int64_t now_ms);
int scan_model_take_scan(struct scan_model *model, const struct scan_gate *gate,
                         int64_t now_ms, uint8_t request[16]);
void scan_model_scan_write(struct scan_model *model,
                           enum scan_write_result result, int64_t now_ms);
void scan_model_tick(struct scan_model *model, int64_t now_ms);
int scan_model_take_cancel(struct scan_model *model, int64_t now_ms,
                           uint8_t request[12]);
void scan_model_cancel_write(struct scan_model *model,
                             enum scan_write_result result, int64_t now_ms);
void scan_model_frame(struct scan_model *model, const uint8_t *frame,
                      size_t length, int64_t now_ms);
void scan_model_cp_lost(struct scan_model *model);

#endif
