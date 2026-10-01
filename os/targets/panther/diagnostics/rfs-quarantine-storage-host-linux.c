/* Linux x86_64 /tmp-only storage fixture. Not a phone RFS broker. */
#define _GNU_SOURCE
#include "rfs-quarantine-storage-host-linux.h"

#include <errno.h>
#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <sys/prctl.h>
#include <sys/random.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#define RQ_CANDIDATE "candidate.bin"
#define RQ_SIDECAR "candidate.sha256"
#define RQ_NO_PROMOTION "NO_PROMOTION"
static const uint8_t no_promotion_text[] =
    "HOST_ONLY_QUARANTINE_NO_PROMOTION\n";

struct rq_storage {
    struct rq_core *core;
    int parent_fd;
    int dir_fd;
    int candidate_fd;
    int marker_fd;
    dev_t parent_dev;
    ino_t parent_ino;
    dev_t dir_dev;
    ino_t dir_ino;
    dev_t candidate_dev;
    ino_t candidate_ino;
    dev_t marker_dev;
    ino_t marker_ino;
    char leaf[48];
    uint8_t *readback;
    int failed;
    int complete;
#ifdef RQ_STORAGE_HOST_TEST
    enum rq_storage_test_fault fault;
#endif
};

static void clear_bytes(void *pointer, size_t size)
{
    volatile uint8_t *bytes = (volatile uint8_t *)pointer;
    while (size--) *bytes++ = 0;
}

static int same_inode(const struct stat *a, const struct stat *b)
{
    return a->st_dev == b->st_dev && a->st_ino == b->st_ino;
}

static int no_core_dumps(void)
{
    struct rlimit limit = {0, 0};
    if (setrlimit(RLIMIT_CORE, &limit) != 0 ||
        prctl(PR_SET_DUMPABLE, 0, 0, 0, 0) != 0) return -1;
    return 0;
}

static int trusted_tmp(int supplied_fd)
{
    int tmp_fd;
    struct stat supplied, actual;
    if (supplied_fd < 0 || fstat(supplied_fd, &supplied) != 0 ||
        !S_ISDIR(supplied.st_mode)) return -1;
    tmp_fd = open("/tmp", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (tmp_fd < 0) return -1;
    if (fstat(tmp_fd, &actual) != 0 || !same_inode(&supplied, &actual) ||
        actual.st_uid != 0 || (actual.st_mode & 01777) != 01777) {
        close(tmp_fd);
        return -1;
    }
    close(tmp_fd);
    return 0;
}

static int fresh_leaf(char out[48])
{
    static const char digits[] = "0123456789abcdef";
    uint8_t random_bytes[16];
    size_t done = 0;
    memcpy(out, "rq-host-", 8);
    while (done < sizeof random_bytes) {
        ssize_t count = getrandom(random_bytes + done,
                                  sizeof random_bytes - done, 0);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) return -1;
        done += (size_t)count;
    }
    for (size_t i = 0; i < sizeof random_bytes; ++i) {
        out[8 + i * 2] = digits[random_bytes[i] >> 4];
        out[9 + i * 2] = digits[random_bytes[i] & 15];
    }
    out[40] = 0;
    clear_bytes(random_bytes, sizeof random_bytes);
    return 0;
}

static int check_parent(struct rq_storage *storage)
{
    struct stat current;
    if (fstat(storage->parent_fd, &current) != 0 ||
        current.st_dev != storage->parent_dev ||
        current.st_ino != storage->parent_ino ||
        trusted_tmp(storage->parent_fd) != 0) return -1;
    return 0;
}

static int check_dir(struct rq_storage *storage)
{
    struct stat by_fd, by_name;
    if (check_parent(storage) != 0 ||
        fstat(storage->dir_fd, &by_fd) != 0 ||
        fstatat(storage->parent_fd, storage->leaf, &by_name,
                AT_SYMLINK_NOFOLLOW) != 0 ||
        !same_inode(&by_fd, &by_name) ||
        by_fd.st_dev != storage->dir_dev ||
        by_fd.st_ino != storage->dir_ino ||
        !S_ISDIR(by_fd.st_mode) || by_fd.st_uid != geteuid() ||
        (by_fd.st_mode & 07777) != 0700 || by_fd.st_nlink < 2)
        return -1;
    return 0;
}

static int check_regular(struct rq_storage *storage, int fd,
                         const char *name, dev_t device, ino_t inode,
                         off_t exact_size)
{
    struct stat by_fd, by_name;
    if (check_dir(storage) != 0 || fstat(fd, &by_fd) != 0 ||
        fstatat(storage->dir_fd, name, &by_name, AT_SYMLINK_NOFOLLOW) != 0 ||
        !same_inode(&by_fd, &by_name) || by_fd.st_dev != device ||
        by_fd.st_ino != inode || !S_ISREG(by_fd.st_mode) ||
        by_fd.st_uid != geteuid() || by_fd.st_nlink != 1 ||
        (by_fd.st_mode & 07777) != 0600 || by_fd.st_size != exact_size)
        return -1;
    return 0;
}

static int pwrite_all(int fd, const uint8_t *bytes, size_t size, off_t offset)
{
    size_t written = 0;
    while (written < size) {
        ssize_t count = pwrite(fd, bytes + written, size - written,
                               offset + (off_t)written);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) return -1;
        written += (size_t)count;
    }
    return 0;
}

static int pread_all(int fd, uint8_t *bytes, size_t size, off_t offset)
{
    size_t read_count = 0;
    while (read_count < size) {
        ssize_t count = pread(fd, bytes + read_count, size - read_count,
                              offset + (off_t)read_count);
        if (count < 0 && errno == EINTR) continue;
        if (count <= 0) return -1;
        read_count += (size_t)count;
    }
    return 0;
}

static int create_regular(struct rq_storage *storage, const char *name,
                          int *fd, dev_t *device, ino_t *inode)
{
    struct stat info;
    if (check_dir(storage) != 0) return -1;
    *fd = openat(storage->dir_fd, name,
                 O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC, 0600);
    if (*fd < 0 || fstat(*fd, &info) != 0 || !S_ISREG(info.st_mode) ||
        info.st_uid != geteuid() || info.st_nlink != 1 ||
        (info.st_mode & 07777) != 0600 || info.st_size != 0 ||
        info.st_dev != storage->dir_dev) return -1;
    *device = info.st_dev;
    *inode = info.st_ino;
    return 0;
}

static int create_private_directory(struct rq_storage *storage)
{
    struct stat info;
    for (unsigned attempt = 0; attempt < 16; ++attempt) {
        if (fresh_leaf(storage->leaf) != 0) return -1;
        if (mkdirat(storage->parent_fd, storage->leaf, 0700) == 0) break;
        if (errno != EEXIST || attempt == 15) return -1;
    }
    storage->dir_fd = openat(storage->parent_fd, storage->leaf,
                             O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    if (storage->dir_fd < 0 || fstat(storage->dir_fd, &info) != 0)
        return -1;
    storage->dir_dev = info.st_dev;
    storage->dir_ino = info.st_ino;
    return check_dir(storage);
}

struct rq_storage *rq_storage_create(const char *host_ack, int parent_dirfd,
                                     const uint8_t *synthetic_baseline,
                                     size_t baseline_len,
                                     const uint8_t pinned_sha256[32])
{
    struct rq_storage *storage;
    struct stat parent_info;
    uint8_t clone_digest[32] = {0};
    if (!host_ack || strcmp(host_ack, RQ_HOST_ACK) != 0 ||
        !synthetic_baseline || !pinned_sha256 ||
        baseline_len != RQ_BASELINE_BYTES || trusted_tmp(parent_dirfd) != 0 ||
        no_core_dumps() != 0) return NULL;
    storage = (struct rq_storage *)calloc(1, sizeof *storage);
    if (!storage) return NULL;
    storage->parent_fd = -1;
    storage->dir_fd = -1;
    storage->candidate_fd = -1;
    storage->marker_fd = -1;
    storage->core = rq_create(host_ack, synthetic_baseline, baseline_len,
                              pinned_sha256);
    if (!storage->core) goto failure;
    storage->parent_fd = fcntl(parent_dirfd, F_DUPFD_CLOEXEC, 3);
    if (storage->parent_fd < 0 ||
        fstat(storage->parent_fd, &parent_info) != 0) goto failure;
    storage->parent_dev = parent_info.st_dev;
    storage->parent_ino = parent_info.st_ino;
    if (create_private_directory(storage) != 0 ||
        create_regular(storage, RQ_NO_PROMOTION, &storage->marker_fd,
                       &storage->marker_dev, &storage->marker_ino) != 0 ||
        pwrite_all(storage->marker_fd, no_promotion_text,
                   sizeof no_promotion_text - 1, 0) != 0 ||
        check_regular(storage, storage->marker_fd, RQ_NO_PROMOTION,
                      storage->marker_dev, storage->marker_ino,
                      (off_t)(sizeof no_promotion_text - 1)) != 0 ||
        fsync(storage->marker_fd) != 0 ||
        create_regular(storage, RQ_CANDIDATE, &storage->candidate_fd,
                       &storage->candidate_dev,
                       &storage->candidate_ino) != 0)
        goto failure;
    storage->readback = (uint8_t *)malloc(RQ_BASELINE_BYTES);
    if (!storage->readback) goto failure;
    for (size_t offset = 0; offset < RQ_BASELINE_BYTES; offset += 4096) {
        if (pwrite_all(storage->candidate_fd, synthetic_baseline + offset,
                       4096, (off_t)offset) != 0) goto failure;
    }
    if (check_regular(storage, storage->candidate_fd, RQ_CANDIDATE,
                      storage->candidate_dev, storage->candidate_ino,
                      RQ_BASELINE_BYTES) != 0 ||
        fsync(storage->candidate_fd) != 0 ||
        pread_all(storage->candidate_fd, storage->readback,
                  RQ_BASELINE_BYTES, 0) != 0 ||
        memcmp(storage->readback, synthetic_baseline,
               RQ_BASELINE_BYTES) != 0 ||
        rq_host_sha256_fixture(storage->readback, RQ_BASELINE_BYTES,
                               clone_digest) != 0 ||
        memcmp(clone_digest, pinned_sha256, sizeof clone_digest) != 0 ||
        fsync(storage->dir_fd) != 0 || fsync(storage->parent_fd) != 0)
        goto failure;
    clear_bytes(clone_digest, sizeof clone_digest);
    clear_bytes(storage->readback, RQ_BASELINE_BYTES);
    return storage;
failure:
    clear_bytes(clone_digest, sizeof clone_digest);
    rq_storage_destroy(storage);
    return NULL;
}

void rq_storage_destroy(struct rq_storage *storage)
{
    if (!storage) return;
    rq_destroy(storage->core);
    if (storage->readback) {
        clear_bytes(storage->readback, RQ_BASELINE_BYTES);
        free(storage->readback);
    }
    if (storage->marker_fd >= 0) close(storage->marker_fd);
    if (storage->candidate_fd >= 0) close(storage->candidate_fd);
    if (storage->dir_fd >= 0) close(storage->dir_fd);
    if (storage->parent_fd >= 0) close(storage->parent_fd);
    clear_bytes(storage, sizeof *storage);
    free(storage);
}

static int candidate_ok(struct rq_storage *storage)
{
    uint8_t marker_readback[sizeof no_promotion_text - 1];
    if (check_regular(storage, storage->candidate_fd, RQ_CANDIDATE,
                      storage->candidate_dev, storage->candidate_ino,
                      RQ_BASELINE_BYTES) != 0 ||
        check_regular(storage, storage->marker_fd, RQ_NO_PROMOTION,
                      storage->marker_dev, storage->marker_ino,
                      (off_t)(sizeof no_promotion_text - 1)) != 0 ||
        pread_all(storage->marker_fd, marker_readback,
                  sizeof marker_readback, 0) != 0 ||
        memcmp(marker_readback, no_promotion_text,
               sizeof marker_readback) != 0)
        return -1;
    return 0;
}

static int finish_candidate(struct rq_storage *storage)
{
    uint8_t digest[32], sidecar_readback[32];
    uint8_t *expected = NULL;
    int sidecar_fd = -1;
    struct stat sidecar_info;
    int success = -1;
    if (candidate_ok(storage) != 0) return -1;
    expected = (uint8_t *)malloc(RQ_BASELINE_BYTES);
    if (!expected || rq_copy_completed_candidate(storage->core, expected,
                                                  RQ_BASELINE_BYTES) != 0)
        goto done;
#ifdef RQ_STORAGE_HOST_TEST
    if (storage->fault == RQ_STORAGE_FAIL_FINAL_FSYNC) goto done;
#endif
    if (fsync(storage->candidate_fd) != 0 ||
        pread_all(storage->candidate_fd, storage->readback,
                  RQ_BASELINE_BYTES, 0) != 0 ||
        memcmp(storage->readback, expected, RQ_BASELINE_BYTES) != 0 ||
        rq_host_sha256_fixture(storage->readback, RQ_BASELINE_BYTES,
                               digest) != 0)
        goto done;
    if (check_dir(storage) != 0 || candidate_ok(storage) != 0) goto done;
    sidecar_fd = openat(storage->dir_fd, RQ_SIDECAR,
                        O_RDWR | O_CREAT | O_EXCL | O_NOFOLLOW | O_CLOEXEC,
                        0600);
    if (sidecar_fd < 0 || fstat(sidecar_fd, &sidecar_info) != 0 ||
        !S_ISREG(sidecar_info.st_mode) ||
        sidecar_info.st_uid != geteuid() || sidecar_info.st_nlink != 1 ||
        (sidecar_info.st_mode & 07777) != 0600 ||
        sidecar_info.st_size != 0 || sidecar_info.st_dev != storage->dir_dev ||
        pwrite_all(sidecar_fd, digest, sizeof digest, 0) != 0 ||
        check_regular(storage, sidecar_fd, RQ_SIDECAR,
                      sidecar_info.st_dev, sidecar_info.st_ino, 32) != 0)
        goto done;
#ifdef RQ_STORAGE_HOST_TEST
    if (storage->fault == RQ_STORAGE_FAIL_SIDECAR_FSYNC) goto done;
#endif
    if (fsync(sidecar_fd) != 0 || fsync(storage->dir_fd) != 0 ||
        fsync(storage->parent_fd) != 0 || candidate_ok(storage) != 0 ||
        pread_all(storage->candidate_fd, storage->readback,
                  RQ_BASELINE_BYTES, 0) != 0 ||
        memcmp(storage->readback, expected, RQ_BASELINE_BYTES) != 0 ||
        pread_all(sidecar_fd, sidecar_readback, sizeof sidecar_readback,
                  0) != 0 ||
        memcmp(sidecar_readback, digest, sizeof digest) != 0 ||
        candidate_ok(storage) != 0 ||
        check_regular(storage, sidecar_fd, RQ_SIDECAR,
                      sidecar_info.st_dev, sidecar_info.st_ino, 32) != 0)
        goto done;
    success = 0;
done:
    if (sidecar_fd >= 0) {
        int close_result = close(sidecar_fd);
        /* On Linux, do not retry close after an error: the fd number may
         * already have been released and reused. Withhold status instead. */
#ifdef RQ_STORAGE_HOST_TEST
        if (storage->fault == RQ_STORAGE_FAIL_SIDECAR_CLOSE &&
            close_result == 0) close_result = -1;
#endif
        if (close_result != 0) success = -1;
    }
    if (expected) {
        clear_bytes(expected, RQ_BASELINE_BYTES);
        free(expected);
    }
    clear_bytes(storage->readback, RQ_BASELINE_BYTES);
    clear_bytes(digest, sizeof digest);
    clear_bytes(sidecar_readback, sizeof sidecar_readback);
    return success;
}

static enum rq_result disarm(struct rq_storage *storage, size_t *reply_len)
{
    if (reply_len) *reply_len = 0;
    if (storage) {
        storage->failed = 1;
        storage->complete = 0;
        rq_destroy(storage->core);
        storage->core = NULL;
        if (storage->readback)
            clear_bytes(storage->readback, RQ_BASELINE_BYTES);
    }
    return RQ_ERROR;
}

enum rq_result rq_storage_accept(struct rq_storage *storage,
                                 const uint8_t *frame, size_t frame_len,
                                 uint8_t *reply, size_t reply_cap,
                                 size_t *reply_len)
{
    uint8_t delayed_reply[RQ_MAX_RESPONSE_BYTES] = {0};
    size_t delayed_len = 0;
    size_t before, after;
    enum rq_state previous;
    enum rq_result result;
    if (reply_len) *reply_len = 0;
    if (!storage) return RQ_ERROR;
    if (storage->complete) return RQ_ERROR;
    if (storage->failed || !reply_len || !reply ||
        reply_cap < RQ_MAX_RESPONSE_BYTES || candidate_ok(storage) != 0)
        return disarm(storage, reply_len);
    previous = rq_get_state(storage->core);
    before = rq_bytes_received(storage->core);
    result = rq_accept(storage->core, frame, frame_len, delayed_reply,
                       sizeof delayed_reply, &delayed_len);
    if (result == RQ_ERROR ||
        (result == RQ_NO_REPLY && delayed_len != 0) ||
        (result == RQ_REPLY && delayed_len != 16 && delayed_len != 20))
        goto failure;
    after = rq_bytes_received(storage->core);
    if (previous == RQ_WAIT_DATA) {
        size_t chunk = after - before;
        if (after <= before || after > RQ_WRITE_BYTES ||
            chunk > RQ_CHUNK_BYTES || frame_len != 20 + chunk)
            goto failure;
#ifdef RQ_STORAGE_HOST_TEST
        if (storage->fault == RQ_STORAGE_FAIL_CHUNK_WRITE) goto failure;
#endif
        if (pwrite_all(storage->candidate_fd, frame + 20, chunk,
                       (off_t)before) != 0 || candidate_ok(storage) != 0)
            goto failure;
    }
    if (rq_get_state(storage->core) == RQ_COMPLETE) {
        if (previous != RQ_WAIT_DATA || result != RQ_REPLY ||
            delayed_len != 16 || finish_candidate(storage) != 0)
            goto failure;
        storage->complete = 1;
        rq_destroy(storage->core);
        storage->core = NULL;
    }
    if (result == RQ_REPLY) {
        if (delayed_len > reply_cap) goto failure;
        memcpy(reply, delayed_reply, delayed_len);
        *reply_len = delayed_len;
    }
    clear_bytes(delayed_reply, sizeof delayed_reply);
    return result;
failure:
    clear_bytes(delayed_reply, sizeof delayed_reply);
    return disarm(storage, reply_len);
}

int rq_storage_complete(const struct rq_storage *storage)
{
    return storage && storage->complete && !storage->failed;
}

int rq_storage_failed(const struct rq_storage *storage)
{
    return !storage || storage->failed;
}

const char *rq_storage_leaf(const struct rq_storage *storage)
{
    return storage ? storage->leaf : NULL;
}

#ifdef RQ_STORAGE_HOST_TEST
void rq_storage_test_set_fault(struct rq_storage *storage,
                               enum rq_storage_test_fault fault)
{
    if (storage) storage->fault = fault;
}

int rq_storage_test_dir_identity(const struct rq_storage *storage,
                                 dev_t *device, ino_t *inode)
{
    if (!storage || !device || !inode || storage->dir_fd < 0) return -1;
    *device = storage->dir_dev;
    *inode = storage->dir_ino;
    return 0;
}
#endif
