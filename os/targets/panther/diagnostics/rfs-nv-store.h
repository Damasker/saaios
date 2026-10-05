/*
 * Dual-handle RFS NV store for the S5300 quarantine owner (MODEM-07B).
 *
 * Serves the CP's RFS NV write-outs for BOTH handles -- 1 (nv_normal) and
 * 3 (nv_protected) -- into a private quarantine store, and selects the
 * verified image the boot loader UDL-loads as NV_NORM / NV_PROT. The real EFS
 * is never opened here; the store root is a caller-supplied directory fd on
 * the SaaiOS data volume. Never logs NV bytes, digests or identifiers.
 *
 * Frame layouts are the ones already proven live by the quarantine owner
 * (handle 3: cmd7 status, cmd6 grant, cmd2 chunks, final status; handle 1:
 * cmd7 status and sequence-echoed cmd6 grant, VERDICT 10/24). The handle-1
 * final status is the same layout with the CP-declared handle and sequence;
 * it is off by default until recovered from stock rfsd (MODEM-07B T2).
 *
 * The caller still owns the live gates: CP ONLINE, exclusive endpoint,
 * original EFS unmounted, deadlines, and sending each reply at most once.
 */
#ifndef SAAIOS_RFS_NV_STORE_H
#define SAAIOS_RFS_NV_STORE_H

#include <stddef.h>
#include <stdint.h>

enum {
    NV_IMAGE_BYTES = 524288,
    NV_CHUNK_MAX = 2012,
    NV_HANDLE_NORMAL = 1,
    NV_HANDLE_PROTECTED = 3,
    RFS_NV_REPLY_MAX = 20
};

enum nv_kind { NV_KIND_NORMAL = 0, NV_KIND_PROTECTED = 1, NV_KIND_COUNT = 2 };

enum nv_check {
    NV_CHECK_OK, NV_CHECK_IO, NV_CHECK_SIZE, NV_CHECK_ZERO, NV_CHECK_MAGIC,
    NV_CHECK_SIDECAR
};

enum rfs_nv_result {
    RFS_NV_IGNORED,   /* not an NV frame this store serves */
    RFS_NV_NO_REPLY,  /* accepted; nothing to send */
    RFS_NV_REPLY,     /* accepted; send reply[0..reply_len) exactly once */
    RFS_NV_COMMITTED, /* flush verified and committed; reply_len may be 0 */
    RFS_NV_REJECTED   /* refused; session aborted, committed images unchanged */
};

struct nv_sha256 {
    uint32_t h[8];
    uint64_t bits;
    uint8_t block[64];
    size_t used;
};

struct rfs_nv_session {
    int active;
    uint16_t seq;
    uint32_t total, received, expected;
    int candidate;
    unsigned grants;
    const char *base;
};

struct rfs_nv_server {
    int store;
    int final_status[NV_KIND_COUNT];
    struct rfs_nv_session session[NV_KIND_COUNT];
    unsigned committed[NV_KIND_COUNT];
    unsigned rejected[NV_KIND_COUNT];
};

void nv_sha256_init(struct nv_sha256 *ctx);
void nv_sha256_update(struct nv_sha256 *ctx, const void *data, size_t len);
void nv_sha256_final(struct nv_sha256 *ctx, uint8_t out[32]);

const char *nv_check_name(enum nv_check check);
const char *nv_kind_name(enum nv_kind kind);

/* Size, not-all-zero and (normal only) the ERIG header magic. */
enum nv_check nv_image_check_fd(int fd, enum nv_kind kind);
/* nv_image_check_fd plus the store's SHA-256 sidecar for that image. */
enum nv_check nv_store_check(int store, const char *name, enum nv_kind kind);

/* Copy a verified source image into the store as the immutable seed. */
int nv_store_seed(int store, enum nv_kind kind, int source_fd);

/* Image the boot loader must UDL-load. Normal: current, then previous, then
 * seed. Protected: always the seed. Returns -1 if nothing verifies. */
int nv_store_boot_image(int store, enum nv_kind kind, const char **name);

void rfs_nv_server_init(struct rfs_nv_server *server, int store);
void rfs_nv_server_abort(struct rfs_nv_server *server);
enum rfs_nv_result rfs_nv_frame(struct rfs_nv_server *server,
                                const uint8_t *frame, size_t len,
                                uint8_t reply[RFS_NV_REPLY_MAX],
                                size_t *reply_len);

#endif
