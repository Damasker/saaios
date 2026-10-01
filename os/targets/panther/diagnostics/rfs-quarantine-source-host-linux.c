#define _GNU_SOURCE
#include "rfs-quarantine-source-host-linux.h"

#include <errno.h>
#include <fcntl.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

static int acceptable(const struct stat *st)
{
    return S_ISREG(st->st_mode) && st->st_uid == geteuid() &&
           (st->st_mode & 07777) == 0600 && st->st_nlink == 1 &&
           st->st_size == RQ_BASELINE_BYTES;
}

int rqs_read_verified_fd(const char *host_ack, int fd,
                         const unsigned char pinned_sha256[32],
                         unsigned char *out, size_t out_len)
{
    struct stat before, after;
    unsigned char digest[32];
    size_t used = 0;
    int result = -1;
    if (!out || out_len != RQ_BASELINE_BYTES) return -1;
    memset(out, 0, out_len);
    if (!host_ack || strcmp(host_ack, RQS_HOST_ACK) || fd < 0 ||
        !pinned_sha256) return -1;
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || (flags & O_ACCMODE) != O_RDONLY ||
        fstat(fd, &before) || !acceptable(&before)) return -1;
    while (used < out_len) {
        ssize_t got = pread(fd, out + used, out_len - used, (off_t)used);
        if (got < 0 && errno == EINTR) continue;
        if (got <= 0) goto done;
        used += (size_t)got;
    }
    if (fstat(fd, &after) || !acceptable(&after) ||
        before.st_dev != after.st_dev || before.st_ino != after.st_ino ||
        before.st_mtim.tv_sec != after.st_mtim.tv_sec ||
        before.st_mtim.tv_nsec != after.st_mtim.tv_nsec ||
        before.st_ctim.tv_sec != after.st_ctim.tv_sec ||
        before.st_ctim.tv_nsec != after.st_ctim.tv_nsec ||
        rq_host_sha256_fixture(out, out_len, digest) ||
        memcmp(digest, pinned_sha256, sizeof digest)) goto done;
    result = 0;
done:
    memset(digest, 0, sizeof digest);
    if (result) memset(out, 0, out_len);
    return result;
}
