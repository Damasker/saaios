# Host-only RFS transport fixture

`rfs-quarantine-transport-host.{h,c}` tests the missing boundary between a
stream of synthetic RFS bytes and the existing host-only protocol model. It
has no modem endpoint, filesystem path, NV input, network code, or real
`write(2)` call. It requires `RFS_QUARANTINE_HOST_ONLY` and Linux x86-64;
AArch64/device builds are rejected. Do not install or link it on a phone.

The fixture accepts one complete frame at a time, including split reads and
the valid no-reply command-3 plus command-6 coalescing. It checks the modeled
7 -> 3 -> 6 -> 95-data-frame sequence, permits only one outstanding grant,
and refuses bytes already buffered after a reply-producing frame. An injected
handler is not invoked for such a malformed coalesced feed, so it cannot
prepare or mutate even the private synthetic candidate. An injected
send callback is called **once** per reply; an error, short result, or
overcount aborts without a retry because a real peer might already have
received the frame. A per-step test-clock deadline cannot be extended by
dribbling partial bytes. The final factory-shaped success status is withheld
unless a separate `durable_complete` callback reports success.
After that status is successfully sent, the fixture is terminal: later feed
calls return `RQT_COMPLETE` without parsing, sending, or calling `abort`.
Any future owner must stop routing frames to this completed transaction and
handle later RFS requests through a separately reviewed path.

The test binds `accept` to the **synthetic in-memory** `rq_core` and explicitly
mocks the durability predicate. This proves the transport's refusal logic,
not actual filesystem durability. The separate
`RFS-QUARANTINE-STORAGE-HOST-LINUX.md` fixture tests private synthetic-file
durability. A new Linux x86-64 integration fixture composes both host models
and verifies final-ACK ordering against real private synthetic-file fsync,
including a fail-closed final-fsync fault. It is still not a phone broker or
an integrated device test.
The callback contract itself cannot attest NV-copy provenance or prevent a
caller from lying about durability. Before any ARM implementation, review a
new verified-fd provenance contract, private candidate lifetime, owner
handoff, endpoint write semantics, and full failure rollback under
`docs/os/sprints/MODEM-07-RFS-QUARANTINE.md`.

From the repository root on a Linux x86-64 host:

```sh
cc -std=c11 -O2 -Wall -Wextra -Werror -pedantic \
  -DRFS_QUARANTINE_HOST_ONLY \
  os/targets/panther/diagnostics/rfs-quarantine-core-host.c \
  os/targets/panther/diagnostics/rfs-quarantine-transport-host.c \
  os/targets/panther/diagnostics/rfs-quarantine-transport-host.test.c \
  -o /tmp/saaios-rqt-host-test
/tmp/saaios-rqt-host-test
```

Tests cover the full 95-chunk synthetic transcript, fragmented and
coalesced frames, final ACK refusal without mocked durability, error/short/
overcount sends at the initial status, first and intermediate grant, and
final ACK without retry, oversize frames, causal response boundaries,
sequence mismatches, timeout, and monotonic-clock/creation contracts. A
failed final mocked send is **indeterminate**: the attempted ACK count does
not prove CP acceptance, and this transport test makes no end-to-end storage
safety claim.

The composed host fixture is `rfs-quarantine-integration-host.test.c`.
CI compiles it with `RFS_QUARANTINE_HOST_ONLY` and `RQ_STORAGE_HOST_TEST`,
both normally and with ASan/UBSan. It uses only generated synthetic baseline
bytes and `/tmp/rq-host-*` quarantine directories that it verifies and
removes by exact inode; it has no phone endpoint or original NV path.
