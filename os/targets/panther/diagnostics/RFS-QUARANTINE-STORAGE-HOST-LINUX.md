# Linux host-only private-storage fixture

This is a filesystem test harness for the pure RFS model, **not a phone
broker**. It has no modem endpoint, boot integration, original EFS path,
baseline path, mount operation, or candidate-promotion API. The build guard
requires `RFS_QUARANTINE_HOST_ONLY` and Linux x86_64. Its only accepted parent
directory is a dirfd resolving to the actual, root-owned sticky `/tmp`;
callers cannot choose an arbitrary source or destination path. The input is
an in-memory, synthetic 524288-byte baseline with an independently pinned
SHA-256 digest. The adapter independently opens `/tmp` with `O_NOFOLLOW` and
compares inode/device; it cannot prove how the caller obtained its fd.

The adapter creates a fresh random `rq-host-*` directory (0700, current
effective UID), a `NO_PROMOTION` marker and exclusive `candidate.bin` (0600).
It checks directory/file owner, type, mode, inode and link count using both
dirfds and `AT_SYMLINK_NOFOLLOW`, and rejects path replacement or unexpected
hard links. The baseline clone is reread and fsynced before the command-7
status can leave the adapter. Each core-accepted data chunk is written only
at its bounded private offset before the next grant is returned. After the
95th chunk it withholds the factory-shaped final status until the private
candidate passes a full reread against the core's detached candidate,
`fsync(candidate)`, an exclusive binary SHA-256 sidecar write/fsync,
`fsync(directory)`, `fsync(parent)`, a second reread and a checked sidecar
`close`. It checks the candidate identity again after that reread and before
success. Failure disarms the
adapter, yields no reply and leaves the `NO_PROMOTION` marker in place. The
marker is deliberately retained after a successful host fixture as well;
the sidecar is evidence for review, not permission to select the file.

The adapter disables process core dumps/dumpability, holds no original NV
file, logs no packet body or checksum, and wipes its in-memory buffers on
failure/destroy. It never deletes the quarantine artifact. The test program
removes only exact directories it created under `/tmp` after assertions.

From `os/targets/panther/diagnostics` on a Linux x86_64 host:

```sh
cc -std=c11 -O2 -Wall -Wextra -Werror \
  -DRFS_QUARANTINE_HOST_ONLY -DRQ_STORAGE_HOST_TEST \
  rfs-quarantine-core-host.c rfs-quarantine-storage-host-linux.c \
  rfs-quarantine-storage-host-linux.test.c \
  -o /tmp/saaios-rq-storage-host-test
/tmp/saaios-rq-storage-host-test
```

The fixture covers full 7→3→6→95-chunk completion, unchanged clone tail,
sidecar digest, wrong parent/pin/consent, malformed data, hardlink/symlink
replacement, candidate size/mode and marker tampering, a pre-existing
checksum path, plus injected chunk write and final durability failures. It
also injects a sidecar close error after successful fsync and confirms that
the final status is withheld. It
was run as UID 0 and an unprivileged UID, and with AddressSanitizer +
UndefinedBehaviorSanitizer.

Limitations: this exercises synthetic bytes and regular files only. It does
not attest that an arbitrary caller-supplied memory buffer is synthetic; do
not pass real NV contents to this host fixture. The caller must keep that
buffer stable during creation. The initial clone is reread and pinned to the
independent digest, but this mutable-memory API is **not** a verified-fd
provenance contract suitable for a device service. It also does
not prove on-device EFS provenance, radio behavior, CP frame timing, power-
loss atomicity, or correctness against a concurrent malicious same-UID or
root writer. A filesystem may report `fsync` success yet violate durability
under power loss. No phone build should include this file; any future device
storage broker requires separate review, explicit opt-in and rollback under
[MODEM-07](../../../../docs/os/sprints/MODEM-07-RFS-QUARANTINE.md).
