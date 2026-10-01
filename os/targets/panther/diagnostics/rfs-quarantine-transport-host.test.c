#include "rfs-quarantine-transport-host.h"

#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <stdio.h>
#include <string.h>

static const uint8_t pinned[32] = {
    0x0d,0x57,0xce,0x7e,0x6f,0x29,0x9b,0x77,
    0xf1,0xaa,0x75,0xb8,0xb0,0x19,0x8a,0xaa,
    0xa9,0x10,0xb6,0xfd,0x2e,0xa0,0x9b,0xfc,
    0x4a,0x5f,0xcc,0x4d,0x20,0x23,0xf5,0xd2
};
static uint8_t baseline[RQ_BASELINE_BYTES];
static const uint8_t cmd7[] = {
    7,0,0,0, 4,0,0,0, 3,0,0,0
};
static const uint8_t cmd3[] = {
    3,0,0,0, 12,0,0,0, 0,0,0,0, 3,0,0,0, 0,0,0,0
};
static const uint8_t cmd6[] = {
    6,0,1,0, 16,0,0,0, 3,0,0,0, 0,0,0,0,
    6,0xe4,2,0, 2,0,0,0
};
static const uint8_t final_status[] = {
    3,0,1,0, 8,0,0,0, 0,0,0,0, 3,0,0,0
};

struct fixture {
    struct rq_core *core;
    struct rqt_transport *transport;
    unsigned accepts;
    unsigned sends;
    unsigned aborts;
    unsigned grants;
    unsigned final_ack_attempts;
    int durable_mock;
    int send_fault; /* -1 error, 1 short, 2 longer-than-requested */
};

static void put16(uint8_t *p, uint16_t n)
{
    p[0] = (uint8_t)n;
    p[1] = (uint8_t)(n >> 8);
}

static void put32(uint8_t *p, uint32_t n)
{
    p[0] = (uint8_t)n;
    p[1] = (uint8_t)(n >> 8);
    p[2] = (uint8_t)(n >> 16);
    p[3] = (uint8_t)(n >> 24);
}

static enum rq_result mock_accept(void *context, const uint8_t *frame,
                                  size_t frame_len, uint8_t *reply,
                                  size_t reply_cap, size_t *reply_len)
{
    struct fixture *f = (struct fixture *)context;
    ++f->accepts;
    return rq_accept(f->core, frame, frame_len, reply, reply_cap, reply_len);
}

static int mock_durable_complete(void *context)
{
    const struct fixture *f = (const struct fixture *)context;
    /* This is an explicit synthetic mock, not a durability claim. */
    return f->durable_mock && rq_get_state(f->core) == RQ_COMPLETE;
}

static int64_t mock_send_once(void *context, const uint8_t *reply,
                              size_t reply_len)
{
    struct fixture *f = (struct fixture *)context;
    ++f->sends;
    if (reply_len == 20 && reply[0] == 2) ++f->grants;
    if (reply_len == sizeof final_status &&
        memcmp(reply, final_status, sizeof final_status) == 0)
        ++f->final_ack_attempts;
    if (f->send_fault == -1) return -1;
    if (f->send_fault == 1) return (int64_t)reply_len - 1;
    if (f->send_fault == 2) return (int64_t)reply_len + 1;
    return (int64_t)reply_len;
}

static void mock_abort(void *context)
{
    ++((struct fixture *)context)->aborts;
}

static const struct rqt_ops mock_ops = {
    mock_accept, mock_durable_complete, mock_send_once, mock_abort
};

static void start(struct fixture *f, int durable_mock, uint32_t deadline_ms)
{
    memset(f, 0, sizeof *f);
    memset(baseline, 0x5a, sizeof baseline);
    f->durable_mock = durable_mock;
    f->core = rq_create(RQ_HOST_ACK, baseline, sizeof baseline, pinned);
    assert(f->core);
    f->transport = rqt_create(RQT_HOST_ACK, &mock_ops, f, 1000,
                              deadline_ms);
    assert(f->transport);
}

static void finish(struct fixture *f)
{
    rqt_destroy(f->transport);
    rq_destroy(f->core);
    f->transport = NULL;
    f->core = NULL;
}

static size_t data_frame(uint8_t out[RQT_MAX_FRAME_BYTES], size_t offset)
{
    size_t remaining = RQ_WRITE_BYTES - offset;
    size_t chunk = remaining < RQ_CHUNK_BYTES ? remaining : RQ_CHUNK_BYTES;
    put16(out, 2);
    put16(out + 2, 1);
    put32(out + 4, (uint32_t)(12 + chunk));
    put32(out + 8, 0);
    put32(out + 12, 3);
    put32(out + 16, (uint32_t)chunk);
    for (size_t i = 0; i < chunk; ++i)
        out[20 + i] = (uint8_t)((offset + i) * 17u + 9u);
    return 20 + chunk;
}

static void open_transfer(struct fixture *f)
{
    uint8_t together[sizeof cmd3 + sizeof cmd6];
    assert(rqt_feed(f->transport, cmd7, sizeof cmd7, 1001) == RQT_ACTIVE);
    memcpy(together, cmd3, sizeof cmd3);
    memcpy(together + sizeof cmd3, cmd6, sizeof cmd6);
    assert(rqt_feed(f->transport, together, sizeof together, 1002) ==
           RQT_ACTIVE);
    assert(f->sends == 2 && f->grants == 1 && f->accepts == 3);
}

static enum rqt_result send_all_data(struct fixture *f, int split)
{
    uint8_t frame[RQT_MAX_FRAME_BYTES];
    enum rqt_result result = RQT_ACTIVE;
    size_t offset = 0;
    uint64_t now_ms = 1003;
    while (offset < RQ_WRITE_BYTES) {
        size_t n = data_frame(frame, offset);
        size_t chunk = n - 20;
        if (split && (offset / RQ_CHUNK_BYTES) % 3 == 0) {
            assert(rqt_feed(f->transport, frame, 7, now_ms++) == RQT_ACTIVE);
            assert(rqt_feed(f->transport, frame + 7, 13, now_ms++) ==
                   RQT_ACTIVE);
            result = rqt_feed(f->transport, frame + 20, chunk, now_ms++);
        } else result = rqt_feed(f->transport, frame, n, now_ms++);
        offset += chunk;
        if (offset < RQ_WRITE_BYTES) assert(result == RQT_ACTIVE);
    }
    return result;
}

static void test_complete_split_and_coalesced(void)
{
    struct fixture f;
    uint8_t together[sizeof cmd3 + sizeof cmd6];
    start(&f, 1, 1000);
    assert(rqt_feed(f.transport, cmd7, 3, 1001) == RQT_ACTIVE);
    assert(f.accepts == 0 && f.sends == 0);
    assert(rqt_feed(f.transport, cmd7 + 3, sizeof cmd7 - 3, 1002) ==
           RQT_ACTIVE);
    memcpy(together, cmd3, sizeof cmd3);
    memcpy(together + sizeof cmd3, cmd6, sizeof cmd6);
    assert(rqt_feed(f.transport, together, sizeof together, 1003) ==
           RQT_ACTIVE);
    assert(f.sends == 2 && f.grants == 1);
    assert(send_all_data(&f, 1) == RQT_COMPLETE);
    assert(f.accepts == 3 + RQ_MAX_GRANTS);
    assert(f.sends == 2 + RQ_MAX_GRANTS);
    assert(f.grants == RQ_MAX_GRANTS && f.final_ack_attempts == 1);
    assert(f.aborts == 0 && rq_get_state(f.core) == RQ_COMPLETE);
    assert(rqt_tick(f.transport, 999999) == RQT_COMPLETE);
    {
        unsigned accepts = f.accepts;
        unsigned sends = f.sends;
        assert(rqt_feed(f.transport, cmd7, sizeof cmd7, 1000000) ==
               RQT_COMPLETE);
        assert(rqt_feed(f.transport, NULL, RQT_MAX_FEED_BYTES + 1u,
                        1000001) == RQT_COMPLETE);
        assert(f.accepts == accepts && f.sends == sends && f.aborts == 0);
        assert(rq_get_state(f.core) == RQ_COMPLETE);
    }
    for (size_t i = 0; i < sizeof baseline; ++i)
        assert(baseline[i] == 0x5a);
    finish(&f);
    assert(f.aborts == 0);
    puts("split/coalesced synthetic 95-chunk completion: PASS");
}

static void test_final_ack_requires_durability(void)
{
    struct fixture f;
    start(&f, 0, 1000);
    open_transfer(&f);
    assert(send_all_data(&f, 0) == RQT_ABORTED);
    assert(f.grants == RQ_MAX_GRANTS);
    assert(f.final_ack_attempts == 0 && f.sends == 1 + RQ_MAX_GRANTS);
    assert(f.aborts == 1 && rq_get_state(f.core) == RQ_COMPLETE);
    finish(&f);
    assert(f.aborts == 1);
    puts("final ACK withheld without mocked durable completion: PASS");
}

static void test_ambiguous_sends_never_retry(void)
{
    const int faults[] = {-1, 1, 2};
    for (size_t i = 0; i < sizeof faults / sizeof faults[0]; ++i) {
        struct fixture f;
        start(&f, 1, 1000);
        f.send_fault = faults[i];
        assert(rqt_feed(f.transport, cmd7, sizeof cmd7, 1001) == RQT_ABORTED);
        assert(f.sends == 1 && f.aborts == 1);
        assert(rqt_feed(f.transport, cmd7, sizeof cmd7, 1002) == RQT_ABORTED);
        assert(f.sends == 1 && f.aborts == 1);
        finish(&f);
    }
    for (size_t i = 0; i < sizeof faults / sizeof faults[0]; ++i) {
        struct fixture f;
        uint8_t together[sizeof cmd3 + sizeof cmd6];
        start(&f, 1, 1000);
        assert(rqt_feed(f.transport, cmd7, sizeof cmd7, 1001) == RQT_ACTIVE);
        f.send_fault = faults[i];
        memcpy(together, cmd3, sizeof cmd3);
        memcpy(together + sizeof cmd3, cmd6, sizeof cmd6);
        assert(rqt_feed(f.transport, together, sizeof together, 1002) ==
               RQT_ABORTED);
        assert(f.sends == 2 && f.grants == 1 && f.aborts == 1);
        assert(rqt_tick(f.transport, 1003) == RQT_ABORTED);
        assert(f.sends == 2);
        finish(&f);
    }
    for (size_t i = 0; i < sizeof faults / sizeof faults[0]; ++i) {
        struct fixture f;
        uint8_t frame[RQT_MAX_FRAME_BYTES];
        size_t n;
        start(&f, 1, 1000);
        open_transfer(&f);
        f.send_fault = faults[i];
        n = data_frame(frame, 0);
        assert(rqt_feed(f.transport, frame, n, 1003) == RQT_ABORTED);
        assert(f.sends == 3 && f.grants == 2 && f.aborts == 1);
        assert(f.final_ack_attempts == 0);
        assert(rqt_feed(f.transport, frame, n, 1004) == RQT_ABORTED);
        assert(f.sends == 3 && f.aborts == 1);
        finish(&f);
    }
    for (size_t i = 0; i < sizeof faults / sizeof faults[0]; ++i) {
        struct fixture f;
        uint8_t frame[RQT_MAX_FRAME_BYTES];
        size_t offset = 0;
        uint64_t now_ms = 1003;
        start(&f, 1, 1000);
        open_transfer(&f);
        for (unsigned grant = 1; grant < RQ_MAX_GRANTS; ++grant) {
            size_t n = data_frame(frame, offset);
            assert(n == RQT_MAX_FRAME_BYTES);
            assert(rqt_feed(f.transport, frame, n, now_ms++) == RQT_ACTIVE);
            offset += RQ_CHUNK_BYTES;
        }
        assert(offset == 94u * RQ_CHUNK_BYTES);
        assert(f.sends == 1 + RQ_MAX_GRANTS && f.final_ack_attempts == 0);
        f.send_fault = faults[i];
        size_t final_len = data_frame(frame, offset);
        assert(final_len == 20u + 318u);
        assert(rqt_feed(f.transport, frame, final_len, now_ms++) ==
               RQT_ABORTED);
        /* This counts attempts, not CP acceptance or storage safety. */
        assert(f.sends == 2 + RQ_MAX_GRANTS && f.final_ack_attempts == 1);
        assert(f.grants == RQ_MAX_GRANTS && f.aborts == 1);
        assert(rq_get_state(f.core) == RQ_COMPLETE);
        assert(rqt_feed(f.transport, frame, final_len, now_ms) == RQT_ABORTED);
        assert(f.sends == 2 + RQ_MAX_GRANTS && f.aborts == 1);
        finish(&f);
    }
    puts("error/short/overcount send never retried: PASS");
}

static void test_framing_deadline_and_order(void)
{
    struct fixture f;
    uint8_t too_large[8] = {7,0,0,0, 0xff,0xff,0xff,0xff};
    uint8_t combined[sizeof cmd7 + sizeof cmd3];
    uint8_t combined_grant[sizeof cmd6 + RQT_MAX_FRAME_BYTES];
    uint8_t combined_data[2 * RQT_MAX_FRAME_BYTES];
    uint8_t bad_data[RQT_MAX_FRAME_BYTES];
    start(&f, 1, 1000);
    assert(rqt_feed(f.transport, cmd7, 5, 1001) == RQT_ACTIVE);
    assert(rqt_tick(f.transport, 2000) == RQT_ABORTED);
    assert(f.accepts == 0 && f.sends == 0 && f.aborts == 1);
    finish(&f);

    start(&f, 1, 1000);
    assert(rqt_feed(f.transport, too_large, sizeof too_large, 1001) ==
           RQT_ABORTED);
    assert(f.accepts == 0 && f.sends == 0 && f.aborts == 1);
    finish(&f);

    start(&f, 1, 1000);
    memcpy(combined, cmd7, sizeof cmd7);
    memcpy(combined + sizeof cmd7, cmd3, sizeof cmd3);
    assert(rqt_feed(f.transport, combined, sizeof combined, 1001) ==
           RQT_ABORTED);
    assert(f.accepts == 0 && f.sends == 0 && f.aborts == 1);
    finish(&f);

    start(&f, 1, 1000);
    assert(rqt_feed(f.transport, cmd7, sizeof cmd7, 1001) == RQT_ACTIVE);
    assert(rqt_feed(f.transport, cmd3, sizeof cmd3, 1002) == RQT_ACTIVE);
    memcpy(combined_grant, cmd6, sizeof cmd6);
    (void)data_frame(combined_grant + sizeof cmd6, 0);
    assert(rqt_feed(f.transport, combined_grant, sizeof combined_grant,
                    1003) == RQT_ABORTED);
    assert(f.accepts == 2 && f.sends == 1 && f.aborts == 1);
    assert(rq_get_state(f.core) == RQ_WAIT_6);
    finish(&f);

    start(&f, 1, 1000);
    open_transfer(&f);
    (void)data_frame(combined_data, 0);
    (void)data_frame(combined_data + RQT_MAX_FRAME_BYTES, RQ_CHUNK_BYTES);
    assert(rqt_feed(f.transport, combined_data, sizeof combined_data,
                    1003) == RQT_ABORTED);
    assert(f.accepts == 3 && f.sends == 2 && f.aborts == 1);
    assert(rq_bytes_received(f.core) == 0);
    finish(&f);

    start(&f, 1, 1000);
    open_transfer(&f);
    (void)data_frame(bad_data, 0);
    bad_data[2] = 2; /* wrong sequence before the callback can accept it */
    assert(rqt_feed(f.transport, bad_data, 20 + RQ_CHUNK_BYTES, 1003) ==
           RQT_ABORTED);
    assert(f.accepts == 3 && f.sends == 2 && f.aborts == 1);
    finish(&f);
    puts("framing, causal boundary, sequence, deadline: PASS");
}

static void test_creation_contract(void)
{
    struct fixture f;
    start(&f, 1, 1000);
    assert(!rqt_create("wrong", &mock_ops, &f, 1000, 1000));
    assert(!rqt_create(RQT_HOST_ACK, &mock_ops, &f, 1000, 0));
    assert(!rqt_create(RQT_HOST_ACK, &mock_ops, &f, 1000, 300001));
    assert(!rqt_create(RQT_HOST_ACK, NULL, &f, 1000, 1000));
    assert(!rqt_create(RQT_HOST_ACK, &mock_ops, NULL, 1000, 1000));
    assert(rqt_tick(f.transport, 999) == RQT_ABORTED);
    assert(f.aborts == 1);
    finish(&f);
    puts("host-only creation and monotonic-clock contract: PASS");
}

int main(void)
{
    test_creation_contract();
    test_complete_split_and_coalesced();
    test_final_ack_requires_durability();
    test_ambiguous_sends_never_retry();
    test_framing_deadline_and_order();
    puts("RFS host transport fixture: PASS");
    return 0;
}
