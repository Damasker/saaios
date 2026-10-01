#include "sit-network-scan-host.h"

#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>

enum { SCAN_TOKEN = 0x10203040, CANCEL_TOKEN = 0x10203041 };

static void put16(uint8_t *p, uint16_t value) {
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
}

static void put32(uint8_t *p, uint32_t value) {
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
    p[2] = (uint8_t)(value >> 16);
    p[3] = (uint8_t)(value >> 24);
}

static struct scan_gate valid_gate(void) {
    struct scan_gate gate = {
        .explicit_opt_in = 1, .exclusive_ipc_owner = 1,
        .local_requests_idle = 1, .opposite_requests_idle = 1,
        .rf_policy_approved = 1, .cp_online = 1,
        .same_boot_status_fresh = 1, .sim_ready = 1,
        .pin_disabled = 1, .radio_on = 1,
        .selection_automatic = 1, .preferred_broad_verified = 1,
        .voice_unregistered = 1, .data_unregistered = 1
    };
    return gate;
}

static void begin(struct scan_model *model) {
    uint8_t request[16];
    struct scan_gate gate = valid_gate();
    scan_model_init(model);
    assert(scan_model_begin(model, &gate, SCAN_TOKEN, CANCEL_TOKEN,
                            100));
    assert(scan_model_take_scan(model, &gate, 100, request));
    static const uint8_t expected[16] = {
        0,0, 0x06,0x07, 16,0, 0x40,0x30,0x20,0x10, 0,0, 0,0,0,0
    };
    assert(memcmp(request, expected, sizeof expected) == 0);
    assert(!scan_model_begin(model, &gate, SCAN_TOKEN + 2,
                             CANCEL_TOKEN + 2, 101));
    scan_model_scan_write(model, SCAN_WRITE_EXACT, 100);
    assert(model->phase == SCAN_WAITING);
    assert(model->deadline_ms == 300100);
}

static void reply(uint8_t *p, size_t length, uint16_t id, uint32_t token,
                  uint16_t error) {
    memset(p, 0, length);
    p[0] = 1;
    put16(p + 2, id);
    put16(p + 4, (uint16_t)length);
    put32(p + 6, token);
    put16(p + 10, error);
}

static void gate_tests(void) {
    struct scan_model model;
    struct scan_gate gate = valid_gate();
    uint8_t request[16];
    memset(request, 0xa5, sizeof request);
    int *fields[] = {
        &gate.explicit_opt_in, &gate.exclusive_ipc_owner,
        &gate.local_requests_idle, &gate.opposite_requests_idle,
        &gate.rf_policy_approved, &gate.cp_online,
        &gate.same_boot_status_fresh, &gate.sim_ready,
        &gate.pin_disabled, &gate.radio_on,
        &gate.selection_automatic, &gate.preferred_broad_verified,
        &gate.voice_unregistered, &gate.data_unregistered
    };
    for (size_t i = 0; i < sizeof fields / sizeof fields[0]; ++i) {
        scan_model_init(&model);
        *fields[i] = 0;
        assert(!scan_model_begin(&model, &gate, SCAN_TOKEN,
                                 CANCEL_TOKEN, 0));
        assert(model.phase == SCAN_IDLE && !model.started);
        *fields[i] = 1;
    }
    for (size_t i = 0; i < sizeof request; ++i) assert(request[i] == 0xa5);
    assert(!scan_model_begin(&model, &gate, 0, CANCEL_TOKEN, 0));
    assert(!scan_model_begin(&model, &gate, SCAN_TOKEN, SCAN_TOKEN,
                             0));
    assert(!scan_model_begin(&model, &gate, SCAN_TOKEN, CANCEL_TOKEN,
                             -1));
    assert(model.phase == SCAN_IDLE && !model.started);

    /* Conditions must still hold immediately before actual dispatch. */
    scan_model_init(&model);
    assert(scan_model_begin(&model, &gate, SCAN_TOKEN, CANCEL_TOKEN, 100));
    gate.same_boot_status_fresh = 0;
    assert(!scan_model_take_scan(&model, &gate, 101, request));
    assert(model.phase == SCAN_DONE && model.result == SCAN_RESULT_GATE_LOST);
}

static void success_and_error_tests(void) {
    struct scan_model model;
    uint8_t frame[44];
    begin(&model);
    reply(frame, 16, 0x0706, SCAN_TOKEN, 0);
    scan_model_frame(&model, frame, 16, 101);
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_NETWORK_COUNT &&
           model.network_count == 0 && model.cancel_cause == SCAN_CANCEL_NONE);
    assert(!scan_model_take_cancel(&model, 101, frame));

    begin(&model);
    reply(frame, sizeof frame, 0x0706, SCAN_TOKEN, 0);
    put32(frame + 12, 2);
    memset(frame + 16, 0x5a, sizeof frame - 16); /* opaque PLMN/name data */
    scan_model_frame(&model, frame, sizeof frame, 101);
    assert(model.result == SCAN_RESULT_NETWORK_COUNT &&
           model.network_count == 2 && model.phase == SCAN_DONE);

    begin(&model);
    reply(frame, 12, 0x0706, SCAN_TOKEN, 2);
    scan_model_frame(&model, frame, 12, 101);
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_REMOTE_ERROR);

    begin(&model);
    reply(frame, 12, 0x0706, SCAN_TOKEN, 0x0100);
    scan_model_frame(&model, frame, 12, 101);
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_REMOTE_ERROR);
}

static void malformed_and_unrelated_tests(void) {
    struct scan_model model;
    uint8_t frame[30];
    begin(&model);
    reply(frame, 16, 0x0706, SCAN_TOKEN + 2, 0);
    scan_model_frame(&model, frame, 16, 101);
    reply(frame, 16, 0x0705, SCAN_TOKEN, 0);
    scan_model_frame(&model, frame, 16, 102);
    assert(model.phase == SCAN_WAITING);

    reply(frame, 16, 0x0706, SCAN_TOKEN, 0);
    put32(frame + 12, 1); /* one entry requires 30 bytes */
    scan_model_frame(&model, frame, 16, 103);
    assert(model.phase == SCAN_CANCEL_READY &&
           model.cancel_cause == SCAN_CANCEL_SCAN_MALFORMED &&
           model.network_count == 0);

    begin(&model);
    reply(frame, 16, 0x0706, SCAN_TOKEN, 0);
    frame[4] = 15;
    scan_model_frame(&model, frame, 16, 101);
    assert(model.phase == SCAN_CANCEL_READY);

    begin(&model);
    reply(frame, 16, 0x0706, SCAN_TOKEN, 0);
    frame[0] = 2;
    scan_model_frame(&model, frame, 16, 101);
    assert(model.phase == SCAN_CANCEL_READY);

    begin(&model);
    reply(frame, 12, 0x0706, SCAN_TOKEN, 0);
    scan_model_frame(&model, frame, 12, 101);
    assert(model.phase == SCAN_CANCEL_READY);

    begin(&model);
    reply(frame, sizeof frame, 0x0706, SCAN_TOKEN, 0);
    put32(frame + 12, UINT32_MAX);
    scan_model_frame(&model, frame, sizeof frame, 101);
    assert(model.phase == SCAN_CANCEL_READY && model.network_count == 0);

    /* Length permits 65 entries; the separate policy cap is 64. */
    uint8_t many[16 + 14 * 65];
    begin(&model);
    reply(many, sizeof many, 0x0706, SCAN_TOKEN, 0);
    put32(many + 12, 65);
    scan_model_frame(&model, many, sizeof many, 101);
    assert(model.phase == SCAN_CANCEL_READY && model.network_count == 0);
    begin(&model);
    reply(many, sizeof many, 0x0706, SCAN_TOKEN, 0);
    put32(many + 12, 64);
    scan_model_frame(&model, many, sizeof many, 101);
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_NETWORK_COUNT &&
           model.network_count == 64);
}

static void timeout_cancel_race_test(void) {
    struct scan_model model;
    uint8_t frame[16], cancel[12];
    static const uint8_t expected_cancel[12] = {
        0,0, 0x07,0x07, 12,0, 0x41,0x30,0x20,0x10, 0,0
    };
    begin(&model);
    scan_model_tick(&model, 300099);
    assert(model.phase == SCAN_WAITING);
    reply(frame, 16, 0x0706, SCAN_TOKEN, 0);
    scan_model_frame(&model, frame, 16, 300100); /* exact deadline is late */
    assert(model.phase == SCAN_CANCEL_READY &&
           model.cancel_cause == SCAN_CANCEL_SCAN_TIMEOUT &&
           model.late_scan_reply_seen && !model.network_count);
    assert(scan_model_take_cancel(&model, 300100, cancel));
    assert(memcmp(cancel, expected_cancel, sizeof cancel) == 0);
    assert(!scan_model_take_cancel(&model, 300100, cancel)); /* no retry */
    scan_model_cancel_write(&model, SCAN_WRITE_EXACT, 300101);
    assert(model.phase == SCAN_CANCEL_WAITING &&
           model.deadline_ms == 305101);
    reply(frame, 12, 0x0707, CANCEL_TOKEN + 1, 0);
    scan_model_frame(&model, frame, 12, 300102);
    assert(model.phase == SCAN_CANCEL_WAITING);
    reply(frame, 16, 0x0706, SCAN_TOKEN, 0);
    put32(frame + 12, 0);
    scan_model_frame(&model, frame, 16, 300103);
    assert(model.phase == SCAN_CANCEL_WAITING &&
           model.result == SCAN_RESULT_NONE);
    reply(frame, 12, 0x0707, CANCEL_TOKEN, 0);
    scan_model_frame(&model, frame, 12, 300104);
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_CANCEL_ACKED &&
           model.cancel_cause == SCAN_CANCEL_SCAN_TIMEOUT);
    scan_model_frame(&model, frame, 12, 300105); /* duplicate ignored */
    assert(model.result == SCAN_RESULT_CANCEL_ACKED);
}

static void cancel_outcome_tests(void) {
    struct scan_model model;
    uint8_t frame[16], cancel[12];
    begin(&model);
    scan_model_tick(&model, 300100);
    assert(scan_model_take_cancel(&model, 300100, cancel));
    scan_model_cancel_write(&model, SCAN_WRITE_EXACT, 300101);
    scan_model_tick(&model, 305100);
    assert(model.phase == SCAN_CANCEL_WAITING);
    scan_model_tick(&model, 305101);
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_CANCEL_TIMEOUT);

    begin(&model);
    scan_model_tick(&model, 300100);
    assert(scan_model_take_cancel(&model, 300100, cancel));
    scan_model_cancel_write(&model, SCAN_WRITE_EXACT, 300101);
    reply(frame, 12, 0x0707, CANCEL_TOKEN, 2);
    scan_model_frame(&model, frame, 12, 300102);
    assert(model.result == SCAN_RESULT_CANCEL_ERROR);

    begin(&model);
    scan_model_tick(&model, 300100);
    assert(scan_model_take_cancel(&model, 300100, cancel));
    scan_model_cancel_write(&model, SCAN_WRITE_EXACT, 300101);
    reply(frame, 12, 0x0707, CANCEL_TOKEN, 0);
    frame[4] = 13;
    scan_model_frame(&model, frame, 12, 300102);
    assert(model.result == SCAN_RESULT_CANCEL_MALFORMED);
}

static void ambiguous_and_cp_loss_tests(void) {
    struct scan_model model;
    struct scan_gate gate = valid_gate();
    uint8_t request[16], cancel[12];
    scan_model_init(&model);
    assert(scan_model_begin(&model, &gate, SCAN_TOKEN, CANCEL_TOKEN,
                            100));
    assert(scan_model_take_scan(&model, &gate, 100, request));
    scan_model_scan_write(&model, SCAN_WRITE_AMBIGUOUS, 101);
    assert(model.phase == SCAN_CANCEL_READY &&
           model.cancel_cause == SCAN_CANCEL_SCAN_WRITE_AMBIGUOUS);
    scan_model_scan_write(&model, SCAN_WRITE_EXACT, 102);
    assert(model.phase == SCAN_CANCEL_READY);
    assert(scan_model_take_cancel(&model, 101, cancel));
    scan_model_cancel_write(&model, SCAN_WRITE_AMBIGUOUS, 103);
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_CANCEL_WRITE_AMBIGUOUS &&
           !scan_model_take_cancel(&model, 103, cancel));

    scan_model_init(&model);
    assert(scan_model_begin(&model, &gate, SCAN_TOKEN, CANCEL_TOKEN,
                            100));
    scan_model_cp_lost(&model);
    assert(model.phase == SCAN_DONE && model.result == SCAN_RESULT_CP_LOST);
    begin(&model);
    scan_model_cp_lost(&model);
    assert(model.result == SCAN_RESULT_CP_LOST);
    begin(&model);
    scan_model_tick(&model, 300100);
    scan_model_cp_lost(&model);
    assert(model.result == SCAN_RESULT_CP_LOST &&
           !scan_model_take_cancel(&model, 300100, cancel));
    begin(&model);
    scan_model_tick(&model, 300100);
    assert(scan_model_take_cancel(&model, 300100, cancel));
    scan_model_cp_lost(&model);
    assert(model.result == SCAN_RESULT_CP_LOST);
    begin(&model);
    scan_model_tick(&model, 300100);
    assert(scan_model_take_cancel(&model, 300100, cancel));
    scan_model_cancel_write(&model, SCAN_WRITE_EXACT, 300101);
    scan_model_cp_lost(&model);
    assert(model.result == SCAN_RESULT_CP_LOST);
}

static void dispatch_deadline_tests(void) {
    struct scan_model model;
    struct scan_gate gate = valid_gate();
    uint8_t request[16], cancel[12];

    scan_model_init(&model);
    assert(scan_model_begin(&model, &gate, SCAN_TOKEN, CANCEL_TOKEN,
                            100));
    assert(scan_model_take_scan(&model, &gate, 100, request));
    scan_model_tick(&model, 5099);
    assert(model.phase == SCAN_WRITE_PENDING);
    scan_model_scan_write(&model, SCAN_WRITE_EXACT, 5100);
    assert(model.phase == SCAN_CANCEL_READY &&
           model.cancel_cause == SCAN_CANCEL_SCAN_WRITE_AMBIGUOUS);
    assert(scan_model_take_cancel(&model, 5100, cancel));
    scan_model_tick(&model, 10099);
    assert(model.phase == SCAN_CANCEL_WRITE_PENDING);
    scan_model_cancel_write(&model, SCAN_WRITE_EXACT, 10100);
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_CANCEL_WRITE_AMBIGUOUS);

    begin(&model);
    scan_model_tick(&model, 300100);
    scan_model_tick(&model, 305099);
    assert(model.phase == SCAN_CANCEL_READY);
    assert(!scan_model_take_cancel(&model, 305100, cancel));
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_CANCEL_NOT_DISPATCHED);

    begin(&model);
    scan_model_tick(&model, 400000); /* event loop woke too late */
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_CANCEL_NOT_DISPATCHED &&
           !scan_model_take_cancel(&model, 400001, cancel));

    scan_model_init(&model);
    assert(scan_model_begin(&model, &gate, SCAN_TOKEN, CANCEL_TOKEN, 100));
    scan_model_tick(&model, 5100);
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_SCAN_NOT_DISPATCHED &&
           !scan_model_take_scan(&model, &gate, 5101, request));
}

static void early_and_late_reply_race_tests(void) {
    struct scan_model model;
    struct scan_gate gate = valid_gate();
    uint8_t request[16], cancel[12], frame[16];

    scan_model_init(&model);
    assert(scan_model_begin(&model, &gate, SCAN_TOKEN, CANCEL_TOKEN, 100));
    assert(scan_model_take_scan(&model, &gate, 100, request));
    reply(frame, 16, 0x0706, SCAN_TOKEN, 0);
    scan_model_frame(&model, frame, 16, 101); /* before write callback */
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_NETWORK_COUNT);
    scan_model_scan_write(&model, SCAN_WRITE_AMBIGUOUS, 102);
    assert(model.result == SCAN_RESULT_NETWORK_COUNT);

    begin(&model);
    scan_model_tick(&model, 300100);
    assert(scan_model_take_cancel(&model, 300100, cancel));
    reply(frame, 12, 0x0707, CANCEL_TOKEN, 0);
    scan_model_frame(&model, frame, 12, 300101); /* before cancel callback */
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_CANCEL_ACKED);
    scan_model_cancel_write(&model, SCAN_WRITE_AMBIGUOUS, 300102);
    assert(model.result == SCAN_RESULT_CANCEL_ACKED);

    begin(&model);
    scan_model_tick(&model, 300100);
    assert(scan_model_take_cancel(&model, 300100, cancel));
    scan_model_cancel_write(&model, SCAN_WRITE_EXACT, 300101);
    scan_model_tick(&model, 305101);
    assert(model.result == SCAN_RESULT_CANCEL_TIMEOUT);
    reply(frame, 12, 0x0707, CANCEL_TOKEN, 0);
    scan_model_frame(&model, frame, 12, 305102); /* late ACK is inert */
    assert(model.result == SCAN_RESULT_CANCEL_TIMEOUT);
}

static void clock_tests(void) {
    struct scan_model model;
    uint8_t request[16];
    struct scan_gate gate = valid_gate();
    begin(&model);
    scan_model_tick(&model, 99);
    assert(model.result == SCAN_RESULT_CLOCK_ERROR);

    scan_model_init(&model);
    assert(scan_model_begin(&model, &gate, SCAN_TOKEN, CANCEL_TOKEN,
                            INT64_MAX - 5000));
    assert(scan_model_take_scan(&model, &gate,
                                INT64_MAX - 5000, request));
    scan_model_scan_write(&model, SCAN_WRITE_EXACT, INT64_MAX - 4999);
    assert(model.phase == SCAN_DONE &&
           model.result == SCAN_RESULT_CLOCK_ERROR);
}

int main(void) {
    gate_tests();
    success_and_error_tests();
    malformed_and_unrelated_tests();
    timeout_cancel_race_test();
    cancel_outcome_tests();
    ambiguous_and_cp_loss_tests();
    dispatch_deadline_tests();
    early_and_late_reply_race_tests();
    clock_tests();
    puts("PASS TD1A host-only available-network scan model");
    return 0;
}
