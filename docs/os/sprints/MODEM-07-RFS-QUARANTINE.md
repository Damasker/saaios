# MODEM-07: quarantined protected-NV RFS experiment

Status: **one-grant diagnostic exercised on phone; separate full-transfer
owner built and host-tested only**. The full transaction and modem service
are not deployed.
Target: Pixel 7 `panther` S5300, explicit diagnostic boots only.

The Linux x86_64 host fixture now composes the protocol, transport and
private-storage models end to end. It proves, on synthetic bytes only, that
the final success response is sent after durable quarantine completion and
is withheld on a final-fsync failure. The immutable synthetic baseline is
checked after both runs. That fixture is not a CP transaction or the separate
ARM owner described below. Original
EFS may be read only for provenance checks; it must never be an RFS destination.

The device's existing private verifier checks each userdata NV copy against
its **adjacent** factory-format MD5 sidecar. That verifies local consistency,
not independent provenance. A new host-only verified-fd fixture additionally
requires a read-only regular descriptor, single link, owner-only mode, exact
length and a SHA-256 pin supplied independently of the source file. Its test
checks refusal on changed digest, mode, link count, length and writable fd.
It cannot attest a phone path or supply a trusted pin by itself. The manual
read-only EFS comparison below establishes provenance for the current copy,
but a repeatable phone-side gate remains necessary before any RFS reply.

## Read-only original-EFS provenance check (2026-10-01)

With the user's explicit read-only permission, the running phone's sysfs
reported `sda5` as `PARTNAME=efs` (major:minor `8:5`) and `sda6` as
`PARTNAME=efs_backup`. The first four bytes at offset 1024 of `sda5` were
the F2FS magic `10 20 f5 f2`. Only `sda5` was opened. A mode-0400 block
node was created and the filesystem was mounted with
`ro,norecovery,nodiscard,nosuid,nodev,noexec,noatime`; `/proc/mounts`
confirmed `ro`, `norecovery` and `nodiscard` before any file access.

`cmp -s` found byte-for-byte equality between original EFS and the existing
userdata copies for `nv_protected.bin`, `nv_normal.bin` and both adjacent
`.md5` sidecars. No NV bytes, identifiers or digests were emitted. The
filesystem was unmounted and the temporary block node and mountpoint were
removed. This is evidence for these four files **at this instant**, not a
permanent integrity pin, a validation of other EFS files, an RFS reply, or
proof of network registration. Repeat the same read-only provenance check
or establish a separately protected digest immediately before any active
RFS experiment.

The separately named, manual
[`verify-original-efs-readonly.sh`](../../../os/targets/panther/diagnostics/verify-original-efs-readonly.sh)
now repeats the identity, read-only mount and four-file comparison with
fail-closed cleanup. Its first test refused a device node under nodev `/tmp`;
the corrected version uses a private `/dev/block` directory. The corrected
script returned `PASS` on the phone, with EFS unmounted, no private nodes left
and CP still `ONLINE`. It is installed as a mode-0700 manual tool under
`/data/saaios/bin`, but it is not called by boot or the passive owner. A
future active RFS launch must run a fresh check; today's PASS is not a pin.

The verifier's explicit `pin-read-only` mode now derives a protected-NV
SHA-256 from original EFS and publishes it only after a second successful
read-only comparison, confirmed unmount and private-node cleanup. Its
same-boot `/run` pin matched the userdata copy in a phone test; the test pin
was then removed. No digest was logged in that preparatory test. The
subsequent separately reviewed one-grant owner consumes a fresh pin before
READY; it cannot reuse the authorization in the same boot.

## First one-grant phone result (2026-10-01)

The separately built ARM64 owner/probe and explicit `rfs-one-grant` wrapper
passed host fault tests, independent source review, installed SHA-256 checks
and an on-device owner self-test. A cold-boot script dependency initially
stopped the wrapper before EFS access or CP boot; another attempt was
interrupted by the serial console during firmware transfer, before the owner
started. Both refusal logs were preserved. The final attempt used a
non-interrupting console after a fresh AP reboot.

That attempt rechecked original EFS read-only, compared both NV files and
their sidecars, unmounted EFS, then consumed a boot-local protected-NV pin.
The CP reached `ONLINE`; the owner logged exactly one file-3 command-2 grant
attempt and one first data chunk stored into a root-only 524288-byte
quarantine candidate. Its `NO_PROMOTION` marker remained present. The owner
entered terminal state without a second grant or final success ACK and held
IPC/RFS while CP was online. An independent silent check confirmed that the
verified source still matched its pin, the candidate tail from byte 2012
matched the source, and only the first chunk differed. No NV bytes or digest
were printed. Original EFS was not mounted during this exchange.

After a controlled AP reboot, the original EFS again matched all four
userdata files through a read-only, no-recovery mount and was unmounted.
The quarantined candidate and logs were retained; the passive no-reply owner
was restored and CP reached `ONLINE`. A later passive SIM refresh reported
`card=1`, `apps=1`, `app_state=5` (READY), but the earlier registration
snapshot was 0, `rmnet0` RX remained 0 and no IPv4 bearer appeared. Do not
attribute SIM READY to the partial RFS attempt without a controlled
comparison. The AP reboot ended the one-grant owner, so its early
`first_chunk_quarantined_no_ack` log is **not** a post-OFFLINE exit-code PASS
and is not evidence of a completed 95-chunk RFS transaction or cellular
service. No candidate was promoted to a boot copy or original EFS.

## Separate full-transfer owner prepared (2026-10-01)

The manual `rfs-full-quarantine` mode has its own ARM64 static owner and
handoff probe; it does not replace the passive default or the one-grant
binary. The host self-test exercises the exact 95-grant/189446-byte exchange,
including the final 318-byte frame; a real temporary candidate plus binary
SHA-256 sidecar passes the durability/integrity gate. Injected sidecar fsync
and close failures prevent any final ACK attempt. The host test also passes
ASan/UBSan, and the owner/probe cross-compile statically for AArch64.

At that stage the result was host-only. Its single IPC0 owner includes bounded,
read-only SIM,
radio and voice/data registration GET snapshots at ONLINE and owner start +60 s.
Host tests cover fragmented/coalesced replies, matching token and length,
timeout as unknown, no retry after ambiguous IPC write, and an IPC fault
co-reported with the final RFS data frame withholding the ACK. Timeout ends
the observer rather than starting another outstanding GET. Only scalar
fields are logged; a bearer check must be collected separately through
read-only network-interface state, never a second IPC reader. Live A/B
registration causality is still unproven.
The copied `cpif.ko` write path was reviewed at binary level. A later
read-only live check found the phone's `/lib/modules/cpif.ko` SHA-256 equal to
the reviewed local copy (`8cdd21d7...e1e79c`); the loaded module's sysfs
GNU build-ID note and Linux note hashes matched the corresponding sections
of that same local binary, and its `scmversion` matched. Its `vermagic`
still differs from `uname -r`; do not infer source-tree parity from version
strings alone. CP remained `ONLINE` during this check, with no modem command
sent. Those gates were checked for the first phone run below; a later
attempt requires its own review and fresh preflight.

## First full-transfer phone attempt (2026-10-01)

The separate owner, probe and wrapper were installed without replacing the
passive defaults. On-device hashes matched the reviewed ARM64 builds; the
owner's synthetic self-test passed. A fresh passive control boot with the
same inserted SIM reached CP `ONLINE`. At +60 seconds the card and app were
READY with PIN1 disabled, radio was on, voice/data registration remained 0,
and `rmnet0` RX/TX were both 0. The passive owner sent no RFS replies.

After a second fresh AP boot, the opt-in full wrapper reverified the four
original-EFS files read-only, unmounted EFS, pinned the protected NV source,
and reached CP `ONLINE` with one IPC/RFS owner. The owner logged 95 grant
attempts, 94 stored chunks and 189128 stored bytes, then failed closed with
`rfs_frame_or_io_refused`. The final success ACK was neither attempted nor
sent. A root-only 524288-byte candidate and `NO_PROMOTION` marker remain in
the unique quarantine directory; no sidecar was published. A silent slice
comparison found the final 318-byte region and untouched tail equal to the
verified baseline, while the earlier transfer prefix differed. No candidate
was promoted or used for boot.

This run does **not** establish whether grant 95 reached the CP. The counter
increments before the grant write; a refused final grant, malformed/missing
final data, deadline, or candidate I/O/gate failure share this terminal log.
The owner deliberately stopped further SIT GETs, so there is no matched +60
SIT snapshot for experimental B. `rmnet0` remained down with RX/TX 0; no
registration or bearer success is claimed. Do not retry in the same boot.
After a controlled AP reboot, the original EFS again matched all four
userdata files through the read-only verifier and was unmounted. The passive
owner was restored, CP returned `ONLINE`, and SIM was READY but unregistered
at its +60-second snapshot. The full-run logs and quarantine artifact remain
on the phone. Next, add scalar failure-stage diagnostics without logging NV
payload or identifiers, review and retest before another cold opt-in run.

## Second full-transfer diagnostic (2026-10-01)

A revised owner added fixed failure-stage/reason labels and a seven-bit
comparison-to-constant mask; it did not add RFS retries or log frame bytes,
NV contents, identifiers or raw header fields. Host sanitizer tests, ARM64
static build, independent review, installed SHA-256 and on-device self-test
passed. The earlier full-run logs and candidate were preserved before a new
AP boot. The wrapper again passed its fresh read-only original-EFS check and
reached CP `ONLINE` with one owner.

The owner again stored 94 chunks (189128 bytes) and attempted grant 95. This
time it identified `failure_stage=final_rfs_frame`,
`failure_reason=malformed`, `frame_mismatch_mask=0x09`. The bitmask means the
received final frame's total length and payload-length field did not match
the strict 318-byte-chunk model; its command, sequence, status, file ID and
chunk-size fields matched the expected constants. Thus a final response was
seen, but its exact framing/padding is still unknown. No final ACK was
attempted or sent, and no registration or bearer success is claimed. The
second root-only candidate and `NO_PROMOTION` marker remain quarantined,
without a sidecar or promotion.

After another controlled AP reboot, the original EFS again passed the
four-file read-only comparison and was unmounted; the passive owner returned
CP to `ONLINE`. Do not relax the parser by guesswork.

Read-only disassembly of the matching factory TD1A `rfsd` clarifies the
compatibility boundary. Its top-level receive path accepts outer
`payload_len + 8 <= read_count` (`0x9080–0x908c`), rather than equality.
For command 2, its handler checks `chunk_len <= remaining` (`0x9f50–0x9f5c`)
and writes exactly `chunk_len` bytes from the data region
(`0xa014–0xa030`); no inner-length-versus-outer-length check was found.
Thus a 2032-byte final frame with outer payload length 2024 and inner chunk
length 318 would be factory-accepted, with its extra 1694 bytes ignored.
The second-run mismatch mask alone did **not** prove those hypothetical
lengths: its two set bits were coupled because SaaiOS derives frame length
from the outer field. The third run below captured the actual bounded shape.
Any parser change must require enough actual bytes for the inner chunk,
accept only that observed shape and keep the existing quarantine durability
gate before ACK; do not copy the factory handler's missing bounds check.

## Third full-transfer diagnostic: exact final frame (2026-10-01)

A third reviewed owner changed only scalar diagnostics, not acceptance or
modem I/O. After a fresh boot and successful read-only original-EFS pin, it
again reached 95 grant attempts and 94 stored chunks before refusing the
final frame without ACK. This time it recorded
`final_parsed_len=340`, `final_outer_payload_len=332`,
`final_trailing=0`, `final_padding_zero=1`. The inner chunk length remained
318 and all previously checked command, sequence, status and file fields
matched. The final two bytes are zero padding at offsets 338–339. This is
consistent with 4-byte alignment, but only this exact final shape has been
observed. No candidate or sidecar was promoted; the third candidate and
`NO_PROMOTION` marker remain in quarantine.

After another controlled AP reboot, the original EFS passed the four-file
read-only comparison and was unmounted. The passive owner restored CP
`ONLINE`; its settled SIM remained READY and registration remained 0. The
narrow parser rule tested in the next run accepts the existing exact 338/330
final frame or the observed 340/332 frame with two zero padding bytes. All
other fields, no-trailing rule, private candidate durability and final ACK
gate remain unchanged. It is manual opt-in, never an automatic boot service.

## Fourth full-transfer opt-in: complete quarantine (2026-10-01)

The exact zero-padding rule passed full 95-grant host transcripts, injected
durability failures and malformed-frame refusals under ASan/UBSan, a static
ARM64 build and independent source review. The installed owner hash and
on-device self-test matched before a fresh cold run. The wrapper again
verified original EFS read-only, unmounted it and handed over one IPC/RFS
owner from before CP FIN. CP reached `ONLINE`.

The owner logged `complete_quarantined_ack`: 95 grant attempts, 95 stored
chunks, 189446 stored bytes, and exactly one final ACK attempted and sent
after candidate fsync/readback, full integrity validation and sidecar
durability checks. The private 524288-byte candidate, 32-byte binary SHA
sidecar and `NO_PROMOTION` marker remain together in a mode-0700 quarantine
directory. Neither candidate nor original EFS was used as a boot source or
promoted. This is a successful **manual RFS exchange**, not a production
modem service or a post-OFFLINE owner exit-code result.

At owner start +60 seconds the same owner observed SIM card 1/app 1/READY with
PIN1 disabled, radio on, but voice and data registration still 0. `rmnet0`
was down with RX/TX 0, and remained so after an additional read-only wait.
The fresh passive control had the same unregistered state. Therefore the
completed RFS exchange did **not** establish cellular service; further
radio-available/camp prerequisites must be investigated separately, without
guessing SET commands. After a controlled AP reboot, original EFS again
passed the four-file read-only comparison and was unmounted. The passive
owner was restored and CP returned `ONLINE`. All experiment logs and
quarantine candidates were retained.

## Next discriminator: same-owner radio/network event timing

The factory radio-available audit below identifies conditional startup SETs,
but none is proven to be the missing camp prerequisite after READY/radio ON
and a completed RFS exchange. In particular, the stock carrier-configuration
route remains a hypothesis, not permission to guess a carrier value. The
stock notes establish local callback order, not a global ordering against
RFS completion. Do not replay those SETs to resolve this uncertainty.

Both diagnostic owners now have a bounded, header-only trace for framed SIT
unsolicited `0x07xx` network and `0x08xx` radio indications on their
**existing exclusive IPC reader**. Each line contains only time since the
pre-FIN owner handoff, indication ID and frame length; at most eight event
lines are emitted per 60-second window, followed by a count/overflow summary.
The first five minutes include zero-traffic window summaries; later quiet
windows are omitted to avoid indefinitely growing logs.
The full-RFS owner also timestamps the return from its local final-ACK write.
The trace logs no payload bytes or subscriber/network identifiers and adds no
endpoint, GET or SET. The full-RFS owner traces during the exchange and after
its final ACK; the
passive owner traces the same ID ranges without answering RFS. Synthetic
host fixtures cover type/length filtering and the trace limit. This change
is host-tested only; the currently running phone owner was not replaced.

A later phone comparison, if separately reviewed, requires two fresh boots
with the same SIM and pinned source: passive control and one manual full-RFS
run, each from pre-FIN ownership. Compare owner-receipt timestamps and the
local ACK-write-return marker, plus the existing matched +60-second redacted
status and `rmnet0` counters. RFS is serviced before IPC and candidate
finalization is synchronous: an indication queued before the ACK can be read
afterward, so receipt times do **not** prove CP emission or causal order.
A missing indication means only that none reached the owner in that capture
window; it does not prove that CP never searched or that RF was idle. Any
framing failure, owner loss, ambiguous RFS write or unexpected EFS preflight
result stops the comparison without retry or candidate promotion.

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

- Never mount original EFS for writing, open its NV paths for writing, run
  factory `rfsd`, flash radio partitions, or alter identity/calibration data.
  Read-only, no-recovery access is permitted solely to verify provenance;
  never point an RFS responder at original EFS.
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

The final CP-visible status was verified in factory code. The isolated
one-grant adapter observed the first real CP data frame and wrote it only to
quarantine. The separate full owner has now completed all 95 chunks and the
final status in a guarded phone run, still without touching original EFS or
promoting its candidate. Factory local-file/backup/checksum transitions are
not copied: quarantine durability replaces them. The synthetic
[protocol](../../../os/targets/panther/diagnostics/RFS-QUARANTINE-C-HOST.md),
[private-storage](../../../os/targets/panther/diagnostics/RFS-QUARANTINE-STORAGE-HOST-LINUX.md)
and [transport](../../../os/targets/panther/diagnostics/RFS-QUARANTINE-TRANSPORT-HOST.md)
fixtures remain independent host models; their integration test is not the
phone broker.

## Implementation sequence and result

1. Keep the present passive owner as the default. Build a separately named,
   manually launched ARM diagnostic variant that uses the same pre-FIN
   single-owner handoff and IPC/RFS lock. The host protocol/storage fixtures
   retain their AArch64 compile rejection; they are test models, not files
   to link into the device image.
2. In that variant, verify the immutable NV-copy provenance and digest again
   before replying to command 7. Create the unique candidate and durable
   `NO_PROMOTION` marker first. Establish descriptor identity, bounded size,
   full re-read and directory durability. Recheck the copy against original
   EFS through a read-only, no-recovery mount or a separately protected
   digest derived from it. Do not treat the adjacent `.md5` as an independent
   pin. The host-only fd model is not an on-device provenance adapter.
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
4. The first live stage is complete: one bounded grant produced a real
   command-2 data frame stored only in the candidate. No final ACK was sent;
   the passive build was restored after a controlled reboot. This was a
   CP-state-changing diagnostic, not a harmless read-only probe. The verified
   boot copy remained byte-identical to original EFS, and the incomplete
   candidate remains quarantined evidence, never a boot source.
5. The separate full owner passed host, sanitizer, ARM build, installed-hash
   and on-device self-test gates. Three guarded refusals exposed the exact
   final-frame padding without ACK; after a narrow zero-padding fix, a fourth
   cold run completed 95/95 chunks and sent the final ACK only after durable
   candidate/sidecar verification. The passive owner and original-EFS
   read-only postflight were restored after each run. SIM READY returned,
   but registration and bearer remained absent. This manual exchange is not
   an autostart modem service or evidence of cellular connectivity.

## Failure and verification gates

Timeout, malformed frame, unexpected state, NV verification failure, short
read/write, insufficient space or owner loss must fail closed. The original
and verified boot copies remain byte-identical; an incomplete candidate is
marked invalid and never selected for boot. Do not retry automatically or
guess PIN/APN/radio commands. Host tests must cover every refusal path,
chunk arithmetic and no-promotion invariants. A phone run requires a
separate reviewed opt-in build and an explicit rollback plan; it is not a
PID-1/autostart feature.

## Gate for the next full-transfer phone experiment

The separate full-transfer owner is a **development artifact**, not a boot
service. Before another live attempt, require
host success and injected failures across the 95-grant sequence, an ARM64
static build and on-device self-test, independent source review, confirmation
of the running CPIF write contract, and a measured rollback to the passive
owner. The original EFS stays read-only; the verified userdata NV source is
also never a write destination. The private candidate is never promoted.

The live comparison must use two fresh AP boots with the same physical SIM
(PIN request disabled), reviewed B firmware and independently verified NV
copies. Control A gives no RFS response; experimental B completes the
quarantined file-3 exchange and releases the final success status only after
candidate/sidecar fsync and reread. Both runs must have one IPC/RFS owner
from before FIN through CP OFFLINE. In that same owner, collect identical
read-only SIM, radio, voice/data registration and bearer observations at
defined times (including after the RFS window and at +60 seconds). A second
IPC reader would invalidate the comparison. An RFS completion without camp
or bearer is a negative result, not cellular-service success; a positive
result needs a repeat control before causal attribution. No blind AP/radio
SET is part of this experiment.

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

## Separate active RF scan gate

The possible MODEM-06 `0x0706` available-network query is **not** part of
this RFS experiment. The exact factory TD1A `vendor.img` `/lib64/libsitril.so`
(SHA-256 `efcca0d5fa5a3eb3a09d8c9f68fc35f8b194bb511379987fd4a353f12ed2d5b1`)
has `Service::IsCurrentStackOccupyRF` at `0x151070` and the factory-spelled
`Service::IsOppsiteStackOccupyRF` at `0x1511b0`. Each returns 1 for its
`NetworkService::IsPlmnSearching` state, 2 for
`CscService::IsInCallState`, or 0 otherwise. `IsPlmnSearching` at
`0x19ecb0` tests `Service::IsInTransaction(48)` and `(5033)`; the latter at
`0x152c50` reads RIL-local active/queued messages. `IsInCallState` at
`0x16cd10` reads a RIL-local call-state counter. These are application-state
arbitration checks, **not** a read-only SIT query of CP RF occupancy.

`NetworkService::DoQueryAvailableNetwork` at `0x198fd0` waits when the
opposite RIL stack reports occupancy, rejects current-stack occupancy, and
only then builds the factory `0x0706` request (16 bytes, argument 0 in the
ordinary no-type path). Its timeout handler at `0x19a0a0` sends the
factory `0x0707` cancel. A single-owner SaaiOS boot with no Android RIL
can establish that *it* has no competing scan/call requests, but cannot
infer CP-autonomous or embedded-SIM RF idleness from registration=0, slot
metadata, or the four passive network GETs. The default owner remains
passive. A separately built, reviewed one-shot owner with strict framing,
exclusive-client checks, redaction, timeout/cancel and fail-closed ambiguous
write handling ran on two guarded PIN-free boots. Both scans returned an
immediate 12-byte error response; the second logged `error_raw=2`. This is
not a working scan and does not by itself establish an RFS or registration
cause. Do not repeat `0x0706` without a new isolated hypothesis.

Evidence and corrections: [runtime notes](../targets/panther/MODEM-RUNTIME-2026-09-24.md)
and [modem roadmap](MODEM-ROADMAP.md).
