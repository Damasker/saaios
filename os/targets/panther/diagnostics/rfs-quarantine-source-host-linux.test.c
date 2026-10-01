/* Synthetic verified-fd boundary only; never opens real NV/EFS. */
#define _GNU_SOURCE
#include "rfs-quarantine-source-host-linux.h"

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

static void write_exact(int fd, const uint8_t *bytes, size_t len)
{
    size_t used = 0;
    while (used < len) {
        ssize_t n = pwrite(fd, bytes + used, len - used, (off_t)used);
        assert(n > 0);
        used += (size_t)n;
    }
    assert(fsync(fd) == 0);
}

static void assert_zero(const uint8_t *bytes, size_t len)
{
    for (size_t i = 0; i < len; ++i) assert(bytes[i] == 0);
}

int main(void)
{
    char path[] = "/tmp/rq-source-XXXXXX";
    char link_path[sizeof path + 6];
    uint8_t digest[32], wrong_digest[32];
    uint8_t *baseline = malloc(RQ_BASELINE_BYTES);
    uint8_t *copy = malloc(RQ_BASELINE_BYTES);
    assert(baseline && copy);
    for (size_t i = 0; i < RQ_BASELINE_BYTES; ++i)
        baseline[i] = (uint8_t)(i * 13u + 5u);
    assert(rq_host_sha256_fixture(baseline, RQ_BASELINE_BYTES, digest) == 0);
    memcpy(wrong_digest, digest, sizeof digest);
    wrong_digest[0] ^= 1;

    int writable = mkstemp(path);
    assert(writable >= 0);
    struct stat created, named;
    assert(fstat(writable, &created) == 0 && S_ISREG(created.st_mode) &&
           (created.st_mode & 07777) == 0600 && created.st_nlink == 1);
    write_exact(writable, baseline, RQ_BASELINE_BYTES);
    int readonly = open(path, O_RDONLY | O_NOFOLLOW | O_CLOEXEC);
    assert(readonly >= 0);
    assert(rqs_read_verified_fd(RQS_HOST_ACK, readonly, digest, copy,
                                RQ_BASELINE_BYTES) == 0);
    assert(memcmp(copy, baseline, RQ_BASELINE_BYTES) == 0);
    assert(rqs_read_verified_fd(RQS_HOST_ACK, readonly, wrong_digest, copy,
                                RQ_BASELINE_BYTES) == -1);
    assert_zero(copy, RQ_BASELINE_BYTES);
    assert(rqs_read_verified_fd("wrong", readonly, digest, copy,
                                RQ_BASELINE_BYTES) == -1);
    assert_zero(copy, RQ_BASELINE_BYTES);
    assert(rqs_read_verified_fd(RQS_HOST_ACK, writable, digest, copy,
                                RQ_BASELINE_BYTES) == -1);
    assert_zero(copy, RQ_BASELINE_BYTES);

    assert(fchmod(writable, 0644) == 0);
    assert(rqs_read_verified_fd(RQS_HOST_ACK, readonly, digest, copy,
                                RQ_BASELINE_BYTES) == -1);
    assert_zero(copy, RQ_BASELINE_BYTES);
    assert(fchmod(writable, 0600) == 0);
    assert(snprintf(link_path, sizeof link_path, "%s.link", path) > 0);
    assert(link(path, link_path) == 0);
    assert(rqs_read_verified_fd(RQS_HOST_ACK, readonly, digest, copy,
                                RQ_BASELINE_BYTES) == -1);
    assert_zero(copy, RQ_BASELINE_BYTES);
    assert(unlink(link_path) == 0);

    assert(ftruncate(writable, RQ_BASELINE_BYTES - 1) == 0);
    assert(rqs_read_verified_fd(RQS_HOST_ACK, readonly, digest, copy,
                                RQ_BASELINE_BYTES) == -1);
    assert_zero(copy, RQ_BASELINE_BYTES);
    assert(ftruncate(writable, RQ_BASELINE_BYTES) == 0);
    write_exact(writable, baseline, RQ_BASELINE_BYTES);
    assert(rqs_read_verified_fd(RQS_HOST_ACK, readonly, digest, copy,
                                RQ_BASELINE_BYTES) == 0);
    assert(memcmp(copy, baseline, RQ_BASELINE_BYTES) == 0);

    assert(fstat(readonly, &created) == 0);
    assert(lstat(path, &named) == 0 && S_ISREG(named.st_mode) &&
           created.st_dev == named.st_dev && created.st_ino == named.st_ino);
    assert(close(readonly) == 0);
    assert(close(writable) == 0);
    assert(unlink(path) == 0);
    free(copy);
    free(baseline);
    puts("PASS synthetic RFS verified-fd source boundary");
    return 0;
}
