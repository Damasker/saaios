# Native modem runtime: status query investigation

**Latest result:** the opt-in handover comparison returned the first matching
SIM status response without a protocol error. See the final section; earlier
stalled-ring observations below are preserved as controls. Cellular service
and SIM state interpretation are not yet established.

CP boot ONLINE is verified separately in MODEM-EXPERIMENTS-2026-09-24.md.
No SIM presence/registration, voice/SMS or data service is yet established.

## Factory protocol reference

Read-only analysis of already extracted host files under
`/home/mike/panthor-backport/vendor-mount/lib64/`, not executing vendor code:

| File | SHA-256 |
|---|---|
| libsitril.so | b488325dc5333d91579da9d205267734eec4837cacb0e71016605752181376bc |
| vendor.radio.protocol.sit.base.so | 9993bc2ea4af64cf761e1101b13f6155ef11e17bc92fd6ad22136dd1a36e34ce |
| vendor.radio.protocol.sit.stream.so | cef8756461c74102f9a78f91177d1baff80fb9af11c14994497fb8854e0f530a |

The build.prop in this host extraction was unreadable to the SSH user;
do not assert exact slot/build equivalence merely from the directory name.

- Stream `ProtocolSimBuilder::BuildSimGetStatus` at 0x7dbb0 constructs ID
  0x0200 with total length 12 and no payload.
- Base `InitRequestHeader` at 0xbd40 zeros 12 bytes, writes type 0 at +0,
  ID u16 at +2, length u16 at +4, token u32 at +6; all little-endian.
- Base `ProtocolRespAdapter::GetErrorCode` at 0xbbf0 requires type 1 and
  minimum length 12, reads raw error byte +10.
- Stream SIM adapter at 0x666a0 checks ID 0x0200, reads the card byte at
  +12 and the application count at +14. The first application record starts
  at +15 and is 63 bytes. The diagnostic prints the card state, the
  application class, and the application state. It does not print the rest
  of the record.
- libsitril `IoChannel::Write` at 0x1d5cf0 passes the serialized packet to
  the device write API. The kernel supplies its own EXYNOS transport header.

## One-shot status diagnostic

`diagnostics/sit-sim-status.c` requires explicit `query-sim-status`, ONLINE,
and matching sysfs/device major/minor. One 12-byte request, token 1, maximum
10-second receive window/15-second process alarm; no resends. A bounded
buffer handles partial/coalesced frames. Unknown events are dropped without
printing payloads. Matching requires response type, ID and token. This
consumes queued events and is a diagnostic, not a shared production reader.
Do not run beside another IPC consumer. The lock excludes only this probe.

Host self-test passed with GCC warnings-as-errors and ASan/UBSan. ARM64
static build succeeded. No filesystem service, EFS access, radio-power
command, PIN/APDU operation, call or SMS is implemented.

First live test after the boot probe exited: timeout, zero observed frames.
CP still ONLINE. FMT TX head=24 tail=0, FMT RX empty: the kernel queued the
12-byte request plus its 12-byte EXYNOS header, CP had not consumed it.
Thus this is NOT evidence that the SIM is absent or the response says error.

Kernel ipc_release purges a receive queue when the last endpoint closes.
The boot probe closed IPC/RFS after ten seconds. Hypothesis: preserve those
descriptors across the first runtime request. One fresh boot comparison
with PROBE_QUERY_SIM is prepared; no blind resend to the old outstanding queue.

## Fresh-boot held-endpoint comparison

The guarded repository boot wrapper was cross-compiled with PROBE_QUERY_SIM
and used on a fresh AP boot. Firmware hash and both factory NV checksum gates
passed. Complete boot again returned rc 0 and ONLINE. IPC/RFS descriptors
remained open while the child made its single SIM status query.

Result: timeout with zero received frames again; FMT TX head=24, tail=0.
No extra commands were sent. This rejects endpoint closure as a sufficient
explanation. Do not claim that the modem processed or rejected GET_SIM_STATUS.
Phone remained ONLINE after the test, no original EFS mounted or written.

Kernel evidence:

- INIT_START received; AP capability part0=3, CP part0=7, part1=0 for both.
- PIF_INIT_DONE and INIT_END sent; COMPLETE succeeded.
- First IPC write waited the driver's normal 150-ms INIT_END interval.
- PCIe event counters: linkdown retries=0 and completion-timeout retries=0.
- Legacy FMT RX and NORM_RAW rings empty at the observed snapshot.

Source `xmit_to_cp` routes normal FMT/OEM channels into IPC_MAP_FMT unless
both link/device select SBD. Normal TX schedules an IPC interrupt, unlike
BOOT. Next research: compare live DT/module routing, ring layout and normal
IPC notification with matching factory configuration; inspect pending RFS
needs without granting any writes to original EFS. No arbitrary doorbell
register writes, protocol fuzzing or radio-power changes are justified yet.

Log: `/data/saaios/var/probe-held-sim-20260924.log` on the phone, copied to
`/tmp/probe-held-sim-20260924.log` on R620. Guarded repository boot packaging
is now live-validated (superseding the earlier compile-only note). Its SIM
child result is logged separately from CP boot success.

## Passive transport audit after the held-endpoint test

No reboot, retransmission, register write or new boot approach in this audit.
The allowlisted `diagnostics/runtime-snapshot.sh snapshot` was syntax-checked
on R620 and executed through the phone's shell. ONLINE and FMT TX 24/0
persisted; both RX rings were empty. PCIe retry counters remained zero.

Reference source: Google's s5300 checkout at
232fb16b3dbc3c4126d9ac0b2a0f0f514e1290c8. Exact equivalence to the installed
cpif.ko remains unproven; source interpretations below need that caveat.

- `cp2ap_msg=0xc8` decodes to VALID|COMMAND|PHONE_START, not an error.
- `ap2cp_msg=0x82` decodes to VALID|SEND_FMT. This proves the shared control
  field was updated, NOT that CP received/handled a PCIe doorbell.
- `pcie_send_ap2cp_irq` writes that field both when sending immediately and
  when reserving an interrupt because PCIe is off or transitioning. Thus
  the field alone cannot distinguish those cases. No matching reserve/send
  failure messages were found in the retained kernel log.
- PCI-MSI `mif_cp2ap_msg`, RX interrupt count and RX poll count all read
  49046. This is cumulative, including firmware-transfer acknowledgements;
  it does not demonstrate receipt of any runtime response.
- `napi/rx_int_enable=0` is NOT sufficient evidence of disabled PCIe IRQs.
  In this source its setters update the field only for INTERRUPT_MAILBOX,
  whereas this device uses PCI-MSI. Do not change interrupt controls based
  on this value alone.

A separate integration defect was observed: at boot completion the kernel
warned in `freq_qos_update_request`, called by `tpmon_set_cpu_freq` in cpif.
The phone has no CPU frequency policy directories. Reference tpmon checks
only a non-null request pointer before updating it; request activation
depends on CPU policy setup. This is consistent with an unregistered QoS
request, not yet proven against the installed module. Execution continued
through INIT_END and ONLINE. There is no causal proof connecting this warning
to the stalled FMT queue; do not present a QoS change as a modem fix.

Next bounded work:

1. Establish installed module/source and CPU-frequency dependency provenance;
   repair/guard inactive QoS requests only with a matching build and tests.
2. Compare the factory CBD post-FIN/COMPLETE sequence and runtime handover
   metadata with the native probe, then instrument notification delivery if
   the passive evidence remains insufficient.
3. Only after a concrete difference is established, run one fresh-boot
   comparison with one SIM query and the same ring/counter snapshots.

The snapshot script does not open modem endpoints, consume events, mount EFS,
or print NV, packet payloads or subscriber identifiers. It is not a service
health verdict: ONLINE with an unconsumed TX request is still a failure.

## Factory handover gap found in the s5100sit path

Read-only disassembly of the previously hashed factory CBD establishes a
specific missing step, not yet a proven explanation of the FMT stall:

- s5100sit caller at 0x17f50 invokes descriptor preparation (0x16fa0),
  then calls 0x20f30 at 0x17f68, before the stage transfer at 0x182a4.
- 0x20f30 calls handover builder 0x1b630 at 0x21158. The ordinary branch
  (property_get_bool false at 0x20f88) also reaches this call.
- The builder issues ioctl 0x6f57 at 0x1bfbc/0x1bfc0. Reference kernel
  names this IOCTL_HANDOVER_BLOCK_INFO; it copies the supplied structure
  into the shared handover control region. Our probe does not call it.
- Successful COMPLETE (0x6f23) at 0xfe70 is followed by 0x6f48 at 0xfef0.
  That second ioctl only clears the CP boot log in the reference kernel;
  it is not an extra runtime-start command. No need to erase evidence to
  reproduce it during diagnostics.

Handover builder evidence: version=1 at 0x1b754; CDT property parser at
0x1c0f0 uses ro.boot.cdt_hwid (fallback/override behavior still under review).
Format at 0x367a is `0x%04x%02x%02x%04x%02x%02x%02x%02x%04x%08x`.
Do not emulate sscanf's overlapping writes or invent default board values.
Table at 0x25230 has sources chosen/config/imei1 and imei2, 16 bytes each,
destination offsets 64 and 80, plus chosen/plat/rfid, 4 bytes at offset 44.
Builder reads 64 bytes from /mnt/vendor/persist/modem/cpsha into offset 96
at 0x1bb74..0x1bb84. The zero-initialized local structure is 161 bytes.
These are device-bound inputs: never substitute identifiers/signatures,
copy them from another phone, or commit/log their contents.

Live source availability check, no ioctl and no source contents printed:

- Both identity properties readable, 16 bytes each; rfid readable, 4 bytes.
- androidboot.cdt_hwid key exists in bootconfig (not cmdline).
- DT handover descriptor is two big-endian cells: type 2, offset 0x82c.
- chosen/plat/hwinfo and chosen/config/modem_flag not available at those
  paths. The former is a candidate input; its exact factory source mapping
  still needs completion. Missing optional sources are not automatically fatal.
- /mnt/vendor/persist/modem/cpsha unavailable. A filename-only search under
  /data/saaios/var, /mnt and /persist found no cpsha; this does NOT establish
  that the signature is absent from its original partition.

`handover-preflight.sh check-sources` reports this inventory without opening
modem endpoints. Exit 1 means at least one listed source is unavailable,
not that the modem or SIM has failed. Live run returned 1 as expected;
host shell syntax check passed. No mounts, partition writes, reboot or new
boot attempt were performed.

Before a handover comparison: finish the field/source/endian mapping and
factory fallback branches, establish the installed ioctl ABI, locate the
original signature through a read-only verified source, and test a strict
builder with synthetic fixtures. Only then send the authentic block to RAM
at the factory-proven point in one bounded fresh-boot test. No claim that
handover alone fixes SIM access is justified yet.

## Signature source located on the original phone

The host vendor extraction's `etc/fstab.persist` maps the `persist` partition
to `/mnt/vendor/persist` (ext4/f2fs alternatives). This explains why checking
the unmounted Android pathname in native SaaiOS found nothing.

Read-only device audit:

- Sysfs identifies sda1 as PARTNAME=persist, device 8:1. Mountinfo showed
  it was not mounted before the audit.
- An initial availability check stopped because blkid is not installed;
  its temporary device node/directory were cleaned without mounting.
- A second check read only the ext-family superblock magic (53 ef at byte
  1080), then explicitly mounted as ext4 with ro,noload,nosuid,nodev,noexec.
  Kernel mount options confirmed `ro,nosuid,nodev,noexec,relatime,norecovery`.
- `modem/cpsha` exists as a readable regular file, 64 bytes, matching the
  factory builder's read length. The path and its parent were not symlinks.
- No signature contents or hashes were printed, copied, committed or sent
  to CP. File existence/length is NOT cryptographic validation.
- The temporary mount was unmounted and temporary node/directories removed.
  Follow-up mount listing confirmed persist was no longer mounted.

The missing signature *source* is therefore resolved. Do not fabricate a
replacement, alter persist, or use the vendor fstab's writable/check/format
options. The remaining gates are exact handover ABI/field mapping and a
tested RAM-only builder. No modem boot experiment was run in this search.

## Serializer and installed-module ABI checkpoint

Host `/home/mike/panthor-backport/factory-r54-modules/modules/cpif.ko`
SHA-256 is 8cdd21d771189af08035dc6b8fc2b90708a83a520ccb0a45570836a0bce1e79c,
matching the earlier phone module fingerprint. Its actual
`update_handover_block_info` disassembly uses 0xa1 (161) for both
copy_from_user (0xec24) and memcpy into shared memory (0xec98). This
establishes this function's copy size against the binary, without asserting
that the entire reference source tree matches this module.

New pure helper `src/sit-handover.h` provides strict fixed-width CDT parsing
and explicit little-endian 161-byte serialization. It has no device/file
access or ioctl; it is deliberately NOT wired into the live boot probe.
The caller must supply all 16 words with reviewed provenance. Neither valid
length nor successful serialization authenticates a signature or proves
correct board values. Identity shape is restricted to 15 ASCII digits plus
NUL; compatibility with actual property contents is not yet checked.

The helper rejects nonzero cpinfo0/1/2 and reserved[3]. Factory disassembly
contains EFS-clear action handling and factory/debug/boot-mode branches
(0x1bc94 onward), including writes into these control fields. We do not
replicate those branches or permit guessed control words. This restriction
may require a separately justified revision if normal board metadata uses
one of these fields; do not silently bypass the guard.

`test-sit-handover.c` uses only synthetic identities/signature bytes. Host
GCC C11 -Wall -Wextra -Werror with ASan/UBSan passed; static ARM64 build and
qemu-aarch64 execution passed. Tests compare every serialized byte against
an independent layout fixture, check boundary canaries, reject truncated
CDT/output buffers and malformed identities, check forbidden controls, and
ensure failed validation leaves output unchanged.

Remaining before live integration: trace revision/hwinfo initialization and
all relevant normal-boot metadata/default branches, validate the bootconfig
CDT and actual identity representation without logging contents, then add
a read-only source adapter and test it. No block was sent to CP at this
checkpoint, and the FMT stall is not claimed fixed.

## Native source-format validation and JSON revision path

The new read-only `diagnostics/handover-source-check.c` was cross-compiled
with C11 -Wall -Wextra -Werror, transferred to phone tmpfs and run explicitly
with `check-sources`. It does not open modem devices, construct an output
block or issue ioctls. Result: bootconfig CDT passed the strict 34-character
parser; both device identity properties passed 15 decimal digits plus NUL.
Only pass/fail labels were printed. Input buffers are cleared after use.
This validates representation, not signature authenticity or field semantics.

Factory revision source tracing has advanced:

- 0x1ade0 calls rfid setter 0x1cb10 with a JSON integer; 0x1adec calls
  hwinfo setter 0x1cba0 with a JSON integer. The latter stores the value at
  global offset 536 and sets presence flag 545 (0x1cc10..0x1cc18).
- Handover builder 0x1b854..0x1b868 checks that presence flag and copies
  hwinfo into block offset 8 (revision). Thus the earlier speculative DT
  path chosen/plat/hwinfo is not established as this factory source.
- Factory strings identify `hardware_config.json`, a property override
  `persist.vendor.modem.hw_config.json_table`, and fallback pathname
  `/mnt/vendor/modem_img/images/default/hardware_config.json`.
- Read-only ext4 ro,noload audit of verified modem_b (sda29, 259:13)
  found NO hardware_config.json anywhere on that filesystem. The regular
  files listed within three directory levels were modem.bin, pw_token_db
  and pw_token_db.csv in the known g5300q-260317-260505-B-15346003 directory.
  Token databases were not read. Temporary mount/node were cleaned.

Consequently, do not transfer new-CBD JSON behavior to this firmware as if
the packaging matched. The no-table fallback and/or matching-generation
CBD still need tracing before deciding whether an unset revision is the
factory-intended result. No hardware test with an invented revision was run.

## No-JSON profile and live in-memory candidate

Further CBD tracing: table lookup is conditional at 0xc224..0xc240.
The lookup failure path 0x1ac04..0x1ac2c tries the literal
`/mnt/vendor/modem_img/images/default/RF_CFG_DEFAULT` using stat helper
0x1b570; it does not call the hwinfo/rfid setters. Builder 0x1b854..0x1b890
logs a missing hwinfo flag but does not overwrite its initially zero revision.
Therefore revision zero is supported for a **fresh process with no applied
JSON override**, not a universal replacement for unknown board revision.

Added an explicit pure no-JSON normal/user profile mapping with these words:
version=1; project=CDT[0]; revision=0; major/minor=CDT[3]/[4];
SKU/HW=CDT[6]/[7]; rf_sub=CDT[8]; rf_config=DT rfid; reserved[0..2]=
CDT[1]/[2]/[5]. Control words and reserved[3] are zero. CDT[9] is parsed
but not mapped by this profile. Reject project 4 (extra factory branches),
missing required CDT header values and fields outside their parsed widths.
Mapping tests compare an explicit synthetic expected array; ASan/UBSan pass.

`handover-source-check candidate-no-json <signature-path>` is **candidate
construction only**, not an executable boot profile. It requires the caller
to establish the no-JSON/normal-user assumptions, refuses a present or
uncheckable modem_flag, reads exact-size sources, converts DT rfid from
big endian, builds 161 bytes in RAM, and immediately clears them. No output
file, ioctl or modem endpoint is implemented. Core dumps are disabled,
process dumpability disabled, and a 15-second alarm bounds execution.

This mode was built with ARM64 GCC warnings-as-errors and run on the phone
using cpsha directly from a temporary persist ro,noload mount. Strict CDT,
both identities, exact source reads and candidate construction all passed.
The final version with core-dump/alarm guards was also run successfully.
Only labels/sizes were printed, no original partition was written, and the
mount was cleaned on exit. This does not authenticate the signature, prove
all factory properties match, or prove CP accepts the candidate.

Next: integrate only the reviewed normal profile into an opt-in probe after
rechecking source/module/firmware guards and the exact pre-stage timing.
Run one fresh-boot handover comparison with one SIM query; retain existing
no-handover result as control. Do not enable boot-time autostart or claim
the runtime transport fixed until the queue is actually consumed.

## Live handover comparison: first successful runtime response

One fresh AP reboot, same verified B modem.bin and factory-checksummed NV
copies, same held IPC/RFS endpoints and one SIM request. The new
PROBE_HANDOVER mode prepares the authentic block before POWER_ON, then
issues HANDOVER_RAM_ONLY (0x6f57) after START/BOOTING and before READY/TOC.
ioctl returned 0. No reset/erase/factory controls were enabled.

Results:

- All firmware stages, FIN and COMPLETE passed; CP remained ONLINE.
- SIM query received a matching response, length 80, error_raw=0; child
  exit status 0. Earlier identical held-endpoint query timed out.
- FMT TX head=24 tail=24: CP consumed the request (previous control: 24/0).
- FMT RX head=2448 tail=2448, RAW RX head=64 tail=64. These are transport
  counters, not a claim that every queued event was serviced by userspace.
- Reply small raw fields: card_state=0, universal_pin=0, applications=0.
  Do not translate these into a SIM-present/absent conclusion until the
  adapter's enum/layout is independently verified.
- cp2ap_msg changed to 0x83; PCIe linkdown/CPL retry counts still zero.
- CP sleep/wakeup counters now move. CPU QoS warning still occurs, so fixing
  that separate integration issue was not necessary for this response.

This provides strong controlled evidence that missing handover was blocking
normal runtime progress. It establishes one successful SIM-status exchange,
not registration, calls, SMS, mobile data or long-term stability. There is
still no RFS filesystem service, and this remains an opt-in diagnostic.

`run-handover-comparison.sh run-once` checks fresh OFFLINE, the audited cpif
hash, current endpoint major/minor values and persist partition identity.
It mounts persist ro,noload,nosuid,nodev,noexec, uses its original 64-byte
cpsha in RAM and cleans its temporary mount/node. Follow-up inspection
confirmed persist unmounted and no temporary audit directory remained.
No original EFS was mounted/written, no partition flashing or slot change.

Phone log: `/data/saaios/var/probe-handover-20260924.log`; host copy:
`/tmp/probe-handover-20260924.log`. No identifiers or signature contents
are included in the documented result. Next: validate status enums and
normal radio/SIM initialization before considering any registration test.

## Runtime follow-up: radio on and later SIM application

Factory stream BuildGetRadioState at 0x746a0 builds ID 0x0801, 12-byte
header, no payload. ProtocolNetRadioStateRespAdapter at 0x48770 reads a
32-bit little-endian state at offset 12. Table 0x2a9c8 maps value 10 to
string 0x2a77b, `SIT_PWR_RADIO_SIM_STATE_ON`. This GET changes no radio power.

The diagnostic now supports `query-radio-state`, token 2 versus SIM token 1.
It retains node/ONLINE checks, lock, single write, no retries and bounded
receive. Raw state is printed only for a matching successful response of
at least 16 bytes. Host sanitizer tests and ARM64 warnings-as-errors build
passed. No identifier/payload dumps were added.

Without reboot or additional initialization writes, GET_RADIO_STATE returned
length=16, error_raw=0, state_raw=10 (ON). TX ring advanced to 48/48. One
subsequent GET_SIM_STATUS returned length=143, error_raw=0, card_state_raw=1,
universal_pin_raw=3, applications=1. The immediate post-boot response had
length 80, state 0 and applications 0. Do not equate that early snapshot
with permanent SIM absence. Deferred initialization is plausible, not proven
without an event timeline and confirmation of unchanged SIM insertion.

Factory libsitril FillRilCardStatusFromAdapter (0x1554c4..0x1554d4) copies
the adapter card byte to RIL card state; setNoSim (0x11c534) stores zero.
The later response demonstrates one reported application. PIN state 3 and
ds_detect=2 are left uninterpreted. No PIN/PUK/APDU or subscriber identifiers
were accessed or changed. Radio ON is not network registration.

Next: verify registration GET builders and response fields, then observe
registration without operator changes, calls or SMS. No automatic polling
or RFS filesystem service was installed.

## Data registration query: not registered, not searching

User confirmed a physical SIM is inserted. Added an explicit bounded
`query-data-registration` mode, ID 0x0701, token 3, no payload or resends.
Factory BuildNetworkRegistrationState at 0x743a0 selects ID 0x0701 for
domain 2 and builds a 12-byte request (domain 1 uses 0x0700, not sent here).
Data adapter accessors at 0x47c50/0x47c80/0x47ce0 read registration byte
12, rejection byte 13 and technology byte 15. Only these fields are logged;
cell/location and subscriber fields are not printed.

One live request on the already-running handover boot returned length 86,
error_raw=0, registration_raw=0, reject_cause_raw=0, radio_tech_raw=0.
The vendor state conversion at 0x47620..0x47698 maps raw zero to RIL zero.
[AOSP RegState](https://android.googlesource.com/platform/hardware/interfaces/+/0adfb810ae03c6f13fc24f30ab4beb89638aea0e/radio/aidl/android/hardware/radio/network/RegState.aidl)
defines zero as not registered and not currently searching. This is a
packet-domain snapshot, not a voice-domain result or proof of operator
rejection. No network-selection or radio-power command was sent.

Host ASan/UBSan self-test and warnings-as-errors static ARM64 build passed;
the self-test also covers the registration ID/token fixture. Next investigate
factory post-SIM initialization and prerequisites for registration. Do not
infer that GET itself initiates attachment or that an APN is already needed
to explain the current registration state.

## 2026-09-29: ONLINE again, RFS spoke, bearer still down

COM13 dropped once while a mount was attempted. `/tmp` is `nodev`, so a
block node created there cannot be opened. The reviewed image is modem_b
`g5300q-260317-260505-B-15346003`, SHA-256
`449eeab3bf70fc4ed0793dce3a1f245447bf54a23b4e666df9881317bfc2344b`.
It was copied to tmpfs from a read-only mount and the partition was
unmounted. No slot was changed and nothing was written to either radio
partition.

`run-handover-comparison.sh` stopped because this rootfs has no `awk`.
The same checks were done by hand, persist was mounted read-only, and
`/tmp/probe-handover boot-b-with-verified-nv-handover` ran. FIN and
COMPLETE returned success. The modem stayed ONLINE after the probe exited.
Persist was then unmounted.

The early SIM snapshot was again length 80 with applications 0. A later
query on the same ONLINE session matched the previous run: radio state 10,
SIM length 143, card_state 1, one application. Data registration was again
length 86, error 0, registration 0, reject 0, technology 0. `rmnet0`
rx_bytes and tx_bytes were both 0, and it had no IPv4 address.

After the ten ONLINE observations the probe read `/dev/umts_rfs0` once.
The CP had queued 12 bytes; the first little-endian word was `0x7`. No
path, payload, or reply was logged or sent. The read shows the CP is
waiting on the RFS channel. It does not by itself identify the command
or create a bearer.

A later nonblocking read of `/dev/umts_rfs0`, while the modem was still
ONLINE, returned no further bytes. The 12-byte message had already been
consumed and was not repeated. `rmnet0` remained at rx=0 tx=0. The other
two header words were not saved, so that message cannot be answered
without a fresh boot that records all three words before any reply.

A fresh boot with the updated probe recorded the whole 12-byte read as
three little-endian words: `00000007 00000004 00000003`
(`/data/saaios/var/probe-rfs-hdr-20260929.log`). The earlier read with a
64-byte buffer also returned exactly 12, so this is the entire delivered
chunk, not a cut-off path. No reply was sent. The modem was ONLINE again
and `rmnet0` still had no traffic. These three words are not yet mapped
to a factory RFS handler on this host, so they are not treated as a file
write.

## 2026-09-29: command 7 is protected-NV unprotect

Read-only mount of stock `vendor_b` from `super` (device-mapper linear,
ext4 `ro,noload`) produced factory `/bin/rfsd`, SHA-256
`58d7f885e7533a328268f0de47ef9eb9995cdfa6b317d755b57973d4f5dfb71b`.
The binary was not executed. Vendor was not left mounted for the CP.

The 12-byte request is a header, not a path: `u16` command, `u16` zero,
`u32` length 4, then a 4-byte file id. Command 7 selects the virtual
method that logs `RFS_NV_UNPROTECT`. File id 3 is the protected-NV object.
The handler checks that the payload word equals 3, then the success
status written back to the channel is 16 bytes:
`03 00 00 00 08 00 00 00 00 00 00 00 03 00 00 00`.
Factory code also chmods its own NV path and reopens it. That local
chmod is not part of the CP-visible reply and is not performed here.
Original EFS stays untouched. The probe now sends only this status when
the 12 bytes match exactly, then logs the next header without printing
a path or file body.

On the following OFFLINE boot the same 12 bytes arrived again. The probe
wrote the 16-byte status (`write` returned 16) and did not chmod or open
a file. The CP then delivered 20 bytes: command 3, length 12, first
payload word 0. In the protected-NV handler command 3 logs
`RFS_OP_STATUS`. The bytes after the first payload word were not stored.
The modem stayed ONLINE. `rmnet0` rx and tx were still 0. Persist was
unmounted afterward. Log: `/data/saaios/var/probe-unprot2-20260929.log`.

The next OFFLINE boot stored the unread tail. The 20-byte packet is
command 3, length 12, file id 3, and the last word 0. That matches the
success checks in `RFS_OP_STATUS`, which updates local state and writes
nothing back. Log: `/data/saaios/var/probe-opstatus-20260929.log`.

## 2026-09-29: command 6 operation 2 is a protected-NV write request

After the unanswered status, the CP sent 24 bytes:
`00010006 00000010 00000003 00000000 0002e406 00000002`.
Command 6 is `RFS_IO_REQUEST` for file id 3. The payload is offset 0,
length 189446, operation 2. Operation 1 is the read path. Operation 2
is the write path, and only after the local object state is 2. The
unprotect-then-status sequence leaves that state at 3, where operation 2
is logged as a bad I/O request and produces no channel reply and no file
write. The probe did the same: no reply, no open, no write. The modem
stayed ONLINE. `rmnet0` rx and tx stayed 0. Persist was unmounted.
Log: `/data/saaios/var/probe-ioread-20260929.log`.

A write grant exists in the factory handler for state 2: a 20-byte
packet, command 2, length 12, then file id, offset, and a chunk capped
at 2012. This boot never entered state 2, so that grant was not sent.
The 189446 bytes are not in the 24-byte request. They would arrive only
after the grant, and writing them is not done here.

The following boot kept the RFS channel open after that refusal. No
further request arrived within 3 seconds. The modem stayed ONLINE and
`rmnet0` stayed at 0/0. Persist was unmounted. Log:
`/data/saaios/var/probe-hold-20260929.log`.

## 2026-09-29: automatic network selection is accepted, registration stays 0

Factory `BuildSetNetworkSelectionAuto` (`vendor.radio.protocol.sit.stream.so`,
SHA-256 `cef8756461c74102f9a78f91177d1baff80fb9af11c14994497fb8854e0f530a`)
builds a 12-byte request, id `0x0704`, with no PLMN and no other payload.
On the already ONLINE modem, one send with token 4 returned length 12,
`error_raw=0`. A data-registration GET (`0x0701`) immediately afterward
and again about 20 seconds later both returned length 86, `error_raw=0`,
`registration_raw=0`, `reject_cause_raw=0`, `radio_tech_raw=0`. `rmnet0`
rx and tx stayed 0. Radio power was already ON, so `BuildRadioPower`
(`0x0800`, three arguments) was not sent. Preferred network type
(`0x070a`) is a separate 16-byte request whose type field is computed;
it was not sent.

## 2026-09-29: SIM is present and PIN-locked; preferred type is LTE_ONLY

On the same ONLINE modem, one SIM GET (`0x0200`, token 1) returned length
143, `error_raw=0`. Factory `covertCardStateToString` names wire byte 12
when it is left unchanged: 0 ABSENT, 1 PRESENT, 2 ERROR, 3 RESTRICTED.
The adapter also rewrites wire 3 to stored 1 and wire 4 to stored 3.
This response had `card_state_raw=1`, so the stored state is PRESENT.
`applications=1`. `BuildRilCardStatusApplications` reads the first record
at packet offset 15. Type 2 is in the GSM/UMTS class (values 1 and 2,
the factory log line calls that class SIM/USIM). State byte at offset 17
was 2. `covertAppStateToString` names 2 as PIN and 5 as READY. The
application is therefore present and waiting for PIN. No PIN, AID, or
subscriber field was printed or sent. `universal_pin_raw` was 3 and is
left unnamed.

`BuildSetPreferredNetworkType` is id `0x070a`, length 16, with the SIT
type as a `u32` at offset 12. The name table at file offset `0x8cfc8`
index 11 is `SIT_NET_PREF_NET_TYPE_LTE_ONLY`. One send, token 5, value 11,
returned length 12, `error_raw=0`. The matching GET (`BuildGetPreferredNetworkType`,
id `0x070b`, length 12, token 6) returned length 16, `error_raw=0`,
`preferred_raw=11`. The CP reports the preferred type as LTE_ONLY.

Data registration afterward was still length 86, `error_raw=0`,
`registration_raw=0`, `reject_cause_raw=0`, `radio_tech_raw=0`.
`sit-base` `toRadioTech` maps 0 to no technology, and maps 14 and 20 to
RIL LTE. Radio GET stayed at 10. `rmnet0` rx and tx stayed 0. The modem
stayed ONLINE. The PIN state is the reason the registration word is still
0; no network-search result is available yet. No PIN verify command was
sent.

## 2026-09-29: PIN1 verify is rejected without consuming a retry

Factory `BuildSimVerifyPin` with its first argument 0 is the PIN1 path
used by `SimLockHandler::DoVerifyPin`: id `0x0201`, length 38. Byte 12 is
the character count, the characters start at byte 13, and a null AID
leaves byte 21 and the following 16 bytes zero. PIN2 would be id `0x0203`
and was not sent. The response adapter accepts id `0x0201` and reads the
remain count as the `u32` at offset 12. `GetErrorCode` reads byte 10 and
maps it through `ConvertProtocolErrorCodeToRilErrorCode`; values 0 through
17 pass through unchanged, so protocol error 6 is RIL code 6.

Two attempts were sent, one each, tokens 7 and 8. Neither was repeated.
Both responses were length 16, `error_raw=6`, `remain_raw=3`. The SIM GET
between and after them stayed length 143, `error_raw=0`, card PRESENT,
one SIM/USIM application, `app0_state=PIN`. It did not become PUK or
SUBSCRIPTION_PERSO. `GetPinState` for PIN1 is packet byte 72 and
`GetPinRemainCount` for PIN1 is byte 74; those read 3 and 3. The remain
count in the verify response matches that byte and did not decrease, so
the CP did not count either attempt as a PIN check.

Preferred-network GET still returns `preferred_raw=11` (`LTE_ONLY`).
Data registration is still `registration_raw=0`, `radio_tech_raw=0`.
`rmnet0` rx and tx are 0. The modem stayed ONLINE. The PIN characters
were not printed. No further verify was sent.

## 2026-09-29: replacement SIM still reports PIN after one reboot

A query before reboot, on the already ONLINE modem, was unchanged:
length 143, error 0, card PRESENT, one SIM/USIM application, app state
PIN. One `sysrq` reboot followed. Signed CPIF modules were loaded again,
the reviewed modem_b image hash matched, and
`probe-handover boot-b-with-verified-nv-handover` reached ONLINE
(`result=0`). Persist was mounted read-only only for the signature read
and then unmounted. modem_b was mounted read-only only to copy the image
and then unmounted.

The probe's own early SIM query was length 80, card ABSENT, applications
0. A later query on the same ONLINE modem was length 143, error 0, card
PRESENT, one SIM/USIM application, app state PIN. PIN1 state byte 72 and
remain byte 74 were both 3. No PIN verify was sent. Preferred network,
data registration, and radio power were not queried again on this boot.
`rmnet0` rx and tx were 0. The modem stayed ONLINE.

## 2026-09-29: same two PIN candidates on the replacement SIM

On this ONLINE boot the SIM GET was still length 143, error 0, card
PRESENT, one SIM/USIM application, app state PIN, PIN1 remain 3. The
factory PIN1 verify (`0x0201`, length 38, null AID) was sent once per
candidate, first then second, tokens 7 and 8. Both responses were length
16, `error_raw=6`, `remain_raw=3`. The SIM GET after each stayed PIN.
The remain byte did not decrease and the app did not become PUK or
SUBSCRIPTION_PERSO. Neither candidate unlocked the application. No third
value was sent. Preferred network and data registration were not queried.
`rmnet0` rx and tx stayed 0. The modem stayed ONLINE.

## 2026-09-29: the sent PIN packet matches the factory builder

`BuildSimVerifyPin` (`sit-stream.so` `0x7dc30`) with its first argument 0
writes id `0x0201` and length 38. Argument 1 would be id `0x0203` and was
not used. The character count is stored at byte 12, and the characters
themselves are copied from byte 13 as a raw string, capped at 8. A null
second pointer skips the AID conversion, leaving byte 21 and bytes 22..37
zero. `SimLockHandler::DoVerifyPin` calls that function with argument 0
and a null AID when the request carries only the PIN string. The bytes
sent by `sit-sim-status` are that packet. Nothing in the layout differs
from the factory builder, so the candidates were not sent again.

`ConvertProtocolErrorCodeToRilErrorCode` maps protocol byte 6 to RIL code
6. `rcmErrorToString` names protocol code 3 `RCM_E_PASSWORD_INCORRECT` and
protocol code 6 `RCM_E_REQUEST_NOT_SUPPORTED`. Error 6 is therefore not a
wrong-PIN result. The remain field staying at 3 matches a request the CP
did not count as a PIN attempt. The application remains PIN. `rmnet0` was
not queried again.

## 2026-09-29: no second factory command unlocks PIN1

`SimLockHandler::DoVerifyPin` is the only caller that passes argument 0 to
`BuildSimVerifyPin`, and that argument selects id `0x0201`.
`DoVerifyPin2` passes argument 1, which selects id `0x0203`. That is the
PIN2 command, and the application state is PIN, not a PIN2 lock.
`BuildSimVerifyPuk` uses `0x0202` or `0x0204`. The application is not in
PUK. `BuildSimVerifyNetworkLock` uses `0x0207`.
`BuildSimVerifyEncryptedPin` uses id `0x4603` and length 63, and
`DoAutoVerifyPin` calls it only for a stored encrypted value. The payload
is a hex string converted to bytes, not the PIN characters. That encoding
is not a second way to submit the same candidates.

No other decoded builder is the PIN1 unlock. `0x0201` was not sent again.
No other PIN command was sent. The application remains PIN. Registration
and `rmnet0` were not queried again.

## 2026-09-29: factory sends 0x0201 with no prior command

`DoVerifyPin` checks that the PIN string is one to eight digits and then
calls `BuildSimVerifyPin` with argument 0. No channel, facility, power, or
status request is sent first. The other `BuildSimVerifyPin` callers pass
argument 1, which is id `0x0203`: SIM I/O for file ids `0xdc` and `0xd6`,
and the FD facility path. Those are PIN2, not a PIN1 unlock.

`CheckAndAutoVerifyPin` calls `DoAutoVerifyPin` only when `GetPinState`
returns 1, an encrypted-PIN property exists, and the handler word at
offset 56 is 4. `DoAutoVerifyPin` then sends `BuildSimVerifyEncryptedPin`,
id `0x4603`, length 63. The payload is a hex string turned into bytes, not
the raw PIN characters. This card's PIN1 state byte is 3, so that gate
does not open. `0x4603` was not sent, and `0x0201` was not sent again.

A fresh SIM GET is still length 143, error 0, card PRESENT, one SIM/USIM
application, app state PIN, PIN1 state 3, remain 3. Data registration is
length 86, error 0, `registration_raw=0`, `reject_cause_raw=0`,
`radio_tech_raw=0`. `rmnet0` rx and tx are 0. The modem stayed ONLINE.

## 2026-09-29: PIN1 state value 3 has no factory name

`ProtocolSimStatusAdapter::GetPinState` (`sit-stream.so` `0x66a20`) returns
one byte. Pin argument 1 uses adapter offset `0x58`. For application 0
that is adapter+88. The status packet is copied to adapter+16, so the
byte is packet offset 72. Pin argument 2 uses the next byte. The live
value at offset 72 is 3.

The only caller, `CheckAndAutoVerifyPin`, compares that return value with
1 and does not name 3. There is no `covertPinStateToString`. `libsitril.so`,
`sit-stream.so`, and `sit-base.so` contain no `RIL_PINSTATE` name and no
`ENABLED_NOT_VERIFIED`, `ENABLED_VERIFIED`, or `ENABLED_BLOCKED` string.
A card-status log prints `PinState(%d, %d)` as integers. Value 3 is
therefore not a proven disabled name and not a proven locked name. No
preferred-network, selection, or PIN command was sent on that basis.

## 2026-09-29: this boot prefers LTE_ONLY, registration stays 0

On the post-swap ONLINE modem a SIM GET was length 143, error 0, card
PRESENT, one SIM/USIM application, app state PIN. Preferred-network GET
(`0x070b`) returned length 16, error 0, `preferred_raw=16`. One
`set-preferred-lte` (`0x070a`, value 11) returned length 12, error 0.
The following GET returned `preferred_raw=11` (`LTE_ONLY`). Data
registration (`0x0701`) was length 86, error 0, `registration_raw=0`,
`reject_cause_raw=0`, `radio_tech_raw=0`. `rmnet0` rx and tx were 0 and
it had no IPv4 address. The modem stayed ONLINE.

`ProtocolMiscBuilder::GetSignalStrength` is id `0x0900`, length 12, with
no payload. The response parser is a versioned bitfield over several
radio blocks, and no factory name isolates an RSRP or level byte from
the rest of that body. That GET was not sent. No PIN command was sent.

## 2026-09-29: automatic selection after LTE_ONLY, still no bearer

On this same ONLINE boot the SIM GET was still length 143, error 0, card
PRESENT, one SIM/USIM application, app state PIN. One
`BuildSetNetworkSelectionAuto` (`0x0704`, length 12, no PLMN, token 4)
returned length 12, `error_raw=0`. Three data-registration GETs about 15
seconds apart were each length 86, error 0, `registration_raw=0`,
`reject_cause_raw=0`, `radio_tech_raw=0`. `rmnet0` rx and tx stayed 0 on
all three reads, and `ip -4 addr` showed no address. The modem stayed
ONLINE. No PIN command was sent.

## 2026-09-29: replacement data SIM still reports app state PIN

A SIM GET on the previous ONLINE boot, after the user inserted another
card, was unchanged: length 143, error 0, card PRESENT, one SIM/USIM
application, app state PIN. One `sysrq` reboot followed. The reviewed
modem_b image hash matched, and `probe-handover` reached ONLINE
(`result=0`). Persist was mounted read-only for the signature read and
then unmounted.

The probe's early SIM query was length 80, card ABSENT, applications 0.
A later query was length 143, error 0, card PRESENT, one SIM/USIM
application, app state PIN. PIN1 state byte 72 was 3 and remain was 3.
No PIN, PUK, or preferred-network command was sent. Registration and
`rmnet0` were not read on this boot because the application was not
READY. The modem stayed ONLINE.

## 2026-09-29: LTE_ONLY and automatic selection on the new boot

The same ONLINE modem, after the data-SIM reboot, still returned a SIM
GET of length 143, error 0, card PRESENT, one SIM/USIM application, app
state PIN. Preferred-network GET was length 16, error 0,
`preferred_raw=16`. One `set-preferred-lte` returned length 12, error 0,
and the following GET returned `preferred_raw=11` (`LTE_ONLY`). One
`set-network-selection-auto` returned length 12, error 0. Three
registration GETs about 15 seconds apart were each length 86, error 0,
`registration_raw=0`, `reject_cause_raw=0`, `radio_tech_raw=0`. `rmnet0`
rx and tx stayed 0, and there was no IPv4 address. No PIN command was
sent. The modem stayed ONLINE.

## 2026-09-29: no card-refresh request; READY is app state 5

`BuildSimGetStatus` (`0x0200`) is the status read already used. There is
no separate no-payload command that only asks the CP to read the card
again. `BuildSetSimCardPower` is id `0x024c`, length 13, with a state
byte at offset 20. `BuildSetUicc` is id `0x0249`, length 13, also with a
state byte. Neither is a re-read, and neither was sent. SAP reset and
STK refresh are not a local card re-read.

`NetworkService::OnSimStatusChanged(int, int)` at `0x13fc80` keeps the
first argument in `w21` and the second in `w20`. It continues only when
the first argument is 1 (`0x13fd18`). At `0x13fd28` it compares the
stored word at `this+916` with 5 and, when that is not equal, compares
`w20` with 5:

```
13fd28: cmp w8, #5
13fd2c: ccmp w20, #5, #0, ne
13fd30: b.eq 13ff3c
```

`covertAppStateToString` names 5 READY and 2 PIN. The branch at
`0x13ff3c` is therefore the transition into READY. App state 2 does not
satisfy the compare with 5, so that transition is not taken. No live
command was sent for this check.

## 2026-09-29: card-power and UICC state bytes are unnamed

`BuildSetSimCardPower` is id `0x024c`, length 13. The caller's integer is
stored as one byte at packet offset 12. `SetSimCardPowerHandler` reads
the request word at message offset 28. On a HAL version of `0x16` or
newer, a request value of 0 is rewritten to 4 before the send; any other
value is forwarded unchanged. Older HAL paths reject values at or above
2, or at or above 3 when the version is `0x11`. The libraries contain no
`POWER_UP` or `POWER_DOWN` name for those numbers.

`BuildSetUicc` is id `0x0249`, length 13, with the same one-byte payload
at offset 12. `SetUiccHandler::OnRequest` does not call it; it completes
the request with code 6. `EnableUiccAppHandler` does call it and passes
the request word through with no rewrite and no named ON or OFF value.

Neither byte is a proven power-on distinct from power-off. Neither
command was sent. The application state was not queried again.

## 2026-09-29: app state really is packet byte 17

`ProtocolSimStatusAdapter::Init` at `0x666a0` copies the status packet to
adapter+16. The application count is packet byte 14. The copy length is
15 plus 63 bytes per application. `BuildRilCardStatusApplications` starts
the first record at adapter+0x54. The type byte is 53 bytes before that
(adapter+31, packet offset 15) and the state byte is 51 bytes before that
(adapter+33, packet offset 17). `covertAppStateToString` names packet
value 2 PIN and 5 READY. The diagnostic already printed those offsets.
The last SIM GET on this boot was length 143, card PRESENT, one SIM/USIM
application, type byte 2, state byte 2.

Holding `/dev/umts_ipc0` open for 30 seconds without writing produced no
frames (`listen_frames=0`). No PIN command was sent. Registration and
`rmnet0` were not read again. The modem stayed ONLINE.

## 2026-09-29: PIN1 byte 3 is the AOSP pin1 field

`GetPinState` for PIN1 returns packet byte 72 with no conversion.
`BuildRilCardStatusApplications` stores that word at application offset 36
and PIN2 at offset 40. `SimLockHandler::GetPinState` reads a filled card
status: if the word at offset 56 is 0 it returns the word at offset 60,
otherwise the word at offset 4. Those are application 0's `pin1_replaced`
and `pin1`, and the card's universal PIN. That is the AOSP
`RIL_AppStatus` rule. AOSP names pin value 1
`RIL_PINSTATE_ENABLED_NOT_VERIFIED` and value 3 `RIL_PINSTATE_DISABLED`.
The factory libraries have no `RIL_PINSTATE` string. The only comparison
is `CheckAndAutoVerifyPin`, which continues only when the value is 1.
Value 3 does not open the encrypted-PIN path. `AUTO_PIN_STATE` is a
different enum: 0 is DISABLED and 3 is ENABLING.

Application state at packet byte 17 is still 2, `RIL_APPSTATE_PIN`.
`NetworkService::OnSimStatusChanged` takes the ready transition only when
the stored state is already 5 or the new state is 5. A disabled PIN field
does not replace that check.

`BuildRadioPower` is id `0x0800`, length 18. The 32-bit value at offset 12
is 1 when the first argument is 0 and 2 when it is nonzero.
`DoRadioPower` passes a nonzero first request integer for
`RADIO_STATE_ON` and zero for off, so the packet values are 2 and 1.
Bytes 16 and 17 are the second and third arguments, each reduced to 0 or
1. `BuildShutdown` stores 3 and `BuildRestartModem` stores 4 in that same
word. This is one radio-power request, not an off/on cycle started from
SIM PIN or ABSENT. It was not sent.

On this boot preferred type still reads 11 (`LTE_ONLY`). Automatic
selection was not sent again. Eight data-registration reads, about 12
seconds apart, were each length 86, error 0, registration 0, reject 0,
tech 0. `rmnet0` rx and tx were 0 and it had no IPv4 address.
`BuildOperator` (`0x0702`, 12 bytes, no payload) returned length 119,
error 0, with the PLMN and both name fields empty. No name bytes were
printed. `BuildSetupDataCall` was not sent: there is no lifecell APN
string in the factory library, and the `PdpContext` layout is not fully
decoded. No PIN command was sent.

## 2026-09-29: no factory command turns PIN app state into READY

`BuildSimVerifyPin` has four callers. `DoVerifyPin` reads the PIN string
and, if the first byte is zero, completes the request locally from the
cached pin word. It does not call the builder. A nonempty string is sent
only when its length is 4 through 8 and every byte is a digit. Facility
lock and SIM I/O also skip the builder when the password length is zero.
`DoVerifyPin2` is the PIN2 form. None of these callers test pin1 value 3.

`BuildRadioPower` has one caller, `DoRadioPower`, which is the radio-power
request. `OnSimStatusChanged`, `OnGetSimStatusDone` and the auto-PIN
handler do not call it, and there is no off-then-on sequence for a
disabled PIN. `CheckAndAutoVerifyPin` returns without sending when the
pin word is not 1, so value 3 does not start the encrypted PIN command.
`OnGetSimStatusDone` publishes the card status and does not send a
follow-up that changes application state 2 into 5.

No empty PIN packet and no radio-power cycle were sent. SIM status was
not queried again. Registration and `rmnet0` are unchanged from the
previous poll.

## 2026-09-30: open channels do not clear app state PIN

The modem was already ONLINE, so it was not booted again. One process
opened `/dev/umts_ipc0` and `/dev/umts_rfs0` and held both for three
minutes. It answered only the exact 12-byte protected-NV unprotect. No
such request arrived. A 24-byte command 6 with operation 2 arrived and
was left unanswered; nothing was written. Twelve SIM reads, about 15
seconds apart, were each length 143, card PRESENT, one SIM/USIM
application, app state 2 (PIN), pin1 3. None became READY.

`DoRadioPower` stores zeros in packet bytes 16 and 17 when the request
has fewer than three integers. After the hold, one power-off (word 1)
and one power-on (word 2) were sent with those bytes zero. Both replies
were length 13, error 0. The following SIM read was unchanged: app state
2, pin1 3. `rmnet0` rx and tx stayed 0, with no IPv4 address. The modem
stayed ONLINE. No PIN command was sent.

## 2026-09-30: channels held from the first ONLINE moment

The phone was rebooted with sysrq. CPIF modules were reloaded, the
reviewed B image hash matched, and persist was mounted read-only only
long enough for the 64-byte signature, then unmounted. The probe opens
`/dev/umts_ipc0` and `/dev/umts_rfs0` before FIN and, after COMPLETE,
keeps those same descriptors open. It answered the 12-byte protected-NV
unprotect, sent nothing for OP_STATUS, and left the operation-2 write
unanswered.

The first SIM read, still in the first seconds of ONLINE, was length 80,
card ABSENT, no applications. Every later read for five minutes was
length 143, card PRESENT, one SIM/USIM application, app state 2 (PIN),
pin1 3. The hold ended `app_state=2 pin1=3 ready=0`. Boot result was 0
and the modem stayed ONLINE. LTE selection, registration and `rmnet0`
were not exercised: the application never became READY. `rmnet0` rx and
tx were 0, with no IPv4 address. No PIN command was sent. Wi-Fi came
back on Vabofabaka at 192.168.0.104.

## 2026-09-30: PIN1 with AID still not supported

`BuildSimVerifyPin` writes an AID only when its hex-string argument is
non-null. The byte count goes at packet offset 21 and at most 16 bytes
follow at offset 22. A null argument leaves those bytes zero.
`DoVerifyPin` is the PIN1 caller. It passes a null AID when the request
has one string, and the second string when the request has two. It does
not call `GetAID`. `GetAID` reads the length at adapter+34 (status byte
18) and the bytes at adapter+35 (status byte 19) only while building the
card-status response.

The live status had a non-zero AID length, so two PIN1 packets were sent
in the two-string factory shape, with that AID copied into the request
and not printed. Each response was length 16, error 6, remain 3. The
application stayed state 2, pin1 3, remain 3. No further PIN command
was sent. Registration was not started. `rmnet0` rx and tx were 0.

## 2026-09-30: no APDU PIN path; voice registration stays 0

`BuildSimTransmitApduBasic` is id `0x020c`. It copies six caller integers
and a hex data string. `TransmitSimApduBasicHandler` passes the request
fields through. The only rewrite is that instruction `0xB0` drops the
data pointer. There is no factory VERIFY CHV template, so no APDU PIN
was sent. `IoChannel::Init` only tests a flag byte and sends nothing.
`BuildSimOpenChannel` is a separate logical-channel command, not a step
that enables `0x0201`. That verify command was not sent again.

Voice registration is `BuildNetworkRegistrationState(1)`: id `0x0700`,
length 12, no payload. The live response was length 88, error 0,
registration byte 12 = 0, reject byte 13 = 0, technology byte 14 = 0.
The empty available-network query and the 16-byte form with word 0
(the value `DoQueryAvailableNetwork` uses when the request has no type)
both returned length 12, error 2. No network names were printed.
`rmnet0` rx and tx stayed 0.

## 2026-09-30: verify packet already matches the factory builder

`InitRequestHeader` writes type 0, the command id at offset 2, the length
as a 16-bit value at offset 4, and a 32-bit token at offset 6.
`BuildSimVerifyPin` always uses length 38. Argument 0 selects id `0x0201`
and argument 1 selects id `0x0203`. Byte 12 is the PIN character count,
capped at 8, and the characters start at byte 13. Byte 21 is the AID
length and the AID bytes start at byte 22, at most 16. A null AID leaves
bytes 21 through 37 zero. The length does not change when an AID is
present. The diagnostic builds that same layout. No corrected verify was
sent.

`ConvertProtocolErrorCodeToRilErrorCode` maps protocol values 0 through
17 onto the same RIL numbers. Protocol 2 is `RCM_E_GENERIC_FAILURE`.
Protocol 6 remains `RCM_E_REQUEST_NOT_SUPPORTED`.

Under `/sys/devices/platform/cpif` the SIM directory contains only
`ds_detect`, which reads 2. `modem_state` is ONLINE. There is no slot
node. `rmnet0` rx and tx are 0.

## 2026-09-30: no slot field in the SIT header

`InitRequestHeader` has two forms. Both zero the first 12 bytes, store
type 0, the id at offset 2 and the length at offset 4. One stores the
next `TokenGen` value at offset 6; the other stores a caller token
there. `ProtocolBaseAdapter` reads type at byte 0, id at offset 2,
length at offset 4, and the token at offset 6 when the type is 0 or 1.
The payload starts at offset 12. Nothing reads or writes a slot,
socket, or phone id.

`BuildSimGetStatus` is id `0x0200`, length 12, and no payload.
`BuildSimVerifyPin` is id `0x0201`, length 38, with the PIN and AID
after byte 12. Both call the same header function. `DoVerifyPin` passes
pin type 0, the PIN string, and the optional AID. `GetRilSocketId`
reads a word on the RIL context; it does not enter the packet. No
corrected verify was sent.

`BuildSimGetFacilityLock` is id `0x0209`, length 71. Byte 12 is an index
into the facility table, and `SC` is index 3. A null password pointer
makes the builder return null; a zero length copies no characters.
Byte 53 is the low byte of the word at `FacilityLock` offset 20.
`FacilityLock::Parse` is not in these libraries, so that byte is not
known and the query was not sent.

A status read is still length 143, card PRESENT, one SIM/USIM
application, app state 2, pin1 3, remain 3. The modem is ONLINE.
`rmnet0` rx and tx are 0.

## 2026-09-30: blocker, then one card-power UP

Proven and not repeated: pin1 value 3 is the disabled pin field, while
the application state stays PIN. Verify PIN `0x0201` matches
`BuildSimVerifyPin` and still returns `RCM_E_REQUEST_NOT_SUPPORTED`.
`ds_detect=2` is the dual-SIM slot count. Holding the channels from
the first ONLINE moment did not leave that application state.

`BuildSetSimCardPower` is id `0x024c`, length 13, with the state in
byte 12. On HAL 22 and newer a request value of 0 is rewritten to 4
before that byte is stored. Value 1 is stored unchanged. One request
with byte 12 set to 1 returned length 12, error 2
(`RCM_E_GENERIC_FAILURE`). After four seconds the card was still
PRESENT, the application still PIN, and pin1 still 3. The card did not
become ABSENT, so the pass-through value was not sent. `rmnet0` rx and
tx are 0.

## Current blocker

The modem is ONLINE. The SIM card is PRESENT. The application state is
PIN. pin1 is 3, the AOSP disabled value. Verify PIN `0x0201` matches
`BuildSimVerifyPin` and returns `RCM_E_REQUEST_NOT_SUPPORTED`. The SIT
header has no slot field. There is no factory APDU VERIFY CHV template.
Holding IPC and RFS from the first ONLINE moment did not change the
application. A radio power OFF then ON did not change it. Card power
with byte 12 set to 1 returned `RCM_E_GENERIC_FAILURE` and left the
card PRESENT. `ds_detect=2` is only the dual-SIM slot count.
`LTE_ONLY` (value 11) was set and read back on the boot before the
early-hold reboot. Voice and data registration read 0. Available
networks return error 2, `RCM_E_GENERIC_FAILURE`. `rmnet0` rx and tx
are 0.

Commands the CP accepted with error 0: SIM status `0x0200`, radio state
`0x0801`, radio power `0x0800` off and on, voice registration `0x0700`,
data registration `0x0701`, operator `0x0702`, preferred-network get
`0x070b`, preferred-network set `0x070a` to 11, and network-selection
auto `0x0704`. Not accepted: verify PIN `0x0201` (error 6), available
networks `0x0706` (error 2), card power `0x024c` with state 1 (error 2).

`BuildSimGetFacilityLock` is id `0x0209`, length 71. Facility `SC` is
table index 3, stored at byte 12. An empty password stores length 0 and
no characters. Byte 53 is the service class, copied from
`FacilityLock` offset 20. `FacilityLock::Parse` is not in these
libraries. The only `serviceClass` string is a call-forwarding debug
print; it is not a constant 7 or 255. That byte is still unknown, so
the query was not sent.

Live read after that decision: SIM length 143, error 0, card PRESENT,
one SIM/USIM application, app state PIN, pin1 3, remain 3. Data
registration length 86, error 0, registration 0, reject 0, technology
none. `rmnet0` rx 0, tx 0.

## 2026-09-30: stock sequence, then three GETs

`NetworkService::OnModemOnline` does not start a GET loop. If a flag
is not 1 it posts internal message 10109, then calls
`SetApSystemTime` and `SendDeviceInfo`. Those last two are sets.
`MiscService::OnRadioAvailable` runs only for socket 0 and calls
`SetDebugTraceOffOnBoot`, `SetModemsConfig`, `SetSlotMapping`,
`SendSGCValue`, and `SendSvnInfo`, also sets. `DeviceInfoHandler`
calls `requestSetDeviceInfo` once. None of those sets were sent.

The GET handlers used when the framework asks are separate.
`GetSignalStrength` is id `0x0900`, length 12, empty. One send returned
length 210, error 0; the measurement bytes were not printed.
`GetBaseBandVersion` is id `0x0901`, length 13, and the handler stores
`0xFF` at byte 12. One send returned length 333, error 0; the version
text was not printed. `GetTtyMode` is id `0x0904`, length 12. One send
returned length 16, error 0, mode word 0. Voice radio technology is not
its own command: `GetRadioTech` reads the voice-registration response,
which was already 0.

After those reads the SIM was still length 143, PRESENT, app state PIN,
pin1 3, remain 3. Data registration was 0, reject 0, technology none.
`rmnet0` rx and tx are 0.

## 2026-09-30: post-online SETs were not sent

`BuildSimSetLogicalSlotMapping` is id `0x0250`, length 17. Byte 12 is
the slot count, which must be 1 through 4. The following bytes are the
low bytes of the slot integers. `SetSlotMapping` fills those integers
from `persist.radio.slotmap.config`. That property is empty, and the
empty path returns without a packet. No slot SET was sent.

`BuildSetModemsConfig` is id `0x093f`, length 13. Byte 12 is 0 when the
argument is 1 and 1 otherwise. A phone count above 1 uses argument 2,
so the byte would be 1, but only when
`persist.vendor.radio.multisim_switch_support` is `true`. The property
is empty, and argument 2 then skips the builder. Not sent.

`SendSGCValue` is id `0x0404`, length 24. Three words follow the
header, and the first is passed through `MappingSGCValue`. The caller
reads the numbers from a property, so they are not a fixed default.
Not sent. Debug trace is sent only for a property value of 1 or 99.
SVN is a string payload and was not sent.

Live read with no new SET: SIM length 143, error 0, card PRESENT, one
SIM/USIM application, app state PIN, pin1 3, remain 3. Data
registration length 86, error 0, registration 0, reject 0, technology
none. `rmnet0` rx 0, tx 0.

## 2026-09-30: stock properties, one modem-config SET

Read-only mounts, then unmapped. `vendor_b` `build.prop` contains
`persist.vendor.radio.multisim_switch_support=true`. It does not set
`persist.radio.slotmap.config` or `vendor.ril.app.target_carrier`.
`product_b` `etc/build.prop` contains
`persist.radio.multisim.config=dsds`. Its `etc` tree also has no
slot-map string and no target-carrier string. Both filesystems were
unmounted and the device-mapper node removed. No vendor file was copied.

`dsds` makes `GetPhoneCount` return 2, so `SetModemsConfig` passes
argument 2. With the switch property equal to `true`, the builder is
used. The packet is id `0x093f`, length 13, byte 12 = 1. One send
returned length 12, error 0.

Slot mapping stays unsent: an empty `persist.radio.slotmap.config`
returns before `0x0250`. SGC stays unsent. `SGCHandler` would call
`SendSGCValue` with the target-carrier integer, then 0 and 0, and the
first word is `MappingSGCValue` of that integer. The stock images do
not define `vendor.ril.app.target_carrier`.

After the modem-config SET the SIM was still length 143, error 0, card
raw 1, one SIM/USIM application, app state PIN, pin1 3, remain 3. Data
registration was 0, reject 0, technology 0. `rmnet0` rx 0, tx 0.

## 2026-09-30: VerifyPin after modem config still not supported

The modem was still ONLINE and `0x093f` had already been accepted on
this boot, so it was not sent again. One `0x0201` VerifyPin followed,
factory length 38, PIN1, the first candidate only, with the AID copied
from the SIM status and not printed. The response was length 16, error
6, remain 3. The following SIM read was still length 143, PRESENT, one
SIM/USIM application, app state PIN, pin1 3, remain 3. The second
candidate was not sent.

`BuildSetApSystemTime` is id `0x0949`, length 18. Bytes 12 through 17
are the low bytes of `tm_year`, `tm_mon`, `tm_mday`, `tm_hour`,
`tm_min`, and `tm_sec` from `localtime`. One send returned length 13,
error 0.

`SendDeviceInfo` is id `0x0922`, length 140. Four fields of 32 bytes
start at offsets 12, 44, 76, and 108, each a copy of at most 31 bytes.
The factory strings are `ro.product.model`, `ro.build.id`,
`ro.product.name`, and `ro.build.version.release`. All four were empty
on this system, so the fields were empty. One send returned length 55,
error 0. The field text was not printed.

Data registration stayed 0, reject 0, technology 0. `rmnet0` rx 0, tx 0.

## 2026-09-30: basic VERIFY CHV returns status 6984

`BuildSimTransmitApduBasic` is id `0x020c`. The session word is at
offset 12. The halfword at offset 16 is 5 plus the decoded data
length. Bytes 18 through 22 are CLA, INS, P1, P2, and P3. Decoded data
starts at offset 23. The header length is 23 plus the hex-string
length, so eight data bytes make a 39-byte packet. The channel builder
is a different command, id `0x020f`. No factory caller builds a PIN
APDU; the basic handler is the generic path, and its log names the
fields session, CLA, instruction, P1, P2, P3.

One basic packet used session 0, CLA `00`, INS `20`, P1 `00`, P2 `01`,
P3 `08`, and eight data bytes (digits left-justified, remaining bytes
`FF`). The data bytes were not printed. `0x0201` was not sent. The
response was length 16, error 0, status `6984`. That status is
reference data not usable, so the second candidate was not sent.

The SIM read stayed length 143, PRESENT, one SIM/USIM application,
app state PIN, pin1 3, remain 3. Data registration was 0, reject 0,
technology 0. `rmnet0` rx 0, tx 0.

## 2026-09-30: SELECT ADF succeeded, VERIFY still 6984

The same basic builder `0x020c`, session word 0, carried one SELECT:
CLA `00`, INS `A4`, P1 `04`, P2 `00`, Lc and data taken from the SIM
status AID. The AID was not printed. The response was length 86, error
0, status `9000`. Logical channel `0x020f` was not sent; its field
layout is not fully proven.

VERIFY with P2 `01` and the first candidate then returned length 16,
error 0, status `6984`. The SIM was unchanged: PIN, pin1 3, remain 3.
One further VERIFY used P2 `00` and returned length 16, error 0, status
`6b00`. That is not a wrong-PIN status, and the attempt count did not
drop, so the second candidate was not sent. `0x0201` was not sent.

Data registration stayed 0, reject 0, technology 0. `rmnet0` rx 0, tx 0.

## 2026-09-30: disabled PIN has no READY command

`CheckAndAutoVerifyPin` sends a PIN command only when the pin word is 1.
Pin word 3 does not start that path, and `OnSimStatusChanged` treats the
application as ready only when the state is already 5. No other caller
turns application state 2 into 5 because the pin field is disabled.
Facility lock `0x0209` is still not sent: its service-class byte is not
a proven constant. VERIFY CHV and `0x0201` were not repeated.

`BuildGetSimLockInfo` is an empty GET, id `0x4104`, length 12. One send
returned length 12, protocol error 22. That code maps to RIL error 6,
request not supported. The lock-code bytes were not present and were not
printed. The following SIM read was still length 143, PRESENT, one
SIM/USIM application, app state PIN, pin1 3, remain 3. Data registration
was 0, reject 0, technology 0. `rmnet0` rx 0, tx 0.

## 2026-09-30: facility service class is a request string

`FacilityLock::Parse` is in `vendor.radio.base.so`. The constructor
writes 0 at offset 20. Parse returns failure unless the request has
exactly 4 or 5 strings, and the handler sends `0x0209` only after a
successful parse. With 4 strings, string 2 is converted with `strtol`
base 10 and stored at offset 20. With 5 strings, string 1 is stored at
offset 4 and string 3 is the value stored at offset 20. Packet byte 53
is the low byte of that word. The function contains no constant 7 or
255. The vendor partition was unmounted after the read, and the library
was not kept.

`BuildRilCardStatusApplications` stores the pin word unchanged. It does
not turn application state 2 into 5 when that word is 3.
`DoAutoDisableUicc` would send `0x0249`, length 13, byte 12 set to 0.
`SetUiccHandler` does not send that packet; it completes the request
with error 6. Byte 0 is the disable value, so it was not sent.

No packet was sent. The unproven field is still byte 53 of `0x0209`.
The next single candidate is that GET, and only after the framework's
service-class integer for facility SC is known.

## 2026-09-30: facility SC query uses service class 7

Stock `telephony-common.jar` method
`UiccCardApplication.queryPin1State` calls
`CommandsInterface.queryFacilityLockForApp` with facility string `SC`,
an empty password string, service class `const/4 7`, and the application
AID. The SET path `setIccLockEnabled` also uses service class 7 for
`SC`. That integer is the proven value for packet byte 53.

One GET `0x0209`, length 71, facility index 3, empty password, byte 53
equal to 7, AID copied and not printed, returned length 14, error 0,
status byte 13 equal to 0 (facility unlocked). The following SIM read
was still length 143, PRESENT, one SIM/USIM application, app state PIN,
pin1 3, remain 3. Data registration was 0, reject 0, technology 0.
`rmnet0` rx 0, tx 0.

Factory `GetFacilityLockHandler::OnResponse` returns that status integer
to the framework. `UiccCardApplication.onQueryFacilityLock` updates the
lock-enabled flag only. It does not rewrite application state PIN into
READY. A successful unlocked SC query therefore does not clear this
incongruent status.

Next single candidate not yet sent this boot: empty
`BuildSimGetATR`, id `0x0212`, length 12. No SET and no PIN material.

## 2026-09-30: GetATR and open channel leave app PIN

`BuildSimGetATR` is an empty GET, id `0x0212`, length 12. One send
returned length 47, error 0, result byte 12 equal to 1, ATR length 23.
ATR bytes were not printed. `GetAtrHandler` only completes the RIL
request with those bytes; no caller turns application state PIN into
READY.

The following SIM read was still PIN, pin1 3, remain 3. The next proven
probe was `BuildSimOpenChannel`, id `0x020d`, length 29: AID length at
byte 12, AID bytes at offset 13. Response layout: session word at offset
12, SW1 at 16, SW2 at 17. One send with the status AID (not printed)
returned length 276, error 0, session 1, SW `9000`. The next SIM read
was still PIN, pin1 3, remain 3. Data registration 0, reject 0,
technology 0. `rmnet0` rx 0, tx 0.

Neither ATR nor open-channel rewrites the incongruent CP status. Next
single candidate: empty `BuildGetVoiceOperation`, id `0x091b`, length 12
(or preferred-network GET `0x070b` on this boot). Neither is expected to
clear app state PIN.

## 2026-09-30: CP reports PIN with pin1 DISABLED; no HAL fixup

Offsets re-checked against factory. `ProtocolSimStatusAdapter` copies the
packet at adapter+16. `BuildRilCardStatusApplications` reads application
type from adapter+31 (packet byte 15) and state from adapter+33 (packet
byte 17). `GetPinState` PIN1 uses adapter offset 0x58 (packet byte 72);
PIN2 uses 0x59 (byte 73). `GetPinRemainCount` PIN1 uses 0x5a (byte 74).
`covertAppStateToString` names 2 PIN and 5 READY. AOSP pin value 3 is
DISABLED. Live audit: card 1, apps 1, type 2, state 2, pin1 3, pin2 1,
remain1 3. The diagnostic labels match the factory parser. This is not a
misread of READY.

Stock order after modem online for this path: solicited GetSimStatus
`0x0200`; `OnGetSimStatusDone` builds RIL card status; `CheckAndAutoVerifyPin`
sends only when pin1 is 1, so DISABLED skips verify; `CheckDisabledIccid`
is property/ICCID only and sends no SIT; `DoAutoDisableUicc` (`0x0249`
byte 12 = 0) runs only when that check matches and is UICC-off, not a
READY path. Framework then may `queryFacilityLockForApp` SC with class 7
(`0x0209`), which we already sent and got status 0. No factory SIT command
rewrites app state PIN into READY when pin1 is 3.

The only fully proven SIT we had not sent after the prior open channel
was `BuildSimCloseChannel` `0x020e`, length 16, session word at offset 12.
One close of session 1 returned length 12, error 0. The following SIM
read was unchanged: state PIN, pin1 3. Registration 0, `rmnet0` 0/0.

Blocker: the CP/CPIF status packet itself reports application PIN while
pin1 is DISABLED. Radio HAL has no fixup. Next single hypothesis with a
proven opcode: empty `BuildSimGetSlotStatus` `0x024d` (report length and
error only; do not print ICCID bytes).

## 2026-09-30: slot status + attach despite PIN

Empty `BuildSimGetSlotStatus` `0x024d` length 12 returned length 433,
error 0. Payload not printed (ICCID/ATR). SIM after that was still
card PRESENT, one SIM/USIM application, app state PIN, pin1 3, remain 3.

Factory SIM/auth builders besides `BuildSimVerifyPin` `0x0201` (pin
index 0) / `0x0203` (pin index 1): `BuildSimVerifyPuk` `0x0202`/`0x0204`,
`BuildSimChangePin`, `BuildSimVerifyNetworkLock` `0x0207`,
`BuildSimGetFacilityLock` / `BuildSimSetFacilityLock` (facility SC
already queried), `BuildSimGetIsimAuth` `0x020b`, `BuildSimGetSimAuth`,
`BuildSimGetGbaAuth` `0x020f`/`0x0211`, `BuildSimVerifyEncryptedPin`,
`BuildSimOpenChannelWithP2` `0x0247`, `BuildSimTransmitApduChannel`.
None of those is an empty GET that can clear PIN without secrets or
APDU/AID inventing. The only empty slot GET left was `0x024d`; it did
not change app state.

Critical attach-despite-PIN: preferred GET returned raw 16, one
`0x070a` set LTE_ONLY, `0x0704` auto selection length 12 error 0, then
four `0x0701` data-registration polls over ~45s. All stayed
`registration_raw=0 reject_cause_raw=0 radio_tech_raw=0`. `rmnet0`
rx/tx remained 0/0. SIM after the path was still PIN, pin1 3. READY is
not only a HAL gate: with this bring-up sequence the CP does not attach
or move rmnet while it still reports app_state PIN.

Blocker unchanged: CP reports PIN with pin1 DISABLED; LTE/auto/reg path
does not produce registration or bearer. Next hypothesis needs a proven
non-secret path that changes CP app state (not more peripheral GETs).

## 2026-09-30: 0x0203 is PIN2; card DOWN/UP cycle ok, still PIN

`BuildSimVerifyPin(arg, pin, aid)`: arg 0 → id `0x0201` (PIN1), arg 1 →
id `0x0203` (PIN2). Length 38; byte 12 = character count; bytes 13.. are
the PIN string (max 8); AID optional at 21+. The builder rejects a null
PIN pointer. `DoVerifyPin` rejects null/empty and non-digit strings and
requires length 1..8 before calling the builder. Stock never sends empty
PIN and never calls verify when `GetPinState` is 3 (DISABLED);
`CheckAndAutoVerifyPin` only continues for pin state 1.
`DoVerifyPin2` / SIM I/O / FD facility use arg 1 → `0x0203`. That is not
a PIN1 unlock for app_state PIN. No empty or DISABLED path exists, so
`0x0203` was not sent.

SIM/UICC power (not CP IOCTL): `BuildSetSimCardPower` `0x024c` length 13,
state at byte 12. HAL >= 0x16 rewrites request 0 to builder value 4
(POWER_DOWN); 1 is POWER_UP; 2 is PASS_THROUGH. Earlier UP-only (state 1)
returned error 2. One paired cycle this turn: state 4 then state 1. Both
returned length 12, error 0. Unsolicited traffic included id 590 during
the cycle. GetSimStatus before and after: applications=1, app0 type
SIM/USIM, app0 state PIN, pin1 3, remain 3. No second application to
select. Registration 0, `rmnet0` 0/0.

STK refresh is indication-only; no local refresh GET. `BuildSetUicc`
`0x0249` enable is still untried (disable previously banned).

Blocker: CP still reports PIN+DISABLED after a successful card power
cycle. Next single proven hypothesis: `BuildRadioPower` `0x0800` length 18
OFF then ON (radio SIT, not CP IOCTL_POWER_OFF), then GetSimStatus.

## 2026-09-30: RadioPower OFF→ON; empty 0x0201 now err 2 not 6

`BuildRadioPower` id `0x0800`, length 18. `DoRadioPower` maps request
OFF (int≤0) → builder arg0=0 → word at offset 12 = 1; ON (int>0) →
arg0=1 → word = 2. Bytes 16 and 17 are optional flag bytes (0 here).
One OFF then ON: both length 13, error 0.

GetSimStatus after ON: still applications=1, app0 PIN, pin1 3, remain 3.
Not READY. One empty PIN1 `0x0201` (length 38, count byte 12 = 0, no
digit bytes, no AID) returned length 16, `error_raw=2`
(`RCM_E_GENERIC_FAILURE`), not 6. Prior digit verifies on this card were
error 6 (`RCM_E_REQUEST_NOT_SUPPORTED`). So after radio ON the opcode is
no longer rejected as unsupported; empty payload is a generic failure.
`DoVerifyPin` never sends empty (requires 1..8 digits); HAL has no
separate capability bit that gates VerifyPin before the CP response.
Data registration after this: `registration_raw=0`, `reject_cause_raw=0`,
`radio_tech_raw=3` (tech nonzero while still not registered). `rmnet0`
0/0.

Blocker: app_state still PIN with pin1 DISABLED; radio cycle did not
make READY. Next single proven hypothesis: one `0x0701`/`0x0704` attach
poll series while `radio_tech_raw` stays nonzero (tech 3 just appeared),
without further PIN commands — confirm whether CP can register despite
PIN now that radio was cycled.

## 2026-09-30: attach poll with tech=3; candidate A still err 6

Preferred already `11` (LTE_ONLY). One `0x0704` auto returned length 12,
error 2. Five `0x0701` samples over ~48s: each `registration_raw=0`,
`reject_cause_raw=0`, `radio_tech_raw=3`, `rmnet0` 0/0. SIM stayed PIN,
pin1 3, remain 3. CP does not register while app_state is PIN even with
nonzero tech after radio ON.

One PIN1 verify candidate A (`0x0201`, stock strlen count, chars at 13,
null AID, trailing zeros in the 8-byte field): length 16, `error_raw=6`,
`remain_raw=3`. SIM remain before and after = 3. Not wrong-PIN (code 3);
attempts unchanged. Digit spray stopped; candidate B not sent. Empty
verify earlier was error 2; nonempty A is again `NOT_SUPPORTED`.

Blocker: PIN+DISABLED; tech 3 without registration; nonempty `0x0201`
still unsupported. Next single proven hypothesis: decode why empty
`0x0201` returns 2 while nonempty returns 6 (CP length/count check) —
if stock always sends strlen≥1, look for a non-digit factory unlock
path that the CP accepts (not more PIN digits).

## 2026-09-30: VerifyPin layout matches; selection already auto

Byte compare of our PIN1 packet vs `BuildSimVerifyPin(0, pin, null)`:
id `0x0201`, length 38, count at byte 12 = strlen (≤8), characters at
13.., AID length at 21 and bytes at 22 only when AID hex is non-null
(else 21..37 zero). No slot or app index field exists in the builder.
Candidate A uses count 4 and four digit bytes — identical to factory
for a 4-digit PIN with null AID. Digits were not resent.

`sit-base` names: error 2 `RCM_E_GENERIC_FAILURE`, 3
`RCM_E_PASSWORD_INCORRECT`, 6 `RCM_E_REQUEST_NOT_SUPPORTED`. Empty
count→2 is a failed/invalid verify body; nonempty with pin1 DISABLED→6
is CP refusing VerifyPin (not a wrong-PIN consume). Remain stays 3.

`radio_tech_raw=3`: `toRadioTech` passes 1..16 through; AOSP
`RADIO_TECH_UMTS` is 3, so this is UMTS while still unregistered.
`0x0704` err 2 after radio cycle: `BuildQueryNetworkSelectionMode`
`0x0703` now returns `selection_mode_raw=0` (auto). Set was skipped.
Redundant auto while already auto explains GENERIC_FAILURE. Radio
state GET is 10 (ON). SIM still PIN/pin1=3; reg 0; tech 3; rmnet 0/0.

VerifyPin is dead for DISABLED pin1. Next single non-PIN hypothesis:
`BuildSetUicc` `0x0249` with enable byte 12 = 1 (EnableUiccApp path;
disable=0 remains banned), then GetSimStatus.

## 2026-09-30: UICC enable=1 accepted; still PIN

`BuildSetUicc` id `0x0249`, length 13, state at byte 12. One enable=1
send returned length 12, error 0. `EnableUiccAppHandler` gates on a
proxy check returning 1, stores the int at SimService+1744, then calls
`BuildSetUicc(enable)`. It does not rewrite PIN/app_state; it only
forwards the RIL enable-UICC-applications flag to the CP.
`SetUiccHandler` still local-completes with code 6 and does not send.

GetSimStatus before and after: still PIN, pin1 3, remain 3. Radio
already 10 (ON). Preferred already 11 (LTE_ONLY). Four `0x0701` samples
~36s: reg 0, tech 3 (UMTS), `rmnet0` 0/0.

Blocker unchanged: CP app_state PIN with pin1 DISABLED. Next single
proven non-PIN hypothesis: `0x070a` set preferred SIT type 12 (factory
table index 12 → value 12, AOSP `PREF_NET_TYPE_LTE_WCDMA`) — LTE_ONLY=11
with reported UMTS tech may be attach-dead; one set then reg poll.

## 2026-09-30: PIN incongruence audit — locks/slot/HAL; preferred=12 secondary

Bearer blocker (unchanged, explicit): CP `GetSimStatus` reports
`app_state=PIN` (2) while `pin1=DISABLED` (3). Attach/reg/`rmnet` stay
dead in that state. HAL does not rewrite PIN→READY when pin1 is 3.
VerifyPin is dead for DISABLED. This incongruence is the cellular bearer
gate, not preferred-mode tuning.

(1) Lock GETs (no secrets):
- `0x0207` VerifyNetworkLock is password SET-shaped — not an empty GET;
  skipped.
- Empty `GetFrequencyLock` `0x073a` length 12 → length 34, error 0.
  First payload words `0xFFFF0B01` / `0xFFFF0000` (sentinel / not an
  active frequency lock).
- Facility PN `0x0209` index 17, class 7 → length 14, error 0,
  `lock_status_raw=0` (unlocked), same shape as prior SC unlock.
- Prior `GetSimLockInfo` `0x4104` remains dead (err 22). No network/SIM
  lock enum explains app_state PIN.

(2) `GetSlotStatus` `0x024d` non-secret parse (factory Init type1
len≥0x1b1: `num_slots@12`; modern fill from pkt+0xd, stride 0x69:
card@+0, atr_len@+1; port count @record+52; no app_state field;
`GetSlotState` returns 0 when not legacy):
length 433, error 0, `num_slots=2`, `slot0_card_state_raw=1` PRESENT,
`slot0_atr_len=27`, `slot0_port_count_raw=2`. Matches GetSimStatus card
PRESENT and one SIM/USIM app still PIN. No clue that slot/card ≠ app0
as the PIN cause (dual-slot modem, slot0 populated, ATR present).

(3) Factory HAL writers of app_state / READY vs PIN when pin1==DISABLED:
`BuildRilCardStatusApplications` / `FillRilCardStatusFromAdapter` copy CP
app_state and pin words unchanged. `CheckAndAutoVerifyPin` continues only
when `GetPinState`==1; pin1==3 skips verify. `OnSimStatusChanged` takes
READY only when state is already 5. `CheckDisabledIccid` / `DoAutoDisableUicc`
are not READY paths. Stock cold-boot after ONLINE solicits GetSimStatus
and does not send a SIT that clears PIN+DISABLED. We are not missing a
HAL rewrite — the CP packet itself is incongruent.

(4) SECONDARY only (after 1–3 yielded nothing actionable): one `0x070a`
preferred=12 (LTE_WCDMA) accepted (GET confirms raw 12). Three short
`0x0701` polls: still `reg=0`, `tech=3` UMTS, `rmnet` 0/0. SIM still
PIN, pin1 3. Preferred tweak is not the path.

Next non-PIN hypothesis: dual-slot / port mapping — `num_slots=2` with
port_count=2 while GetSimStatus shows one app; look for a proven empty
modem-config or slot-mapping GET/SET stock sends on cold boot that we
skip (no invented opcodes; no secrets).

## 2026-09-30: dual-slot/port — mapping SET not sendable; capability GET done

Factory multi-slot / port mapping:
- `BuildSimSetLogicalSlotMapping` id `0x0250`, length 17. Byte 12 =
  count (1..4); bytes 13+ = low bytes of physical-slot ints.
- `BuildSimSetLogicalSlotPortMapping` same id `0x0250`, length 21.
  Byte 12 = count; then (physical, port) byte pairs. Used when request
  meta > 0x1f (HAL SlotPortMapping path).
- `MiscService::SetSlotMapping` reads `persist.radio.slotmap.config`;
  empty → log and return, **no packet**. Stock images leave that
  property empty (already confirmed). `0x093f` DSDS modems-config was
  already sent earlier.
- `BuildSetPreferredDataModem` id `0x0740`, length 13, modem byte at
  offset 20; index comes from the RIL request (must be < 3). No fixed
  default in the handler.
- Signal-strength reporting criteria builders exist; not slot mapping.

First proven mapping/switch we never sent: **none fully proven** —
`0x0250` needs the slot (or slot+port) integer array. Missing field:
those integers (property string or framework `setSimSlotMapping`
payload). Not invented; not sprayed.

One proven never-sent radio-config GET (sibling of slot status):
`BuildGetPhoneCapability` id `0x0615`, length 12. Live: length 16,
error 0, `max_active_data=2`, `max_active_internet=1`, `lingering=1`,
`logical_modem_list_size=2`.

Fresh `0x024d` non-secret port parse (modern fill: ports at record+0x40,
stride 0xd; logical@+0, state@+1; 0xFF logical → unassigned):
`num_slots=2`.
- slot0: card PRESENT, atr_len 27, ports 2; p0 logical=1 state=1;
  p1 logical=255 state=0.
- slot1: card PRESENT, atr_len 23, ports 1; p0 logical=0 state=1.
Crossed map: logical0→phys slot1, logical1→phys slot0. Both slots have
cards. GetSimStatus still one USIM app, PIN + pin1 DISABLED. reg 0,
tech 3, `rmnet` 0/0. No READY → no bearer chase.

Blocker unchanged: PIN+DISABLED. Mapping SET still blocked on missing
integers. Next hypothesis: prove which logical modem `0x0200` binds to
(or how stock queries the other stack) given crossed slot1↔slot0 map —
still no invented `0x0250` payload.

## 2026-09-30: 0x0200 binds by IPC channel; ipc1 has no apps

Factory binding (re-checked):
- `InitRequestHeader` / `BuildSimGetStatus` (`0x0200`, len 12): no
  phone/slot/subscription field in the SIT header or payload.
- Dual stacks are separate `IoChannel` paths: `/dev/umts_ipc0` and
  `/dev/umts_ipc1` (both strings in `libsitril`; live sysfs `493:0` /
  `493:1`). `GetRilSocketId` is a RIL-context word only — never packed.
- Our probes had always opened `umts_ipc0` only → that channel's stack.
- `0x0250` is still **not** built from GetSlotStatus ports:
  `SetSlotMapping` needs `persist.radio.slotmap.config`;
  `SimSlotMappingHandler` needs framework ints. No send.

Live: mknod `umts_ipc1` from sysfs, shared `umts_rfs0`.
- ipc0 `0x0200`: length 143, card PRESENT, apps=1, app_state=PIN,
  pin1=DISABLED(3), remain=3. Same incongruence as before.
- ipc1 `0x0200`: length 80, card PRESENT, **apps=0**, no pin fields.
  Not READY. reg on ipc1: 0 / reject 0 / tech 0. `rmnet` 0/0.

COMPARE: ipc0 PIN+DISABLED vs ipc1 empty apps. Neither READY → no
bearer chase; `0x0740` still skipped (modem index unproven).

Under the prior cross-map (logical0→phys1, logical1→phys0), ipc0≈logical0
carries the only USIM app (PIN). ipc1≈logical1 sees a card but no apps.
Next hypothesis: why ipc1 apps=0 (inactive port / missing subscription
bring-up) — still without inventing `0x0250` ints.

## 2026-09-30: ipc1 RadioPower ON; apps still 0

Factory cold-boot SETs for the second RIL instance (same opcodes, other
`IoChannel` — no slot field):
- `BuildRadioPower` `0x0800` len 18, ON → word@12=2 (DoRadioPower).
- `BuildSetSimCardPower` `0x024c` len 13, UP byte@12=1.
- `BuildSetUicc` `0x0249` len 13, enable=1 (EnableUiccApp path).
- `BuildAllowData` exists; needs framework allow flag — not a fixed empty
  default. `UiccSubscription` is a RIL request creator only; no standalone
  empty SIT builder in sit-stream. `0x0250` still not derived from ports.

ONE bring-up on ipc1 only: RadioPower ON. Response length 13, error 0.

Re-`0x0200` after:
- ipc0: PRESENT, apps=1, PIN, pin1 DISABLED — unchanged.
- ipc1: PRESENT, apps=0 — unchanged. Neither READY. ipc1 reg 0/tech 0;
  `rmnet` 0/0. No `0x0740`.

Life SIM without ICCID: only ipc0 reports a USIM app; cross-map
logical0→phys1 ⇒ that app-bearing SIM is on **physical slot 1**.
(slot0→logical1 has card PRESENT but ipc1 apps=0.)

Blocker unchanged on ipc0. ipc1 RadioPower alone does not populate apps.
Next hypothesis: one CardPower UP `0x024c` state=1 on ipc1 only (proven
layout; not yet sent on that channel).

## 2026-09-30: ipc1 CardPower UP err 2; EnableUicc=1 still apps=0

ONE CardPower UP on ipc1 (`0x024c` state=1, no DOWN): length 12,
`error_raw=2` (GENERIC_FAILURE — likely already powered / no-op).
Re-`0x0200`: ipc0 still PIN+DISABLED apps=1; ipc1 PRESENT apps=0.

Same turn, still no READY and apps=0 → ONE EnableUicc `0x0249`=1 on
ipc1: length 12, error 0. Re-`0x0200` unchanged: ipc1 apps=0. Neither
READY. ipc1 reg 0/tech 0; `rmnet` 0/0.

ipc1 bring-up on this channel exhausted for proven fixed layouts:
RadioPower ON (err 0), CardPower UP (err 2), EnableUicc=1 (err 0) —
none populate apps. Life SIM remains on phys1 via ipc0 map; bearer
blocker still ipc0 PIN+DISABLED.

Next hypothesis: CardPower DOWN(4)→UP(1) cycle on ipc1 only (factory
HAL1.6 path when lone UP fails), or abandon ipc1 apps=0 and re-focus
ipc0 PIN incongruence without inventing `0x0250`.

## 2026-09-30: ipc1 DOWN→UP ok apps=0; ipc0 radio poll no transient

(1) ipc1 CardPower DOWN(4)→UP(1): both length 12, **error 0** (lone UP
was err 2). Re-`0x0200`: ipc1 still PRESENT apps=0. ipc0 later polls
confirm PIN+DISABLED.

(2) New ipc0 PIN angles:
- **(a)** no send: `SetSlotMapping` still needs `slotmap.config`; no
  builder path copies GetSlotStatus port ints (incl. p1=255) into
  `0x0250`.
- **(c)** `OnGetSimStatusDone` → `BuildSimStatus` → RIL notify only. No
  second SIT opcode for PIN+DISABLED. `AutoVerifyPin` needs encrypted
  property — not applicable.
- **(b)** done: ipc0 Radio OFF→ON (both err 0), eight `0x0200` polls
  ~20s: every sample `app_state=2` `pin1=3` apps=1. `poll_stable_diff=0`.
  No transient READY/ENABLED. reg 0 tech 3; `rmnet` 0/0.

Blocker unchanged: ipc0 PIN+DISABLED is stable through radio cycle.
ipc1 DOWN→UP does not create apps (empty eSIM likely). Next: factory
decode of CP/HAL paths that **force app_state past PIN when pin1 is
already DISABLED** (non-VerifyPin), or OEM/nv read-only signals — still
no invented mapping ints.

## 2026-09-30: MODEM-06 — CP soft-lock PIN+DISABLED (no AP escape)

### Factory decode (app_state vs pin1)

1. **Independent CP wire fields.** `ProtocolSimStatusAdapter::Init`
   `memcpy`s the `0x0200` packet (15+63*apps). Only **card** byte is
   rewritten (3→PRESENT, 4→RESTRICTED). App state stays packet **byte 17**;
   pin1 stays **byte 72** (`GetPinState` adapter+0x58). HAL does **not**
   compute app_state from pin1.
2. **Enums.** `covertAppStateToString`: 2=`PIN`, 5=`READY`. AOSP pin 3=
   `DISABLED`. `CheckAndAutoVerifyPin` continues only when pin1==1
   (ENABLED_NOT_VERIFIED); DISABLED skips. `NetworkService` ready/attach
   path requires app_state==5 — no stock case treats PIN+DISABLED as READY
   or ignores PIN for attach.
3. **No override SIT.** `OnGetSimStatusDone` → `BuildSimStatus` → RIL
   notify only. No Terminal Profile / PROFILE DOWNLOAD / STK SET_PROFILE
   empty SIT in these builders for this path. AutoVerifyPin needs encrypted
   property. Prior closes/UICC/Radio/Card/slot/mapping do not rewrite
   byte 17.
4. **One never-sent empty GET:** `BuildGetVoiceOperation` `0x091b` len 12.
   Live ipc0: length 16, error 0, `mode_raw=3`. SIM still PIN+DISABLED;
   reg 0 tech 3; `rmnet` 0/0.

### Soft-lock verdict (MODEM-06)

CP firmware reports incongruent status (app PIN + pin1 DISABLED). AP/HAL
copies it unchanged; VerifyPin is rejected (err 6) when pin1 DISABLED;
attach stays blocked while app_state≠READY. Under current bans (no PIN
spray, no invented `0x0250`, no EFS RW, no POWER_OFF IOCTL) **there is no
proven AP SIT that clears this**.

Sysrq+ordered bring-up already reproduced the same PIN+DISABLED (prior
boots). ipc1 is empty-apps (likely no profile). Highest-evidence remaining
non-AP lever is **physical**: reseat/move the Life SIM to the other tray
position and re-map observation — not automatable here; not run.

**Next (when allowed):** physical SIM tray experiment, or CP-side
firmware/NV analysis outside AP SIT constraints. Goal stays incomplete
without live `rmnet`/IPv4.

## 2026-09-30: cold sysrq reload + STATUS APDU (still PIN+DISABLED)

Cold path (documented safe sequence — no POWER_OFF/crash ioctls):
`echo b > /proc/sysrq-trigger` → reload `shm_ipc`/`cpif_page`/`cpif`/
`cp_thermal_zone` (cpif.ko hash matched) → OFFLINE → persist RO mount for
64-byte signature only → `/tmp/probe-handover
boot-b-with-verified-nv-handover` → ONLINE (`COMPLETE: OK rc=0`).

Probe hold (15× SIM): first read length 80 card ABSENT apps=0; every
later read length 143 PRESENT apps=1 `app0_state=2` `pin1=3`. Identical
to prior boots.

Ordered ipc0 bring-up after ONLINE (`cold-apdu-status`):
- RadioPower ON → length 13, error 0.
- Four `0x0200` polls ~2s apart: every sample app=2 pin1=3 apps=1 card=1
  ready=0. `pin_disabled_identical=1`. Card already PRESENT → no CardPower.

APDU via proven TransmitApduBasic `0x020c` (TS 102.221; not VERIFY CHV):
- SELECT MF `00 A4 00 0C 02 3F00` → SW `9000`
- STATUS after MF `80 F2 00 00` → SW `9000` (resp length 75)
- SELECT ADF by AID from GetSimStatus (AID not printed) `00 A4 04 00` →
  SW `9000`
- STATUS after ADF `80 F2 00 00` → SW `9000` (resp length 86)
- STATUS P2=01 `80 F2 00 01` → SW `9000` (resp length 30)

Post-APDU `0x0200`: still app_state=2 pin1=3 remain=3 ready=0.
Registration: raw=0 reject=0 tech=3. `rmnet` rx=0 tx=0. No IPv4.
No LTE/auto chase (READY never reached). Modem stayed ONLINE.

Verdict: UICC answers SELECT/STATUS under MF and ADF with SW9000, yet CP
app_state stays PIN while pin1 DISABLED — soft-lock survives cold reload
and card-side STATUS. Goal incomplete.

**Next remote (highest evidence, not physical tray):** decode STATUS-after-ADF
response TLVs for TS 102.221 PIN Status Template (tag `0xC6`) / life-cycle
only (no IMSI/ICCID/AID print). If card shows PIN-not-required while CP
byte17=PIN: one factory `BuildSimOpenChannelWithP2` (`0x0247`) then
channel TransmitApdu STATUS — last stock UICC session path not exercised.
If still PIN+DISABLED, AP SIT path is closed; escalate to read-only CP
MAIN xref of the writer that sets app_state=PIN independently of pin1.

## 2026-09-30: STATUS TLV C6 + OpenChannelWithP2 — still soft-lock

Factory decode:
- `BuildSimOpenChannelWithP2` id `0x0247`, length 30. AID length@12,
  AID@13..28, **P2@29**. `OpenSimChannelHandler`: request P2==-1 →
  `BuildSimOpenChannel` `0x020d`; else P2 is SELECT P2 (stock often 0 =
  return FCI).
- `BuildSimTransmitApduChannel` id `0x020f`, base length 38. Words
  session@12 CLA@16 INS@20 P1@24 P2@28 P3@32; data_len@36; data@38.
  Handler encodes CLA with logical-channel bits for session < 4.
  Response: SW1@12 SW2@13, apdu_len@14, data@16 (unlike basic `0x020c`).

Live ipc0 (ONLINE, no secrets printed):
1. SELECT ADF + STATUS basic `0x020c` → SW `9000`, body_len 70.
   TLV **C6 present**, PS_DO present: `card_pin1_enabled=0`,
   `cp_pin1=3`, `life_cycle=0x05` →
   `match=card_disabled_vs_cp_disabled`. Card agrees PIN disabled and
   app activated; CP still reports app_state=PIN.
2. ONE `0x0247` OpenChannelWithP2 P2=0 → length 276, error 0,
   session=1, SW `9000`.
3. Channel STATUS `0x020f` P3=0 → SW `6C46` (wrong Le). One ISO retry
   P3=0x46 → SW `9000`, apdu_len=70. Close channel error 0.
4. Post `0x0200`: app_state=2 pin1=3 remain=3 ready=0. reg 0 tech 3;
   `rmnet` 0/0. No LTE chase.

Verdict: UICC PIN-disabled + activated; logical-channel STATUS OK; CP
app_state remains PIN. Soft-lock is CP-side app_state, not card PIN.

**Next remote (highest evidence):** read-only CP MAIN xref / string scan
for the path that writes GetSimStatus app_state=PIN while pin1 is already
DISABLED (and card PS_DO disabled). No further AP APDU/SIT unlock
candidates under current bans.


| class | items |
|-------|-------|
| **RO MAIN** | SET_APP/`+0xBF4`; Present/`+0xBF6`; FN_A×2 CDMA-only; FN_B not PresentObj; SIT `0x0200` mirror; Pin1Verified VerifyPin-only; PIN_SKIP eSIM stub; RatMap/TCS init; handover no RAP; modem_a same |
| **Live (prior)** | handover ONLINE; `0x0200` PIN+DISABLED; radio ON; reg0; rmnet 0; STATUS/OpenChannel APDU; preferred 11/12; cold sysrq |
| **Not done (banned)** | EFS RW; CDMA preferred; slot switch; PinSkip; PIN spray; invent opcodes |

## 2026-09-30: SET_APP exhaustive + CDMA ID non-alias + AllowData live

### 1) Every BL → SET_APP `0x19916d2` / sole STRB `+0xBF4`

MAIN scan: **14** BL sites. Sole `STRB.W #+0xBF4` = `0x1991734` inside SET_APP.

| BL site | imm → app_state | PresentGate (`LDRB +0xBF6` ∧ `CMP #2`) |
|---------|-----------------|----------------------------------------|
| `0x146aaba` | 1 | no |
| `0x14c64a6` | 6 | no |
| `0x14c65e2` | 0 | no |
| `0x14c6db6` | 0 | no |
| `0x14fb3de` | 3 (PUK) | **yes** |
| `0x14fb406` | 2 (PIN) | **yes** |
| `0x14fb526` | 4 | **yes** |
| `0x14fb5c6` | **5 (READY)** | **yes** |
| `0x18ec4fe` | 7 | no |
| `0x18fe4ce` | 0 | no |
| `0x1940f34` | 0 | no |
| `0x1a2ab44` | 0 | no |
| `0x1a2ab72` | 0 | no |
| `0x1a2ac00` | 0 | no |

**Verdict:** `#5` only at `0x14fb5c6`, and **only** behind Present==2. No READY outside that gate.

### 2) CDMA_MEAS / TIMING_LATCH not aliased to LTE/NR

| evidence | result |
|----------|--------|
| String VA | `MMC_LTEL1_CDMA_MEAS_RESULT_IND` = `0x4106ec0b`; `…CDMA_TIMING_LATCH_CNF` = `0x4106efb3` — unique; no `LTE_MEAS_RESULT_IND` / `NR_MEAS_RESULT_IND` names |
| MMC case lo16 | `0xec0b` → sole case @`0x14b7752` → BL WRAP_A `0x14c380e`; `0xefb3` → sole @`0x14b79ce` → BL LATCH `0x14c388e` |
| Sibling lo16 | UMTS `0xeee0` → `0x14c5f28`; UMTS_TDD `0xef38` → `0x14c60dc` (different handlers) |
| WRAP callers | BL→WRAP_A **1**; BL→LATCH_body **1**; BL→WRAP_SIM **1** |
| Non-0x4106 MOVT for those lo16 in MMC | **none** |

**Verdict:** MEAS/LATCH log IDs on this MAIN bind only to FN_A CDMA wraps — not reused for LTE/NR cases.

### 3) Live COM13/USB — GMC/MM/NAS + missing NET

ONLINE (`uptime` ~3.5h). `info_region` = srinfo/capability **offsets only** (no Present). dmesg: no GMC/MM/NAS string hits (CP logs not in kmsg).

| probe | result |
|-------|--------|
| `0x0200` | app=PIN(2) pin1=DISABLED(3) apps=1 |
| `0x0801` | radio=10 ON |
| `0x0701` data | reg=0 tech=3 UMTS |
| `0x0700` voice | **reg=3** tech=3 UMTS (MM answers; not silent) |
| `0x0702` operator | plmn_present=1 |
| rmnet0/1/2 | 0/0, no IPv4 |

Factory cold-boot `0x0801…0x0701` already exhausted earlier. Missing proven NET after that: **`BuildAllowData`** id **`0x0710`**, length 13, byte12=1 when allow==1 (sit-stream `0x74df0`). Never sent before.

**ONE send (COM13):** `0x0710` allow=1 → response length 12, **error 0**. Recheck: still PIN+DISABLED; data reg=0; voice reg=3; rmnet 0/0.

`BuildSetupDataCall` / IMS PDU still need APN/READY — not sent (no invent APN). `0x091b` / `0x0615` already done prior boots.

**GMC/MM/NAS:** not directly visible in srinfo/kmsg; voice reg + operator PLMN prove registration/NAS path is **alive** under PIN soft-lock. AllowData accepted ⇒ NET PS allow path also alive. Neither clears Present/READY.

### 4) MODEM-06 status

Soft-lock **holds**: sole READY=`SET_APP#5`∧Present==2; Present=2 only FN_A CDMA; IDs not aliased; RatMap no CDMA; EFS banned. Live NET gap (`0x0710`) closed — no `rmnet`/IPv4. Goal incomplete.

**Next remote (falsify MODEM-06, ≠ tray only):** RO listen on **`umts_dm0` / `oem_ipc*`** for live log lines `CDMA_MEAS_RESULT_IND` / `CDMA_TIMING_LATCH_CNF` / Present / SET_APP while radio ON + preferred LTE — if Present→2 or SET_APP#5 fires **without** those CDMA IND, FN_A-only model is false. Fallback: map PresentObj into any RO-mapped CP shm/info window and read `+0xBF6` live.

