/* Synthetic host-only RFS stream/one-send fixture. Never link on a phone. */
#ifndef RFS_QUARANTINE_TRANSPORT_HOST_H
#define RFS_QUARANTINE_TRANSPORT_HOST_H

#if !defined(RFS_QUARANTINE_HOST_ONLY) || !defined(__linux__) || \
    !defined(__x86_64__) || defined(__aarch64__)
#error "RFS transport fixture is Linux x86_64 host-only"
#endif

#include "rfs-quarantine-core-host.h"

#include <stddef.h>
#include <stdint.h>

#define RQT_HOST_ACK "HOST_ONLY_RFS_TRANSPORT_NO_DEVICE_IO"
#define RQT_MAX_FRAME_BYTES (20u + RQ_CHUNK_BYTES)
#define RQT_MAX_FEED_BYTES (2u * RQT_MAX_FRAME_BYTES)

struct rqt_transport;

enum rqt_result {
    RQT_ABORTED = -1,
    RQT_ACTIVE = 0,
    RQT_COMPLETE = 1
};

struct rqt_ops {
    /* The accept callback must validate the complete synthetic RFS frame.
     * rq_storage_accept is suitable for a Linux host fixture; rq_accept is
     * suitable only with a separately mocked durability predicate. */
    enum rq_result (*accept)(void *context, const uint8_t *frame,
                             size_t frame_len, uint8_t *reply,
                             size_t reply_cap, size_t *reply_len);
    /* Must report durable, quarantined completion before the final ACK.
     * The transport cannot establish filesystem provenance by itself. */
    int (*durable_complete)(void *context);
    /* Called exactly once for each reply. Any non-exact result is terminal;
     * no automatic retry, even if the result is ambiguous. No device I/O. */
    int64_t (*send_once)(void *context, const uint8_t *reply, size_t reply_len);
    /* Called once on any failure or destruction before completion. */
    void (*abort)(void *context);
};

struct rqt_transport *rqt_create(const char *host_ack,
                                 const struct rqt_ops *ops, void *context,
                                 uint64_t now_ms, uint32_t step_deadline_ms);
void rqt_destroy(struct rqt_transport *transport);

/* Feed only synthetic bytes; a reply-producing frame must end the feed.
 * No following frame is processed until the mocked send has succeeded.
 * now_ms is a caller-supplied monotonic test clock. */
enum rqt_result rqt_feed(struct rqt_transport *transport,
                         const uint8_t *bytes, size_t len, uint64_t now_ms);
enum rqt_result rqt_tick(struct rqt_transport *transport, uint64_t now_ms);

#endif
