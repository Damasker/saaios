/* End-to-end synthetic Linux host fixture: no modem or source NV access. */
#define _GNU_SOURCE
#include "rfs-quarantine-transport-host.h"
#include "rfs-quarantine-storage-host-linux.h"

#ifdef NDEBUG
#undef NDEBUG
#endif
#include <assert.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static const uint8_t cmd7[] = {7,0,0,0, 4,0,0,0, 3,0,0,0};
static const uint8_t cmd3[] = {
    3,0,0,0, 12,0,0,0, 0,0,0,0, 3,0,0,0, 0,0,0,0
};
static const uint8_t cmd6[] = {
    6,0,1,0, 16,0,0,0, 3,0,0,0, 0,0,0,0, 6,0xe4,2,0, 2,0,0,0
};

struct fixture {
    int parent;
    uint8_t *baseline;
    struct rq_storage *storage;
    struct rqt_transport *transport;
    unsigned sends;
    unsigned grants;
    unsigned final_acks;
    unsigned aborts;
};

static void put32(uint8_t *p, uint32_t n)
{
    for (unsigned i = 0; i < 4; ++i) p[i] = (uint8_t)(n >> (8u * i));
}

static enum rq_result storage_accept(void *ctx, const uint8_t *frame,
                                     size_t frame_len, uint8_t *reply,
                                     size_t reply_cap, size_t *reply_len)
{
    struct fixture *f = ctx;
    return rq_storage_accept(f->storage, frame, frame_len,
                             reply, reply_cap, reply_len);
}

static int durable_complete(void *ctx)
{
    struct fixture *f = ctx;
    return rq_storage_complete(f->storage);
}

static int64_t send_once(void *ctx, const uint8_t *reply, size_t len)
{
    struct fixture *f = ctx;
    ++f->sends;
    if (len == 20 && reply[0] == 2) ++f->grants;
    if (len == 16 && reply[0] == 3 && reply[2] == 1) {
        /* The final CP-visible success is forbidden before durability. */
        assert(rq_storage_complete(f->storage));
        ++f->final_acks;
    }
    return (int64_t)len;
}

static void abort_once(void *ctx)
{
    ++((struct fixture *)ctx)->aborts;
}

static const struct rqt_ops ops = {
    storage_accept, durable_complete, send_once, abort_once
};

static void start(struct fixture *f, enum rq_storage_test_fault fault)
{
    uint8_t digest[32];
    memset(f, 0, sizeof *f);
    f->parent = open("/tmp", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    assert(f->parent >= 0);
    f->baseline = malloc(RQ_BASELINE_BYTES);
    assert(f->baseline);
    for (size_t i = 0; i < RQ_BASELINE_BYTES; ++i)
        f->baseline[i] = (uint8_t)(i * 29u + 7u);
    assert(rq_host_sha256_fixture(f->baseline, RQ_BASELINE_BYTES, digest) == 0);
    f->storage = rq_storage_create(RQ_HOST_ACK, f->parent, f->baseline,
                                   RQ_BASELINE_BYTES, digest);
    assert(f->storage);
    assert(strncmp(rq_storage_leaf(f->storage), "rq-host-", 8) == 0);
    rq_storage_test_set_fault(f->storage, fault);
    f->transport = rqt_create(RQT_HOST_ACK, &ops, f, 1000, 1000);
    assert(f->transport);
}

static size_t data_frame(uint8_t out[RQT_MAX_FRAME_BYTES], size_t offset)
{
    size_t remaining = RQ_WRITE_BYTES - offset;
    size_t chunk = remaining < RQ_CHUNK_BYTES ? remaining : RQ_CHUNK_BYTES;
    memset(out, 0, 20 + chunk);
    out[0] = 2;
    out[2] = 1;
    put32(out + 4, (uint32_t)(12 + chunk));
    put32(out + 12, 3);
    put32(out + 16, (uint32_t)chunk);
    for (size_t i = 0; i < chunk; ++i)
        out[20 + i] = (uint8_t)((offset + i) * 37u + 11u);
    return 20 + chunk;
}

/* Remove only this fixture's uniquely created directory after checking its
 * opened and named inodes. Never target the parent or a glob. */
static void finish(struct fixture *f)
{
    char leaf[48];
    dev_t device;
    ino_t inode;
    struct stat opened, named;
    const char *files[] = {"candidate.sha256", "candidate.bin", "NO_PROMOTION"};
    assert(rq_storage_test_dir_identity(f->storage, &device, &inode) == 0);
    assert(strlen(rq_storage_leaf(f->storage)) < sizeof leaf);
    strcpy(leaf, rq_storage_leaf(f->storage));
    int dir = openat(f->parent, leaf, O_RDONLY | O_DIRECTORY | O_NOFOLLOW |
                                 O_CLOEXEC);
    assert(dir >= 0);
    assert(fstat(dir, &opened) == 0 && opened.st_dev == device &&
           opened.st_ino == inode && S_ISDIR(opened.st_mode));
    assert(fstatat(f->parent, leaf, &named, AT_SYMLINK_NOFOLLOW) == 0 &&
           named.st_dev == device && named.st_ino == inode);
    rqt_destroy(f->transport);
    rq_storage_destroy(f->storage);
    for (size_t i = 0; i < sizeof files / sizeof files[0]; ++i) {
        if (fstatat(dir, files[i], &named, AT_SYMLINK_NOFOLLOW) == 0) {
            assert(S_ISREG(named.st_mode) && named.st_uid == geteuid());
            assert(unlinkat(dir, files[i], 0) == 0);
        }
    }
    assert(fstatat(f->parent, leaf, &named, AT_SYMLINK_NOFOLLOW) == 0 &&
           named.st_dev == device && named.st_ino == inode);
    assert(close(dir) == 0);
    assert(unlinkat(f->parent, leaf, AT_REMOVEDIR) == 0);
    assert(close(f->parent) == 0);
    for (size_t i = 0; i < RQ_BASELINE_BYTES; ++i)
        assert(f->baseline[i] == (uint8_t)(i * 29u + 7u));
    free(f->baseline);
}

static void run_transfer(enum rq_storage_test_fault fault, int expected_success)
{
    struct fixture f;
    uint8_t frame[RQT_MAX_FRAME_BYTES];
    start(&f, fault);
    assert(rqt_feed(f.transport, cmd7, sizeof cmd7, 1001) == RQT_ACTIVE);
    assert(rqt_feed(f.transport, cmd3, sizeof cmd3, 1002) == RQT_ACTIVE);
    assert(rqt_feed(f.transport, cmd6, sizeof cmd6, 1003) == RQT_ACTIVE);
    assert(f.sends == 2 && f.grants == 1);
    size_t offset = 0;
    uint64_t now = 1004;
    enum rqt_result result = RQT_ACTIVE;
    while (offset < RQ_WRITE_BYTES) {
        size_t n = data_frame(frame, offset);
        offset += n - 20;
        result = rqt_feed(f.transport, frame, n, now++);
        if (offset < RQ_WRITE_BYTES) assert(result == RQT_ACTIVE);
    }
    assert(f.grants == RQ_MAX_GRANTS);
    if (expected_success) {
        assert(result == RQT_COMPLETE && rq_storage_complete(f.storage));
        assert(f.final_acks == 1 && f.aborts == 0);
    } else {
        assert(result == RQT_ABORTED && rq_storage_failed(f.storage));
        assert(f.final_acks == 0 && f.aborts == 1);
    }
    finish(&f);
}

int main(void)
{
    run_transfer(RQ_STORAGE_NO_FAULT, 1);
    run_transfer(RQ_STORAGE_FAIL_FINAL_FSYNC, 0);
    puts("PASS synthetic RFS transport + durable quarantine integration");
    return 0;
}
