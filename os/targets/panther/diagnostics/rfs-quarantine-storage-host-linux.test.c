/* Synthetic host fixture only. Every created file lives under real /tmp. */
#define _GNU_SOURCE
#include "rfs-quarantine-storage-host-linux.h"

#include <assert.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

static const uint8_t request_7[] =
    {7,0,0,0, 4,0,0,0, 3,0,0,0};
static const uint8_t request_3[] =
    {3,0,0,0, 12,0,0,0, 0,0,0,0, 3,0,0,0, 0,0,0,0};
static const uint8_t request_6[] =
    {6,0,1,0, 16,0,0,0, 3,0,0,0, 0,0,0,0,
     0x06,0xe4,0x02,0, 2,0,0,0};

static uint32_t le32(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) |
           ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static void put32(uint8_t *p, uint32_t value)
{
    p[0] = (uint8_t)value;
    p[1] = (uint8_t)(value >> 8);
    p[2] = (uint8_t)(value >> 16);
    p[3] = (uint8_t)(value >> 24);
}

static void make_data(uint8_t frame[20 + RQ_CHUNK_BYTES],
                      unsigned offset, unsigned chunk)
{
    assert(chunk > 0 && chunk <= RQ_CHUNK_BYTES);
    memset(frame, 0, 20 + chunk);
    frame[0] = 2;
    frame[2] = 1;
    put32(frame + 4, 12 + chunk);
    put32(frame + 12, 3);
    put32(frame + 16, chunk);
    for (unsigned i = 0; i < chunk; ++i)
        frame[20 + i] = (uint8_t)((offset + i) * 37u + 11u);
}

static uint8_t *baseline_fixture(uint8_t digest[32])
{
    uint8_t *baseline = (uint8_t *)malloc(RQ_BASELINE_BYTES);
    assert(baseline);
    for (size_t i = 0; i < RQ_BASELINE_BYTES; ++i)
        baseline[i] = (uint8_t)(i * 29u + 7u);
    assert(rq_host_sha256_fixture(baseline, RQ_BASELINE_BYTES, digest) == 0);
    return baseline;
}

static int tmp_fd(void)
{
    int fd = open("/tmp", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    assert(fd >= 0);
    return fd;
}

static struct rq_storage *new_storage(int parent, const uint8_t *baseline,
                                       const uint8_t digest[32])
{
    struct rq_storage *storage =
        rq_storage_create(RQ_HOST_ACK, parent, baseline,
                          RQ_BASELINE_BYTES, digest);
    assert(storage);
    assert(strncmp(rq_storage_leaf(storage), "rq-host-", 8) == 0);
    return storage;
}

static int storage_dir(int parent, const struct rq_storage *storage)
{
    struct stat info;
    int fd = openat(parent, rq_storage_leaf(storage),
                    O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    assert(fd >= 0 && fstat(fd, &info) == 0);
    assert(S_ISDIR(info.st_mode) && info.st_uid == geteuid());
    assert((info.st_mode & 07777) == 0700);
    return fd;
}

static int exists(int dir_fd, const char *name)
{
    struct stat info;
    return fstatat(dir_fd, name, &info, AT_SYMLINK_NOFOLLOW) == 0;
}

static void cleanup(int parent, struct rq_storage *storage)
{
    char leaf[48];
    int directory = storage_dir(parent, storage);
    dev_t expected_device;
    ino_t expected_inode;
    struct stat opened, named;
    assert(rq_storage_test_dir_identity(storage, &expected_device,
                                         &expected_inode) == 0);
    strncpy(leaf, rq_storage_leaf(storage), sizeof leaf);
    leaf[sizeof leaf - 1] = 0;
    assert(fstat(directory, &opened) == 0);
    assert(fstatat(parent, leaf, &named, AT_SYMLINK_NOFOLLOW) == 0);
    assert(S_ISDIR(named.st_mode) && opened.st_dev == expected_device &&
           opened.st_ino == expected_inode &&
           named.st_dev == expected_device && named.st_ino == expected_inode);
    rq_storage_destroy(storage);
    assert(fstat(directory, &opened) == 0);
    assert(fstatat(parent, leaf, &named, AT_SYMLINK_NOFOLLOW) == 0);
    assert(opened.st_dev == expected_device &&
           opened.st_ino == expected_inode &&
           named.st_dev == expected_device && named.st_ino == expected_inode);
    if (exists(directory, "candidate-hardlink.bin"))
        assert(unlinkat(directory, "candidate-hardlink.bin", 0) == 0);
    if (exists(directory, "candidate.sha256"))
        assert(unlinkat(directory, "candidate.sha256", 0) == 0);
    if (exists(directory, "candidate.bin"))
        assert(unlinkat(directory, "candidate.bin", 0) == 0);
    if (exists(directory, "NO_PROMOTION"))
        assert(unlinkat(directory, "NO_PROMOTION", 0) == 0);
    assert(fstatat(parent, leaf, &named, AT_SYMLINK_NOFOLLOW) == 0);
    assert(named.st_dev == expected_device && named.st_ino == expected_inode);
    assert(close(directory) == 0);
    assert(unlinkat(parent, leaf, AT_REMOVEDIR) == 0);
}

static void start_transfer(struct rq_storage *storage,
                           uint8_t grant[RQ_MAX_RESPONSE_BYTES])
{
    size_t length = 999;
    assert(rq_storage_accept(storage, request_7, sizeof request_7,
                             grant, RQ_MAX_RESPONSE_BYTES, &length) == RQ_REPLY);
    assert(length == 16 && grant[0] == 3 && le32(grant + 12) == 3);
    assert(rq_storage_accept(storage, request_3, sizeof request_3,
                             grant, RQ_MAX_RESPONSE_BYTES,
                             &length) == RQ_NO_REPLY);
    assert(length == 0);
    assert(rq_storage_accept(storage, request_6, sizeof request_6,
                             grant, RQ_MAX_RESPONSE_BYTES,
                             &length) == RQ_REPLY);
    assert(length == 20 && grant[0] == 2 && grant[2] == 1);
    assert(le32(grant + 12) == 0 && le32(grant + 16) == 2012);
}

static void send_chunk(struct rq_storage *storage,
                       uint8_t grant[RQ_MAX_RESPONSE_BYTES],
                       unsigned expected_offset, int final,
                       enum rq_result expected_result)
{
    uint8_t frame[20 + RQ_CHUNK_BYTES];
    unsigned offset = le32(grant + 12);
    unsigned chunk = le32(grant + 16);
    size_t length = 999;
    assert(offset == expected_offset && chunk > 0 && chunk <= RQ_CHUNK_BYTES);
    make_data(frame, offset, chunk);
    assert(rq_storage_accept(storage, frame, 20 + chunk, grant,
                             RQ_MAX_RESPONSE_BYTES, &length) == expected_result);
    if (expected_result == RQ_ERROR) {
        assert(length == 0 && rq_storage_failed(storage));
    } else if (final) {
        assert(expected_result == RQ_REPLY && length == 16 && grant[0] == 3);
        assert(grant[2] == 1 && rq_storage_complete(storage));
    } else {
        assert(expected_result == RQ_REPLY && length == 20 && grant[0] == 2);
        assert(le32(grant + 12) == offset + chunk);
    }
}

static void test_provenance_and_pin(int parent, const uint8_t *baseline,
                                    const uint8_t digest[32])
{
    uint8_t wrong[32];
    int other = open("/", O_RDONLY | O_DIRECTORY | O_NOFOLLOW | O_CLOEXEC);
    assert(other >= 0);
    assert(rq_storage_create(RQ_HOST_ACK, other, baseline,
                             RQ_BASELINE_BYTES, digest) == NULL);
    assert(close(other) == 0);
    memcpy(wrong, digest, sizeof wrong);
    wrong[0] ^= 1;
    assert(rq_storage_create(RQ_HOST_ACK, parent, baseline,
                             RQ_BASELINE_BYTES, wrong) == NULL);
    assert(rq_storage_create("no-host-ack", parent, baseline,
                             RQ_BASELINE_BYTES, digest) == NULL);
    puts("PASS parent provenance, pinned digest, consent");
}

static void test_complete(int parent, const uint8_t *baseline,
                           const uint8_t digest[32])
{
    uint8_t grant[RQ_MAX_RESPONSE_BYTES], final_digest[32], sidecar[32];
    uint8_t *candidate = (uint8_t *)malloc(RQ_BASELINE_BYTES);
    struct rq_storage *storage = new_storage(parent, baseline, digest);
    int directory = storage_dir(parent, storage);
    int fd = openat(directory, "candidate.bin", O_RDONLY | O_NOFOLLOW);
    struct stat info;
    assert(candidate && fd >= 0 && fstat(fd, &info) == 0);
    assert(S_ISREG(info.st_mode) && info.st_size == RQ_BASELINE_BYTES);
    assert(info.st_nlink == 1 && info.st_uid == geteuid());
    assert((info.st_mode & 07777) == 0600);
    assert(read(fd, candidate, RQ_BASELINE_BYTES) == RQ_BASELINE_BYTES);
    assert(memcmp(candidate, baseline, RQ_BASELINE_BYTES) == 0);
    assert(exists(directory, "NO_PROMOTION"));
    assert(!exists(directory, "candidate.sha256"));
    start_transfer(storage, grant);
    for (unsigned offset = 0, grant_number = 0; offset < RQ_WRITE_BYTES;
         ++grant_number) {
        unsigned chunk = le32(grant + 16);
        assert(grant_number < RQ_MAX_GRANTS);
        send_chunk(storage, grant, offset, offset + chunk == RQ_WRITE_BYTES,
                   RQ_REPLY);
        offset += chunk;
    }
    assert(rq_storage_complete(storage) && !rq_storage_failed(storage));
    assert(lseek(fd, 0, SEEK_SET) == 0);
    assert(read(fd, candidate, RQ_BASELINE_BYTES) == RQ_BASELINE_BYTES);
    for (size_t i = 0; i < RQ_WRITE_BYTES; ++i)
        assert(candidate[i] == (uint8_t)(i * 37u + 11u));
    assert(memcmp(candidate + RQ_WRITE_BYTES, baseline + RQ_WRITE_BYTES,
                  RQ_BASELINE_BYTES - RQ_WRITE_BYTES) == 0);
    assert(rq_host_sha256_fixture(candidate, RQ_BASELINE_BYTES,
                                  final_digest) == 0);
    int sidecar_fd = openat(directory, "candidate.sha256",
                            O_RDONLY | O_NOFOLLOW);
    assert(sidecar_fd >= 0);
    assert(read(sidecar_fd, sidecar, sizeof sidecar) == sizeof sidecar);
    assert(memcmp(sidecar, final_digest, sizeof sidecar) == 0);
    assert(exists(directory, "NO_PROMOTION"));
    assert(close(sidecar_fd) == 0 && close(fd) == 0);
    assert(close(directory) == 0);
    free(candidate);
    cleanup(parent, storage);
    puts("PASS 95 grants, tail preservation, reread, sidecar, no promotion");
}

static void test_malformed(int parent, const uint8_t *baseline,
                            const uint8_t digest[32])
{
    uint8_t grant[RQ_MAX_RESPONSE_BYTES], bad[20 + RQ_CHUNK_BYTES];
    struct rq_storage *storage = new_storage(parent, baseline, digest);
    int directory = storage_dir(parent, storage);
    size_t length = 999;
    start_transfer(storage, grant);
    make_data(bad, 0, 2012);
    bad[2] = 2;
    assert(rq_storage_accept(storage, bad, sizeof bad, grant,
                             sizeof grant, &length) == RQ_ERROR);
    assert(length == 0 && rq_storage_failed(storage));
    assert(!exists(directory, "candidate.sha256"));
    assert(exists(directory, "NO_PROMOTION"));
    assert(close(directory) == 0);
    cleanup(parent, storage);
    puts("PASS malformed frame refuses grant and leaves quarantine");
}

static void test_tamper(int parent, const uint8_t *baseline,
                         const uint8_t digest[32])
{
    uint8_t grant[RQ_MAX_RESPONSE_BYTES], frame[20 + RQ_CHUNK_BYTES];
    size_t length;
    struct rq_storage *storage = new_storage(parent, baseline, digest);
    int directory = storage_dir(parent, storage);
    start_transfer(storage, grant);
    assert(linkat(directory, "candidate.bin", directory,
                  "candidate-hardlink.bin", 0) == 0);
    make_data(frame, 0, 2012);
    length = 999;
    assert(rq_storage_accept(storage, frame, sizeof frame, grant,
                             sizeof grant, &length) == RQ_ERROR);
    assert(length == 0 && rq_storage_failed(storage));
    assert(!exists(directory, "candidate.sha256"));
    assert(close(directory) == 0);
    cleanup(parent, storage);

    storage = new_storage(parent, baseline, digest);
    directory = storage_dir(parent, storage);
    start_transfer(storage, grant);
    assert(unlinkat(directory, "candidate.bin", 0) == 0);
    assert(symlinkat("NO_PROMOTION", directory, "candidate.bin") == 0);
    length = 999;
    assert(rq_storage_accept(storage, frame, sizeof frame, grant,
                             sizeof grant, &length) == RQ_ERROR);
    assert(length == 0 && rq_storage_failed(storage));
    assert(!exists(directory, "candidate.sha256"));
    assert(close(directory) == 0);
    cleanup(parent, storage);
    puts("PASS hardlink and symlink replacement refuse responses");
}

static void test_metadata_tamper(int parent, const uint8_t *baseline,
                                 const uint8_t digest[32])
{
    uint8_t grant[RQ_MAX_RESPONSE_BYTES], frame[20 + RQ_CHUNK_BYTES];
    static const char *const target[] = {
        "candidate.bin", "candidate.bin", "NO_PROMOTION"
    };
    for (unsigned kind = 0; kind < 3; ++kind) {
        struct rq_storage *storage = new_storage(parent, baseline, digest);
        int directory = storage_dir(parent, storage);
        int fd = openat(directory, target[kind], O_RDWR | O_NOFOLLOW);
        size_t length = 999;
        assert(fd >= 0);
        start_transfer(storage, grant);
        make_data(frame, 0, 2012);
        if (kind == 0) {
            assert(ftruncate(fd, RQ_BASELINE_BYTES - 1) == 0);
        } else if (kind == 1) {
            assert(fchmod(fd, 0400) == 0);
        } else {
            uint8_t changed = 'X';
            assert(pwrite(fd, &changed, 1, 0) == 1);
        }
        assert(rq_storage_accept(storage, frame, sizeof frame, grant,
                                 sizeof grant, &length) == RQ_ERROR);
        assert(length == 0 && rq_storage_failed(storage));
        assert(!exists(directory, "candidate.sha256"));
        assert(close(fd) == 0 && close(directory) == 0);
        cleanup(parent, storage);
    }
    puts("PASS candidate size/mode and quarantine marker tampering");
}

static void test_fault(int parent, const uint8_t *baseline,
                        const uint8_t digest[32],
                        enum rq_storage_test_fault fault)
{
    uint8_t grant[RQ_MAX_RESPONSE_BYTES];
    struct rq_storage *storage = new_storage(parent, baseline, digest);
    int directory = storage_dir(parent, storage);
    start_transfer(storage, grant);
    if (fault == RQ_STORAGE_FAIL_CHUNK_WRITE) {
        rq_storage_test_set_fault(storage, fault);
        send_chunk(storage, grant, 0, 0, RQ_ERROR);
    } else {
        for (unsigned offset = 0; offset < RQ_WRITE_BYTES;) {
            unsigned chunk = le32(grant + 16);
            int final = offset + chunk == RQ_WRITE_BYTES;
            if (final) rq_storage_test_set_fault(storage, fault);
            send_chunk(storage, grant, offset, final,
                       final ? RQ_ERROR : RQ_REPLY);
            offset += chunk;
        }
    }
    assert(!rq_storage_complete(storage) && rq_storage_failed(storage));
    assert(exists(directory, "NO_PROMOTION"));
    if (fault == RQ_STORAGE_FAIL_FINAL_FSYNC)
        assert(!exists(directory, "candidate.sha256"));
    assert(close(directory) == 0);
    cleanup(parent, storage);
}

static void test_preexisting_sidecar(int parent, const uint8_t *baseline,
                                     const uint8_t digest[32])
{
    uint8_t grant[RQ_MAX_RESPONSE_BYTES];
    struct rq_storage *storage = new_storage(parent, baseline, digest);
    int directory = storage_dir(parent, storage);
    start_transfer(storage, grant);
    assert(symlinkat("NO_PROMOTION", directory, "candidate.sha256") == 0);
    for (unsigned offset = 0; offset < RQ_WRITE_BYTES;) {
        unsigned chunk = le32(grant + 16);
        int final = offset + chunk == RQ_WRITE_BYTES;
        send_chunk(storage, grant, offset, final,
                   final ? RQ_ERROR : RQ_REPLY);
        offset += chunk;
    }
    assert(!rq_storage_complete(storage) && rq_storage_failed(storage));
    assert(exists(directory, "NO_PROMOTION"));
    assert(close(directory) == 0);
    cleanup(parent, storage);
    puts("PASS pre-existing checksum path withholds final status");
}

int main(void)
{
    uint8_t digest[32];
    uint8_t *baseline = baseline_fixture(digest);
    int parent = tmp_fd();
    test_provenance_and_pin(parent, baseline, digest);
    test_complete(parent, baseline, digest);
    test_malformed(parent, baseline, digest);
    test_tamper(parent, baseline, digest);
    test_metadata_tamper(parent, baseline, digest);
    test_fault(parent, baseline, digest, RQ_STORAGE_FAIL_CHUNK_WRITE);
    test_fault(parent, baseline, digest, RQ_STORAGE_FAIL_FINAL_FSYNC);
    test_fault(parent, baseline, digest, RQ_STORAGE_FAIL_SIDECAR_FSYNC);
    test_fault(parent, baseline, digest, RQ_STORAGE_FAIL_SIDECAR_CLOSE);
    puts("PASS injected chunk-write, fsync and close failures withhold status");
    test_preexisting_sidecar(parent, baseline, digest);
    assert(close(parent) == 0);
    memset(baseline, 0, RQ_BASELINE_BYTES);
    free(baseline);
    return 0;
}
