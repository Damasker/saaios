# Native modem runtime: status query investigation

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
- Stream SIM adapter at 0x666a0 checks ID 0x0200, reads state at +12 and
  application count at +14. The diagnostic prints raw small status fields
  only, never application records or identifiers. Enum meaning is not assumed.
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
