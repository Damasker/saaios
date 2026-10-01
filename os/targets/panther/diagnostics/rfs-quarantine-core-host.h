/* Host-only pure C RFS model. No device, filesystem, or promotion API. */
#ifndef RFS_QUARANTINE_CORE_HOST_H
#define RFS_QUARANTINE_CORE_HOST_H

#if !defined(RFS_QUARANTINE_HOST_ONLY) || defined(__aarch64__)
#error "This experimental RFS core is for x86 host fixtures only"
#endif

#include <stddef.h>
#include <stdint.h>

#define RQ_HOST_ACK "HOST_ONLY_QUARANTINE_NO_DEVICE_IO"
#define RQ_BASELINE_BYTES 524288u
#define RQ_WRITE_BYTES 189446u
#define RQ_CHUNK_BYTES 2012u
#define RQ_MAX_GRANTS 95u
#define RQ_MAX_RESPONSE_BYTES 20u

struct rq_core;

enum rq_state {
    RQ_WAIT_7,
    RQ_WAIT_3,
    RQ_WAIT_6,
    RQ_WAIT_DATA,
    RQ_COMPLETE,
    RQ_ABORTED
};

enum rq_result {
    RQ_ERROR = -1,
    RQ_NO_REPLY = 0,
    RQ_REPLY = 1
};

/* The baseline must be synthetic; pinned_sha256 is supplied independently.
 * The consent string and build flag intentionally block casual reuse. */
struct rq_core *rq_create(const char *host_ack, const uint8_t *baseline,
                          size_t baseline_len,
                          const uint8_t pinned_sha256[32]);
void rq_destroy(struct rq_core *core);

/* reply_cap must be >= RQ_MAX_RESPONSE_BYTES. Any invalid frame or output
 * contract permanently aborts the transaction and returns RQ_ERROR. */
enum rq_result rq_accept(struct rq_core *core, const uint8_t *frame,
                         size_t frame_len, uint8_t *reply, size_t reply_cap,
                         size_t *reply_len);

enum rq_state rq_get_state(const struct rq_core *core);
unsigned rq_grants_issued(const struct rq_core *core);
size_t rq_bytes_received(const struct rq_core *core);
int rq_candidate_prepared(const struct rq_core *core);

/* Copies a detached candidate only after completion; there is no path,
 * writer, or promotion function. Returns 0 on success, -1 otherwise. */
int rq_copy_completed_candidate(const struct rq_core *core, uint8_t *out,
                                size_t out_len);

/* Bounded digest helper for independent host SHA-256 fixture vectors. */
int rq_host_sha256_fixture(const uint8_t *data, size_t len, uint8_t out[32]);

#endif
