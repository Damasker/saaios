# Bounded modem investigation: up to ten approaches

User authorized up to ten approaches on 2026-09-24. This is a ceiling, not
an obligation to perform ten hardware boots. One changing variable per live
test; preserve logs first; stop on unexpected kernel/device faults. No EFS,
calibration, identity, radio partition writes, slot changes, vendor daemon,
POWER_OFF, or repeated reset loops. No NV/stage completion in partial-MAIN probes.

| # | Approach | Status / next gate |
|---|---|---|
| 1 | A/B firmware comparison with identical bounded RAM transfer | Completed before this authorization: identical 0x201b098 stall; reference baseline, not a new attempt. |
| 2 | Exact four-byte ACK reader preserving partial bytes | Four host failures before, four passes after. Live B probe reproduced the identical stall; not the wall fix. |
| 3 | Match actual s5100sit CBD dispatch, stage descriptors and initial stage | SUCCESS at bounded transfer: all 36 MiB ACKed after initial READY + TOC, past old wall. Full MAIN validation next. |
| 4 | Match installed kernel/module with source and live DT | Static/read-only; CPIF version alone is insufficient. No module replacement without matching provenance. |
| 5 | Validate shared-memory/IOMMU range and ring mapping | Existing evidence places TX inside mapped IPC. Need observed mapping discrepancy before changing memory layout. |
| 6 | Inspect outstanding frame at the stall | Use supported read-only driver diagnostics if available. /dev/mem previously failed ENXIO; no arbitrary register pokes. |
| 7 | Check cache attributes and memory publication ordering | Source/DT first. No blanket cache-flush patch without evidence and a matching build. |
| 8 | Check CP bootloader receive/framing path | Offline disassembly; prior Msg Size Err function is not a proven UDL checker. No guessed opcodes. |
| 9 | Check runtime power/PCIe events against transfer timeline | Read-only logs first; a pacing test requires evidence of a timing dependency, not repetition of prior pause experiments. |
| 10 | Independent host model of frame boundaries, offsets and failure handling | Reproducible host tests; deploy a correction only after its failing test is demonstrated. |

Numbers are tracking slots, not claims of completed tests or guaranteed fixes.
Already tested chunk sizes, reset/no-reset, and a 20-second near-wall pause
must not be rebranded as new approaches without a changed hypothesis.

## ACK reader finding

The private working-copy loader's `sit_wait_u32` reads up to 64 bytes, returns
only the first four, and resets its partial-byte count on zero/EINTR/EAGAIN.
Host simulation demonstrates loss of a second queued word and corrupt
assembly of a split word across each transient condition. A diagnostic copy
changes the read length to `4 - done` and preserves `done` on these conditions.
All four simulated cases then return 0xc12b followed by 0xc120.

Inspected bootdump_read retains the unconsumed skb tail when the caller reads
fewer bytes; exact-size reads therefore do not intentionally discard the next
word. This is a reader defect, not yet evidence for the fixed-offset modem stall.
The user's dirty cp-boot.c is unchanged; no production fix is claimed.

Live ACK4 evidence: `/data/saaios/var/probe-b-ack4-20260924.log`. Fresh AP
boot, same verified B image, same 0x7e8 ring-fit MAIN, only ACK read behavior
changed. Still fails at 0x201b098 / chunk 16633, head 0xad000 tail 0xac800,
BOOTING and bad_cfg=0. No additional stage or retry.

Host regression source: `test-cp-ack.c` in this directory. Build on a Linux
host with `gcc -O2 -DCP_BOOT_SOURCE='"/absolute/path/to/cp-boot.c"' test-cp-ack.c -o test-cp-ack`.
It redirects read/poll and the original main; it does not access modem
devices. Use the private snapshot, not a missing file in this branch.
The two-line diagnostic correction is `read(..., 4 - done)` in sit_wait_u32
and removal of `done = 0` in its transient-error branch. The split-word and
coalesced-word cases are simulations, not claims about actual driver traffic.

## Approach 3: concrete pre-MAIN protocol difference

Newer CBD SHA-256 is recorded in MODEM-RESEARCH-2026-09-24.md. Static trace:

- `b054..b068`: s5100sit string selects `be34` then `bf14`, table `0x250a0`.
- Table fields +188 = 1 (TOC stage) and +192 = 0 (BOOT stage), normal
  handler pointer +24 = `17bb0`.
- Handler calls prepare at `17f50 -> 16fa0`; prepare builds descriptors
  at `17534 -> 19fe0`. This is not an unrelated modem's descriptor builder.
- Builder `1a1e0` puts TOC in stage 1 with flags from `0x78b0`:
  START=1, BIN=1, CRC=0, DONE=1. `1a210` puts BOOT in stage 0 with flags
  from `0x78c0`: START=0, BIN=0, CRC=1, DONE=1.
- `f2d8..f348` starts transfer at configured BOOT stage 0. `f3e0` skips
  BIN/CRC for BOOT, then `f8cc..f920` selects READY 0xa00b/0xc00b for that
  initial stage, rather than ordinary DONE. Next is TOC stage 1, then MAIN.

Thus native probes have omitted a demonstrated preamble, even though the
MAIN header itself matches. Verified B TOC size is 0x410 at file offset 0.
New diagnostic sends READY, TOC START 0xa110, a single TOC BIN 0xa11b with
total 0x410, DONE 0xa11d; each must receive its exact ACK. No guessed opcode.
On any failure, no MAIN. Otherwise the same 36-MiB-bounded MAIN follows.
This explicitly permits the initial READY and **TOC** DONE for approach 3;
MAIN CRC/DONE, final FIN/COMPLETE and NV remain prohibited. No live result
is assumed until its log is inspected.

### Live result: preamble resolves the bounded-transfer wall

Fresh AP boot and the same B image, ACK4 reader and 0x7e8 ring-fit transfer.
The new preamble's acknowledgements succeeded. MAIN reached exactly
0x02400000 (36 MiB), 18652 acknowledged BIN frames, last offset 0x23fff10.
TX head=tail=0xa3908, empty=1. `PROBE END result=2` is the intentional bound
success. CP stays BOOTING as expected: no full-image completion/NV/ONLINE.
Log: `/data/saaios/var/probe-b-preamble-20260924.log`.

This live differential supports the missing preamble as the cause of the
observed transfer failure, but does not establish which of READY or TOC is
individually necessary, nor guarantee full boot. Do not deliberately omit
one simply to consume another hardware trial. Follow the factory sequence.

Next bounded extension: full MAIN, then its real CRC/DONE, stopping before
VSS/APM/NV and final FIN/COMPLETE. Never send a full-image CRC for a partial
image. This is a new explicit experiment scope, not relaxing identity/EFS safety.
