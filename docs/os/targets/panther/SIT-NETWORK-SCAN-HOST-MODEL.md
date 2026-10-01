# TD1A available-network scan: host-only state model

Status: **synthetic test fixture only; no phone build or live-send approval**.
The pure C core under `os/targets/panther/diagnostics/sit-network-scan-host.*`
has no IPC transport, device path, thread, sleep, or logging. Compilation
requires `SAAIOS_NETWORK_SCAN_HOST_ONLY` and rejects ARM targets. The existing
`modem-channel-owner` is unchanged. Nothing from this model is deployed.

## Factory provenance and scope

The pinned TD1A `vendor.img` `/lib64/libsitril.so` SHA-256 is
`efcca0d5fa5a3eb3a09d8c9f68fc35f8b194bb511379987fd4a353f12ed2d5b1`.
The [factory RF arbitration audit](../../sprints/MODEM-07-RFS-QUARANTINE.md#separate-active-rf-scan-gate)
identifies `NetworkService::DoQueryAvailableNetwork` at `0x198fd0` and its
timeout handler at `0x19a0a0`. The ordinary request is SIT `0x0706`, 16 bytes,
with a zero LE32 argument at +12. After a 300-second scan timeout, factory
code issues SIT `0x0707` cancellation; its ACK wait is five seconds. The
corresponding TD1A `sit-stream.so` SHA-256 is
`cef8756461c74102f9a78f91177d1baff80fb9af11c14994497fb8854e0f530a`.
Its `ProtocolNetworkBuilder::BuildQueryAvailableNetwork(int)` is at `0x74ad0`
and `ProtocolNetAvailableNetworkAdapter::GetCount` at `0x48ce0`; the count is
read at response +12 and its entry stride is 14 bytes. This core deliberately
copies no entry bytes and accepts a successful count only when
`count <= 64` and `count <= floor((declared_length - 16) / 14)`.

The request is an **active RF scan**, not a passive status GET. In the stock
RIL, current/opposite stack arbitration checks RIL-local transaction and call
state. They cannot prove that the CP or embedded-SIM stack is autonomously RF
idle in SaaiOS. The model's all-true gate is therefore only a test seam:
`explicit_opt_in`, exclusive IPC owner, idle local/opposite requests,
an externally reviewed RF-risk policy decision, CP online, SIM ready, and
radio on must all be established by a separately reviewed owner. The gate
also requires fresh same-boot status showing PIN disabled, automatic
selection, verified broad preferred RAT, and voice/data both unregistered;
it rejects a stale, locked, restricted, or already-registered control. The
`rf_policy_approved` flag is **not** a CP/eSIM RF occupancy measurement;
present logs and slot metadata cannot establish one.

## Bounded transaction

The model starts at most one scan. Tokens for scan and cancel must be nonzero,
distinct, and fresh in the owning IPC token namespace. `begin` reserves them
but emits no request; `take_scan` rechecks every gate immediately before
dispatch and only then builds the one 16-byte `0x0706` request. An exact
write begins the 300-second deadline; an
ambiguous write (including a short write after queue acceptance) is never
retried and schedules one 12-byte `0x0707` cancel. A matching malformed scan
response also schedules cancel. A valid remote scan error terminates without
reading a count. A valid success retains only the bounded count, never PLMN,
operator name, identifier, or entry bytes.

Initial dispatch, each request-write callback, and cancel dispatch are also
limited to five seconds by a local fail-closed policy. A late event-loop tick
cannot restart the cancel window after the original scan/write deadline.
Any future owner must call `take_scan` and the nonblocking write in one
serialized event-loop turn, with no unrelated callback between them; this
host model cannot stop a caller from retaining and sending an old request.
The future transport must be
nonblocking or otherwise arrange to deliver timeout events: the pure model
cannot preempt a blocked system call. If cancel is not dispatched by its
deadline, or its write result is ambiguous, the transaction terminates and
must not be retried or followed by another active command.

At or after the scan deadline, cancel takes precedence over a simultaneous
scan reply. Only one cancel can be taken; it uses the fresh cancel token.
An exact cancel write starts a five-second ACK deadline. Cancel ACK/error,
malformed ACK, timeout, ambiguous cancel write, and CP loss are distinct
terminal outcomes; none causes a retry. Late `0x0706` responses during cancel
are recorded only as a boolean and cannot turn the transaction into success.
A valid matching scan response or cancel ACK may arrive before the transport
write callback; its fresh token resolves that phase, and the later callback
is ignored. A cancel ACK after terminal timeout cannot rewrite the outcome.
The parser requires matching response type, ID, token and actual/declared
length, checks the full 16-bit result field, and never reads a count before
success and minimum-length validation. Backwards/overflowing monotonic time
fails closed. CP loss stops all further actions.

The host tests cover begin and pre-dispatch gate rejection, exact request
bytes, zero/two/64 networks,
remote errors, malformed/truncated and mismatched frames, oversized counts,
deadline ties, stale/late scan replies, cancel ACK/error/malformed/timeout,
early replies before write callbacks, delayed event-loop ticks,
write/cancel-dispatch ambiguity and deadline expiry, no retry, CP loss at
each active phase, and clock failure.
CI compiles with warnings as errors and runs the fixture normally and under
ASan/UBSan. A passing fixture is **not** evidence that a live scan is safe or
that it would cause automatic registration. Before any live integration,
require a separate safety review, an explicit RF-risk decision and a recovery
plan for cancel ambiguity; do not issue the scan from the current owner.
