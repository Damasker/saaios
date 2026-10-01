# Host-only C RFS quarantine core

`rfs-quarantine-core-host.{h,c}` is a pure C protocol core and **not** a
filesystem broker. It has no file path, fd, `openat`, modem, network, or
candidate-promotion operation. Compilation requires the explicit
`RFS_QUARANTINE_HOST_ONLY` define and rejects AArch64 builds. Do not install
or link it into the phone owner. Tests use only a synthetic 524,288-byte
baseline filled with `0x5a`, checked against an independently known SHA-256.
The API cannot itself prove that arbitrary caller-supplied bytes are
synthetic; the caller must honor this host-only contract.

The core clones and re-hashes the private baseline **before** returning
status 0 for the exact cmd7 request. It accepts exact cmd3 and cmd6 frames,
then 95 bounded cmd2 data frames (94 x 2,012 bytes plus 318 bytes). Cmd2
must echo cmd6 sequence 1; requiring each block to equal the advertised
grant is an additional fail-closed SaaiOS policy. Candidate bytes are not
externally copyable until complete. Before the final 16-byte success status,
the core verifies the pinned baseline hash, a digest of the received prefix
against the candidate prefix, the unchanged candidate tail, the 189,446-byte
total, and the 95-grant bound. It wipes the private candidate on refusal.
No result is persisted, so this is **not** a durability or device-readiness
claim; the modeled success status must not be connected to a modem.

The separate [Linux host-only storage fixture](RFS-QUARANTINE-STORAGE-HOST-LINUX.md)
now exercises synthetic private-file durability, but it is **not** a
device-side verified-fd storage broker. The remaining device design and
independent review must cover:

1. Caller provenance: the caller must open a unique private directory with
   `O_DIRECTORY|O_NOFOLLOW|O_CLOEXEC`, and the immutable, separately verified
   NV **copy** with `O_RDONLY|O_NOFOLLOW|O_CLOEXEC`; never open original EFS.
   A callee receiving only an fd cannot reconstruct whether a symlink was
   traversed while the caller opened it.
2. On both supplied fds: `fstat` type, owner, mode, link count, size and
   access flags; empty directory and pinned baseline digest checks. Reject
   wrong owner, non-regular baseline, hard links, unexpected entries, and
   shared or writable baseline descriptors. Do not use original EFS paths.
3. `openat` candidate under that directory with `O_CREAT|O_EXCL|O_NOFOLLOW`
   and mode `0600`; validate its inode, owner, mode and link count. Clone the
   exact 524,288-byte verified baseline, `fsync` candidate and directory
   before any cmd7 success. Handle every short read/write and failure.
4. Bounded positional writes for the 95 accepted blocks. Before final
   success, `fsync`, re-read and verify the candidate size, prefix and tail,
   and establish durable quarantine metadata. No rename/copy into the boot
   NV baseline and no automatic promotion or retry. Explicitly fail closed
   on I/O errors and keep payload/NV bytes out of logs.
5. Timeouts, a single IPC/RFS reader, CP state and open-count checks, plus
   an explicit rollback plan. None belong to this host-only library.

The separate [host-only transport fixture](RFS-QUARANTINE-TRANSPORT-HOST.md)
tests synthetic stream boundaries, send failures and deadlines; it is not
wired to this device owner or a phone-side storage broker.

Run host fixtures on x86-64, for example with Zig 0.14.1 or GCC:

```sh
zig cc -target x86_64-windows-gnu -std=c11 -Wall -Wextra -Werror \
  -pedantic -O2 -DRFS_QUARANTINE_HOST_ONLY \
  -o rfs-quarantine-core-host-test.exe \
  os/targets/panther/diagnostics/rfs-quarantine-core-host.c \
  os/targets/panther/diagnostics/rfs-quarantine-core-host.test.c
```

On a Linux x86-64 host, also compile and run the same fixture with
`-fsanitize=address,undefined -fno-omit-frame-pointer`. Its malformed-frame
fixture allocates one extra byte so it can submit a 2,033-byte invalid frame
without overflowing the test's own buffer.

The nine fixture groups cover SHA-256 known vectors (empty string, `abc`,
and 55/56/63/64-byte padding boundaries), consent/hash pinning, the 95-block
success path and exact final ACK, malformed order/frames, malformed data,
invalid output buffers, a failing last block without ACK, post-completion
refusal, and fresh independent candidates. The authoritative safety design is
`docs/os/sprints/MODEM-07-RFS-QUARANTINE.md`.
