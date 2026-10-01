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
   sequence (1 in the observed run), and chunk length remains bounded.
   Requiring the chunk to equal the advertised grant is an additional
   fail-closed SaaiOS policy; stock code checks `chunk <= remaining` and may
   allow smaller chunks. The factory receive handler is at
   `0xe50c→0xe9d0→0x9e10`; the grant builder is at `0x9ba4–0x9bec`.
   Command 6 stores the sequence in `[obj+0x40]` at `0xe90c`; the grant
   copies it at `0x9bb4–0x9bd4`. For incoming command 2, the common dispatch
   compares the frame sequence with `[obj+0x40]` at `0xe2e4–0xe2ec` and an
   active-transfer mismatch yields status 6. Command 3 alone bypasses that
   comparison (`0xe2d8–0xe2e0`).
4. With exact-grant-size policy, 189446 bytes take 95 chunks: 94 × 2012
   and a final 318.
   Write only into the private candidate at verified offsets, with checked
   short-write handling. Require exact total length, fsync, size 524288,
   candidate integrity validation and a private checksum sidecar. Do not
   overwrite the baseline or use the factory's non-atomic destination copy.
5. After the final chunk, stock code fsyncs, calls `OnWriteDone`, then sends
   a 16-byte command-3 success status with the same sequence and file id 3
   (`0xa150–0xa180→0xa7f0`). It does not check the callback return before
   the success status. A SaaiOS broker must improve this: send modeled
   success only after quarantine durability/integrity gates; otherwise fail
   without claiming completion or touching the baseline.
6. Keep the owner alive after completion, then compare read-only SIM, radio,
   registration and bearer observations with the no-reply control. ONLINE,
   RFS completion and SIM READY are separate milestones; cellular service
   requires observed registration and a real bearer.

The final CP-visible status was verified in factory code. Remaining local
file/backup/checksum transitions and real CP data-frame bytes still require
review before enabling phone I/O. Until then, only host-only synthetic
[protocol](../../../os/targets/panther/diagnostics/RFS-QUARANTINE-C-HOST.md)
and [private-storage](../../../os/targets/panther/diagnostics/RFS-QUARANTINE-STORAGE-HOST-LINUX.md)
fixtures are permitted; neither is a phone broker.

## Implementation sequence before any phone-side RFS reply

1. Keep the present passive owner as the default. Build a separately named,
   manually launched ARM diagnostic variant that uses the same pre-FIN
   single-owner handoff and IPC/RFS lock. The host protocol/storage fixtures
   retain their AArch64 compile rejection; they are test models, not files
   to link into the device image.
2. In that variant, verify the immutable NV-copy provenance and digest again
   before replying to command 7. Create the unique candidate and durable
   `NO_PROMOTION` marker first. Establish descriptor identity, bounded size,
   full re-read and directory durability without opening original EFS.
3. Make RFS packet parsing stop at each complete response boundary. Allow
   at most one outstanding CP request/grant, with a bounded deadline and a
   fail-closed transition. The current passive owner's callback cannot
   report an RFS write failure to its parser, so it must not be reused for
   active grant/ACK handling unchanged. Review the kernel driver's write
   contract before deciding how to handle a short or ambiguous device write;
   never blindly retransmit a potentially accepted grant.
   The inspected Google S5300 source at commit `232fb16b3dbc3c4126d9ac0b2a0f0f514e1290c8`
   (`ipc_io_device.c:241-425`) returns the full userspace count after
   nonnegative link sends, or an error; a successful return means kernel
   queue acceptance, **not** CP consumption. A new `write` receives new
   link framing. Unexpected short results and response timeouts are therefore
   indeterminate and must not trigger an automatic retry. Exact parity of
   this checkout with the running phone module remains unverified.
4. First live stage, only after separate review and a documented physical-SIM
   state: allow *one* bounded grant and inspect the first real command-2
   metadata, storing any CP data solely in the candidate. Stop without a
   final success ACK and return to the passive build after a controlled
   reboot. This is a CP-state-changing diagnostic even though it never
   promotes NV; do not describe it as a harmless read-only probe. Check that
   the verified boot copy remained byte-identical and preserve failed
   candidates as invalid evidence.
5. Only after the real command-2 sequence, RFS write semantics, failure
   paths and ARM build are verified may a separately reviewed full exchange
   attempt all 95 chunks and consider the final durable-quarantine ACK.
   A completed RFS transfer alone does not establish SIM READY, network
   registration or a data bearer.

## Failure and verification gates

Timeout, malformed frame, unexpected state, NV verification failure, short
read/write, insufficient space or owner loss must fail closed. The original
and verified boot copies remain byte-identical; an incomplete candidate is
marked invalid and never selected for boot. Do not retry automatically or
guess PIN/APN/radio commands. Host tests must cover every refusal path,
chunk arithmetic and no-promotion invariants. A phone run requires a
separate reviewed opt-in build and an explicit rollback plan; it is not a
PID-1/autostart feature.

## Factory radio-available path: evidence, not a replay list

In the stock TD1A `libsitril.so` (SHA-256
`efcca0d5fa5a3eb3a09d8c9f68fc35f8b194bb511379987fd4a353f12ed2d5b1`),
`NetworkService::UpdateRadioState` at `0x192690` broadcasts system event
`0x101` at `0x19285c`. `ServiceInterface::HandleInternalMessage` dispatches
that event at `0x158dd8-0x158e14`; the radio-state notifier at `0x156160`
invokes `OnRadioAvailable` on transition to state 1. On RIL socket 0,
`MiscService::OnRadioAvailable` (`0x179ae0`) invokes these four actions in
order. The request IDs below come from the named factory builders, not from
guessing SIT names:

| Action and factory route | SIT request and payload source | Gate / risk |
| --- | --- | --- |
| `SetDebugTraceOffOnBoot` `0x179b30` -> `DoSetDebugTrace` `0x17b8e0` -> `ProtocolMiscBuilder::SetDebugTrace` `0x22b5b0` | `0x090b`, 13 bytes, one byte set to 0 | Only when `persist.vendor.ril.cpdebugoff.onboot` is 1, or 99 with an additional global flag (`0x179bac-0x179be4`). Changes CP debug configuration. |
| `SetModemsConfig` `0x179c40` -> `DoSetModemsConfig` `0x188820` -> `ProtocolMiscBuilder::BuildSetModemsConfig` `0x22d1c0` | `0x093f`, 13 bytes, byte 0 for one modem or 1 for two; count comes from `IsMultiSimEnabled` (`0x179cbc-0x179cd0`) | Requested on socket 0 when a RIL context exists; the two-modem case is rejected unless `persist.vendor.radio.multisim_switch_support=true` (`0x188994-0x188a50`). Changes CP modem configuration; necessity for this single-SIM diagnostic is unproven. |
| `SendSGCValue` `0x179d30` -> `DoSendSGC` `0x17f920` -> `ProtocolMiscBuilder::SendSGCValue` `0x22c2a0` | `0x0404`, 24 bytes; carrier value originates in `vendor.ril.app.target_carrier` (`0x179db0-0x179dc8`), then passes through the stock mapping | Requested when a RIL context exists. Changes CP carrier configuration; a possible camp dependency, not yet a demonstrated one. |
| `SendSvnInfo` `0x179e70` -> `ProtocolNetworkBuilder::BuildSvNumber` `0x238050` | `0x4605`, 14 bytes; two decimal characters from `ro.vendor.build.svn` (`0x179efc-0x17a068`) | Sent directly only after nonempty, at-most-two-digit validation. Supplies version metadata to CP. |

The more directly camp-related factory path is
`NetworkService::OnRadioAvailable` (`0x192e90`): its log at `0xb6eee`
identifies fields `mUseCampOnEarlier` (`+0x345`), `mFirstRunOnBoot`
(`+0x346`) and `mDelayedRadioPower` (`+0x2d4`). When the first two are true,
the RIL is not connected and `persist.radio.airplane_mode_on` is 0, it calls
`TrySetRadioPower(10)` at `0x1930d4`. A deferred radio-power request is
another conditional route (`0x192ff8-0x193084`). `DoRadioPower` at
`0x193860` builds SIT `0x0800` via `ProtocolNetworkBuilder::BuildRadioPower`
at `0x236350`. These gates explain why stock code may request camp-on without
the Android framework; they do **not** prove it was necessary in the observed
SaaiOS run. None of the four MiscService actions is read-only. Do not replay
any of them, or radio-power-on, merely because it appears in the factory
sequence: require an isolated hypothesis, exact payload review and a bounded
control comparison first. This RFS quarantine sprint remains read-only on
IPC apart from the specifically reviewed RFS transaction.

Evidence and corrections: [runtime notes](../targets/panther/MODEM-RUNTIME-2026-09-24.md)
and [modem roadmap](MODEM-ROADMAP.md).
