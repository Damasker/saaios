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

### Full MAIN validation: successful

Fresh AP boot; verified B image; same ACK4 and 0x7e8 ring-fit transport.
The probe now uses the repository's `sit-boot-preamble.h` through a callback
that sends one whole packet and validates the expected four-byte ACK.
The 36-MiB stop was disabled ONLY for this full-MAIN probe; 180-second process
alarm and stop-before-other-stages remain. All 0x5917acc = 93,420,236 bytes
were sent in 46157 BIN frames. The CP then returned:

```text
MAIN CRC request 0xa321 / expected CRC 0xb0f14905 -> ACK 0xc320
MAIN DONE request 0xa12d -> ACK 0xc12d
UDL MAIN complete
PROBE END result=0
modem_state=BOOTING
```

This verifies transfer and stage acceptance, not ONLINE, SIM service or calls.
No VSS, APM, NV, final FIN or COMPLETE was sent. Device log:
`/data/saaios/var/probe-b-fullmain-20260924.log`; host copy on R620 at
`/tmp/probe-b-fullmain-20260924.log`. Diagnostic binary SHA-256:
`00a21cad5898a831e952b6c02e4af32749a7d49e6f2dd26d1770ad6bc803f056`.

## Reusable implementation and integration gate

`os/targets/panther/src/sit-boot-preamble.h` implements only the four
pre-MAIN exchanges, explicitly little-endian, without device opens or retries.
`test-sit-boot-preamble.c` checks all packet bytes/order, early termination
at each exchange, malformed input and null callbacks. Verified with GCC
`-std=c11 -Wall -Wextra -Werror -fsanitize=address,undefined` on R620.
That exact helper was used in the successful full-MAIN live test.

The dirty original cp-boot.c remains untouched. Production integration must:

1. Keep the ACK reader correction and its tests.
2. Call the preamble after successful START_CP_BOOTLOADER and before MAIN.
3. Remove the temporary 0x201b098 diagnostic stop only on the tested path.
4. Do not repeat the initial BOOT READY as an end-of-download operation.
5. Validate remaining factory stage descriptors and final FIN independently,
   before enabling later stages or declaring modem boot complete.

No need to spend the remaining experiment allowance on blind cache, firmware,
or chunk-size changes now that this specific failure has a verified fix.

## Firmware-only extension: MAIN + VSS + APM accepted

One fresh-boot B-slot probe added VSS and APM after successful MAIN, stopping
before NV, FIN and COMPLETE. Before POWER_ON, both extra entries were checked
for expected indices (3, 4), nonempty in-file extents and nonzero offsets.

Additional descriptor finding: CBD `1a148..1a160` compares the TOC name with
`MAIN` (string at 0x48db) and writes the CRC-enabled flag only on equality.
`f828..f82c` gates the CRC exchange on that flag. Thus VSS and APM use
START/BIN/DONE without CRC despite nonzero CRC fields in the image TOC.
The private native loader's unconditional CRC for every stage is incorrect
for this factory sequence. The diagnostic now gates CRC on MAIN.

Observed acknowledgements:

| Stage | Result |
|---|---|
| MAIN (2) | All BINs, CRC 0xc320 and DONE 0xc12d accepted again |
| VSS (3) | All BINs and DONE 0xc13d accepted, no CRC sent |
| APM (4), size 0xb498 | All BINs and DONE 0xc14d accepted, no CRC sent |

`FIRMWARE ONLY END`, result 0, CP BOOTING. This is the intentional NV boundary,
NOT a modem crash or proof of ONLINE service. No original or copied NV was
read by this probe. Log: `/data/saaios/var/probe-b-fwonly-20260924.log`.

Reusable helper `saaios_sit_firmware_crc_required` captures this policy with
an allowlist for reviewed stage names/indices and returns -1 for anything
else, including NV. Host tests cover allowed stages, wrong indices and NV
rejection. The live diagnostic used the equivalent MAIN-name comparison;
the new policy helper does not by itself integrate the production loader.

Next gate: review NV-copy provenance, exact NV descriptor handling, final FIN
and kernel completion/ONLINE handshake before any expansion beyond this probe.
Never use missing/zero-filled NV or original EFS as a shortcut to boot.

## Full native boot: ONLINE achieved

The existing userdata NV copies have documented provenance in the legacy
September 6 journal: one read-only/norecovery copy, then original EFS unmounted.
Both copies are 524288 bytes with 32-byte checksum sidecars. Plain MD5 and
the older Samsung_Android_RIL suffix did NOT match; that was not corruption.

Read-only extraction of factory `/bin/rfsd` from the existing host vendor
image identified SHA-256
`58d7f885e7533a328268f0de47ef9eb9995cdfa6b317d755b57973d4f5dfb71b`.
At `bf30..bf84` and `fdd0..fe24`, it hashes file bytes through EOF, then
appends `Samsung_SIT_RIL` from 0x4219 using strlen (no trailing NUL).
Both saved NV checksums match this exact algorithm. Only match results,
not NV contents, identifiers or checksum values, were displayed.
Factory rfsd was NOT executed. No sidecar or NV modification occurred.

One fresh-OFFLINE native test then performed:

1. Reviewed BOOT/READY/TOC and MAIN/VSS/APM sequence.
2. NV_NORM index 5 and NV_PROT index 6, from verified copies, 0x80000 bytes
   each; START/BIN/DONE, no CRC, matching factory descriptor policy.
3. Open ipc0 and rfs0 without a filesystem-serving daemon.
4. FIN 0xa400 -> 0xc400, then COMPLETE ioctl 0x6f23 -> rc 0.
5. Ten consecutive one-second observations of ONLINE, and another ONLINE
   observation after the diagnostic exited.

Kernel log confirms `PHONE_START <- s5300`, then `INIT_END -> s5300`, and
successful return from complete_normal_boot. This is kernel/CP boot success,
NOT proof of SIM registration, calls, SMS, packet data, or sustained uptime.
No RFS/NV writes were serviced; no vendor cbd/rild/rfsd was launched.

Evidence: phone `/data/saaios/var/probe-b-complete-20260924.log`, host
`/tmp/probe-b-complete-20260924.log`. Tested binary SHA-256:
`b0159200174ab0b8db94bb3fe8fc033e02c6f0caec29be5e3c303104f42d5183`.
The phone remains ONLINE at the last observation; no auto-start installed.

The separate `diagnostics/` source pair now preserves the tested transfer
implementation without modifying the dirty original cp-boot.c. Repackaging
adds explicit full-mode argument, firmware digest and NV verifier gates,
and compiles out the historical loader entry. ARM64 static compilation and
no-argument refusal were checked; that guarded packaging has not had a
second complete device run. See its README before use.

Next work: integrate a maintained loader into the active development branch,
test recovery/lifetime handling, and design isolated RFS plus SIT telephony
services before claiming a usable cellular stack. Never expose original EFS
to a newly implemented filesystem server.
