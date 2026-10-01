#include "rfs-quarantine-transport-host.h"

#include <stdlib.h>
#include <string.h>

enum rqt_phase {
    RQT_WAIT_7,
    RQT_WAIT_3,
    RQT_WAIT_6,
    RQT_WAIT_DATA,
    RQT_DONE,
    RQT_FAILED
};

struct rqt_transport {
    struct rqt_ops ops;
    void *context;
    uint8_t rx[RQT_MAX_FRAME_BYTES];
    size_t used;
    size_t frame_len;
    size_t received;
    uint32_t expected_chunk;
    uint64_t last_progress_ms;
    uint32_t step_deadline_ms;
    enum rqt_phase phase;
};

static const uint8_t status_7[16] = {
    3,0,0,0, 8,0,0,0, 0,0,0,0, 3,0,0,0
};
static const uint8_t status_complete[16] = {
    3,0,1,0, 8,0,0,0, 0,0,0,0, 3,0,0,0
};

static uint16_t get16(const uint8_t *p)
{
    return (uint16_t)((uint16_t)p[0] | ((uint16_t)p[1] << 8));
}

static uint32_t get32(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static enum rqt_result fail(struct rqt_transport *t)
{
    if (t->phase != RQT_FAILED) {
        t->phase = RQT_FAILED;
        memset(t->rx, 0, sizeof t->rx);
        t->used = 0;
        t->frame_len = 0;
        t->ops.abort(t->context);
    }
    return RQT_ABORTED;
}

struct rqt_transport *rqt_create(const char *host_ack,
                                 const struct rqt_ops *ops, void *context,
                                 uint64_t now_ms, uint32_t step_deadline_ms)
{
    struct rqt_transport *t;
    if (!host_ack || strcmp(host_ack, RQT_HOST_ACK) != 0 || !ops ||
        !ops->accept || !ops->durable_complete || !ops->send_once ||
        !ops->abort || !context || step_deadline_ms == 0 ||
        step_deadline_ms > 300000u) return NULL;
    t = (struct rqt_transport *)calloc(1, sizeof *t);
    if (!t) return NULL;
    t->ops = *ops;
    t->context = context;
    t->last_progress_ms = now_ms;
    t->step_deadline_ms = step_deadline_ms;
    t->phase = RQT_WAIT_7;
    return t;
}

void rqt_destroy(struct rqt_transport *t)
{
    if (!t) return;
    if (t->phase != RQT_DONE && t->phase != RQT_FAILED) (void)fail(t);
    memset(t, 0, sizeof *t);
    free(t);
}

static int grant_valid(const uint8_t *p, size_t n, size_t received,
                       uint32_t *chunk)
{
    size_t remaining;
    uint32_t expected;
    if (received >= RQ_WRITE_BYTES || n != 20 || get16(p) != 2 ||
        get16(p + 2) != 1 || get32(p + 4) != 12 ||
        get32(p + 8) != 3 || get32(p + 12) != received) return 0;
    remaining = RQ_WRITE_BYTES - received;
    expected = (uint32_t)(remaining < RQ_CHUNK_BYTES ?
                          remaining : RQ_CHUNK_BYTES);
    if (get32(p + 16) != expected) return 0;
    *chunk = expected;
    return 1;
}

static int data_valid(const struct rqt_transport *t)
{
    const uint8_t *p = t->rx;
    return t->expected_chunk &&
           t->frame_len == 20u + t->expected_chunk &&
           get16(p) == 2 && get16(p + 2) == 1 &&
           get32(p + 4) == 12u + t->expected_chunk &&
           get32(p + 8) == 0 && get32(p + 12) == 3 &&
           get32(p + 16) == t->expected_chunk;
}

/* A reply-producing frame must be the last frame in this feed. This avoids
 * accepting data that was already buffered before its grant was sent. */
static enum rqt_result complete_frame(struct rqt_transport *t,
                                      size_t trailing_bytes, uint64_t now_ms)
{
    uint8_t reply[RQ_MAX_RESPONSE_BYTES] = {0};
    size_t reply_len = 0;
    uint32_t next_chunk = 0;
    enum rq_result result;
    enum rqt_phase next_phase = t->phase;
    /* The observed WAIT_7, WAIT_6 and WAIT_DATA steps require a reply.
     * Reject already-buffered follow-on bytes before the callback can clone
     * or mutate even a private synthetic candidate. WAIT_3 has no reply and
     * may legitimately share a read with WAIT_6. */
    if (trailing_bytes && t->phase != RQT_WAIT_3) return fail(t);
    if (t->phase == RQT_WAIT_DATA && !data_valid(t)) return fail(t);
    result = t->ops.accept(t->context, t->rx, t->frame_len,
                           reply, sizeof reply, &reply_len);
    if (result == RQ_ERROR || (result == RQ_NO_REPLY && reply_len != 0) ||
        (result == RQ_REPLY && (reply_len == 0 ||
                                reply_len > sizeof reply))) return fail(t);

    if (t->phase == RQT_WAIT_7) {
        if (result != RQ_REPLY || reply_len != sizeof status_7 ||
            memcmp(reply, status_7, sizeof status_7) != 0) return fail(t);
        next_phase = RQT_WAIT_3;
    } else if (t->phase == RQT_WAIT_3) {
        if (result != RQ_NO_REPLY) return fail(t);
        next_phase = RQT_WAIT_6;
    } else if (t->phase == RQT_WAIT_6) {
        if (result != RQ_REPLY ||
            !grant_valid(reply, reply_len, 0, &next_chunk)) return fail(t);
        next_phase = RQT_WAIT_DATA;
    } else if (t->phase == RQT_WAIT_DATA) {
        t->received += t->expected_chunk;
        if (t->received < RQ_WRITE_BYTES) {
            if (result != RQ_REPLY ||
                !grant_valid(reply, reply_len, t->received, &next_chunk))
                return fail(t);
        } else if (t->received == RQ_WRITE_BYTES) {
            if (result != RQ_REPLY ||
                reply_len != sizeof status_complete ||
                memcmp(reply, status_complete, sizeof status_complete) != 0 ||
                t->ops.durable_complete(t->context) != 1) return fail(t);
            next_phase = RQT_DONE;
        } else return fail(t);
    } else return fail(t);

    if (result == RQ_REPLY) {
        int64_t accepted;
        if (trailing_bytes != 0) return fail(t);
        accepted = t->ops.send_once(t->context, reply, reply_len);
        /* An error or short result may already have reached the peer. Do not
         * retry it or send another grant; the transaction is indeterminate. */
        if (accepted != (int64_t)reply_len) return fail(t);
    }
    t->phase = next_phase;
    if (next_chunk) t->expected_chunk = next_chunk;
    t->last_progress_ms = now_ms;
    memset(t->rx, 0, sizeof t->rx);
    t->used = 0;
    t->frame_len = 0;
    return next_phase == RQT_DONE ? RQT_COMPLETE : RQT_ACTIVE;
}

enum rqt_result rqt_tick(struct rqt_transport *t, uint64_t now_ms)
{
    if (!t || t->phase == RQT_FAILED) return RQT_ABORTED;
    if (t->phase == RQT_DONE) return RQT_COMPLETE;
    if (now_ms < t->last_progress_ms ||
        now_ms - t->last_progress_ms >= t->step_deadline_ms) return fail(t);
    return RQT_ACTIVE;
}

enum rqt_result rqt_feed(struct rqt_transport *t, const uint8_t *bytes,
                         size_t len, uint64_t now_ms)
{
    enum rqt_result status;
    if (!t) return RQT_ABORTED;
    /* The final ACK has already been accepted by the mocked sender. A late
     * frame cannot revoke that completed quarantine transaction. The owner
     * of this fixture must stop routing bytes here after completion. */
    if (t->phase == RQT_DONE) return RQT_COMPLETE;
    status = rqt_tick(t, now_ms);
    if (status != RQT_ACTIVE) return status;
    if ((!bytes && len) || len > RQT_MAX_FEED_BYTES) return fail(t);
    for (size_t i = 0; i < len; ++i) {
        if (t->used >= sizeof t->rx) return fail(t);
        t->rx[t->used++] = bytes[i];
        if (t->used == 8) {
            uint32_t payload_len = get32(t->rx + 4);
            if (payload_len < 4 ||
                payload_len > RQT_MAX_FRAME_BYTES - 8u) return fail(t);
            t->frame_len = 8u + payload_len;
        }
        if (t->frame_len && t->used == t->frame_len) {
            status = complete_frame(t, len - i - 1u, now_ms);
            if (status != RQT_ACTIVE) return status;
        }
    }
    return RQT_ACTIVE;
}
