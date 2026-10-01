# MODEM-07: quarantined protected-NV RFS experiment

Status: design under review; **not deployed and not a modem service**.
Target: Pixel 7 `panther` S5300, one explicit diagnostic boot only.

## Why this exists

The no-gap boot owner observed RFS command 7 at +7.282 s and command 6 at
+17.282 s while the CP was ONLINE. It sent no RFS responses. The first SIT
snapshot showed SIM card 0/apps 0, registration 0 and no `rmnet0` RX. This
shows an early request path, not that it is the sole cause of failed network
registration. A prior partial diagnostic answered command 7 but did not
implement the subsequent protected-NV write transaction.

Factory `/bin/rfsd` (SHA-256
`58d7f885e7533a328268f0de47ef9eb9995cdfa6b317d755b57973d4f5dfb71b`)
maps file id 3 to `nv_protected.bin`. Its default prefix is
`/mnt/vendor/efs/`, overridable via `vendor.ril.exynos.nvpath`. After
command 7 and command 3, command 6 requests an operation-2 write at offset
0, length 189446. With valid local file bounds/descriptor, factory code
grants chunks up to 2012 bytes. It later copies the temporary data over
the beginning of the destination NV file and updates a checksum. Running
factory `rfsd` or pointing it at original EFS is **out of scope**.

## Non-negotiable boundaries

- Never mount original EFS for writing, open its NV paths, run factory
  `rfsd`, flash radio partitions, or alter identity/calibration data.
- The existing verified 524288-byte userdata NV copy is an immutable input.
  Re-run the existing provenance/hash/MD5 verifier before creating a
  candidate. Never modify or rename that boot copy in this experiment.
- Create a new root-owned, mode-0700 private quarantine directory with a
  unique name; reject symlinks, unexpected hard links, pre-existing temp
  paths and wrong owners. Create candidate/temp with exclusive creation,
  mode 0600, `O_NOFOLLOW` and bounded size. Copy the verified baseline into
  the candidate before applying any CP bytes. Do not truncate its tail.
- Keep packet bodies, NV contents, identifiers and checksum values out of
  logs, crash dumps and network output. Disable core dumps/dumpability.
- No automatic candidate promotion to the boot copy or original EFS. Even a
  successful experiment leaves a quarantined artifact for later review.
- Keep one IPC/RFS owner from before FIN until CP OFFLINE. Require the common
  SIT lock and check endpoint open counts. No parallel one-shot readers.

## Bounded protocol model

1. Before reporting command-7 unprotect success, establish that the verified
   baseline and a fresh private candidate are usable. A fake success without
   local file preparation is not factory-equivalent.
2. Accept only the recorded file-3 command 7, then command 3 with status 0,
   then command 6 with offset 0, length 189446 and operation 2. Enforce
   exact frame sizes, file id, order, sequence rules and per-step deadlines.
   Unknown/duplicate/out-of-order frames disarm the transaction without
   a guessed success response.
3. Send the factory command-2 grant with file id, current offset and at most
   2012 bytes. The request is 20 bytes: command/sequence, payload length 12,
   file id, offset, chunk length. Accept the corresponding CP data frame
   only if its status is 0, file id is 3, sequence echoes the command-6
   sequence (1 in the observed run), and chunk length matches the grant and
   remaining bound. The factory receive handler is at
   `0xe50c→0xe9d0→0x9e10`; the grant builder is at `0x9ba4–0x9bec`.
   Command 6 stores the sequence in `[obj+0x40]` at `0xe90c`; the grant
   copies it at `0x9bb4–0x9bd4`. For incoming command 2, the common dispatch
   compares the frame sequence with `[obj+0x40]` at `0xe2e4–0xe2ec` and an
   active-transfer mismatch yields status 6. Command 3 alone bypasses that
   comparison (`0xe2d8–0xe2e0`).
4. For 189446 bytes the maximum is 95 chunks: 94 × 2012 and a final 318.
   Write only into the private candidate at verified offsets, with checked
   short-write handling. Require exact total length, fsync, size 524288,
   candidate integrity validation and a private checksum sidecar. Do not
   overwrite the baseline or use the factory's non-atomic destination copy.
5. Keep the owner alive after completion, then compare read-only SIM, radio,
   registration and bearer observations with the no-reply control. ONLINE,
   RFS completion and SIM READY are separate milestones; cellular service
   requires observed registration and a real bearer.

Any final CP-visible status and all remaining state transitions must be
verified from factory code or a redacted capture before enabling phone I/O.
Until then, only a host-only parser/state-machine fixture is permitted.

## Failure and verification gates

Timeout, malformed frame, unexpected state, NV verification failure, short
read/write, insufficient space or owner loss must fail closed. The original
and verified boot copies remain byte-identical; an incomplete candidate is
marked invalid and never selected for boot. Do not retry automatically or
guess PIN/APN/radio commands. Host tests must cover every refusal path,
chunk arithmetic and no-promotion invariants. A phone run requires a
separate reviewed opt-in build and an explicit rollback plan; it is not a
PID-1/autostart feature.

Evidence and corrections: [runtime notes](../targets/panther/MODEM-RUNTIME-2026-09-24.md)
and [modem roadmap](MODEM-ROADMAP.md).
