# Panther modem: evidence review, 2026-09-24

Status: research, NOT a modem fix or device acceptance. No modem boot,
partition write, reset, EFS access or radio installation in this review.

## Finding: establish a coherent reference before more loads

The modem journal calls the firmware base CP2A.260705.006, but records MAIN
as `g5300q-251202-260127-B-14784800`. The local factory archive
`panther-cp2a.260705.006-factory-ed94a24e.zip` contains
`radio-panther-g5300q-260317-260505-b-15346003.img` (105894028 bytes).
Reading that ZIP entry without extracting/flashing it confirmed an FBPK
header `g5300q-260317-260505-M-15346003` and payload string
`g5300q-260317-260505-B-15346003`.

This is a provenance discrepancy, NOT proof of incompatibility in the running
phone and NOT permission to upgrade radio. Live slot/kernel/modules/radio
have not been re-read. Archive authenticity has not been checked against a
published digest.

The reverse-engineering reference is older too: AP3A/Android 15 cbd,
149600 bytes, SHA-256
`58e92a061019826d9047ddf1604d51fec98083528f53297a6fc7c7fc6f83afd5`.
Its disassembly describes THAT binary, not a complete contract for 2026.
Obtain matching vendor cbd/init configuration before declaring no startup
step or command missing.

## Sources actually reviewed

- SaaiOS modem.md including uncommitted September 24 sections and cp-boot.c
  in the wip/local-pixel7-orphan working copy. Those changes remain untouched
  and are not imported into this research branch.
- [Google s5300 source](https://android.googlesource.com/kernel/google-modules/radio/samsung/s5300/),
  existing local checkout `232fb16b3dbc3c4126d9ac0b2a0f0f514e1290c8`:
  bootdump_io_device.c, modem_io_device.c, link_device_memory_legacy.c,
  link_device.c, modem_ctrl_s5100.c. This shallow snapshot has Google's
  origin. A CPIF version string alone does not prove equivalence to installed
  cpif.ko. Gitiles file rendering failed; inspection used the local source.
- Factory AP3A cbd.dis: START f270–f294, chunk selection f2f0–f2fc,
  BIN fields f55c–f568, writer 1fec0, ACK path f5b8 onward.
- [Google gs201 configuration](https://android.googlesource.com/device/google/gs201/+/89841bf5/device.mk)
  selects CBD_USE_V2 and CBD_PROTOCOL_SIT. Generic Samsung SIPC boot recipes
  are not interchangeable with SIT.
- [LineageOS gs201 configuration](https://github.com/LineageOS/android_device_google_gs201/blob/lineage-23.2/device.mk):
  integration comparison, not an independent native loader.
- [ShannonBaseband](https://github.com/grant-h/ShannonBaseband): firmware
  analysis/Ghidra tools, not a demonstrated S5300 boot fix.
- [S5000 skeleton](https://github.com/grant-h/shannon_s5000) and
  [Galaxy S7 loader](https://gist.github.com/tonyg/4ea14f4dfe414422c0648c6e0a8bcb5d):
  historical comparison only, different generations/transports.

No applicable independently demonstrated Pixel 7 native-modem fix was found
in this search. This is not a claim that none exists.

## Source findings and limitations

### 2 KiB is not a proven boot-frame maximum

In inspected bootdump_write, check_add_overflow computes allocation from
remaining bytes plus header; SZ_2K is its overflow fallback. Optional
max_tx_size can restrict it. exynos_build_fr_config returns SINGLE for
IPC_BOOT before normal IPC fragmentation. The journal records no max_tx_size
in the boot DT node. Its early claim that 0x7E8 is the only PCIe-legal payload
is not established by this source; later journal sections already correct it.

AP3A cbd defaults to 0xC000 payload, fallback 0x7D00. Inspected SIT fields
match cp-boot. Recorded large-frame device tests nevertheless fail: reverting
to 48 KiB is not a proven fix.

### Queue consumption is not firmware acceptance

xmit_to_legacy_link copies a frame into the ring and advances head. Tail
movement proves consumption, not BIN ACK. xmit_to_cp deliberately bypasses
normal IPC interrupt handling for boot; adding arbitrary doorbells from
userspace is not justified.

init_shmem_maps can map the TX-containing prefix noncached when
legacy_raw_rx_buffer_cached is enabled. A physical region labelled cached
alone does not prove a missing TX flush; match the actual DT/module path.

### The wall is not a demonstrated 32 MiB hardware limit

Journal: 0x7E8 payload stops at next offset 0x201b098; 0x3E8 at 0x201abf0.
These are about 32.105 MiB, not 33.66 MiB (33.66 is decimal MB). The 1192-byte
difference and changed frame counts weaken a fixed frame-count hypothesis,
but prove neither an address aperture nor a parser bug. 0xF00 stopped much
earlier; a 20-second pause did not move the recorded 0x7E8 wall. Do not repeat
already-tested variants blindly.

CRC for the FULL MAIN after only partial MAIN is not a valid acceptance
test. No reply cannot prove CRC is unsupported or partial MAIN complete;
the observation can describe queue behavior only.

## Current helper audit

Uncommitted cp-boot unconditionally returns failure before offset >=
0x201b098, recording diagnostics. It intentionally cannot finish this MAIN;
do not package it as a production fix. The new diagnostic file is
`/data/saaios/var/cp-boot-ipc-20260924.txt`; existence/content unconfirmed here.

ACK reader resets partial accumulated data on EINTR/EAGAIN/zero and consumes
up to 64 bytes, discarding everything after the first word. This differs
from the reference four-byte read. These deserve isolated fault-injection
tests, but are NOT demonstrated causes of this stall. Preserve the journal's
16-byte RX-frame accounting as evidence against casually blaming ACK loss.

## Ordered investigation and acceptance gates

1. **Read-only access/provenance:** active slot, kernel identity, installed
   cpif.ko hash, existing boot/radio hashes, cbd version/hash and DT. No IMEI,
   NV payloads or secrets in reports. Read existing diagnostics; do not run
   cp-boot as a probe. SSH to historical phone IP returned Permission denied
   from Windows (default and configured laptop key) and R620. No bypass.
2. **Matched factory reference, host only:** inspect cbd from a verified
   matching factory set; compare startup arguments, ioctls, stage order,
   read/write sizes and short-I/O handling. Do not execute vendor cbd against
   live nodes to obtain a trace. Match TOC/BOOT/MAIN hashes first.
3. **Offline transcript tests:** START/BIN/CRC/DONE bytes, file/stage bounds,
   ring-end shrink, partial reads, EINTR, timeout and unsolicited/duplicate
   ACKs. No success from consumption alone; failed MAIN cannot advance stages.
4. **One instrumented test after a specific hypothesis:** bounded deadline,
   timestamps/hashes and before/after state. No automatic retries, reset
   loops, invented commands or partition writes. Kernel instrumentation or
   matched-radio flashing requires a separate proposal and rollback approval.
5. **Separate milestones:** completed firmware load and stable ONLINE; then
   SIM/control-plane registration; packet data/IP; SMS; calls/IMS. Boot alone
   does not establish cellular service, and rmnet nodes prove neither.

## Outcome

Concrete reference-version discrepancy found; several unsupported hypotheses
narrowed. Root cause remains unproven. No new device load or claim of ONLINE.
Next gate: source/artifact matching and existing diagnostics, not another
chunk-size sweep. No installable modem fix is produced by this research.
