/* Host-only Linux filesystem fixture for the quarantined RFS model. */
#ifndef RFS_QUARANTINE_STORAGE_HOST_LINUX_H
#define RFS_QUARANTINE_STORAGE_HOST_LINUX_H

#if !defined(RFS_QUARANTINE_HOST_ONLY) || !defined(__linux__) || \
    !defined(__x86_64__) || defined(__aarch64__)
#error "This RFS storage fixture is Linux x86_64 host-only"
#endif

#include "rfs-quarantine-core-host.h"

#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>

struct rq_storage;

/* parent_dirfd must identify the real /tmp directory. The adapter opens /tmp
 * with O_NOFOLLOW and compares inode/device; it cannot attest how the caller
 * originally obtained its fd. No source path or device endpoint is accepted.
 * synthetic_baseline is host memory, not a verified-fd or EFS provenance
 * contract; keep it stable for the duration of create. A fresh rq-host-*
 * directory is left quarantined after destroy, including on failure; the
 * caller's fixture may remove only its own exact test directory. */
struct rq_storage *rq_storage_create(const char *host_ack, int parent_dirfd,
                                     const uint8_t *synthetic_baseline,
                                     size_t baseline_len,
                                     const uint8_t pinned_sha256[32]);
void rq_storage_destroy(struct rq_storage *storage);

/* Same frame contract as rq_accept. The final status is withheld until the
 * private candidate and checksum sidecar pass reread and durability gates.
 * Any transfer failure permanently disarms this adapter, returning RQ_ERROR
 * with reply_len zero. After completion, extra frames receive RQ_ERROR but
 * do not invalidate the completed quarantined fixture. The candidate is
 * never promoted by this API. */
enum rq_result rq_storage_accept(struct rq_storage *storage,
                                 const uint8_t *frame, size_t frame_len,
                                 uint8_t *reply, size_t reply_cap,
                                 size_t *reply_len);

int rq_storage_complete(const struct rq_storage *storage);
int rq_storage_failed(const struct rq_storage *storage);
const char *rq_storage_leaf(const struct rq_storage *storage);

#ifdef RQ_STORAGE_HOST_TEST
enum rq_storage_test_fault {
    RQ_STORAGE_NO_FAULT,
    RQ_STORAGE_FAIL_CHUNK_WRITE,
    RQ_STORAGE_FAIL_FINAL_FSYNC,
    RQ_STORAGE_FAIL_SIDECAR_FSYNC,
    RQ_STORAGE_FAIL_SIDECAR_CLOSE
};
void rq_storage_test_set_fault(struct rq_storage *storage,
                               enum rq_storage_test_fault fault);
int rq_storage_test_dir_identity(const struct rq_storage *storage,
                                 dev_t *device, ino_t *inode);
#endif

#endif
