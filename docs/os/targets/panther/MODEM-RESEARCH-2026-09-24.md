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

## Follow-up: live read-only evidence (2026-09-24 session)

Access recovered through the existing dedicated reconnect SSH identity on
R620. No key replacement or phone authorization change. Active slot is `_a`.
Kernel: `6.1.157-android14-11-gbd23337e42e7-ab14791245`.
Installed cpif.ko SHA-256:
`8cdd21d771189af08035dc6b8fc2b90708a83a520ccb0a45570836a0bce1e79c`.
The cpif modem_state sysfs node is absent on this boot; no modules were loaded.

Both radio partitions were checked against sysfs PARTNAME and mounted only
as ext4 `ro,noload`, then unmounted. Temporary device nodes removed:

| Partition | modem.bin version | SHA-256 |
|---|---|---|
| modem_a / sda19 | g5300q-251202-260127-B-14784800 | 57465ab9b78d06027e3cb39df36fec99bd158f0f2b6a05fd1bdfcb0c40d694ef |
| modem_b / sda29 | g5300q-260317-260505-B-15346003 | 449eeab3bf70fc4ed0793dce3a1f245447bf54a23b4e666df9881317bfc2344b |

The different versions are now directly confirmed, not inferred from old
logs. Different A/B versions can be normal after updates; incompatibility
is still unproven. No slot switch or firmware copy to the phone was performed.

Existing `cp-boot-ipc-20260924.txt` is 2162 bytes. It contains sysfs snapshots,
NOT physical IPC/MSI memory: both reads failed (`ipc=-1 msi=-1`, /dev/mem open
reported ENXIO). After 16633 successful BIN replies, before sending the next
frame, TX head=tail=706560 and RX head=tail=266144. GET_CP_STATUS=3. The
helper then deliberately stopped. This run itself did NOT attempt the failing
next frame; it does not independently reproduce or prove the older stall.
cp2ap_msg changed from 0 to 0x83; no interpretation as a new command is claimed.

### New vendor cbd reference recovered without executing it

Read super metadata from the sysfs-identified super partition sda30. Checked
geometry/header/table SHA-256 checksums and decoded linear vendor_a extents
according to AOSP liblp metadata_format.h. Copied only those extents to R620:
`/home/mike/saaios-audit-20260924-vendor_a.img` (about 744 MiB).
No device-mapper setup, super write, phone reboot, or filesystem repair.

Important limitation: partition attributes are 5 (READONLY|UPDATED), and
vendor_a/vendor_b share extents. The base vendor_a image is not a proven
snapshot-aware reconstruction of Android's effective vendor_a. Ordinary
debugfs reports an allocation-bitmap read failure; read-only `debugfs -c`
can recover the CBD inode and its 39 contiguous blocks. Do not boot or flash
this reconstructed image and do not claim filesystem consistency.

Recovered build.prop says `google/panther/panther:17/CP2A.260705.006/15641320:user/release-keys`.
Recovered cbd: 157744 bytes, Android 37 ELF, build ID
`dd163105a986f1c6d81ad5a21bbeb625`, SHA-256
`9b2fc0a9f3c28f3b611c6983e7ba181393dd6f2ae54310e2599cb955988ebc2b`.
Host files: `/home/mike/saaios-audit-20260924-cbd` and corresponding `.dis`.
Not committed: vendor binaries, images or device-specific state.

Static disassembly confirms for this newer reference:

- f3b0–f3d4: the same stage-index START/request and expected response encoding;
- f420–f434: 0xC000 default payload, 0x7D00 fallback;
- f66c–f674: BIN ACK still 0xC10B OR stage bits;
- f690–f6d8: min(remaining, block), zero-based offset and total-size fields;
- 21920–2197c: copy payload, length+8 field, ONE write(payload+12), exact-length check;
- ee38–ee58: read four bytes and compare reply.

Thus switching from AP3A to this newer cbd reference does NOT reveal a new
BIN header or chunk-size solution in the inspected routines. Init rc selects
`modem${ro.boot.slot_suffix}`; this supports checking slots separately, not
silently substituting modem_b. Whole startup-sequence equivalence and the
effective snapshot view still need validation. Root cause remains open.

## B-slot RAM-only experiment (2026-09-24)

User authorized a bounded live probe. Loaded the existing kernel modules from
the running installation; initial CP state OFFLINE. Mounted sysfs-verified
modem_b (sda29, 259:13) ext4 read-only with `noload`, copied modem.bin to
tmpfs, verified SHA-256 against the B hash above, then unmounted it and removed
the temporary block node. No partition writes or slot change.

Built a separate static ARM64 diagnostic from a snapshot of the uncommitted
local cp-boot.c, without changing that user's working copy. Its entry point
requires an explicit argument, OFFLINE state and no BAD CFG. It reads only
the verified tmpfs firmware, validates BOOT/MAIN bounds and stage indices,
loads BOOT, starts CP and transfers MAIN with the existing 0x7e8 ring-fit
path. Bound: 0x02400000 bytes, 180-second process alarm. No NV reads,
CRC/DONE/READY/FIN, COMPLETE, POWER_RESET or POWER_OFF in the executed path.
No automatic retry. The ordinary loader entry point is not dispatched.

Result: **same exact stall with B as previously observed with A**:

```text
BIN ACK fail at 0x201b098 chunk=16633 last_good=0x201a8b0
head=0xad000 tail=0xac800 consumed=0
modem_state=BOOTING; bad_cfg=0
PROBE END result=-1
```

The diagnostic's inherited `ACKs continued past` message is misleading:
it logs when the NEXT offset reaches the threshold, before sending that
frame. It is NOT evidence that an ACK arrived beyond the wall.

The probe exited normally after its ACK timeout, before its 36-MiB bound.
CP remains BOOTING, not ONLINE; cellular service is not implemented or fixed.
Phone UI/network host remained reachable. Do not immediately retry against
this CP state; a fresh AP boot is required for another controlled experiment.

This weakens a firmware-version-specific explanation. It does not distinguish
between shared BOOT behavior, kernel/shared-memory layout, transport and
loader protocol/state. Next investigation should trace the exact outstanding
frame and CP memory/ring mapping against the matching kernel and the actual
CBD s5100sit dispatch path, rather than change firmware or chunk sizes blindly.

Private local evidence: `/data/saaios/var/probe-b-20260924.log` on the phone;
R620 `/tmp/cp-boot-probe-base.c`, `/tmp/cp-boot-probe-b.c`,
`/tmp/saaios-probe-b`. These are experimental artifacts, not a production
loader and not installed into startup. Firmware and device data stay out of Git.
