/* Host-only verified-fd model. No phone path, device, or RFS write. */
#ifndef RFS_QUARANTINE_SOURCE_HOST_LINUX_H
#define RFS_QUARANTINE_SOURCE_HOST_LINUX_H

#if !defined(RFS_QUARANTINE_HOST_ONLY) || !defined(__linux__) || \
    !defined(__x86_64__) || defined(__aarch64__)
#error "RFS source fixture is Linux x86_64 host-only"
#endif

#include "rfs-quarantine-core-host.h"

#define RQS_HOST_ACK "HOST_ONLY_VERIFIED_FD_NO_DEVICE_IO"

/* Reads a caller-opened, read-only synthetic regular file only when its
 * independent pinned SHA-256, owner, 0600 mode, unique link, and exact size
 * agree. The caller remains responsible for proving the fd came from an
 * allowlisted O_NOFOLLOW path; this host model cannot attest pathname or
 * original-NV provenance. On failure, zeroes the whole output buffer. */
int rqs_read_verified_fd(const char *host_ack, int fd,
                         const unsigned char pinned_sha256[32],
                         unsigned char *out, size_t out_len);

#endif
