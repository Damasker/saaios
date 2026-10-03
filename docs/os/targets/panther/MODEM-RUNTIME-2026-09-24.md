# Native modem runtime: status query investigation

**2026-10-03 TERMINAL BOUNDARY: foreign 3G PLMN + no LTE acquisition (VERDICT 13):**
queried serving operator 0x0702 (read-only) and forced LTE-only (recovered SIT value 0x0b
from BuildSetPreferredNetworkType table @0xd8c5c). Serving PLMN = 25501 (MCC 255 Ukraine /
MNC 01 Vodafone Ukraine) on UMTS/3G — FOREIGN vs the SIM's home lifecell (255-06). Forcing
LTE-only (set ACKed error_raw=0) never acquired any LTE cell (tech UMTS×36, none×2, LTE×0);
stayed on the denied Vodafone 3G. So host-side SIT replay cannot reach a bearer: home
lifecell LTE isn't acquired (RF/band in NV we won't touch, and/or no reachable home LTE
here), only a foreign 3G cell that denies the SIM — environmental/subscription + RF-cal
reality, not a host defect. 0x0702 layout: PLMN numeric at payload[12..17] ('#'=2-digit
MNC). Owner 2a4e07ed, CP ONLINE, default LTE+WCDMA restored. Bearer NOT achieved. Full
writeup in [MODEM-BLOCKER](MODEM-BLOCKER.md) V13.

**2026-10-03 TRUE reject-cause decoded = 0; modem camped on UMTS/3G only (VERDICT 12):**
recovered the exact 0x0700/0x0701 reg-state layout from `libsitril.so` (efcca0d5)
`ProtocolNet{Voice,Data}RegStateAdapter` and extended the owner's parser (read-only,
self-tested, -Werror) to decode RAT/LAC/cell/PSC plus the true reject. Stock reads
reject_cause at offset 13 — where our owner already read it — so reject=0 is genuine, not
a misread. One boot: data `reg=0 reject=0 tech=3(UMTS) lac=36291 cid=85793345 psc=187`,
voice `reg=3(DENIED) reject=0` same cell. The modem sees a real cell but only on UMTS(3);
CS denied with no cause, PS not searching, despite LTE+WCDMA preferred. RAT/coverage
situation (LTE RF/band is NV we won't touch), not a cause-coded reject. Serving PLMN is in
0x0702 (not queried). Owner `69d3d1c2`, CP ONLINE. Next (read-only): operator 0x0702 for
serving PLMN + LTE-only RAT trial. Full writeup in [MODEM-BLOCKER](MODEM-BLOCKER.md) V12.

**2026-10-03 SET_INITIAL_ATTACH_APN recovered + replayed (accepted) but NOT the gate:**
recovered the stock attach chain from `libsitril.so` (efcca0d5) — the one missing command
was `SIT_SET_INITIAL_ATTACH_APN` (opcode `0x0603`, 250-byte body; `[218]`=pdpType
`GetPdpType("IP")=1`, APN at `[16]`, all enum bytes recovered constants). Implemented
`make_initial_attach_apn_request` (guarded, self-tested byte-exact, `-Werror`), inserted
before ALLOW_DATA in stock order. One boot: modem ACKed it (`error_raw=0`) but registration
unchanged — data NOT_SEARCHING(0) tech=3, voice REG_DENIED(3), reject=0. The full accepted
stock host sequence (radio-on→auto-select→pref-RAT→IA-APN→allow-data, all error_raw=0) does
not register the modem ⇒ the gate is not a missing host SIT command. Device known-good (CP
ONLINE, owner `bb9398f2`). Next (read-only): decode the full reg-state response for the true
EMM/GMM reject cause. Full writeup in [MODEM-BLOCKER](MODEM-BLOCKER.md) VERDICT 11.

**2026-10-03 CP normal-NV self-downgrade FALSIFIED (read-only capture + diff):** extended
the quarantine owner with a guarded `SAAIOS_RFS_NORMAL_CAPTURE` block to capture the CP's
handle-1 (normal-NV) write-out into a quarantine-only file (never real EFS/nv_normal; no
payload logged). Needed a sequence-echo fix in the grant (handle-1 request uses seq 2 vs
handle-3's seq 1) after a first status-6 refusal. Captured intact (476454 bytes,
`received=476454 grants=237`), CP ONLINE/SIM-READY. Anchored diff vs fed-in `nv_normal.bin`
(byte-identical to sda5 per V7): 476361 equal bytes, 10 tiny edit regions, static config
body byte-identical (0 mismatches / 4000 samples). All edits are write-gen counters,
per-record checksums, one timestamp-shaped 9-byte value, and a non-persisted ~47 KB tail —
no op-mode/service-domain/limited-service/PLMN/RAT/GCFMODE/attach field changed. CP does not
self-downgrade normal-NV (matches protected-NV V7). Device known-good: proven owner restored
(`90f403df`), CP ONLINE; READ-ONLY throughout. With V7/V9/V10 falsified, best remaining
hypothesis = missing host-side RIL/SIT bring-up (radio online + automatic PLMN selection /
PS attach). Full writeup in [MODEM-BLOCKER](MODEM-BLOCKER.md) VERDICT 10.

**2026-10-03 `IOCTL_REQ_SECURITY` FALSIFIED as the gate (live-tested):** implemented the
GENUINE handshake (modes 2→0→1, params 0 = kernel-ignored → byte-identical SMC) as a
guarded, self-tested, non-fatal `PROBE_SECURITY` block (proven rebuild byte-identical
`e32538e8…`), and issued it on one controlled boot. All three returned `EINVAL`; dmesg:
`cpif: bootdump_ioctl: umts_boot0: security_req is null` — EL3/ldfw SMC never ran. Cause:
cpif `create_link_device@0x9708` installs the `security_request` pointer (off 976) only
when arg2==0 and a DT link-attr bit is set; panther leaves it NULL. Identical stock
`cpif.ko`+DT ⇒ stock cbd's `REQ_SECURITY` also `EINVAL`/non-fatal ⇒ vestigial, not the
gate. MAIN DONE still passed, CP ONLINE, registration unchanged (CS DENIED, PS
NOT_SEARCHING). Reverted to proven probe, CP ONLINE; no NV/EFS write, nothing forged.
Next (RO): capture+diff the ~476 KB normal-NV write-out vs fed-in. Full writeup in
[MODEM-BLOCKER](MODEM-BLOCKER.md) VERDICT 9.

**2026-10-03 secure-boot handshake characterized (`IOCTL_REQ_SECURITY` we omit):**
Stock `cbd` (pulled RO) issues `ioctl(boot_fd, 0x40106f53, &{mode,p2,p3,0})` three times
(mode 2 flag; mode 0 main-auth `p2=[cfg+0x260]`/`p3=[cfg+0x28c]`; mode 1); the kernel
`shmem_security_request` maps the CP shmem region and `__arm_smccc_smc`s to **EL3/ldfw**.
Our probe does the handover (`0x6f57`) + integrity-validated signed MAIN but issues none
of the three. **Feasibility: legitimately reproducible from SaaiOS** (genuine kernel→EL3
SMC, device-fused keys, AP supplies only mode+layout params — no secret, no forge); it's
the EL3/ldfw path, **not** the ADR-092 GSA mailbox. Open (live-only): does EL3 accept it
in our boot context — a `security check fail` would be the terminal boundary. Caveat: CP
already runs validated MAIN w/ RF-rx + SIM, so REQ_SECURITY may only map secure DRAM, not
gate MM (unproven). NV diff (RO): CP-written protected-NV == fed-in except 2 bytes (off
20/189444, each +4 = write-gen counter) → no protected-NV downgrade; normal-NV write-out
not captured. No reboot/NV write this session. Next: pin the two layout params, add the
three genuine REQ_SECURITY calls at cbd order, one controlled boot + re-check. Full
writeup in [MODEM-BLOCKER](MODEM-BLOCKER.md) VERDICT 8.

**2026-10-03 NV-starvation disproven (op-mode-NV falsified as the gate):** traced how
the CP gets NV at boot. The probe **pushes** `NV_NORM`+`NV_PROT` (`0x80000` each) from
`/data/saaios/var/efs-copy/` into the CP as SIT boot stages (like stock cbd), before
the owner attaches; the CP does **not** read op-mode via RFS. The RO verifier PASSes —
those blobs are **byte-identical to the real sda5 EFS** — and across boot + 42 min the
CP issues **zero RFS reads** (all frames are NV write-OUTs). So the CP is fed the exact
real stock NV (op-mode normal) and still denies, on **stock firmware** (`449eeab3…`)
with a clean boot (SIM READY, signal present). **`SAE_UE_OPERATION_MODE` is therefore
NOT the registration gate; the earlier (c)/NV-write conclusion is corrected and moot.**
Next, non-NV: the secure-boot/auth path (`HANDOVER_RAM_ONLY 0x6f57`, no
`IOCTL_REQ_SECURITY`, vs stock GSA) and a CP-written-vs-fed normal-NV diff. Full writeup
in [MODEM-BLOCKER](MODEM-BLOCKER.md) VERDICT 7. No NV/EFS write; bearer not established.

**2026-10-03 stock-registration capture (feasibility + pivotal verdict):** a live
stock-stack registration is **not feasible under SaaiOS** — no `/vendor/bin`/`rild`;
`cbd` only boots the (already-ONLINE) CP; `rild` needs the Android telephony
framework (binder/`hwservicemanager`/`system_server`, absent) to initiate
radio-on/registration; no `cpif` kernel frame logger exists. Equivalent evidence
pinned instead: our owner already matches stock's full known stage-1 trio
(`0x093f`→`0x0404` europen `0x0101`→`0x0800`, all ACK `error_raw=0`, `mask_low7=1`)
yet stays voice `REG_DENIED(3)`/data `NOT_SEARCHING(0)`. Pivotal verdict **(c)**: the
registration delta is CP-internal state from FLASH-NV `SAE_UE_OPERATION_MODE`, not a
missing command — (a) ruled out by command parity + exhausted surface; (b) ruled out
because RAM op-mode SETs ACK with the target values already present yet reg is
unchanged. Minimal delta = the operator-gated NV write (NO-GO, VERDICT 5). No stock
run, no NV/EFS write; device on safe owner, CP ONLINE; bearer not established. Full
writeup in [MODEM-BLOCKER](MODEM-BLOCKER.md) VERDICT 6.

**2026-10-02 de-risk (READ-ONLY):** characterized the `SAE_UE_OPERATION_MODE` NV
gate and built a tested, reversible backup/revert harness; **recommendation NO-GO**,
no NV write performed. The gate + siblings are **name-keyed** SAE-L3 flash NV items
(accessor loads by name string @≈`0x3DBF0xx`), so the on-disk byte offset is not
statically determinable; the 512 KB protected-NV blob is plaintext flash (entropy
3.28, 81/128 `0xFF`) but holds **no** name keys; the CP integrity validator is
unconfirmed; and the CP writes NV **out** without a confirmed read-back, so no AP-side
write path is proven to reach the CP (authoritative store is CP-side `sda5`/
`nv_protected`, forbidden). Harness: `nv-edit-harness.py` (selftest PASS),
`nv-backup.sh` (RO, sha-verified, source untouched), `nv-revert.sh` (guarded refusal
verified). Full go/no-go in [MODEM-BLOCKER](MODEM-BLOCKER.md) VERDICT 5. Bearer not
established.

**2026-10-02 final:** recovered + live-tested the last command-only lever,
`0x072B` (SET_DUAL_NETWORK_AND_ALLOW_DATA), from the full `libsitril.so`
(`BuildSetDualNetworkAndAllowData` @ `0x2375a0`): a 28-byte, 4×int32 body
`[translate(primNet), translate(secNet), primAllow, secAllow]`, filled with proven
wire values (net `12` = `0x070a`, allow `1` = `0x0710`); caller
`DoSetDualNetworkTypeAndAllowData` ends in `SendRequest` (no NvWrite) → command-safe.
Live (owner `90f403df`, step=`dual`): SET acked `error_raw=0` but registration did
not move (voice REG_DENIED(3)/reject 0, data NOT_SEARCHING(0)/tech 3, mask UMTS(2))
and no `rmnet` got an IPv4. **Command-only avenue is now fully exhausted** — the
only remaining path is a scoped, operator-gated FLASH-NV write of
`SAE_UE_OPERATION_MODE` (NOT authorized, NOT done; see
[MODEM-BLOCKER](MODEM-BLOCKER.md) VERDICT 4). Bearer not established.

**2026-10-02 latest:** issued every constraint-safe SIT operational-mode SET LIVE
from the unified owner (one SET per guarded warm-reboot handoff, single
`umts_ipc0` lock), after recovering their wire ids + body shapes from the full
`libsitril.so` (extracted READ-ONLY from `vendor.img` via `debugfs`; SHA
`efcca0d5…2d5b1`, exact TD1A match). Results — all four ack `error_raw=0`, none
move registration: `voice` `0x091A` m3 (GET already=3), `intps` `0x0933` m1 (ack
`0`, not removed), `stack` `0x080F` m1 (GET already enabled=1), `devsvc` `0x0956`
m2 data-centric (GET was 1 voice-centric). Throughout: voice `REG_DENIED(3)`
/reject 0, data `NOT_SEARCHING(0)`/tech 3, `mask_low7=2`. The op-mode GETs show
the modem is already in the target operational state, so the denial sits
below/outside the AP→CP operational-SET surface. `0x072B` not sent (body not
pinned); NV/`0x0937`/`POWER_OFF(3)` hard-barred → **command-only avenue
exhausted**. Owner hash `ecdf874f…` (self-test PASS); also fixed the short
`allow_data` ACK being swallowed before `reg_complete`. Bearer not established.
Details in the [MODEM-BLOCKER one-pager](MODEM-BLOCKER.md).

**2026-10-02 late:** early-boot RFS READ sequence instrumented (owner logs every
RFS header + keeps reading `umts_rfs0` past the write-out, through RadioPower-ON
and the MM gate). Result: the CP's entire RFS traffic is OPEN/STAT/WRITE — a
protected-NV write-out (handle 3, 189446 B) plus post-RadioPower-ON ~476 KB
write attempts (handle 1), with **zero read-expecting-data requests**. The MM
operational-mode/cal gate parameters (`SAE_FLASH_UE_OPERATION_MODE`,
`SAE_FLASH_GCFMODE`, `SAE_FLASH_PLMN_SEL_MODE`, RF-cal `CalDone`) are CP-internal
FLASH-NV, read via internal accessors, not RFS. **Verdict: gate value is
CP-STORE, not AP-served → no allowed read-divert; real-EFS/NV boundary reached.**
Owner `69f9b62d…` (self-test PASS); one-time data-chunk mis-log fixed + on-phone
log scrubbed. Details in the [MODEM-BLOCKER one-pager](MODEM-BLOCKER.md).
Bearer not established.

**2026-10-02 pm:** registration blocker isolated to a CP-internal pre-PLMN local
MM gate (reject_cause=0, no PLMN latched); not quarantine, not forbidden-PLMN.
PCIe endpoint wedge after RadioPower-ON is now mitigated automatically in the
handoff (RC `power/control=on` + bounded `pcie-stabilize-cp.sh` EP L1.2 disable);
validated on a fresh warm reboot (link recovers, IPC stays up, SIM READY,
registration reproduces the local deny). Details in the
[MODEM-BLOCKER one-pager](MODEM-BLOCKER.md). Bearer not established.

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
length 189446, operation 2. Operation 1 is the read path; operation 2
requests a write. The probe sent no reply, performed no open or write,
and the modem stayed ONLINE with
`rmnet0` rx and tx at 0. Persist was unmounted.
Log: `/data/saaios/var/probe-ioread-20260929.log`.

**Correction, 2026-10-01:** the earlier categorical claim that the *factory*
handler produces no channel reply was wrong. In the extracted factory
`/bin/rfsd` (SHA-256
`58d7f885e7533a328268f0de47ef9eb9995cdfa6b317d755b57973d4f5dfb71b`),
a failure branch at `0xea8c` calls helper `0x98c0`, which builds and writes
a 16-byte status-6 reply through `0x13de0`/`__write_chk`:
`03 00 01 00 08 00 00 00 06 00 00 00 03 00 00 00`.
But the observed 7→3 sequence sets local state `[obj+0x38]=3`; its command-6
route is `0xe414→0xe628→0xe8ec`, not the state-0 branch through `0xe7ac`
to `0xea8c`. For file 3, a valid local NV descriptor/size and successful
seek can take `0xea5c→0xec84→0x9aa0` and send a 20-byte command-2 write
grant. Invalid id/bounds or local file failure can instead send status 6.
The 2026-09-29 probe did not implement either file-backed path, so its
negative network result does not establish that RFS is irrelevant. The
newer fixed status-6 reply in `rfs-error-probe.c` was an incorrect claim
of factory equivalence; it is disabled pending a copy-backed RFS design.

A write grant exists in the factory handler: a 20-byte packet, command 2,
length 12, then file id, offset, and a chunk capped at 2012. The 189446
bytes are not in the 24-byte request. They would arrive only after a
grant. The probe sent no grant and accepted no data; the factory branch
also depends on local NV-file checks that this probe did not reproduce.
Disassembly review on 2026-10-01 identified the continuation: command-2
data echoes the command-6 sequence; 94 full 2012-byte chunks plus a final
318-byte chunk cover 189446 bytes if each uses the full grant. After the
last chunk, the handler fsyncs and calls `OnWriteDone`, then sends a 16-byte
command-3 success status (`0xa150–0xa180→0xa7f0`). Its caller does not
check the callback return before that status. A native implementation must
instead verify a private quarantine candidate before claiming success and
must never overwrite the verified boot NV copy or original EFS.

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

`BuildSimVerifyPin(arg, pin, aid)`: arg 0 ? id `0x0201` (PIN1), arg 1 ?
id `0x0203` (PIN2). Length 38; byte 12 = character count; bytes 13.. are
the PIN string (max 8); AID optional at 21+. The builder rejects a null
PIN pointer. `DoVerifyPin` rejects null/empty and non-digit strings and
requires length 1..8 before calling the builder. Stock never sends empty
PIN and never calls verify when `GetPinState` is 3 (DISABLED);
`CheckAndAutoVerifyPin` only continues for pin state 1.
`DoVerifyPin2` / SIM I/O / FD facility use arg 1 ? `0x0203`. That is not
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

## 2026-09-30: RadioPower OFF?ON; empty 0x0201 now err 2 not 6

`BuildRadioPower` id `0x0800`, length 18. `DoRadioPower` maps request
OFF (int?0) ? builder arg0=0 ? word at offset 12 = 1; ON (int>0) ?
arg0=1 ? word = 2. Bytes 16 and 17 are optional flag bytes (0 here).
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
without further PIN commands ? confirm whether CP can register despite
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
`0x0201` returns 2 while nonempty returns 6 (CP length/count check) ?
if stock always sends strlen?1, look for a non-digit factory unlock
path that the CP accepts (not more PIN digits).

## 2026-09-30: VerifyPin layout matches; selection already auto

Byte compare of our PIN1 packet vs `BuildSimVerifyPin(0, pin, null)`:
id `0x0201`, length 38, count at byte 12 = strlen (?8), characters at
13.., AID length at 21 and bytes at 22 only when AID hex is non-null
(else 21..37 zero). No slot or app index field exists in the builder.
Candidate A uses count 4 and four digit bytes ? identical to factory
for a 4-digit PIN with null AID. Digits were not resent.

`sit-base` names: error 2 `RCM_E_GENERIC_FAILURE`, 3
`RCM_E_PASSWORD_INCORRECT`, 6 `RCM_E_REQUEST_NOT_SUPPORTED`. Empty
count?2 is a failed/invalid verify body; nonempty with pin1 DISABLED?6
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
table index 12 ? value 12, AOSP `PREF_NET_TYPE_LTE_WCDMA`) ? LTE_ONLY=11
with reported UMTS tech may be attach-dead; one set then reg poll.

## 2026-09-30: PIN incongruence audit ? locks/slot/HAL; preferred=12 secondary

Bearer blocker (unchanged, explicit): CP `GetSimStatus` reports
`app_state=PIN` (2) while `pin1=DISABLED` (3). Attach/reg/`rmnet` stay
dead in that state. HAL does not rewrite PIN?READY when pin1 is 3.
VerifyPin is dead for DISABLED. This incongruence is the cellular bearer
gate, not preferred-mode tuning.

(1) Lock GETs (no secrets):
- `0x0207` VerifyNetworkLock is password SET-shaped ? not an empty GET;
  skipped.
- Empty `GetFrequencyLock` `0x073a` length 12 ? length 34, error 0.
  First payload words `0xFFFF0B01` / `0xFFFF0000` (sentinel / not an
  active frequency lock).
- Facility PN `0x0209` index 17, class 7 ? length 14, error 0,
  `lock_status_raw=0` (unlocked), same shape as prior SC unlock.
- Prior `GetSimLockInfo` `0x4104` remains dead (err 22). No network/SIM
  lock enum explains app_state PIN.

(2) `GetSlotStatus` `0x024d` non-secret parse (factory Init type1
len?0x1b1: `num_slots@12`; modern fill from pkt+0xd, stride 0x69:
card@+0, atr_len@+1; port count @record+52; no app_state field;
`GetSlotState` returns 0 when not legacy):
length 433, error 0, `num_slots=2`, `slot0_card_state_raw=1` PRESENT,
`slot0_atr_len=27`, `slot0_port_count_raw=2`. Matches GetSimStatus card
PRESENT and one SIM/USIM app still PIN. No clue that slot/card ? app0
as the PIN cause (dual-slot modem, slot0 populated, ATR present).

(3) Factory HAL writers of app_state / READY vs PIN when pin1==DISABLED:
`BuildRilCardStatusApplications` / `FillRilCardStatusFromAdapter` copy CP
app_state and pin words unchanged. `CheckAndAutoVerifyPin` continues only
when `GetPinState`==1; pin1==3 skips verify. `OnSimStatusChanged` takes
READY only when state is already 5. `CheckDisabledIccid` / `DoAutoDisableUicc`
are not READY paths. Stock cold-boot after ONLINE solicits GetSimStatus
and does not send a SIT that clears PIN+DISABLED. We are not missing a
HAL rewrite ? the CP packet itself is incongruent.

(4) SECONDARY only (after 1?3 yielded nothing actionable): one `0x070a`
preferred=12 (LTE_WCDMA) accepted (GET confirms raw 12). Three short
`0x0701` polls: still `reg=0`, `tech=3` UMTS, `rmnet` 0/0. SIM still
PIN, pin1 3. Preferred tweak is not the path.

Next non-PIN hypothesis: dual-slot / port mapping ? `num_slots=2` with
port_count=2 while GetSimStatus shows one app; look for a proven empty
modem-config or slot-mapping GET/SET stock sends on cold boot that we
skip (no invented opcodes; no secrets).

## 2026-09-30: dual-slot/port ? mapping SET not sendable; capability GET done

Factory multi-slot / port mapping:
- `BuildSimSetLogicalSlotMapping` id `0x0250`, length 17. Byte 12 =
  count (1..4); bytes 13+ = low bytes of physical-slot ints.
- `BuildSimSetLogicalSlotPortMapping` same id `0x0250`, length 21.
  Byte 12 = count; then (physical, port) byte pairs. Used when request
  meta > 0x1f (HAL SlotPortMapping path).
- `MiscService::SetSlotMapping` reads `persist.radio.slotmap.config`;
  empty ? log and return, **no packet**. Stock images leave that
  property empty (already confirmed). `0x093f` DSDS modems-config was
  already sent earlier.
- `BuildSetPreferredDataModem` id `0x0740`, length 13, modem byte at
  offset 20; index comes from the RIL request (must be < 3). No fixed
  default in the handler.
- Signal-strength reporting criteria builders exist; not slot mapping.

First proven mapping/switch we never sent: **none fully proven** ?
`0x0250` needs the slot (or slot+port) integer array. Missing field:
those integers (property string or framework `setSimSlotMapping`
payload). Not invented; not sprayed.

One proven never-sent radio-config GET (sibling of slot status):
`BuildGetPhoneCapability` id `0x0615`, length 12. Live: length 16,
error 0, `max_active_data=2`, `max_active_internet=1`, `lingering=1`,
`logical_modem_list_size=2`.

Fresh `0x024d` non-secret port parse (modern fill: ports at record+0x40,
stride 0xd; logical@+0, state@+1; 0xFF logical ? unassigned):
`num_slots=2`.
- slot0: card PRESENT, atr_len 27, ports 2; p0 logical=1 state=1;
  p1 logical=255 state=0.
- slot1: card PRESENT, atr_len 23, ports 1; p0 logical=0 state=1.
Crossed map: logical0?phys slot1, logical1?phys slot0. Both slots have
cards. GetSimStatus still one USIM app, PIN + pin1 DISABLED. reg 0,
tech 3, `rmnet` 0/0. No READY ? no bearer chase.

Blocker unchanged: PIN+DISABLED. Mapping SET still blocked on missing
integers. Next hypothesis: prove which logical modem `0x0200` binds to
(or how stock queries the other stack) given crossed slot1?slot0 map ?
still no invented `0x0250` payload.

## 2026-09-30: 0x0200 binds by IPC channel; ipc1 has no apps

Factory binding (re-checked):
- `InitRequestHeader` / `BuildSimGetStatus` (`0x0200`, len 12): no
  phone/slot/subscription field in the SIT header or payload.
- Dual stacks are separate `IoChannel` paths: `/dev/umts_ipc0` and
  `/dev/umts_ipc1` (both strings in `libsitril`; live sysfs `493:0` /
  `493:1`). `GetRilSocketId` is a RIL-context word only ? never packed.
- Our probes had always opened `umts_ipc0` only ? that channel's stack.
- `0x0250` is still **not** built from GetSlotStatus ports:
  `SetSlotMapping` needs `persist.radio.slotmap.config`;
  `SimSlotMappingHandler` needs framework ints. No send.

Live: mknod `umts_ipc1` from sysfs, shared `umts_rfs0`.
- ipc0 `0x0200`: length 143, card PRESENT, apps=1, app_state=PIN,
  pin1=DISABLED(3), remain=3. Same incongruence as before.
- ipc1 `0x0200`: length 80, card PRESENT, **apps=0**, no pin fields.
  Not READY. reg on ipc1: 0 / reject 0 / tech 0. `rmnet` 0/0.

COMPARE: ipc0 PIN+DISABLED vs ipc1 empty apps. Neither READY ? no
bearer chase; `0x0740` still skipped (modem index unproven).

Under the prior cross-map (logical0?phys1, logical1?phys0), ipc0?logical0
carries the only USIM app (PIN). ipc1?logical1 sees a card but no apps.
Next hypothesis: why ipc1 apps=0 (inactive port / missing subscription
bring-up) ? still without inventing `0x0250` ints.

## 2026-09-30: ipc1 RadioPower ON; apps still 0

Factory cold-boot SETs for the second RIL instance (same opcodes, other
`IoChannel` ? no slot field):
- `BuildRadioPower` `0x0800` len 18, ON ? word@12=2 (DoRadioPower).
- `BuildSetSimCardPower` `0x024c` len 13, UP byte@12=1.
- `BuildSetUicc` `0x0249` len 13, enable=1 (EnableUiccApp path).
- `BuildAllowData` exists; needs framework allow flag ? not a fixed empty
  default. `UiccSubscription` is a RIL request creator only; no standalone
  empty SIT builder in sit-stream. `0x0250` still not derived from ports.

ONE bring-up on ipc1 only: RadioPower ON. Response length 13, error 0.

Re-`0x0200` after:
- ipc0: PRESENT, apps=1, PIN, pin1 DISABLED ? unchanged.
- ipc1: PRESENT, apps=0 ? unchanged. Neither READY. ipc1 reg 0/tech 0;
  `rmnet` 0/0. No `0x0740`.

Life SIM without ICCID: only ipc0 reports a USIM app; cross-map
logical0?phys1 ? that app-bearing SIM is on **physical slot 1**.
(slot0?logical1 has card PRESENT but ipc1 apps=0.)

Blocker unchanged on ipc0. ipc1 RadioPower alone does not populate apps.
Next hypothesis: one CardPower UP `0x024c` state=1 on ipc1 only (proven
layout; not yet sent on that channel).

## 2026-09-30: ipc1 CardPower UP err 2; EnableUicc=1 still apps=0

ONE CardPower UP on ipc1 (`0x024c` state=1, no DOWN): length 12,
`error_raw=2` (GENERIC_FAILURE ? likely already powered / no-op).
Re-`0x0200`: ipc0 still PIN+DISABLED apps=1; ipc1 PRESENT apps=0.

Same turn, still no READY and apps=0 ? ONE EnableUicc `0x0249`=1 on
ipc1: length 12, error 0. Re-`0x0200` unchanged: ipc1 apps=0. Neither
READY. ipc1 reg 0/tech 0; `rmnet` 0/0.

ipc1 bring-up on this channel exhausted for proven fixed layouts:
RadioPower ON (err 0), CardPower UP (err 2), EnableUicc=1 (err 0) ?
none populate apps. Life SIM remains on phys1 via ipc0 map; bearer
blocker still ipc0 PIN+DISABLED.

Next hypothesis: CardPower DOWN(4)?UP(1) cycle on ipc1 only (factory
HAL1.6 path when lone UP fails), or abandon ipc1 apps=0 and re-focus
ipc0 PIN incongruence without inventing `0x0250`.

## 2026-09-30: ipc1 DOWN?UP ok apps=0; ipc0 radio poll no transient

(1) ipc1 CardPower DOWN(4)?UP(1): both length 12, **error 0** (lone UP
was err 2). Re-`0x0200`: ipc1 still PRESENT apps=0. ipc0 later polls
confirm PIN+DISABLED.

(2) New ipc0 PIN angles:
- **(a)** no send: `SetSlotMapping` still needs `slotmap.config`; no
  builder path copies GetSlotStatus port ints (incl. p1=255) into
  `0x0250`.
- **(c)** `OnGetSimStatusDone` ? `BuildSimStatus` ? RIL notify only. No
  second SIT opcode for PIN+DISABLED. `AutoVerifyPin` needs encrypted
  property ? not applicable.
- **(b)** done: ipc0 Radio OFF?ON (both err 0), eight `0x0200` polls
  ~20s: every sample `app_state=2` `pin1=3` apps=1. `poll_stable_diff=0`.
  No transient READY/ENABLED. reg 0 tech 3; `rmnet` 0/0.

Blocker unchanged: ipc0 PIN+DISABLED is stable through radio cycle.
ipc1 DOWN?UP does not create apps (empty eSIM likely). Next: factory
decode of CP/HAL paths that **force app_state past PIN when pin1 is
already DISABLED** (non-VerifyPin), or OEM/nv read-only signals ? still
no invented mapping ints.

## 2026-09-30: MODEM-06 ? CP soft-lock PIN+DISABLED (no AP escape)

### Factory decode (app_state vs pin1)

1. **Independent CP wire fields.** `ProtocolSimStatusAdapter::Init`
   `memcpy`s the `0x0200` packet (15+63*apps). Only **card** byte is
   rewritten (3?PRESENT, 4?RESTRICTED). App state stays packet **byte 17**;
   pin1 stays **byte 72** (`GetPinState` adapter+0x58). HAL does **not**
   compute app_state from pin1.
2. **Enums.** `covertAppStateToString`: 2=`PIN`, 5=`READY`. AOSP pin 3=
   `DISABLED`. `CheckAndAutoVerifyPin` continues only when pin1==1
   (ENABLED_NOT_VERIFIED); DISABLED skips. `NetworkService` ready/attach
   path requires app_state==5 ? no stock case treats PIN+DISABLED as READY
   or ignores PIN for attach.
3. **No override SIT.** `OnGetSimStatusDone` ? `BuildSimStatus` ? RIL
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
attach stays blocked while app_state?READY. Under current bans (no PIN
spray, no invented `0x0250`, no EFS RW, no POWER_OFF IOCTL) **there is no
proven AP SIT that clears this**.

Sysrq+ordered bring-up already reproduced the same PIN+DISABLED (prior
boots). ipc1 is empty-apps (likely no profile). Highest-evidence remaining
non-AP lever is **physical**: reseat/move the Life SIM to the other tray
position and re-map observation ? not automatable here; not run.

**Next (when allowed):** physical SIM tray experiment, or CP-side
firmware/NV analysis outside AP SIT constraints. Goal stays incomplete
without live `rmnet`/IPv4.

## 2026-09-30: cold sysrq reload + STATUS APDU (still PIN+DISABLED)

Cold path (documented safe sequence ? no POWER_OFF/crash ioctls):
`echo b > /proc/sysrq-trigger` ? reload `shm_ipc`/`cpif_page`/`cpif`/
`cp_thermal_zone` (cpif.ko hash matched) ? OFFLINE ? persist RO mount for
64-byte signature only ? `/tmp/probe-handover
boot-b-with-verified-nv-handover` ? ONLINE (`COMPLETE: OK rc=0`).

Probe hold (15? SIM): first read length 80 card ABSENT apps=0; every
later read length 143 PRESENT apps=1 `app0_state=2` `pin1=3`. Identical
to prior boots.

Ordered ipc0 bring-up after ONLINE (`cold-apdu-status`):
- RadioPower ON ? length 13, error 0.
- Four `0x0200` polls ~2s apart: every sample app=2 pin1=3 apps=1 card=1
  ready=0. `pin_disabled_identical=1`. Card already PRESENT ? no CardPower.

APDU via proven TransmitApduBasic `0x020c` (TS 102.221; not VERIFY CHV):
- SELECT MF `00 A4 00 0C 02 3F00` ? SW `9000`
- STATUS after MF `80 F2 00 00` ? SW `9000` (resp length 75)
- SELECT ADF by AID from GetSimStatus (AID not printed) `00 A4 04 00` ?
  SW `9000`
- STATUS after ADF `80 F2 00 00` ? SW `9000` (resp length 86)
- STATUS P2=01 `80 F2 00 01` ? SW `9000` (resp length 30)

Post-APDU `0x0200`: still app_state=2 pin1=3 remain=3 ready=0.
Registration: raw=0 reject=0 tech=3. `rmnet` rx=0 tx=0. No IPv4.
No LTE/auto chase (READY never reached). Modem stayed ONLINE.

Verdict: UICC answers SELECT/STATUS under MF and ADF with SW9000, yet CP
app_state stays PIN while pin1 DISABLED ? soft-lock survives cold reload
and card-side STATUS. Goal incomplete.

**Next remote (highest evidence, not physical tray):** decode STATUS-after-ADF
response TLVs for TS 102.221 PIN Status Template (tag `0xC6`) / life-cycle
only (no IMSI/ICCID/AID print). If card shows PIN-not-required while CP
byte17=PIN: one factory `BuildSimOpenChannelWithP2` (`0x0247`) then
channel TransmitApdu STATUS ? last stock UICC session path not exercised.
If still PIN+DISABLED, AP SIT path is closed; escalate to read-only CP
MAIN xref of the writer that sets app_state=PIN independently of pin1.

## 2026-09-30: STATUS TLV C6 + OpenChannelWithP2 ? still soft-lock

Factory decode:
- `BuildSimOpenChannelWithP2` id `0x0247`, length 30. AID length@12,
  AID@13..28, **P2@29**. `OpenSimChannelHandler`: request P2==-1 ?
  `BuildSimOpenChannel` `0x020d`; else P2 is SELECT P2 (stock often 0 =
  return FCI).
- `BuildSimTransmitApduChannel` id `0x020f`, base length 38. Words
  session@12 CLA@16 INS@20 P1@24 P2@28 P3@32; data_len@36; data@38.
  Handler encodes CLA with logical-channel bits for session < 4.
  Response: SW1@12 SW2@13, apdu_len@14, data@16 (unlike basic `0x020c`).

Live ipc0 (ONLINE, no secrets printed):
1. SELECT ADF + STATUS basic `0x020c` ? SW `9000`, body_len 70.
   TLV **C6 present**, PS_DO present: `card_pin1_enabled=0`,
   `cp_pin1=3`, `life_cycle=0x05` ?
   `match=card_disabled_vs_cp_disabled`. Card agrees PIN disabled and
   app activated; CP still reports app_state=PIN.
2. ONE `0x0247` OpenChannelWithP2 P2=0 ? length 276, error 0,
   session=1, SW `9000`.
3. Channel STATUS `0x020f` P3=0 ? SW `6C46` (wrong Le). One ISO retry
   P3=0x46 ? SW `9000`, apdu_len=70. Close channel error 0.
4. Post `0x0200`: app_state=2 pin1=3 remain=3 ready=0. reg 0 tech 3;
   `rmnet` 0/0. No LTE chase.

Verdict: UICC PIN-disabled + activated; logical-channel STATUS OK; CP
app_state remains PIN. Soft-lock is CP-side app_state, not card PIN.

**Next remote (highest evidence):** read-only CP MAIN xref / string scan
for the path that writes GetSimStatus app_state=PIN while pin1 is already
DISABLED (and card PS_DO disabled). No further AP APDU/SIT unlock
candidates under current bans.

## 2026-09-30: RO CP MAIN xref ? app_state PIN vs pin1 DISABLED

**Image (worktree host copy):**
`saaios-modem-research/os/targets/panther/diagnostics/fw/saaios-probe-b-modem.bin`
(SHA-256 `449eeab3?` = modem_b B-15346003). MAIN stage
`file_off=0x16c10` `size=0x05917acc` `VA_base=TOC m_off 0x40010000`.
Analysis: Thumb-2 RO strings + isolated MOVW log-id xrefs only. No EFS
RW, no crash/power-off ioctls, no secret dumps.

**State machine strings / log ids (SIT0/USIM):**
| id | string |
|----|--------|
| `0x347` | `Tx SIM Status(app_state=%d, perso_substate=%d)` |
| `0xc6b` | `sitSetPin1Status:- PIN 1 Status : (%d ----> %d)` |
| `0x7ab` | FCP: Pin Status Disabled ? `changing PinStatus as PIN_DISABLED` |
| `0x2c5e` | `>> DetermineSimStatus` |
| `0x106a` | `SIM STATUS update: Present:%d, Pin1Verified: %d, MePerVerified:%d` |
| `0x11d6` | `PIN SKIP FAILED: NOT eSIM` |
| `0x11ce` | `PIN SKIP FAILED: Invalid SimState(%d)` |

**Who writes app_state PIN(2) vs READY(5):**
- Candidate setter `file 0x19916d2` / `VA 0x4198aac2`.
- Across MAIN, **only one** call site loads `#2` then BL that setter:
  `BL @ file 0x14fb406` / `VA 0x414f47f6` ? **PIN**.
- **Only one** loads `#5` then BL same setter:
  `BL @ file 0x14fb5c6` / `VA 0x414f49b6` ? **READY**.
- Both sit in the same SIM-STATUS-update function that logs `0x106a`
  (`Pin1Verified` / `MePerVerified`). READY path: after that log,
  `CMP ?,#2` then `MOVS #5` + BL setter. PIN path: `MOVS #2` + BL setter
  on the non-verified branch.

**pin1 DISABLED vs app_state (desync root):**
- FCP/PS_DO ?Pin Status Disabled? path logs `0x7ab` and updates **PinStatus**
  (e.g. USIM site `file 0x2b36782` / `VA 0x42b2fb72`: `MOVS #3` then BL
  `0x20e184e` ? pin-status helper, **not** the app_state setter).
- That cluster has **zero** BL to `0x19916d2`. So CP can report
  `pin1=DISABLED(3)` while `app_state` remains whatever SIM STATUS last
  wrote ? typically **PIN(2)** until `Pin1Verified` clears the gate.
- Matches live: card C6/PS_DO disabled + `pin1=3`, but `app_state=2`.

**PIN SKIP (not an AP escape for this card):**
- CP-internal `NS_SIM_PIN_SKIP_REQ` / `DecodeSimPinSkipReq` only.
- Fail logs: NOT eSIM (`0x11d6`), Invalid SimState (`0x11ce`).
- No `SIT_*PIN_SKIP*` string; HAL (`libsitril` / `sit-stream` / `sit-base`)
  has **no** PinSkip builder. Do not invent an opcode.

**AP action this turn:** none. No proven AP-visible trigger (SIT / already-
proven opcode / property / bring-up order) that sets `Pin1Verified` or
forces the READY writer under current bans.

**Next (least-bad, allowed):** RO MAIN xref of **who sets `Pin1Verified=1`**
(and whether FCP `PIN_DISABLED` / PS_DO disabled is supposed to set it but
does not). If a live CP log buffer is readable RO, one sample of
`SIM STATUS update: Present/Pin1Verified/MePerVerified` to confirm the
gate value ? still no PIN digits, no EFS RW, no crash ioctls.

## 2026-09-30: RO Pin1Verified writers ? VerifyPin-only; FCP never sets

Same MAIN image. Log ids (Shannon packed, id = `(hdr>>8)` when lo=0x44):

| id | role |
|----|------|
| `0x18e` / `0x188` | SIT0/1 `First PIN1 Verification is done : sitSendNsSimInfoReq()` |
| `0x20f` | `First PIN1 Unblock is done` (parallel path) |
| `0x17e` / `0x167` | SIT0/1 `Rx NS_USIM_VERIFYPIN_RSP` |
| `0x106a` | `SIM STATUS update: Present / Pin1Verified / MePerVerified` |
| `0xbda` | `Reset IsSimVerifyCompleteSent ? Pin1Verified` (clear) |
| `0x7ab` | FCP ? `PIN_DISABLED` (pin status only) |

**Sets Pin1Verified=1 (evidence):**
- Real SIT0 sites (ctx `0x410a`): `file 0x1f0456c` / `VA 0x41efd95c` and
  `0x1f0469a` / `VA 0x41efda8a` log `0x18e`, then
  `MOVS #1; STRB [rN,#20]` at `0x1f04576` / `0x1f046a4`
  (`VA 0x41efd966` / `0x41efda94`). Object offset **+20** = Pin1Verified.
- Trigger chain: successful `NS_USIM_VERIFYPIN_RSP` ? ?First PIN1
  Verification is done? ? `sitSendNsSimInfoReq`. AP face is stock
  VerifyPin SIT (`0x0201`). Unblock success uses sibling log `0x20f`.
- Callers BL into this cluster (`0x1f044d6`?`0x1f046f4`) from SIT verify
  handlers (e.g. `0x1d26f18`, `0x1d4e892`, ?) ? not from FCP parse.

**Does not set Pin1Verified:**
- FCP `PIN_DISABLED` (`0x7ab`, USIM window `0x2b366f0`?): STRB to
  `#14/#9/#15` only ? **no STRB `#20`**. Confirms pin1=DISABLED without
  verified flag.
- No HAL/SIT PinSkip; eSIM `NS_SIM_PIN_SKIP` unchanged / out of scope.

**Clears:** reset string `0xbda` (`Reset ? Pin1Verified`) ? soft-reset /
SIM-reinit path clears the flag (pairs with STATUS update staying PIN).

**Map vs proven live paths:**
| Event | Pin1Verified | Evidence |
|-------|--------------|----------|
| VerifyPin SIT success | **sets 1** | `0x18e` + STRB `#20` |
| VerifyPin when pin1 DISABLED | **blocked** | live err6; never reaches First-PIN1-done |
| CHV VERIFY APDU | would need digits + success into same NS verify path | banned / not needed when card PS_DO disabled |
| FCP/PS_DO pin-disabled | **never** | `0x7ab` window, no `#20` store |

**AP action:** none. Only stock unlock to set the flag is VerifyPin/Unblock
success; with pin1 already DISABLED that path is closed (err6). No new
opcode invented.

**Next (least-bad, allowed):** RO sample of live CP log line
`SIM STATUS update: Present/Pin1Verified/MePerVerified` (confirm
Pin1Verified=0 under soft-lock), **or** RO xref whether any
pin-status=`DISABLED` transition is *supposed* to call the `0x18e`
setter (missing call = CP bug; fix would be CP-internal under bans).

## 2026-09-30: READY re-audit ? sole branch; DISABLED?READY

**Setter inventory (whole MAIN ? `0x19916d2` / VA `0x4198aac2`):**

| imm | site (file / VA) | gate |
|-----|------------------|------|
| **5 READY** | `0x14fb5c6` / `0x414f49b6` | after log `0x106a`, `LDRB.W [r0,#0xBF6]`, **CMP #2 EQ** |
| **2 PIN** | `0x14fb406` / `0x414f47f6` | after `0x106a`, same byte **?1** (fallthrough) |
| 3 PUK | `0x14fb3de` / `0x414f47ce` | same byte **==1** |
| 4 PERSO | `0x14fb526` / `0x414f4916` | same byte **==3** |
| 0/1/6/7 | other sites | not READY/PIN |

**No other READY(#5) or PIN(#2) writers** via this setter exist in MAIN.

**Flag layout in same function:**
- `LDRB [r5,#0]` ? Present (logged in `0x106a`)
- `LDRB [r5,#1]` ? Pin1Verified
- READY/PUK/PERSO decisions use a **different** byte at **`base+0xBF6`**
  (not pin1 SIT field): `1?PUK`, `2?READY`, `3?PERSO`, else early **PIN**.

**DISABLED / PS_DO in READY gate?** **No.**
- `pin1==DISABLED(3)` is the SIT pin-status field (`sitSetPin1` / FCP
  `0x7ab`), not the `+0xBF6` decision byte.
- Live proof: `pin1=3` but `app_state=PIN(2)` ? if `+0xBF6` were pin1,
  CMP#3 would have stored **PERSO(4)**, not PIN. So DISABLED does not
  enter READY (nor even the PERSO overwrite path in this boot).
- FCP/PS_DO path still never stores Pin1Verified / never `SET_APP #5`.

**Live RO (COM13, modem ONLINE, no secrets):**
- `0x0200`: card PRESENT, apps=1, **app_state=PIN(2)**, **pin1=DISABLED(3)**,
  remain=3
- Present (readable) = **1**
- Pin1Verified / MePerVerified: **not exported** on `0x0200`; Shannon log
  `0x106a` not present in AP `logbuffer_*` / cpif sim sysfs (`ds_detect=2`
  only). Inferred **Pin1Verified=0** (sole READY needs `+0xBF6==2`, which
  follows VerifyPin-done / `0x18e`; never reached ? live VerifyPin?err6
  when DISABLED)
- reg raw=0 tech=3; `rmnet0` rx=0 tx=0; radio_state=10

**AP action:** none ? no missed AP-triggerable READY path under bans.

**Hole (hardened):** stock PIN-disabled SIMs that never run VerifyPin OK
never set Pin1Verified and never hit the only READY store. That is a
CP-side coupling gap (DISABLED pin status ? verified/READY enum), not an
AP SIT gap. Goal incomplete (no `rmnet`/IPv4).

**Next (least-bad, allowed):** RO whether FCP `PIN_DISABLED` / PS_DO
disabled is *documented in CP* to update `+0xBF6` to `2` (missing store =
firmware hole); optional: enable a **read-only** CP log path if a
non-secret Shannon sink appears ? still no EFS RW / no invented opcodes.

## 2026-09-30: +0xBF6 semantics ? Present enum; FCP never writes it

**Field map (object via MLA base `0x48C649B8` + stride):**

| off | source | meaning |
|-----|--------|---------|
| **+0xBF6** | STATUS copies trio `[r5,#0]` (`STRB.W` only at `0x14fb380`) | **Present** enum (not bool) |
| **+0xBF5** | STATUS copies `[r5,#1]` (`0x14fb432`) | Pin1Verified (0/1) |
| **+0xBF4** | `SET_APP` itself (`0x1991734`) stores the new app_state imm | app_state mirror |

**Present enum ? app_state (after copy to +0xBF6):**
`0` clear ? `1`?PUK ? `2`?**READY** ? `3`?PERSO ? else?PIN.

**Writers of +0xBF6:** **one** in all MAIN ? STATUS Present copy. No
`MOVS #2; STRB #0xBF6`. Value `2` only if Present source already `2`.

**Present=2 writers (SIM module `0x14f0000`):**
- `0x14f6a14` / VA `0x414efe04`: `CMP arg0,#3` IT EQ ? Present=`2` else `3`
  (fn `0x14f692c`, sole caller `0x14c37f2`). Also stores Pin1V byte to `[+#1]`.
- `0x14f9578`: Present=`2` under other local CMP (separate builder).
- Also Present=`0`/`1` stores in same module.

**FCP?PIN_DISABLED (`0x7ab`)?** **Does not write +0xBF6 / Present=2.**
No BL from FCP body into `0x14f0000` Present builders. Nearby FCP stores
are pin-status offsets (`STRB #2`), not Present. **No string/comment that
FCP DISABLED ?should? set Present/`+0xBF6=2`** ? coupling is absent, not
an omitted store inside FCP itself.

**VerifyPin OK / `0x18e`?** Sets **Pin1Verified obj+20 = 1 only**. Does
**not** STRB Present=2 / +0xBF6. Callees after `0x18e` have no `#0xBF6`
store. Stock Present=2 is the **separate** builder above, not VerifyPin.

**AP-visible way to force Present/+0xBF6=2?** None under bans (would need
CP to run Present=2 builder; no proven SIT opcode). No live probe.

**Hole (refined):** READY needs Present==2 (+0xBF6). FCP DISABLED and
VerifyPin-fail/DISABLED leave Present?2. Present=2 builder exists but is
**not** reached from FCP; whether caller `0x14c37f2` should pass arg0=3
after DISABLED is the next RO question. Goal incomplete (no `rmnet`/IPv4).

**Next (least-bad, allowed):** RO caller `0x14c37f2` ? what is arg0 (`r4`?
`r9`), when invoked vs FCP/`sitSetPin1` DISABLED; does stock path call
Present=2 builder after pin-disabled FCP?

## 2026-09-30: Present=2 callers ? not after FCP DISABLED

**Call graph:**

| Present=2 site | function | callers | arg0 / gate |
|----------------|----------|---------|-------------|
| `0x14f6a14` | FN_A `0x14f692c` | WRAP_A `0x14c380e` ? switch case log **`MMC_LTEL1_CDMA_MEAS_RESULT_IND`**; and SIM wrap `0x14f6d02` ? `0x14c3986` | WRAP_A: `LDRB` payload `[msg+0]`; **`==3 ? Present=2` else 3**. Internal: `LDRB [obj,#8]` |
| `0x14f9578` | FN_B `0x14f9108` | `0x14c5fe6` (LTEL1 meas neighbour; near `UMTS_*_MEASURE_CNF` cases) | local `[r7,#0xb4]==0` ? Present=2 |

Note: earlier ?caller `0x14c37f2`? is WRAP_H?HELPER `0x14f6900` (LDRB return only), **not** FN_A Present=2.

**Invoked after FCP PIN_DISABLED / VerifyPin / ATR / app-select?** **No.**
- FCP `0x7ab`, Pin1Verified/`0x18e`, `sitSetPin1` regions: **zero** BL into FN_A/FN_B/WRAP_A.
- Present=2 on observed graph is **L1/MMC measure IND/CNF** dispatch (and a separate gated SIM-internal wrap), not USIM FCP/PIN.

**Why live soft-lock (app=PIN, not READY):** Present enum never becomes `2`.
STATUS: `1?PUK`, `2?READY`, `3?PERSO`, else?**PIN**. Live **app=PIN** ? Present **?1,?2,?3** (likely 0 / never built) ? not ?stuck at 1/PUK?. FCP DISABLED does not call Present=2 builders; arg0==3 path not taken on our bring-up.

**AP-visible trigger for arg0==3 / Present=2?** None under bans (would be inventing L1 meas injection). No live probe.

**Hole:** pin-disabled FCP never couples to Present=2; READY blocked. Goal incomplete (no `rmnet`/IPv4).

**Next (least-bad, allowed):** RO **`SIM START IND`** / `SIM_PRESENT_IND` / `SIM_PIN_STATUS_IND` handlers ? do they write Present enum or call `0x14f6d02`/`FN_A` with arg that yields Present=2 after pin1 DISABLED?

## 2026-09-30: SIM START / PIN_STATUS IND ? no Present=2

**What they are:** CP?AP **TX** (`USIM ==>`), not AP Rx handlers.
Message-name table: `USIM ==> SIM_PIN_STATUS_IND` @ litpool `0x10fb2bc`,
`USIM ==> SIM_START_IND` @ `0x10fb87c`. Payload log
`SIM START IND (SimPresent=%d, SimState=%d, Pin1Status=%d, Pin2Status=%d)` ?
reports fields; does not write STATUS Present enum.

**Builders (START):**
| site | ctx | emit helper |
|------|-----|-------------|
| `0x191687a` | `0x4107` + log **`0x105a`** | `0x19a2b54` |
| `0x1916e70` / `0x19170e6` | same | `0x189bcf6` |
| `0x191724e` | same | `0x18d2258` |
| Prepare | log **`0x5b5`** `PrepareSimStartIndParameters` | (param fill only) |

Same helpers in USIM band `0x1910000..0x1950000` only seen with log `0x105a`
(no separate PIN_STATUS emit sites found via that path).

**Present=2 / FN_A / FN_B / +0xBF6 / Pin1Verified after pin1 DISABLED?**
**No.** Windows around START sites and scan `0x1916000..0x191a000`:
zero `BL` FN_A/`0x14f692c`, FN_B/`0x14f9108`, WRAP_SIM/`0x14f6d02`;
zero `STRB.W #0xBF6` / `#0xBF5`. FN_A callers remain only L1 WRAP
`0x14c3872` and SIM-wrap `0x14f6d7a` (log `0x1068` nearby) ? not START/
PIN_STATUS. Pin1V sites `0x1f04576`/`0x1f046a4` unrelated.

**Stock order before those IND:**
1. AP?CP **`USIM <== SIM_INIT_REQ`** (CP: *Waiting for SIM_INIT_REQ*,
   state `USIM_WAIT_FOR_INIT_REQ`).
2. From `USIM_WAIT_FOR_INIT_REQ` / `USIM_CARD_PRESENT` ? **`SIM_PRESENT_IND`**
   to PBM.
3. Then **`SIM_START_IND`** / **`SIM_PIN_STATUS_IND`**.
4. After START, AP may send **`SIM_START_STACK_SERVICES_REQ`**.
SIT log `sitInformSimInit()` exists; our `saai-modemd` has no INIT/START symbols.

**Missing AP/SIT to force Present=2?** None proven. Live already has
pin1=DISABLED + app=PIN ? INIT/FCP path already ran; re-sending INIT or
STACK_SERVICES is not shown to call Present=2 builders. No live probe.

**Hole (unchanged):** READY needs Present==2; START/PIN_STATUS IND do not
build it; L1/SIM-wrap builders still uncoupled from DISABLED.

**Next:** done ? see ? ?Present writers + SIM-wrap? below.

## 2026-09-30: Present writers + SIM-wrap = CDMA_TIMING_LATCH_CNF

**Present enum source** (byte STATUS copies via `LDRB [obj,#0]` ? sole
`+0xBF6` store `0x14fb380`). Pattern: `STRB.W [PresentObj,#0]` with
related `+#1` / `+#8` on same object.

| value | store site | function | gate |
|------:|------------|----------|------|
| **0** | `0x14f962e` | FN_B `0x14f9108` | clear/fail path in FN_B |
| **1** | `0x14f924e`, `0x14f94c2` | FN_B | intermediate FN_B branches |
| **2** | `0x14f6a16` | FN_A `0x14f692c` | **`arg0==3`** (`MOV r9,r0`; `CMP r9,#3` IT EQ) |
| **2** | `0x14f957c` | FN_B | `[r7,#0xb4]==0` |
| **3** | `0x14f6a16` | FN_A | **`arg0!=3`** (MOVS `#3` then same STRB) |

No other Present-object stores found in SIM module `0x14f0000..0x1508000`.
`+0xBF6` is copy-only.

**SIM-wrap `0x14c3986`:** not a fn entry ? site inside handler **`0x14c388e`**.
- Dispatcher caller: `0x14b79e2` (same MMC/L1LC switch as WRAP_A).
- **Message:** `MMC_LTEL1_CDMA_TIMING_LATCH_CNF` (name VA `0x4106efb3` before case).
- Flow: `r5=msg` ? `BL WRAP_SIM 0x14f6d02` ? `LDRB r0,[r5,#8]` ? **`BL FN_A`**.
- **arg0 = payload `[msg+8]`**; Present=`2` iff that byte **`==3`**, else `3` (PERSO).
- Sibling WRAP_A `0x14c380e` ? `MMC_LTEL1_CDMA_MEAS_RESULT_IND`: arg0=`[msg+0]`, same FN_A rule.

**AP SIT for `SIM_INIT_REQ` / `SIM_START_STACK_SERVICES_REQ`?** **No.**
Factory `sit-stream.so` `BuildSim*` list has neither (only known SIM builders:
status/PIN/PUK/IO/ATR/channel/auth/facility/slotmap/?). Those names are
internal USIM/NS/GMC (`USIM <== ?`, `GMC ==> SIM__ [START_STACK_SERVICES_REQ]`).
`sitInformSimInit()` is CP SIT log only. **No live probe.**

**Hole:** Present=2 still only via L1/MMC CDMA meas/timing paths (or FN_B
local gate) ? not INIT/STACK/FCP/DISABLED. Goal incomplete (no `rmnet`/IPv4).

**Next (least-bad, allowed):** RO meaning of **`[CDMA_TIMING_LATCH_CNF+8]==3`**
(who fills that CNF) **or** FN_B Present=0/1 callers ? still no invent opcodes.

## 2026-09-30: LATCH +8 = rat_mode; FN_B = UMTS_MEASURE_CNF

**Producer chain (internal L1/MMC, not AP SIT):**
1. L1LC **`Send LTEL1_MMC_CDMA_TIMING_LATCH_REQ`**
2. Forward **`CDMA_TIMING_LATCH_REQ to EVDO`** / `L1C_MMC_CDMA_TIMING_LATCH_REQ_Handler`
3. CNF back as **`MMC_LTEL1_CDMA_TIMING_LATCH_CNF`** (also name `MMC_L1C_CDMA_TIMING_LATCH_CNF`)
4. MMC?L1LC decode log: **`Decode ?_CNF(rat_mode:%d)`**
5. Handler `0x14c388e`: **`LDRB [msg,#8]`** logged then passed ? WRAP_SIM ? FN_A

**Byte +8:** not SIM Present ? decode names it **`rat_mode`**. Same byte is FN_A
`arg0`; **`rat_mode==3` ? Present enum := 2**, else := 3. Exact RAT label for
numeric `3` not string-proven (no `MMC_SRC_RAT_*` table with =3); do not invent.

**FN_B Present=0/1:** no separate callers. Sole entry `FN_B` via wrapper
`0x14c5f28` ? dispatcher `0x14b772a` = **`MMC_LTEL1_UMTS_MEASURE_CNF`**.
Stores 0/1/2 are **internal branches** of that one measure-CNF handler
(gates on payload / `[r7,#0xb4]` for =2). Not a prerequisite before LATCH.

**Naming / live app=PIN:**
| name | what |
|------|------|
| log `Present` / `+0xBF6` | SIM status enum ? app_state (`1`PUK `2`READY `3`PERSO else **PIN**) |
| CNF `[msg+8]` | **`rat_mode`** (L1 latch), coupled into Present only via FN_A when ==3 |

Live **app=PIN** ? Present **?{1,2,3}** (likely **0** / never built). Does **not**
require clearing PIN-stuck before LATCH; LATCH+`rat_mode==3` would *set*
Present=2. Our soft-lock = that L1 CNF path never delivered `+8==3`.

**AP/SIT to force CNF+8==3?** None proven (IRAT CDMA/EVDO latch; no
`BuildSim*` / invent opcode). **No live probe.**

**Hole:** Present=2 still needs L1 MMC CDMA latch (`rat_mode==3`) or FN_B
UMTS measure gate ? neither on pin-disabled bring-up.

**Next (least-bad, allowed):** RO **who writes `rat_mode` into CNF** on EVDO/L1C
side (values seen) **or** whether stock LTE-only boot ever sends
`CDMA_TIMING_LATCH_REQ` ? still no invent opcodes.

## 2026-09-30: FN_B Present=2 dead; latch needs CDMA IRAT

**Correction ? FN_B event map (not all ?UMTS_MEASURE_CNF?):**

| MMC message | wrapper | Present writer | stores |
|-------------|---------|----------------|--------|
| `MMC_LTEL1_UMTS_MEASURE_CNF` | `0x14c5f28` | `0x14f9108` | no Present=2 STRB |
| `MMC_LTEL1_UMTS_PARTIAL_SEARCH_CNF` | `0x14c6002` | `0x14f91a6` | **Present=1** only |
| `MMC_LTEL1_UMTS_TDD_PARTIAL_SEARCH_CNF` | `0x14c60dc` | `0x14f9416` | Present 0/1/(2) |

**FN_B Present=2 gate (`0x14f9416`, TDD partial-search only):**
- Switch on **`[r7,#0xac]`** (= Phase; logs `Phase0/1`, `step value of Phase`).
- Phase **2** ? `0x14f952c` with `r0==2` ? **`CBZ r0` not taken** ? **Present=0**.
- Alternate CBZ path (`LDRB [PresentObj,#8]==0` then `[r7,#0xb4]!=0` ? Present=2)
  sits in a block with **no live CFG edge** (prior insn is unconditional `B`
  exit; only other inbound is that Phase-2 branch with `r0?0`).
- **Verdict: Present=2 STRB at `0x14f957c` is unreachable** in this build.
  Reachable FN_B outcomes: Present **0** or **1** (? app PIN or PUK), never READY.

**Live UMTS (`radio_tech=3` FDD):** can hit MEASURE / PARTIAL_SEARCH CNF, **not**
TDD-partial Present=2 (dead anyway). Matches soft-lock: tech UMTS + app=PIN.

**`rat_mode` in `CDMA_TIMING_LATCH_CNF`:** filled by L1C/EVDO (or 1xRTT) when
answering `LTEL1_MMC_CDMA_TIMING_LATCH_REQ`. No REQ ? no CNF ? no `+8`.
`LTE_ONLY` (and LTE_WCDMA) do not enable CDMA IRAT latch; string
`L1LC_IratProcLteTimingLatchReq is skip` shows latch can be skipped.
Panther live path never delivered latch CNF with `rat_mode==3`.

**Preferred re-eval:** `0x070a=12` (LTE_WCDMA) already done ? still
`app=PIN`, `tech=3`, `reg=0`, `rmnet` 0/0. New Present semantics do **not**
make another preferred tweak a proven Present=2 enabler (FN_B?2; CDMA latch
needs CDMA IRAT, not WCDMA prefer). **No live probe.**

**Hole:** sole *reachable* Present=2 remains **FN_A** ? CDMA TIMING_LATCH
`rat_mode==3` (IRAT CDMA) ? not exercised on this bring-up.

**Next (least-bad, allowed):** RO whether panther CP ever emits
`LTEL1_MMC_CDMA_TIMING_LATCH_REQ` under any stock preferred/op-mode (or
prove CDMA IRAT absent) ? still no invent opcodes / no PinSkip.

## 2026-09-30: Challenge ? LATCH never on EU; READY model re-opened

**Who sends `CDMA_TIMING_LATCH_REQ`?** L1LC only
(`Send LTEL1_MMC_CDMA_TIMING_LATCH_REQ` ? EVDO / 1xRTT). It is **CDMA IRAT**,
not a SIM/USIM bring-up step. Gated by CDMA meas/IRAT state; can
`L1LC_IratProcLteTimingLatchReq is skip`.

**EU / LTE-only / this SKU:** CP has explicit **`No CDMA in InitRapMap`** /
**`No CDMA in SupportedRatMap`**. With preferred LTE_ONLY or LTE_WCDMA (live),
CDMA IRAT latch **does not emit**. Factory enum *has* CDMA preferred types, but
setting them is **not proven** to add CDMA to SupportedRatMap on panther EU /
Life LTE ? **no live CDMA-preferred probe** (unsafe/unproven vs constraint).

**READY re-audit (challenge to ?only LATCH?Present=2?):**

| claim | re-check |
|-------|----------|
| `SET_APP` READY(#5) | **still sole** site `0x14fb5c6`, gate `LDRB +0xBF6` **CMP #2** |
| `+0xBF6` writers | **still sole** `0x14fb380` (STATUS copy `[PresentObj,#0]`) |
| Present=2 STRB | **only** FN_A `0x14f6a16` (reachable) + FN_B `0x14f957c` (**unreachable**) |
| Thumb16 / other SIM ranges | no extra Present=2; `0x1996738` is `[r4,#0]=0` false positive |
| STATUS arg | wrap `0x14c6626` passes PresentObj; identity with FN_A getobj#4 holds |

**Stock LTE-only PIN-disabled ? READY?** Under this MAIN image: **no AP/SIT path
found**. Present=2 builders are **only** CDMA L1 (`MEAS_RESULT_IND` /
`TIMING_LATCH_CNF` with payload byte **==3**). That is absurd for EU LTE SKUs
where CDMA is absent from SupportedRatMap ? **hole is CP/SKU coupling**, not
misidentified `+0xBF6`. FCP DISABLED / preferred / radio ON still do not write
Present=2.

**Live:** none (CDMA preferred not proven safe/effective; LTE_WCDMA already
negative).

**Hole (challenged, still holds):** READY needs Present==2; on panther EU bring-up
Present=2 never becomes reachable without CDMA IRAT L1. Goal incomplete (no
`rmnet`/IPv4).

**Next (least-bad, allowed):** RO **PresentObj ctor / USIM INSERT init** ? does
any non-L1 path set `[PresentObj,#0]=2` at card-ready (missed addressing)? Or
confirm stock Pixel uses a different CP feature flag that enables a Present=2
builder we lack ? still no invent opcodes.

## 2026-09-30: PresentObj ctor ? no Present=2 outside L1

**PresentObj allocation:** `getobj` `0x20ea040` with **`r1=#0x636c`**,
`r0=#4`, debug tag `L1LC_IratController.c` (`r2` VA). **7** call sites in MAIN
(FN_A + `0x1551*` / `0x1553*` / `0x1a17*` / `0x1a34*` / `0x1a55*`).

**Default / ctor stores to `[PresentObj,#0]`:**
| site | `[+#0]` write |
|------|----------------|
| FN_A `0x14f6a16` | **2** iff arg0==3 else **3** (L1 only) |
| other `0x636c` getobj sites | **no** Present=2; at most `+1`/`+2` clear, or `[+#0]=0` / `=3` |
| FN_B `0x14f957c` | Present=2 **unreachable** (prior) |
| INSERT / `SIM_PRESENT_IND` | MM boolean `SimPresent` ? **not** this enum |
| `PIN_DISABLED` / `SIM_INIT_*` | **no** STRB Present=2 |

**Exhaustive `MOVS #2` + `STRB [*,#0]`:** 970 hits firmware-wide; in PresentObj
family / SIM module only **FN_A** (reachable) + **FN_B** (dead). Nearby
`0x14f900a` is `MOVS#2`+`B` (false positive). No AP-triggerable non-L1 writer.

**Default value:** ctor paths do not set Present=2; live app=PIN fits
**Present left 0 / unset** ? STATUS else?PIN.

**Live:** none (no new AP trigger).

**MODEM-06 hardened:** On this MAIN, **EU LTE-only + PIN-disabled cannot
reach READY**: Present=2 only via CDMA L1 FN_A, and CDMA absent from
SupportedRatMap. Not an AP SIT gap under bans.

**Next (least-bad, still bearer-aimed):** RO factory cold-boot
`BuildSim*` / NET order when stock sees pin1=DISABLED ? any **proven** SIT
we never sent before READY. If none ? AP-terminal under bans (needs CP-side
Present=2 without CDMA IRAT).

## 2026-09-30: cold-boot SIT vs VerifyPin?READY (no CDMA)

### A) Factory cold-boot BuildSim*/NET order vs ours

Proven stock/HAL order after modem ONLINE (`libsitril` /
`sit-stream` builders):

| # | Builder | id | role |
|---|---------|-----|------|
| 1 | `BuildGetRadioState` | `0x0801` | GET |
| 2 | `BuildSimGetStatus` | `0x0200` | GET (post-ONLINE solicit) |
| 3 | `BuildGetPreferredNetworkType` | `0x070b` | GET |
| 4 | `BuildSetPreferredNetworkType` | `0x070a` | SET RAT |
| 5 | `BuildSetNetworkSelectionAuto` | `0x0704` | SET |
| 6 | `BuildNetworkRegistrationState` | `0x0701` | GET poll |
| 7 | `BuildSimVerifyPin` | `0x0201` | **only if** app=PIN and pin1 enabled |

Already sent on bring-up (incl. PIN-disabled soft-lock): 1?6, plus
facility SC `0x0209`, ATR `0x0212`, OpenChannel/`WithP2`, channel APDU,
SlotStatus `0x024d`, card-power `0x024c`, UICC `0x0249`, radio ON,
preferred=11/12. **No builders** for `SIM_INIT_REQ` /
`START_STACK_SERVICES_REQ`.

**Unsent proven command that could flip PIN-disabled ? READY:** **none.**
**Live:** none this turn.

### B) VerifyPin ? SET_APP#5 without CDMA?

**First PIN1 Verification** sites `0x1f0456c` / `0x1f0469a`:
- `STRB Pin1Verified=#20` (=1)
- BL SimInfo-ish `0x1dcb028` / `0x1f1458c`
- **no** BL FN_A / STATUS_WRAP / `+0xBF6` / `SET_APP`

**READY gate re-audit (not falsified):**
| check | result |
|-------|--------|
| `SET_APP` READY(#5) | **sole** `0x14fb5c6`, gate `LDRB +0xBF6` **CMP #2** |
| `+0xBF6` STRB writers | **sole** `0x14fb380` (STATUS copy PresentObj[0]) |
| Direct BL?FN_A | **only** `0x14c3872` (LATCH wrap) + `0x14f6d7a` (MEAS) |
| FN_A VA fptrs | **0** literals |
| STATUS_WRAP BL | L1 dispatch only ? not VerifyPin/USIM |

**Present==2/CDMA writer model:** as a **complete** explanation of stock
Pixel 7 EU READY (PIN entry or PIN-disabled) ? **falsified / incomplete**.
On this MAIN, no LTE-only path from `NS_USIM_VERIFYPIN_RSP` / First PIN1
to Present=2 ? STATUS ? `SET_APP#5`. PIN-disabled cannot use an
?equivalent? of VerifyPin: VerifyPin itself never writes Present=2.

**READY gate itself** remains Present==2 / `+0xBF6==2`. Hole is the
**missing non-CDMA Present=2 writer** (or stock CP image differs) ? not
a second `SET_APP#5`.

**Live / bearer:** none (no READY). Goal incomplete (no `rmnet`/IPv4).

**Next:** RO who writes PresentObj[0]=2 via non-STRB / aliased store /
different object identity; or confirm stock CP ? this MAIN for READY.
Still no invent opcodes / CDMA preferred live under bans.

## 2026-09-30: image match + broader Present=2

### 1) CP image identity

| check | result |
|-------|--------|
| Host `fw/saaios-probe-b-modem.bin` SHA-256 | `449eeab3bf70fc4ed0793dce3a1f245447bf54a23b4e666df9881317bfc2344b` |
| Embedded version string in MAIN | `g5300q-260317-260505-B-15346003` |
| Matches documented modem_b (sda29 RO) | **yes** (same SHA + version) |
| Live phone re-hash (RO mount /tmp probe) | **blocked** ? `192.168.0.104` unreachable, no adb |

RO analysis remains on this B image (bring-up historically hash-gated to the
same SHA before load). Live slot/`modem_a` (A-14784800) re-check deferred
until phone is reachable. Not treated as ?wrong MAIN? without a live mismatch.

### 2) Broader Present=2 search (this MAIN)

| probe | result |
|-------|--------|
| `getobj(#0x636c)` sites | **7**; Present=2 STRB **only** FN_A `0x14f6a16`; one other writes `0` |
| memcpy/memmove size 1/4 after getobj | **none** |
| Thumb reg-offset STRB near FN_A/STATUS/WRAP | **none** |
| BL?MEAS_FN `0x14f6d02` | **sole** caller CDMA wrap `0x14c3986` |
| BL?FN_A | still only LATCH `0x14c3872` + MEAS `0x14f6d7a` |
| Non-CDMA NS ? FN_A with payload/arg==3 | **not found** |
| `No CDMA in SupportedRatMap` | present in this image |

**New writer:** none. Non-CDMA Present=2 path: **not found** ? **no live**.

**Hole (unchanged):** READY gate Present==2 holds; stock EU READY still
unexplained on this MAIN. Goal incomplete (no `rmnet`/IPv4).

**Next:** when phone is up ? RO re-hash active `modem.bin` / loaded probe;
if mismatch, switch RO to live MAIN. Else hunt PresentObj alias / indirect
store beyond Thumb STRB patterns.

## 2026-09-30: link restore + live match (still PIN+DISABLED)

### 1) Device link

| path | result |
|------|--------|
| Wi?Fi `192.168.0.104:22` | down |
| USB NCM host `172.31.7.2` / phone `172.31.7.1` | **up** (ping; TCP 22/38127/38128 open) |
| SSH `root@172.31.7.1` | port open; **pubkey denied** (no matching key here) |
| **COM13** (ACM `VID_1D6B/PID_0104`) | **works** ? shell via `com13.ps1` |
| adb | empty |

**Working path:** COM13 (+ USB NCM for host services). Not Wi?Fi.

### 2) Live CP vs MAIN

| check | result |
|-------|--------|
| `modem_state` | ONLINE |
| `/tmp/saaios-probe-b-modem.bin` ? `/data/saaios/bin/?` SHA-256 | **`449eeab3?`** = host MAIN |
| Version string | `g5300q-260317-260505-B-15346003` |
| Partition `modem_a` (sda19) RO hash (`images/default`) | `57465ab9?` = A-14784800 (**?** running) |
| Partition `modem_b` (sda29) | present (running image is the verified B probe copy) |

**Match:** live loaded CP **is** analyzed MAIN. Not an outdated-image miss.

### 3) Live SIM / bearer

`sit-sim-status query-sim-status`: card PRESENT, apps=1, **app_state=PIN(2)**,
**pin1=DISABLED(3)**, remain=3. Radio ON (`0x0801` raw=10). Data reg
`registration_raw=0` tech UMTS. **rmnet\*** rx/tx all **0**. Soft-lock
unchanged ? no READY ? no bearer chase.

### 4) Stock-EU READY paradox (offline+live)

- Factory HAL does **not** remap PIN+DISABLED ? READY (`GetPinState`/
  `CheckAndAutoVerifyPin` prior; libsitril has READY/PIN/DISABLED symbols
  but no DISABLED?READY policy string path).
- Live SIT byte17/72 are CP truth on this MAIN; HAL would publish the same.
- `modem_a` exists and differs, but **running** CP is B ? paradox is on
  **this** image, not ?wrong slot loaded?.
- Present=2 still only FN_A(CDMA) under RO; EU stock READY remains an
  unexplained CP hole (missing writer / init path), not AP SIT gap.

**Live Present=2 probe:** none (no new AP-triggerable path). Goal incomplete.

## 2026-09-30: READY gate provenance + modem_a Present=2

### 1) Gate ? **confirmed**, not falsified (MAIN B)

STATUS `0x14fb322`: arg `r0` = PresentObj ? `r5`;
`LDRB r7,[r5,#0]` (Present enum); sole `STRB.W r7,[base,#0xBF6]` at
`0x14fb380`. Sole READY: `0x14fb5c6` `LDRB +0xBF6` **CMP #2** ? `SET_APP #5`.
STATUS_WRAP `0x14c6626` passes PresentObj (`LDRB [r4,#0]/[#1]`).
**+0xBF6 is the PresentObj[0] mirror** on the SIM status object base.

### 2) Live Present / +0xBF6 / Pin1Verified

cpif `sim/` only `ds_detect`; `info_region` = srinfo offsets (no Present);
dmesg has no `Present`/`Pin1Verified` lines. **No AP-exported RO** for those
bytes without secrets. Inferred from live `app_state=PIN`: Present?{1,2,3}
after STATUS (else PUK/READY/PERSO).

### 3) modem_a RO (`57465ab9?` / A-14784800) ? pulled via USB httpd

Same gate shape (shifted): `+0xBF6` copy `0x14fc75c` from `LDRB [PresentObj,#0]`;
READY `0x14fc9a2` CMP#2?`SET_APP`. Present=2 only FN_A-ish `0x14f7e10`
(**CMP arg==3**) + FN_B-like; callers **2** (CDMA wrap/MEAS). Strings:
`No CDMA in SupportedRatMap`, `CDMA_TIMING_LATCH`, `CDMA_MEAS_RESULT`.
**No non-CDMA Present=2 path on A.**

### 4) Slot switch

MODEM-ROADMAP bans radio/modem slot switches. **Skipped.** A would not help
EU READY anyway.

### 5) Live / bearer

None (gate holds; no new path). Goal incomplete (no `rmnet`/IPv4).

## 2026-09-30: SIT-boundary falsify ? 0x0200 app_state / RatMap / PIN_SKIP

MAIN B `449eeab3?`. RO only; no live (no proven trigger).

### 1) SIT GetSimStatus `0x0200` app_state byte ? **SET_APP mirror only**

| site | role |
|------|------|
| `0x18ec8c0` GET_APP | sole `LDRB.W [obj,#0xBF4]` ? return (38 callers) |
| `0x1991734` SET_APP | **sole** `STRB.W` to `+0xBF4` in all MAIN |
| `0x19916d2` SET_APP body | reads old `+0xBF4`, stores imm arg, notifies via `0x20e184e` |
| HAL `ProtocolSimStatusAdapter::Init` | `memcpy` packet; byte17 = CP field; **no** pin1/Pin1Verified remap |

**Verdict:** AP-visible `0x0200` app_state@17 = **SET_APP field (`+0xBF4`)**, not
Pin1Verified (`+0xBF5`), not pin1 status, not a TX-time composite. STATUS
`0x106a` path decides the imm passed into SET_APP; TX only mirrors the store.

### 2) PIN-disabled / Pin1Verified ? READY without Present==2?

**No.** SET_APP callers with imm (14 total): sole `#5` at `0x14fb5c6` still
gated `LDRB +0xBF6` **CMP #2**. `#2` PIN sole at `0x14fb406`. Pin1Verified
writers (`0x18e` / obj+20 / `+0xBF5`) never STRB `+0xBF4`. FCP pin1 DISABLED
never BL SET_APP. So neither PIN-disabled nor Pin1Verified alone can place
READY(5) into the `0x0200` response.

### 3) ?No CDMA in SupportedRatMap? ? fixed vs mutable?

**Fixed at CP init (this image).** Strings only under QM_MM /
`RRM_RRC_INIT_REQ_Handler` / `InitRapMap!`. **No** `SetRapMap` /
`UpdateSupportedRat` / `SetSupportedRat`. sit-stream `BuildSetCdma*` =
roaming/hybrid/preferred ? **not** SupportedRatMap. No property string that
writes the map. Prior preferred=12 already negative. **No proven SIT/property
? no live RatMap poke.**

### 4) NS_SIM_PIN_SKIP ? non-eSIM trigger?

**No.** Runtime fail `MOVW r2,#0x11d6` @ `0x172b0e2` (?NOT eSIM?) after
`BL 0x26061d6` with `r1=#2`; callee is stub `STRB [r0,#4]; MOVS r0,#0; BX LR`
? always ?1 ? always NOT eSIM on this MAIN. `0x11ce` Invalid SimState at
`0x21ae106`. Sole caller into PinSkip cluster: `0x1cc0b78`. HAL still has
**no** PinSkip builder. No non-eSIM success path.

### 5) Live / bearer

None. Soft-lock unchanged. Goal incomplete (no `rmnet`/IPv4).

## 2026-09-30: Exhaustive FN_A callers + InitRapMap inputs

MAIN B `449eeab3?`. RO only; **no live** (no non-banned lever).

### 1) Exhaustive Present=2 / FN_A / WRAP_A

**BL?FN_A `0x14f692c`:** exactly **2** sites in all MAIN.

| site | via | MMC switch case (name VA load) | arg?Present=2 | LTE-only arg==3? |
|------|-----|--------------------------------|---------------|------------------|
| `0x14c3872` | WRAP_A `0x14c380e` ? `0x14b7766` | `MOVW r1,#0xec0b`+`MOVT #0x4106` = **`MMC_LTEL1_CDMA_MEAS_RESULT_IND`** | `LDRB [msg,#0]` **==3** | **No** ? needs CDMA MEAS IND |
| `0x14f6d7a` | WRAP_SIM `0x14f6d02` ? case body `0x14c388e` ? `0x14b79e2` | `MOVW r1,#0xefb3`+`MOVT #0x4106` = **`MMC_LTEL1_CDMA_TIMING_LATCH_CNF`** | `LDRB [msg,#8]` (**rat_mode**) **==3** | **No** ? needs CDMA IRAT latch; RatMap has no CDMA |

Sibling switch cases (same dispatcher, **do not** BL FN_A): UMTS_MEASURE_CNF (`#0xeee0`?`0x14c5f28`), UMTS_TDD_PARTIAL (`#0xef38`?`0x14c60dc`), others.

**PresentObj (`#0x636c`) Present=2 store:** sole `0x14f6a14` inside FN_A. Other `MOVS #2; STRB #0` in `0x14f0000..0x1508000` (**6**) have **no** `#0x636c` nearby ? not PresentObj. FN_B body `0x14f9108..0x14f9800` has **zero** `#0x636c` (prior FN_B ?Present=2? is not a PresentObj store on this pass; UMTS path still does not help EU READY).

**HELPER `0x14f6900`:** sole caller `0x14c37f2` ? LDRB return only, not Present=2.

### 2) InitRapMap / SupportedRatMap ? what decides ?No CDMA??

- Logged at **`QM_MM_INIT_REQ_Handler`** (`SupportedRatMap(0x%X)` then optional `No CDMA in InitRapMap!`) and STOP (`No CDMA in SupportedRatMap`). Also `RRM_RRC_INIT_REQ_Handler - SupportedRatMap(%d)`.
- Map is an **input to / field of QM init** (with RoutingInfo / DSDS domains), not written by preferred SIT.
- **No** `SetRapMap` / `UpdateSupportedRat`. `DS_TCS_GV_CDMA_SUPPORT`, `bcProductCode`, `Gmss_IsCdmaSupportedPlmn` exist as **unrelated** strings ? no proven BL/store into InitRapMap.
- **saaios handover** (`sit-handover.h`): 16 CDT/rfid words + identity + 64-byte signature ? **no RAP/RatMap field**. No proven CPIF/handover boot flag changes RAP.

### 3) Live lever?

**None.** CDMA preferred enums exist in sit-stream (`CDMA_EVDO_AUTO`, `NR_LTE_CDMA_EVDO`, ?) but: RatMap has no CDMA on this image; preferred=11/12 already live-negative; CDMA preferred **not** factory-proven to emit LATCH/MEAS on panther EU; ROADMAP bans slot switch. **No experiment run.**

### 4) VerifyPin?Present?

**Not newly found.** VerifyPin still only Pin1Verified (`+0xBF5` / obj+20). **Do not** ask for PIN re-enable as the next step on that basis.

**Next still allowed:** (a) physical SIM tray reseat/other position (manual); (b) deeper RO of **who fills SupportedRatMap into QM_MM_INIT** (RRM/NV image build ? still no EFS RW); (c) different CP/product image that includes CDMA in RatMap ? not a slot switch. Goal incomplete (no `rmnet`/IPv4).

## 2026-09-30: SupportedRatMap population ? CDMA bit never 1 on this EU build

MAIN B `449eeab3?`. RO only; **no live**.

### 1) Who fills SupportedRatMap before QM_MM_INIT?

| stage | evidence |
|-------|----------|
| **QM consumes** | `@QM_MM_INIT_REQ_Handler: SupportedRatMap(0x%X), RoutingInfo?` then optional **`No CDMA in InitRapMap!`**; STOP: **`No CDMA in SupportedRatMap`**. Map is an **inbound field** of INIT/STOP, not computed inside the No-CDMA log. |
| **RRM also logs** | `@RrmFt::RRM_RRC_INIT_REQ_Handler - SupportedRatMap(%d)` ? same map on RRC init path. |
| **GMC sender** | Symbol/name `GmcM_CommSendMmInitReq` @ `0x17bfa10` (body follows); carries init toward QM. No `SetSupportedRatMap` / `UpdateRatMap` / `BuildRatMap` strings. |
| **Product / TCS** | `A[I][[TCS_CDMA_SUPPORT]]` dump (`: 0x%x`); `DS_TCS_GV_CDMA_SUPPORT` registered (GV id cluster `r3=#0x127`); **`@[TCS] GV updated from reg`** ? TCS GVs load from **registry/NV**, not from SIT preferred. |
| **Not sources** | saaios handover (no RAP word); `BuildSetPreferred*` / `BuildSetCdma*` (preferred/roaming only); `CDMA_CAL` / `ATI_CDMA_CAL_*` (cal messages, not RatMap); `bcProductCode` (unrelated log). |

**Chain:** NV/reg TCS + product/RRM capability ? SupportedRatMap on INIT ? QM logs and gates CDMA IRAT. AP cannot write the map under bans.

### 2) Can CDMA bit be 1 on this panther EU build?

**Not without EFS RW / different image.**  
- Only **negative** log strings exist (`No CDMA in InitRapMap!` / `No CDMA in SupportedRatMap`); no `EnableCdmaRat` / `Found CDMA` / positive InitRapMap path.  
- Live boots already hit No-CDMA (prior). FN_A needs CDMA MEAS/LATCH ? unreachable while map lacks CDMA.  
- Flipping `TCS_CDMA_SUPPORT` / `DS_TCS_GV_CDMA_SUPPORT` = **NV/reg write** ? **EFS RW** ? banned. Documented: cal/TCS disable is RO-visible as absent CDMA; we cannot flip it under constraints.

### 3) Non-EFS lever for IRAT latch?

**None proven.** CDMA preferred enums exist but do not add CDMA to SupportedRatMap (preferred=11/12 already live-neg). No live experiment.

### 4) MODEM-06 status ? READY unreachable

**Soft-lock:** `0x0200` app_state=PIN while pin1=DISABLED; READY only via Present==2 ? FN_A ? CDMA L1; **RatMap without CDMA** ? Present=2 never; attach/`rmnet` blocked.

**Next allowed:** (1) **physical SIM tray** reseat/other slot (manual); (2) else **goal blocked pending new evidence** (CP/SKU with CDMA in RatMap, or card/PIN state change outside current bans ? VerifyPin?Present still holds, so no PIN-reenable ask). Goal incomplete (no `rmnet`/IPv4).

### 5) Capabilities exercised (inventory)

| class | items |
|-------|-------|
| **RO MAIN** | SET_APP/`+0xBF4`; Present/`+0xBF6`; FN_A?2 CDMA-only; FN_B not PresentObj; SIT `0x0200` mirror; Pin1Verified VerifyPin-only; PIN_SKIP eSIM stub; RatMap/TCS init; handover no RAP; modem_a same |
| **Live (prior)** | handover ONLINE; `0x0200` PIN+DISABLED; radio ON; reg0; rmnet 0; STATUS/OpenChannel APDU; preferred 11/12; cold sysrq |
| **Not done (banned)** | EFS RW; CDMA preferred; slot switch; PinSkip; PIN spray; invent opcodes |

## 2026-09-30: SET_APP exhaustive + CDMA ID non-alias + AllowData live

### 1) Every BL ? SET_APP `0x19916d2` / sole STRB `+0xBF4`

MAIN scan: **14** BL sites. Sole `STRB.W #+0xBF4` = `0x1991734` inside SET_APP.

| BL site | imm ? app_state | PresentGate (`LDRB +0xBF6` ? `CMP #2`) |
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
| String VA | `MMC_LTEL1_CDMA_MEAS_RESULT_IND` = `0x4106ec0b`; `?CDMA_TIMING_LATCH_CNF` = `0x4106efb3` ? unique; no `LTE_MEAS_RESULT_IND` / `NR_MEAS_RESULT_IND` names |
| MMC case lo16 | `0xec0b` ? sole case @`0x14b7752` ? BL WRAP_A `0x14c380e`; `0xefb3` ? sole @`0x14b79ce` ? BL LATCH `0x14c388e` |
| Sibling lo16 | UMTS `0xeee0` ? `0x14c5f28`; UMTS_TDD `0xef38` ? `0x14c60dc` (different handlers) |
| WRAP callers | BL?WRAP_A **1**; BL?LATCH_body **1**; BL?WRAP_SIM **1** |
| Non-0x4106 MOVT for those lo16 in MMC | **none** |

**Verdict:** MEAS/LATCH log IDs on this MAIN bind only to FN_A CDMA wraps ? not reused for LTE/NR cases.

### 3) Live COM13/USB ? GMC/MM/NAS + missing NET

ONLINE (`uptime` ~3.5h). `info_region` = srinfo/capability **offsets only** (no Present). dmesg: no GMC/MM/NAS string hits (CP logs not in kmsg).

| probe | result |
|-------|--------|
| `0x0200` | app=PIN(2) pin1=DISABLED(3) apps=1 |
| `0x0801` | radio=10 ON |
| `0x0701` data | reg=0 tech=3 UMTS |
| `0x0700` voice | **reg=3** tech=3 UMTS (MM answers; not silent) |
| `0x0702` operator | plmn_present=1 |
| rmnet0/1/2 | 0/0, no IPv4 |

Factory cold-boot `0x0801?0x0701` already exhausted earlier. Missing proven NET after that: **`BuildAllowData`** id **`0x0710`**, length 13, byte12=1 when allow==1 (sit-stream `0x74df0`). Never sent before.

**ONE send (COM13):** `0x0710` allow=1 ? response length 12, **error 0**. Recheck: still PIN+DISABLED; data reg=0; voice reg=3; rmnet 0/0.

`BuildSetupDataCall` / IMS PDU still need APN/READY ? not sent (no invent APN). `0x091b` / `0x0615` already done prior boots.

**GMC/MM/NAS:** not directly visible in srinfo/kmsg; voice reg + operator PLMN prove registration/NAS path is **alive** under PIN soft-lock. AllowData accepted ? NET PS allow path also alive. Neither clears Present/READY.

### 4) MODEM-06 status

Soft-lock **holds**: sole READY=`SET_APP#5`?Present==2; Present=2 only FN_A CDMA; IDs not aliased; RatMap no CDMA; EFS banned. Live NET gap (`0x0710`) closed ? no `rmnet`/IPv4. Goal incomplete.

**Next remote (falsify MODEM-06, ? tray only):** RO listen on **`umts_dm0` / `oem_ipc*`** for live log lines `CDMA_MEAS_RESULT_IND` / `CDMA_TIMING_LATCH_CNF` / Present / SET_APP while radio ON + preferred LTE ? if Present?2 or SET_APP#5 fires **without** those CDMA IND, FN_A-only model is false. Fallback: map PresentObj into any RO-mapped CP shm/info window and read `+0xBF6` live.

## 2026-09-30: DM/OEM listen + voice/PIN paradox + no PDN

Live COM13, radio ON (`0x0801`=10), preferred set LTE_ONLY (`0x070a`=11, get=11).

### 1) RO listen `umts_dm0` / `oem_ipc*`

Proven open: `mknod` from `/sys/class/cpif/*/dev`. `umts_dm0` (493:4) blocking `dd` returns **EOF immediately** (0 bytes; `timeout 40` does not wait). Same for `oem_ipc0/1` (and later nodes). No ASCII `CDMA_MEAS_RESULT_IND` / `CDMA_TIMING_LATCH` / `SET_APP` / `No CDMA`. Matches earlier project note: `umts_*` poll yields no data without a DM-enable protocol (not invented). **No correlation** with Present/app_state: `0x0200` stayed PIN through the window.

### 2) SetupDataCall / APN ? not sent

`BuildSetupDataCall` id **`0x0600`**, length **983**, gated by `isValidPdpApn(PdpContext)`. APN comes from RIL `ApnSetting` / `TelephonyProvider` carrier DB ? **no** lifecell/default APN string in `sit-stream`/`libsitril`. Null-APN helpers exist for P-CSCF type only, not a proven empty Setup. **No guess, no send.**

Proven empty PDN inventory: `BuildGetDataCallList` id **`0x0602`**, length 12. After drain: response length 13, error 0, payload implies **0 calls**. Still no `rmnet`/IPv4.

### 3) Re-GetSimStatus after listen

app=PIN(2) pin1=DISABLED(3); data `0x0701` reg=0 tech=3; voice `0x0700` **reg=3** tech=3 (reject 0); preferred 11; rmnet0/1/2 0/0.

**Paradox:** CS/MM answers voice-reg=3 + prior PLMN present while UICC app_state is PIN. Data domain stays 0. SET_APP#5 still not taken. Raw byte 12 not yet mapped to AOSP HOME vs DENIED.

### 4) MODEM-06

Holds. Goal incomplete.

**Next remote ? tray:** decode factory **voice `registration_raw` enum** (is 3 HOME or DENIED) on this SIT adapter ? if HOME, NAS CS attaches without READY (falsifies ?READY required for MM?); if DENIED, paradox is reporting-only. Parallel: map PresentObj `+0xBF6` in any RO CP shm/srinfo window.

## 2026-09-30: voice raw=3 is DENIED, not HOME

Opcode: voice GET is **`0x0700`** (`BuildNetworkRegistrationState(1)`), not `0x0701`. Data is `0x0701` domain 2. Both adapters read **packet byte 12** then the **same** convert at `sit-stream` `0x47620` (`ProtocolNetVoiceRegStateAdapter::GetRegState` / Data jumps there). Invalid voice packet ? RIL 4; invalid data ? 0.

Convert: `raw==0` ? RIL **0**; else jump table `raw-1`. Mapped:

| SIT byte12 | RIL GetRegState | AOSP `RegState` |
|------------|-----------------|-----------------|
| 0 | 0 | NOT_REG_MT_NOT_SEARCHING |
| 1 | 1 | REG_HOME |
| 2 | 2 | NOT_REG_MT_SEARCHING |
| **3** | **3** | **REG_DENIED** |
| 4 | 4 | UNKNOWN |
| 5 | 5 | REG_ROAMING |

Cite: HAL `GetRegState` `0x475f0`/`0x47c50` + table `.rodata` `0x2a4a0`; AOSP `RegState.aidl` (DENIED=3, HOME=1).

**Live (COM13):** voice `0x0700` raw=3 reject=0 tech=3 ? **DENIED**, not registered. Data `0x0701` raw=0 ? **not registered, not searching**. SIM still PIN+DISABLED. rmnet 0/0.

Voice vs data under PIN: CS domain **tried and was denied**; PS **never searched**. Matches factory: attach/data wait on app_state READY. PLMN-present earlier is not HOME. **No PDN chase** (not HOME/ROAM). SetupDataCall still unsent.

Present `+0xBF6`: not in `info_region`/sysfs (offsets only). No live Present read without new RO shm map.

**MODEM-06 holds.** Goal incomplete.

**Next remote ? tray:** factory path **CS REG_DENIED + reject_cause=0 while pin1 DISABLED** (does MM deny because app?READY?). Or RO-map PresentObj `#636c` `+0xBF6` in CP shm.

## 2026-09-30: attach gated on SIM READY ? DENIED is local

### 1) Gates (AP HAL + CP SIT)

**HAL:** `NetworkService::OnSimStatusChanged` `libsitril` `0x13fc80`: continues slot path when arg0==1; READY transition only if stored app **or** new app **==5** (`0x13fd28` `cmp w8,#5` / `ccmp w20,#5`). PIN(2) does not enter `0x13ff3c`. No PS attach/APN from this path.

**CP SIT NET:** log `START_NETWORK Ignored: SIM is not ready` (also `Stop other stack ? SIM is not ready`). Radio SIM enum in sit-stream: `SIT_PWR_RADIO_SIM_STATE_SIM_NOT_READY` vs `_SIM_READY`. CP `GET_APP` @ `0x18ec8c0` then `CMP #1/#4/#5` @ `0x18e831e..0x18e832e` ? PIN(2) is **not** in that set ? start-network skip. Present `+0xBF6` is the SET_APP#5 gate, not this CMP list.

### 2) Voice DENIED + reject=0

`GetRejectCause` = packet **byte 13**. Live 0 = **no NAS cause IE** (not 24.008 cause from the network). Local/internal deny. CS still published REG_DENIED; PS stayed NOT_SEARCHING ? consistent with START_NETWORK ignored for PS.

### 3) Present live

`info_region` still offsets only. No proven debug dump of PresentObj `+0xBF6`. Inference unchanged: app=PIN ? SET_APP#5 not taken ? Present?2.

### 4) No extra SIT

RadioPower already ON. START_NETWORK is internal; sending another 0x0800 would not clear SIM-not-ready. No proven ?start PS search? opcode independent of READY.

**Live recheck:** PIN+DISABLED; voice 3/reject 0; data 0; rmnet0 0/0.

**MODEM-06 confirmed** for PS/rmnet. Goal incomplete.

**Next remote ? tray:** name the function around `0x18e831a` vs `START_NETWORK Ignored` (log-id xref ? strings not MOVW-bound). Or RO-map Present `#636c+0xBF6` in CP shm. Still not tray-only.

## 2026-09-30: app_state 1/4 are not PIN-disabled READY

Factory `covertAppStateToString` / sit-sim-status: **1=DETECTED**, **4=SUBSCRIPTION_PERSO**, **5=READY**. AOSP same. 4 is perso lock, not pin1 DISABLED.

### SET_APP stores (sole sites)

| imm | name | BL site | condition |
|-----|------|---------|-----------|
| 1 | DETECTED | `0x146aaba` | early detect (`LDRB [r8,#1]`); **no** `+0xBF6` gate |
| 4 | SUBSCRIPTION_PERSO | `0x14fb526` | `LDRB +0xBF6` **CMP #3** (Present==3) then `#4` |
| 5 | READY | `0x14fb5c6` | `+0xBF6` **CMP #2** |

No other SET_APP #1/#4. FCP DISABLED / Pin1Verified still do **not** BL SET_APP 1 or 4.

### START_NETWORK gate `0x18e831a`

`GET_APP` then allow **only** `{1,4,5}`: `CMP #1` BEQ continue; else `#4` BEQ; else `#5`; else `BNE.W` ignore (`START_NETWORK Ignored: SIM is not ready`). PIN(2) denied. 1 and 4 are ready-**equivalent for start-network only**, not substitutes for PIN-disabled.

### Hole vs stock

PIN-disabled EU stock should be **#5**, not #1/#4. #4 needs Present==3 (CDMA FN_A else). #1 is insert DETECTED, not DISABLED. Missing: SET_APP#5 without Present==2. **No AP SIT** to 1 or 4 (inventing perso/detect banned). No live probe.

**MODEM-06 holds.** Goal incomplete.

**Next ? tray:** in `0x14fb3xx` cluster, who should skip SET_APP#2 when pin1==DISABLED and go #5 (or any non-CDMA Present=2). Or RO-map `+0xBF6`.

## 2026-09-30: STATUS `0x14fb3xx` ? no pin1=DISABLED skip of SET#2

Prolog `0x14fb322`. Gate: `GET_APP` `0x18ec8c0`; **CMP #1** `0x14fb344` BEQ continue; else **CMP #4** `0x14fb34c` **BNE.W ? `0x14fb4ca`** (APP 6/7). PIN(2) **skips** Present copy / SET 3/2/4/5 on later STATUS.

### CMP order (taken when app ? {1,4})

1. `LDRB [r5,#0]` Present ? **STRB +0xBF6** `0x14fb380` (sole copy).
2. `LDRB +0xBF6` `0x14fb3a8`; **CBZ** `0x14fb3ac` ? **`0x14fb404` SET_APP#2 PIN** if Present==0.
3. Else `LDRB +0xBF6` `0x14fb3d2` **CMP #1** `0x14fb3d6`: EQ ? `#3` PUK `0x14fb3de`; **BNE.W ? `0x14fb4f8`** (skips PIN).
4. After PIN only: `LDRB [r5,#1]` Pin1Verified ? **STRB +0xBF5** `0x14fb432`.
5. `LDRB [r4,#1]` then **`LDRB [r4,#3]` CMP #0** `0x14fb4a0`?`0x14fb4a4` (**BEQ.W** if pin==0). **No CMP #3.** **After** SET#2.
6. `+0xBF6` **CMP #3** `0x14fb520` ? SET#4 PERSO `0x14fb526`.
7. `+0xBF6` **CMP #2** `0x14fb5c0` ? SET#5 READY `0x14fb5c6`. **No** pin1 operand.

**pin1==DISABLED(3) does not skip SET#2 and does not take #5 if Present?2.** Present==0 ? PIN; Present==1 ? PUK; Present?{2,3} ? PERSO/READY. FCP log `RemainingAttemptCount ? Pin Status Disabled in APP FCP so changing PinStatus as PIN_DISABLED` (`0x44d0e936`, DBT `0x7ab`) is **PinStatus only** ? not in this CMP chain.

Strings: `PIN_DISABLED` (ATI + FCP). No `PIN_NOT_REQUIRED` / `PIN not required` on this image. No missing **BL** to fire via SIT.

**Missing CMP (hole vs stock):** before `MOVS r0,#2` `0x14fb404`, there is **no** `CMP pin1,#3` / branch to `MOVS r0,#5` `0x14fb5c4`. Post-PIN check is **CMP #0** at `0x14fb4a2`, not DISABLED.

### Present object (STATUS)

Incoming `r5`: `[0]`=Present, `[1]`=Pin1Verified. PresentObj `#636c`: `+0xBF6`/`+0xBF5` copies; app `+0xBF4` via SET_APP. USIM `r4`: `[1]` verified, `[3]` pin status (0 vs ?0 only).

**MODEM-06 holds.** Goal incomplete. No live.

**Next ? tray:** RO-map PresentObj `+0xBF6` live (expect 0 under PIN). Or FCP `0x7ab` writer vs `r4+3` ? still not a SIT skip.

## 2026-09-30: live Present/+0xBF6 ? no AP peek; infer Present==0

### 1) Live RO attempts (COM13, ONLINE)

| source | result |
|--------|--------|
| `0x0200` | card PRESENT; **app=PIN(2)**; pin1=DISABLED(3); remain 3 |
| `info_region` | text offsets only (`srinfo_offset`?) ? **no** Present/`+0xBF6` |
| `legacy/region` | FMT queue offsets ? **no** Present |
| `cpif/sim/` | `ds_detect=2` only |
| dmesg / debugfs | no Present/Pin1Verified/SET_APP lines |
| radio | `0x0801` state=10; data `0x0701` reg=0 tech=UMTS(3); voice raw=3 DENIED; **rmnet0 0/0** |

**No proven CP memory peek / shm map for PresentObj `#636c`.** Mirrored non-secret SIT fields do not include Present/`+0xBF6`/`+0xBF5`.

### 2) Inference (control-flow, not peek)

STATUS only copies Present?`+0xBF6` and can SET#2/#3/#4/#5 when **GET_APP ? {1,4}**. Live app=PIN ? that transition already fired: **Present==0** ? CBZ `0x14fb3ac` ? SET#2. **Not** Present?{1,2,3} (would be PUK/READY/PERSO). Later STATUS while PIN: BNE.W `0x14fb34e`?`0x14fb4ca` (APP 6/7 only; else exit `0x14fb616`) ? **no** re-copy Present, **no** SET#5 even if Present later became 2.

Expect: Present==0, `+0xBF6==0`, Pin1Verified unset for READY path. Matches soft-lock.

### 3) DETECTED(#1) ? START_NETWORK race?

START_NETWORK allows `{1,4,5}`. Sole SET#1 `@0x146aaba` (fn prolog `0x146a99c`). **Proven live already:** card-power `0x024c` 4?1 error 0; RadioPower OFF?ON ? app stayed **PIN**, never observed DETECTED. No AP SIT writes SET_APP#1. **RO: race not reachable from current PIN** without unproven CP reload/tray. **No live race this turn.**

### 4) MODEM-06 hole (hardened)

1. Missing `pin1==DISABLED` ? skip SET#2 / READY (`0x14fb404` / `0x14fb5c4`).
2. Present stuck **0** without CDMA FN_A (sole Present=2).
3. Once PIN, STATUS **cannot** promote to READY on Present change.

**Next remote ? tray-only:** (a) `listen-sim` during **proven** card-power 4?1 ? catch any brief DETECTED/ABSENT bytes (timing only; prior endpoints stayed PIN); (b) RO ABSENT?SET_APP clear / re-entry to `#1`; (c) manual tray reseat if (a)(b) negative. Still no EFS/CDMA-preferred invent.

**MODEM-06 holds.** Goal incomplete (no rmnet/IPv4).

## 2026-09-30: live CardPower race ? no DETECTED window for START_NETWORK

### Method (ONE)

Fast `0x0200` poll on `umts_ipc0` around proven `0x024c` **4?1** (CardPower DOWN?UP); on `app==DETECTED(1)` pulse `0x0800` Radio ON. Follow-up dense poll with `timeout` reads after UP.

### Timeline (non-secret)

| phase | observation |
|-------|-------------|
| pre | card=PRESENT apps=1 **app=PIN(2)** pin1=3 |
| after DOWN (err0) | card=PRESENT **apps=0** app=n/a ? **not** ABSENT, **not** DETECTED |
| after UP / dense | several `0x0200` with **card=0 apps=0** (ABSENT); **never app=1** |
| Radio ON pulse | rsp err=2 once (already ON / reject); later ON attempted |
| end `0x0200` | PRESENT apps=1 **app=UNKNOWN(0)** pin1=3 |
| `0x0701` | reg=0 tech=3; **rmnet0 0/0** no IPv4 |

**RESULT:** `saw_detected=0`. ABSENT seen in follow-up poll; DETECTED **never**. UNKNOWN?`{1,4,5}` ? START_NETWORK still blocked.

### RO ABSENT ? SET#1

SET#1 fn `0x146a99c` / store `0x146aaba`. Callers **2**: `0x14538e4`, `0x1a2abf8`. SET_APP **#0** sites include `0x1a2ac00` (same cluster as caller2) and `0x14c65e2`/`0x14c6db6` ? ABSENT clears toward **0**, not a guaranteed published DETECTED(1) on `0x0200`. No AP SIT ? SET#1.

**Verdict:** CardPower race to catch DETECTED for START_NETWORK is **not workable** on this MAIN/live path (window absent or shorter than poll; published states are apps=0 / ABSENT / UNKNOWN). Hole unchanged: missing DISABLED?READY + Present=0 without CDMA + PIN/UNKNOWN skip STATUS READY.

**Next ? tray-only:** RO who publishes SET#1 vs SET#0 on PRESENT after ABSENT (timing vs `0x0200`); or tray reseat if need cold INSERT. Goal incomplete.

## 2026-09-30: restore + SET#1 callers ? no stable AP DETECTED

### 1) Restore

After race UNKNOWN, live recheck (no extra SET): **ONLINE**, `0x0200` PRESENT apps=1 **PIN(2)** pin1=3 remain=3; radio=10; data reg=0 tech=3; rmnet 0/0. Usable known app_state restored spontaneously / already.

### 2) RO SET#1 graph (stable event?)

| node | role | callers |
|------|------|---------|
| SET1FN `0x146a99c` ? SET_APP#1 `0x146aaba` | sole DETECTED store | **only** `0x14538e4`, `0x1a2abf8` |
| fn `0x14535a2` | SIM event path (`LDRB [r9,#1]` vs 12/14/16; GET_APP PIN/PUK) | **only** `0x1432d0a` |
| fn `0x1a2aac8` | clears SET_APP#0 then may SET1FN | **only** `0x1a4239e` |
| `0x1a421ae` | CP init table (MOVS 21/24/25/?/44 + many BLs) | **only** `0x1a42684` |

**No SIT/AP opcode** in this graph. DETECTED is CP-internal insert/init only. CardPower live already: no DETECTED on `0x0200`.

### 3) Can we keep DETECTED?

**No** under soft-lock: STATUS while app?{1,4} + Present==0 ? SET#2 PIN. Even a stable SET#1 is overwritten. START_NETWORK allows #1 but published state does not stay #1.

**No live DETECTED hold / START_NETWORK attempt** (no AP trigger; keep impossible).

### 4) MODEM-06

Holds. Hole: missing DISABLED?READY; Present=0 without CDMA; DETECTED not AP-reachable/keepable.

**Next ? tray-only:** physical tray only if seeking cold INSERT evidence; else blocked pending new CP/SKU evidence (non-CDMA Present=2 or DISABLED skip). Goal incomplete (no rmnet/IPv4).

## 2026-09-30: cold INSERT dense race ? no DETECTED (pin_disabled_identical)

Proven path: `sysrq` ? CPIF reload (`cpif` SHA prefix `8cdd21d771189af0`) ?
`probe-handover` ONLINE (`result=0`). Hold from first ONLINE densified to
**200 ms** `0x0200` for 45 s; **RadioPower ON** as soon as SIT accepted the
first exchange.

Observed transitions on `0x0200` (no secrets):

| window | published |
| --- | --- |
| early (~27 samples) | card ABSENT, apps=0, no app_state |
| token?29 onward | PRESENT + app **PIN(2)** + pin1 **DISABLED(3)** |
| DETECTED(1) | **never** (`saw_detected=0`) |
| UNKNOWN / READY | never |

`HOLD RESULT`: `app_state=2(PIN) pin1=3 ready=0 radio_on=1 saw_absent=1
saw_unknown=0 saw_detected=0 saw_pin=1 saw_ready=0
pin_disabled_identical=1`. `rmnet0` rx/tx **0/0**, no IPv4.

**Verdict:** cold INSERT race failed the same way as CardPower ? DETECTED is
not published (or shorter than 200 ms and not holdable). AllowData /
0700/0701 path never armed. **MODEM-06 blocked** pending physical tray reseat
or new CP/SKU evidence (non-CDMA Present=2 / DISABLED?READY). Goal incomplete.

## 2026-09-30: A/B MAIN re-scan + no CP DRAM poke ? MODEM-06 hardened

### 1) Other MAIN images (host `diagnostics/fw/`)

| image | identity | sole `SET_APP#5` | preceding CMP at READY |
| --- | --- | --- | --- |
| A `saaios-probe-a-modem.bin` | A-14784800 / `57465ab9?` | `0x14fc9a2` | **only CMP #2** (`0x14fc97a`, `0x14fc99c`) = Present==2 |
| B `saaios-probe-b-modem.bin` | B-15346003 / `449eeab3?` (live) | `0x14fb5c6` | **only CMP #2** (`0x14fb59e`, `0x14fb5c0`) = Present==2 |

**Missing on both:** any `CMP pin1,#3` (DISABLED) ? `SET_APP#5` without the
Present==2 gate. Slot switch to A is ROADMAP-banned and would not add a
DISABLED?READY path.

### 2) CP DRAM peek/poke / force Present or SET_APP

In-repo search: **no** proven CP DRAM peek/poke tool previously used for
Present/`+0xBF6` or SET_APP. EXPERIMENTS: `/dev/mem` ENXIO; no arbitrary
register pokes. `info_region` = srinfo offsets only. **No one-shot force
Present=2 / app_state=5 experiment** (tool does not exist under constraints).

### 3) Live poll (tray reseat check)

ONLINE; `0x0200` still card PRESENT, app **PIN(2)**, pin1 **DISABLED(3)**;
`rmnet` 0/0; no IPv4. No reseat observed.

### 4) Tooling for next physical tray

`sit-sim-status tray-watch`: dense `0x0200` (200 ms / 60 s), logs every
`card/apps/app/pin1` transition plus **Present inference** from published
app_state (READY?was_2, PUK?was_1, PERSO?was_3, PIN?notin_1_2_3;
DETECTED/UNKNOWN/ABSENT ? n/a). `0x0200` never carries `+0xBF6` itself.

**MODEM-06 remains blocked** for normal SIT. Goal incomplete.

## 2026-09-30: tray-watch 180s ? no reseat (still PIN+DISABLED)

Live ONLINE. `/data/saaios/bin/sit-sim-status tray-watch` for **180 s**
(dense `0x0200` ~200 ms):

- One transition only: card PRESENT, app **PIN(2)**, pin1 **DISABLED(3)**,
  `present_infer=notin_1_2_3`
- `saw_absent=0 saw_detected=0 saw_ready=0 pin_disabled_identical=1`
- End poll identical; `rmnet` 0/0; no IPv4

No chase (Radio/LTE/reg) ? state never left PIN.

**Operator (when physical tray available):**
```
rm -f /run/saaios-sit-status.lock
/data/saaios/bin/sit-sim-status tray-watch | tee /data/saaios/var/tray-watch.log
```
Pull/reseat during the window. On `ABSENT?PRESENT`, `DETECTED`, `READY`,
or pin1?DISABLED ? immediately Radio ON ? preferred LTE ? auto-select ?
reg ? rmnet/IPv4. Mode exits early on those edges (180 s max).

Goal incomplete.

## 2026-09-30: no CPIF Present poke; RO surface peek only

### Write paths (kernel s5300 + live sysfs)

| surface | R/W | role vs Present/SET_APP |
| --- | --- | --- |
| `sim/ds_detect` | RW | Dual-SIM mailbox **module param** only (1/2); **not** Present enum |
| `do_cp_crash` | WO | banned |
| boot/ipc ioctls | ? | no SIM Present / `SET_APP` writer |
| debugfs `cpif` | ? | **absent** on live panther |
| mailbox / united_status | IRQ/ctrl | ds_det bit at bring-up; not `+0xBF6` |

**No safe poke** to Present=2 or app_state=5 ? no experiment.

### Peek (live `peek-present-surfaces`)

ONLINE. `info_region` = srinfo/capability/united_status offsets/ctrl only.
`legacy/status` = FMT/RAW ring heads. No iod `mmap` in s5300 sources;
`/dev/mem` not used (prior ENXIO). PresentObj/`+0xBF6` stays in CP heap ?
**no AP RO map** to read the byte. Concurrent tray-watch:
`present_infer=notin_1_2_3` (PIN) ? Present?{1,2,3}.

### Future poke gap

Needs a new CPIF RO export of PresentObj or a reviewed CP debug channel ?
neither in-tree today. Tool: `sit-sim-status peek-present-surfaces`.

Goal incomplete (no `rmnet`/IPv4).

## 2026-09-30: modified MAIN load ? refused (superseded)

Earlier note: probe SHA-256 alone blocked any modified image. That gate is
**ours**, not OEM. Superseded by the live READY-patch attempt below.

## 2026-09-30: READY-patch MAIN COPY ? live load aborted at DONE

### 1) Hash gate = our probe only

`cp-boot-probe.c` before any hardware op:

- Soft SHA-256 of `/tmp/saaios-probe-b-modem.bin` must be stock
  `449eeab3?` **or** READY-patch COPY `193e4f48?`, else die
  (`B firmware hash mismatch`).
- `IOCTL_REQ_SECURITY` is **defined** in `cp-boot-probe-support.inc` and
  **never issued** on the complete/handover path (POWER_ON ? START ?
  HANDOVER ? preamble ? UDL stages ? FIN ? COMPLETE only).

### 2) Patched COPY (not radio partition)

- Site: MAIN file offset `0x14fb404` Thumb `MOVS r0,#2` (PIN) ?
  `MOVS r0,#5` (READY) at STATUS CBZ/SET#2 (Present==0 path).
- TOC MAIN `crc=0xb0f14905` left unchanged (algo unknown).
- Assets: `.STOCK` / `.PATCHED` under `/data/saaios/bin/`; host script
  `tmp-make-patched-main.py`.

### 3) Live result (sysrq ? CPIF ? handover)

| step | result |
| --- | --- |
| sha accept | **OK** (`193e4f48?`, dual-hash probe) |
| POWER_ON / START / HANDOVER / preamble | **OK** |
| UDL MAIN BIN (46157 chunks) | **ACK** |
| UDL MAIN CRC `0xb0f14905` | **ACK** (`0xc320`) |
| UDL MAIN DONE | **FAIL** ? req `0xa12d`, timeout waiting `0xc12d` |
| modem_state | stuck **BOOTING** (never ONLINE) |
| restore | stock `449eeab3?` restored immediately; sysrq recover |

No COMPLETE, no `0x0200`, no Radio/LTE/rmnet chase (boot never ONLINE).

### Verdict

OEM integrity **binds after POWER_ON** at **UDL MAIN DONE** (post-CRC),
not at our SHA gate. One-byte MAIN body change ? DONE reject. Further
unsigned MAIN patches **aborted**. Stock B restored on device. MODEM-06
firmware hole remains. Goal incomplete (no rmnet/IPv4).

## 2026-09-30: stock ONLINE restore + tray-watch ?5 min

### Stock reload

After DONE abort: `fw_sha=449eeab3?` enforced; sysrq ? CPIF ? handover.
`COMPLETE: OK`, `modem_state=ONLINE`. Live `0x0200` (non-secret):
PRESENT, app=PIN, pin1=DISABLED(3). `rmnet*` ifaces exist, **no IPv4**,
operstate down, rx/tx 0.

### Tray-watch ?5 min

`tray-watch-5m.sh`: two dense 180 s rounds (window ?320 s). No physical
reseat observed:

- both rounds: `saw_pin=1 saw_ready=0 saw_detected=0 saw_absent=0`
- `present_infer=notin_1_2_3`, `pin_disabled_identical=1`
- end still ONLINE + PIN+DISABLED; no bearer chase

### MODEM-06 levers

Unsigned MAIN **dead** (DONE bind). Allowed next: physical tray reseat
during tray-watch, or new RO evidence not already closed (no AP Present
poke / no unsigned MAIN). Goal incomplete.

## 2026-09-30: CP DRAM poke after ONLINE ? mapping missing

Question: can AP write running MAIN / PresentObj after stock ONLINE
(without unsigned MAIN reload)?

### Live surfaces (ONLINE, stock `449eeab3?`)

| surface | access | role |
| --- | --- | --- |
| `info_region` / `legacy/*` | **RO** text | srinfo/ring offsets only ? no PresentObj |
| `sim/ds_detect` | RW | Dual-SIM mailbox param only ? **not** Present |
| `do_cp_crash` | WO | banned |
| `/dev/mem` | **absent** | prior ENXIO; node missing on this boot |
| `/sys/kernel/debug/cpif` | **absent** | no debugfs export |
| CPIF iod (`umts_boot0/ipc0/rfs0`) | char RDWR open OK | **no `.mmap`** |

### Empiric mmap probe

Tool: `diagnostics/cp-dram-mmap-probe.c` (`probe-mmap-only`).
On live ONLINE: open RDWR ok; `mmap(PROT_READ|PROT_WRITE)` and RO ?
**ENODEV** for 4K/64K/1M on boot0/ipc0/rfs0. dm0/oem_ipc0 nodes not
created. `cpif.ko` bootdump fops strings: open/read/write/ioctl/poll/
release ? **no mmap**. Kernel-internal `vss_base` / `cp_shmem_get_*` are
not userspace-mapped.

`sit-sim-status peek-present-surfaces` (RO): same GAP ? PresentObj/`+0xBF6`
in CP heap; no AP map.

### Verdict

**Not writable.** Missing: CPIF userspace mmap (or reviewed debug export)
of MAIN VA / PresentObj heap. **No poke attempted** (SET#2 / Present=2 /
`+0xBF4`). Short tray-watch: no edge. Safety: stock reload path
(`stock-online-bringup.sh` / sysrq?handover) remains. Goal incomplete.

## 2026-09-30: why mmap?ENODEV; kernel gap (next impl task)

### Why userspace `mmap` returns ENODEV

Kernel source tree (local `s5300-src/`, Google radio/samsung/s5300):

1. `bootdump_io_device.c` `bootdump_io_fops` (?665?674):
   `open/release/poll/unlocked_ioctl/write/read` ? **no `.mmap`**.
2. `ipc_io_device.c` `ipc_io_fops` (?484?493): same ? **no `.mmap`**.
3. Repo-wide: **zero** `remap_pfn_range` / `.mmap` / `vm_ops` under
   `s5300-src/`. Linux default when `f_op->mmap == NULL` ? **`-ENODEV`**.
4. Live confirm: `cp-dram-mmap-probe` open RDWR OK, mmap RW/RO ? ENODEV.

### What a map would need (and still may not poke READY)

| AP kernel window | after ONLINE | contains SET#2 / PresentObj? |
| --- | --- | --- |
| `mld->boot_base` (SHMEM_CP) | **vunmap'd** at START (`link_device.c` ?2289?2294: ?boot_base is in no use?) | N |
| PCIE boot load dst | `mld->base + boot_img_offset` in **SHMEM_IPC** only for BOOT (`link_load_cp_image`, `LINK_ATTR_XMIT_BTDLR_PCIE`) | N ? MAIN arrives via SIT UDL PCIe, runs in **CP DRAM** |
| `mld->base` / VSS / rings | kernel-mapped; sysfs `info_region`/`legacy` RO text only | N ? IPC/VSS, not PresentObj `#636c` heap |
| `/dev/mem`, debugfs `cpif` | absent on live panther | ? |

Even a new `iod->mmap` of SHMEM_IPC/VSS would **not** expose running MAIN
Thumb `@0x14fb404` nor PresentObj/`+0xBF6` in CP heap. READY poke needs a
**CP-DRAM** (or CP debug) path that does not exist in-tree.

### Rebuild this turn?

`s5300-src/Makefile` needs full `KERNEL_SRC` +
`private/google-modules/soc/gs` symbols. Phone ships stock
`/lib/modules/cpif.ko` (no SaaiOS out-of-tree rebuild path for CPIF).
**Not buildable/deployable this turn** ? no mmap patch shipped, no poke.

### Other live write primitives checked

- `bootdump_ioctl`: POWER_*/LOAD_CP_IMAGE/START/COMPLETE/HANDOVER/?
  ? no copy_to_cp_arbitrary / no DRAM poke.
- `boot_base` gone after START; LOAD after ONLINE not a READY poke.
- `sim/ds_detect` RW ? Present; `do_cp_crash` banned; no `cp_btl` node;
  `logbuffer_cpif` misc log only.
- Unsigned MAIN reload still **dead** (UDL DONE).

### Next implementation task (exact gap)

Add a **reviewed** CPIF userspace export that can R/W the CP region
holding PresentObj or STATUS SET_APP sites ? or a CP debug channel ?
then ONE poke + STATUS refresh. Until then: tray only. Short poll:
still PIN+DISABLED, no rmnet IPv4. Goal incomplete.

## 2026-09-30: boot_base vs UDL MAIN ? no post-DONE write window

### 1) What is `boot_base`?

**AP staging / reserved SHMEM**, not a window into CP-private DRAM.

- Alloc: `cp_shmem_get_nc_region(SHMEM_CP)` (`link_device.c`
  `link_load_cp_image` ?2082?2084). Phys base from `cp_shmem` reserved
  memory (`shm_ipc.c`).
- Non-PCIE: `IOCTL_LOAD_CP_IMAGE` `copy_from_user_memcpy_toio` into
  `boot_base` (BOOT) or `mld->base` (dump mode) (?2092?2116).
- **PCIE** (`LINK_ATTR_XMIT_BTDLR_PCIE`, panther): LOAD **does not use
  `boot_base`**. Copies BOOT into **`mld->base + boot_img_offset`**
  (carveout inside **SHMEM_IPC**) (?2066?2078). Then
  `set_cp_rom_boot_img` (`modem_ctrl_s5100.c` ?1274?1304) writes that
  **AP PA** into MSI `img_addr_*` so CP fetches BOOT from AP SHMEM.
- At START: `vunmap(boot_base); boot_base=NULL` (?2289?2294) ? ?in no
  use?.

### 2) Where does MAIN go on our path?

Probe order (`cp-boot-probe.c`): LOAD **BOOT** ? **START** (unmaps
boot_base) ? handover/preamble ? **`sit_send_stage(MAIN)`** via
`bootdump_write` SIT BIN/CRC/DONE over PCIe ? **not**
`IOCTL_LOAD_CP_IMAGE`. MAIN bytes stream to CP; they are **not** left as
a writable AP mirror of running MAIN. After DONE, code runs in **CP
DRAM**; PresentObj stays in **CP heap**.

### 3) Any AP write window after stock MAIN DONE?

| moment | writable AP surface | can patch SET#2 / Present? |
| --- | --- | --- |
| before START | SHMEM_IPC boot carveout / SHMEM_CP | **BOOT only** (PCIE); MAIN not loaded yet |
| during MAIN UDL | SIT BIN frames (userspace chooses bytes) | only by sending patched MAIN ? **OEM DONE rejects** (already tried) |
| after DONE / ONLINE | no map of CP MAIN / PresentObj | **no** |

No second map, no ioctl copy_to_cp of running MAIN. **No poke this turn.**

### 4) KERNEL_SRC / rebuild

Host: `s5300-src` only; **no** `KERNEL_SRC`, `Module.symvers`, or
`google-modules/soc/gs` tree (WSL = Microsoft kernel; panthor-backport
not on this machine). Cannot start cpif mmap/keep-bootmap rebuild here.

Goal incomplete (no rmnet/IPv4).

## 2026-09-30: MODEM-06 hard-blocked (no CP DRAM write path)

### PCIe / ioremap after ONLINE (live + s5300-src)

| surface | what it is | CP MAIN / PresentObj? |
| --- | --- | --- |
| s51xx EP `0000:01:00.0` BAR0 | driver forces **4 KiB**, `doorbell_addr = ioremap(?, SZ_4)` only (`s51xx_pcie.c` ?417?430) | **no** ? doorbell word |
| BAR2?5 | probe zeros then dummy 1-byte assigns | **no** |
| `msi_reg_base` | AP `SHMEM_MSI` (`cp_shmem_get_region`) | boot_stage / img_addr ? **not** MAIN |
| SHMEM_IPC / VSS / rings | AP reserved mem; sysfs RO text | **no** PresentObj |
| `s5100_set_outbound_atu` | PCIe ATU for **cp_btl** dump (`modem_ctrl_s5100.c` ?1907?1918) | crash-log window; **no** `/dev` BTL node live; not SET#2 layout |
| `/dev/mem` | absent | ? |

Sysfs `resource0` looks RW but is the **doorbell BAR**, not CP DRAM.
**No write attempted** (unproven layout = invent).

### Factory / SIT mem-write

Documented live SIT: radio/SIM/NET/misc proven GETs+SETs only.
**No** proven opcode that writes CP instruction/heap RAM (EFS/RFS banned).
No ONE-poke experiment.

### Hard block

MODEM-06 cannot force READY without: (1) **physical tray reseat**, or
(2) **KERNEL_SRC + reviewed CP-DRAM map** (or OEM-signed MAIN).
Stop inventing AP poke paths. Live poll: ONLINE, PRESENT, **PIN**,
pin1=DISABLED; **no rmnet IPv4**. Goal incomplete.

## 2026-09-30: KERNEL_SRC / BTL re-check + unblock checklist

### Host search (this machine)

Under `C:\Users\Admin\Projects`: only `s5300-src` (headers/sources, no
build). **No** `google-modules`, `gs201` kernel tree, `Module.symvers`,
`vmlinux`, or `panthor-backport`. D:/E: absent. WSL kernel is Microsoft
WSL2 ? cannot link phone `cpif.ko`. Project docs point KERNEL work at
**R620** `/home/mike/panthor-backport/` (not mounted here).

### BTL / memdump / UART (RO source + live)

- `cp_btl.c` fops: `open/release/read/ioctl` ? **no `.write`**. PCIE path
  uses outbound ATU + `memcpy_fromio` for **dump read** of a fixed CP
  log window (`cp_p_base` ?0x87200000), not MAIN SET#2 / PresentObj poke.
- Live ONLINE: **no** `/dev/*btl*`, no `cp_btl` module param, no
  `androidboot.cp_btl` / reserved_mem in cmdline, no memdump node.
- ttySAC18 / ttyGS0 present ? not a proven CP DRAM write channel.

**No poke path.** Unsigned MAIN UDL still banned (DONE reject).

### MODEM-06 unblock checklist

**A ? Physical tray (no kernel work)**

1. Keep stock ONLINE (`449eeab3?`); Radio ON if needed.
2. `rm -f /run/saaios-sit-status.lock`
3. `/data/saaios/bin/sit-sim-status tray-watch | tee ?/tray-watch.log`
4. Physically eject/reseat SIM for ?ABSENT then PRESENT/DETECTED.
5. On READY (or DETECTED?leave PIN): preferred LTE ? auto-select ?
   `0x0701` ? AllowData ? chase `rmnet`/IPv4.
6. If still PIN after reseat: log `TRAY RESULT`; stop.

**B ? KERNEL_SRC + CP-DRAM map (engineering)**

1. Obtain pantah 6.1 tree matching phone modules, e.g.:
   - s5300:
     https://android.googlesource.com/kernel/google-modules/radio/samsung/s5300/
     branch `android-gs-pantah-6.1-android16`
   - gs symbols / includes:
     `private/google-modules/soc/gs` (AOSP device tree or
     https://github.com/kerneltoast/android_kernel_google_gs201 )
   - Or copy from R620: `/home/mike/panthor-backport/` (see ADR-024 /
     MODEM-RUNTIME host notes).
2. Build `cpif.ko` against that `KERNEL_SRC` + `Module.symvers`.
3. Add reviewed userspace export: mmap/ioctl of CP region that holds
   PresentObj or STATUS SET_APP (doorbell BAR is **not** enough).
4. Deploy module ? stock boot ONLINE ? **one** poke ? STATUS refresh ?
   READY ? bearer chase.
5. Keep stock reload path for rollback.

Until A or B lands: MODEM-06 remains hard-blocked. Goal incomplete.

## 2026-09-30: path B started ? s5300 clone + poke draft (no load)

- Cloned:
  `https://android.googlesource.com/kernel/google-modules/radio/samsung/s5300`
  `@ android-gs-pantah-6.1-android16` ?
  `/home/mike/kernel-work/s5300-pantah`.
- Draft (not built/loaded): `IOCTL_SAAIOS_CP_POKE_BYTE` (0x61) + PCIe
  outbound-ATU one-byte write in `bootdump_io_device.c` (BTL AP window
  `0x14200000`). Magic `0x53414149`.
- **Blocked on build:** phone needs matching
  `6.1.157-android14-11-gbd23337e42e7-ab14791245` `KERNEL_SRC` +
  `google-modules/soc/gs/Module.symvers` (R620 `panthor-backport` or
  full pantah tree). No `Module.symvers` on device.
- Commands/checklist: `docs/os/targets/panther/MODEM-06-KERNEL-PATH-B.md`.
- Live: still ONLINE PIN+DISABLED; **no poke**. Goal incomplete.

## 2026-09-30: path B build attempt ? blocked on host flex/m4

Fetched `kernel/common@bd23337e42e7` (6.1.157) and phone `config.gz`.
`modules_prepare` needs `/usr/bin/m4` (flex); no passwordless sudo; WSL
blocks remount for bind. Missing artifact: root install of
flex/bison/m4 **or** R620 prepared tree. Drafts kept
(`IOCTL_SAAIOS_CP_POKE_BYTE`, `saaios_cp_poke.c`). Commands in
`MODEM-06-KERNEL-PATH-B.md`. No deploy/poke. Goal incomplete.

## 2026-09-30: SET#2 hyp PA `0x814f47f4` ? ATU blocked (PCIe down)

**Cite / compute:** BTL `cp_btl.c` LINKDEV_PCIE: CP `0x47200000` +
`0x40000000` ? ATU `0x87200000`. Same `+0x40000000` on SET#2 VA
`0x414f47f4` ? hyp PA **`0x814f47f4`**. No MAIN `cp_p_base` symbol.

**Live:** RO `@0x814f47f4` and BTL `@0x87200000` both ATU **-32**
(`link_status=0`). EP config `ff`, width `0`, `PM_STATE=2`. SIT frames=0.
No write. Next: restore PCIe then one RO `@0x814f47f4` for `02 20`.
Goal incomplete (no rmnet/IPv4).

## 2026-09-30: hyp PA ATU ok but RO=0xff ? not SET#2

Restored via sysrq + stock probe-handover ? ONLINE, SIT ok. Warm link
(width=2): ATU `@0x814f47f4` **ret=0**, read **`0xff`** (not `02 20`).
Refine `0x14f47f4` also `0xff`. **No write.** Hyp VA+0x40000000 rejected
for MAIN code. Goal incomplete.

## 2026-09-30: UDL MAIN = SHMEM TX only; no MAIN PA map

SIT UDL: `bootdump_write` ? `xmit_to_legacy_link` ? AP **SHMEM_IPC
NORM_RAW TX** (`legacy/region` RAW `+0x3000`). CP pulls frames; MAIN
lands in CP-private DRAM (`m_off 0x40010000`). Live BOOT only:
`boot_img 0xEA410000/0x16800`. Kallsyms: **outbound ATU only** (BTL
`0x87200000`). No inbound ATU; SET#2 outside BTL CP range. **Missing:
MAIN DRAM PA after ONLINE.** No RO-scan/write. Goal incomplete.

## 2026-09-30: OB0/1/2 inventory ? MAIN not covered

`pcie-exynos-rc.c`: OB0=CFG, OB1=EP BAR (`0x40000000`+2MiB), OB2=
**BTL-only** programmable (`res.start+2MiB`?`0x40200000`, comment
`Only for BTL`). No inbound in `pcie-exynos-gs.ko`. No CP remap-MAIN
API; PresentObj heap PA unknown ? BTL `0x472xxxxx`. No poke. Goal
incomplete.

## 2026-09-30: OB1 live RO 0xff ? ATU poke dead

Live OB1 `@0x40000000` / `@0x40010000` = **`0xff`** (not MAIN
`88f09fe5`). OB1=EP BAR only; LIMIT fixed 2MiB; SET#2 +21MiB unreachable.
**MODEM-06 ATU poke dead.** SIM still PIN+DISABLED. Next: tray. Goal
incomplete.

## 2026-09-30: Adversarial VerifyPin?READY ? corrected MODEM-06 model

**Paradox:** stock Pixel enters PIN ? READY on this MAIN without CDMA;
prior model said only Present==2?SET_APP#5 and only CDMA/FN_A sets
Present=2.

### 1) Forward slice: FirstPIN (`0x18e` / `0x1f0452c`) ? SET#5?

| step | site | effect |
|------|------|--------|
| Pin1Verified=1 | `STRB [obj,#20]` @ `0x1f04578` / sibling | only |
| SimInfo / bitop | `BL 0x1f1458c`; `LDRB/LSLS/STRB [r4,#0]` | flag @ caller `r8+4`, **not** PresentObj |
| BFS depth?8 | 677?26k funcs | **0** BL STATUS / FN_A / `+0xBF6` |
| Reachable SET_APP | via `??0x18ec31c` | **SET_APP#7** @ `0x18ec4fe` **iff** GET_APP==6 |
| SET#7 case | TBB case **10** of `0x18ec31c` | after VerifyPin chain |

**No** PresentObj[0] / `+0xBF6` / SET_APP#5 on VerifyPin path.

### 2) SET_APP#5 gate re-verified

- Sole `#5`: `0x14fb5c6`; `LDRB.W +0xBF6` **CMP #2** ? still Present mirror.
- Sole `+0xBF6` STRB: `0x14fb380` (STATUS copy PresentObj[0]).
- Sole `+0xBF4` STRB: inside SET_APP `0x1991734`.
- VerifyPin `MOVS #2; STRB` hits are **obj+20** (Pin1Verified), **not**
  Present / `+0xBF6` under another name.

**STATUS CFG (corrected Bcc.W):**

| GET_APP | behavior |
|---------|----------|
| ?{1,4} | copy Present?`+0xBF6`; Present==0 ? **skip** SET#2; Present==1?#3; else?**#2 PIN**; copy Pin1Verified; **no #5** |
| ?{6,7} | **no** Present re-copy; `+0xBF6==3`?#4; **`+0xBF6==2`?#5 READY** |
| else (e.g. PIN=2) | leave via skip; no #5 |

So after SET#2, READY needs **promote out of PIN** to app?{6,7} **and**
stale `+0xBF6==2`.

### 3) Who sets app 6/7 (the missed post-verify stage)

| SET_APP | site | trigger |
|---------|------|---------|
| **#6** | `0x14c64a6` in `0x14c643c` | sole BL `0x14b7cbc` ? **`MMCIF_L1LC_DSL1C_SADR_GAP_MEASURE_PAUSE_REQ`** |
| **#7** | `0x18ec4fe` | FirstPIN chain ? `0x18ec31c` case10; **requires GET_APP==6** |
| gate #6 | `0x18c2acc` | pin-state helper; **accepts path for state 3 (DISABLED)** |

VerifyPin BFS: **REACH** `0x18ec31c` / SET#7; **MISS** SET#6 / STATUS_WRAP /
STATUS. SET#6 is **L1 SADR**, not SIT `0x0201`.

**AP after 0x0201 OK:** stock sends nothing extra proven; CP internally
needs prior/peer **SADR_GAP ? SET#6**, then VerifyPin ? SET#7, then
STATUS?#5. **DISABLED:** cannot 0x0201 OK (err6); **should share** STATUS
{6,7}?#5 **if** `+0xBF6==2` and SET#6 fires ? SET#6 gate allows pin1=3;
no AP SIT for SADR_GAP ? **no live probe** this turn.

### 4) Corrected MODEM-06 model (stock PIN unlock without CDMA)

```
SET#1 DETECTED
  ? STATUS(GET?{1,4}): Present?+0xBF6; Present?0?1 ? SET#2 PIN
  ? (need Present==2 here so +0xBF6 latches 2 while UI is PIN)
L1 SADR_GAP_MEASURE_PAUSE_REQ ? SET#6
User 0x0201 OK ? FirstPIN ? Pin1Verified=1 ? ? ? SET#7 (if app==6)
STATUS(GET?{6,7}) ? +0xBF6==2 ? SET#5 READY ? attach/rmnet
```

**Retract:** ?VerifyPin alone must write Present=2 / call SET#5?.
**Hold:** sole #5 still `+0xBF6==2`; sole reachable PresentObj=2 STRB still
FN_A(CDMA LATCH/MEAS).
**New:** stock EU READY is **multi-stage** ? early Present latch + **L1
SET#6** + VerifyPin **SET#7** + STATUS#5. CDMA FN_A is **not** the
VerifyPin step; SADR_GAP is a separate L1 event (may fire on LTE).
**Open hole:** how Present==2 is first latched on EU RatMap-without-CDMA
(still no non-FN_A Present=2 writer). Soft-lock = stuck at SET#2, no
VerifyPin, likely no SET#6/7, `+0xBF6` unknown live.

**Live:** none (no AP trigger for SADR; DISABLED blocks 0x0201). Goal
incomplete (no rmnet/IPv4).

## 2026-09-30: SADR_GAP SET#6 producer + Present=2 re-audit

### 1) Who posts `SADR_GAP_MEASURE_PAUSE_REQ`?

| item | evidence |
|------|----------|
| Consumer (SET#6) | Dispatcher `0x14b7074` case label `MMCIF_L1LC_DSL1C_SADR_GAP_MEASURE_PAUSE_REQ` ? `BL 0x14c643c` @ `0x14b7cbc` |
| Dispatcher callers | sole `0x19d15e0` in `0x19d1548`; parents `0x14ae31c` / `0x194ff56` / `0x1a2c700` (MMCIF msg pump) |
| Direction (logs) | **`[SR_IF ==> LTE_L1LC] MMCIF_L1LC_SADR_GAP_MEASURE_PAUSE_REQ`**; DONE_CNF reverse `L1LC ==> SR_IF` |
| Radio condition | **LTE-only** ? `GetIsGapMeasurePause should use only LTE`; RSM `GapMeasurePause` / SRL1RC meas resource |
| Meaning | Internal **LTE gap-measure pause** IPC (IRAT/neigh meas), not USIM/SIT |

**AP/SIT/preferred/band lever:** **none.** No SIT/Build* for SADR/GapMeasure. Live already `preferred=LTE_ONLY` + Radio ON without SET#6/READY. Forcing PAUSE_REQ would be inventing L1 IPC ? banned.

### 2) Present=2 / `+0xBF6=2` writers (re-audit w/ SET#6)

| store | site | verdict |
|-------|------|---------|
| `+0xBF6` STRB.W | **sole** `0x14fb380` | STATUS copy PresentObj[0] only |
| PresentObj=2 | `0x14f6a14` FN_A | reachable; CDMA LATCH/MEAS only (BL?2) |
| PresentObj=2 | `0x14f957c` FN_B | still unreachable (prior) |
| SET#6 body | `0x14c643c` | SET_APP**#6** only; **no** Present/`+0xBF6`/FN_A |
| false positive | `0x14f900a` MOVS#2 | **not** Present STRB (branch to other helper) |

**No non-FN_A path** sets Present=2 or `+0xBF6=2` before STATUS#5. SET#6 does not latch Present.

### 3) Live

**No AP-triggerable SET#6 sequence** under constraints (PAUSE_REQ is SR_IF?L1LC LTE gap meas). DISABLED can pass SET#6 gate (`0x18c2acc` state-3 path) **if** PAUSE_REQ arrives ? it does not on soft-lock. No live probe.

### 4) MODEM-06 model (updated)

```
Present=2 (FN_A/CDMA only known) ? STATUS +0xBF6=2 + SET#2 PIN
SR_IF LTE GapMeasurePause ? PAUSE_REQ ? SET#6
VerifyPin OK ? SET#7 (needs #6)
STATUS(GET?{6,7}) ? +0xBF6==2 ? SET#5 READY
```

**Soft-lock:** stuck SET#2; no PAUSE_REQ (no LTE gap meas); no VerifyPin; Present=2 EU writer still unknown. ATU poke dead. Goal incomplete (no rmnet/IPv4).

## 2026-09-30: GapMeasurePause vs SIM READY ? chicken-egg holds

### 1) Does GapMeasurePause require READY / START_NETWORK?

| layer | SIM READY / GET_APP gate? | evidence |
|-------|---------------------------|----------|
| SET#6 consumer `0x14c643c` / dispatcher | **No** GET_APP | only `0x18c2acc` (pin1 2/3) |
| PAUSE_REQ poster `0x261ac84` (SR_IF log) | **No** GET_APP | build msg ? post `0x2be2ffc` |
| RSM producer | needs **SRL1RC registered** + meas | `ReceiveGapMeasurePauseFromSRL1RC`; fail `Already Deregister` |
| LTE meas | needs **camp / frequency** | `measurementModifyReq: Not camped on any frequency` |
| START_NETWORK (SIT) | **Yes** ? allow GET_APP?{**1,4,5**} only | `START_NETWORK Ignored: SIM is not ready`; PIN(2) denied @ `0x18e831a` |

**Verdict:** PAUSE_REQ path has **no explicit SIM-READY CMP**, but producer needs LTE meas resource ? camp ? START_NETWORK ? app?{1,4,5}. With soft-lock **PIN(2)** + Radio ON + LTE_ONLY (already live): stack not started for PS ? no camp ? no GapMeasurePause ? no SET#6.

### 2) AP enable sequence?

**None proven.** No MAIN `BuildEmergency*` / camp SIT; E-Camping logs are eSIM/PS-disabled internal, not PIN-unlock. Preferred/Radio already exhausted. **No live** (would invent L1/SRL1RC poke).

### 3) Early Present=2 before first STATUS

| site | store |
|------|-------|
| getobj `#636c` @ `0x1a55204` (init) | **Present=0** (`MOVS #0; STRB [r6,#0]` @ `0x1a552a4`) |
| other getobj `#636c` | no Present=2 |
| Present=2 | **sole** FN_A `0x14f6a14` (CDMA) |

No missed BOOT/USIM init Present=2 writer.

### 4) MODEM-06

Chicken-egg closed: SET#6?before START_NETWORK; START_NETWORK?while PIN; Present=2?without FN_A. Goal incomplete (no rmnet/IPv4).

## 2026-09-30: live ? avoid AP `0x0200` (DETECTED window?)

**Hypothesis:** AP `0x0200` forces STATUS Present==0?SET#2 PIN; without GET, GET_APP might stay DETECTED(1) so START_NETWORK (allows #1) can run.

### Run A (invalid) ? stock phone `probe-handover` polluted

Phone binary had **HOLD** dense `0x0200` (45s). Abort as control contamination.

### Run B (valid) ? clean handover

- Rebuilt `probe-handover-clean` from `diagnostics/cp-boot-probe.c` with
  `PROBE_HANDOVER` **without** `PROBE_QUERY_SIM` / HOLD.
- sysrq ? CPIF ? OFFLINE ? handover ONLINE; log check: **no** SIM/HOLD/`0x0200`.
- ASAP (no prior `0x0200`): RadioPower ON (`0x0800` err0) ? radio=10;
  Pref LTE_ONLY `0x070a` err0; AllowData `0x0710` err0; AutoSel `0x0704` err2.
- ~75s poll **only** `0x0700`/`0x0701`/rmnet: voice?reg**3** tech195; data reg**0**
  tech3; rmnet0/1/2 **0/0**, no IPv4. `saw_reg=0 saw_rmnet=0`.
- **One** final `0x0200`: len143 err0, app_state=**PIN(2)**, pin1=DISABLED(3)
  (same soft-lock as prior boots).

### Verdict

Avoiding AP `0x0200` **does not** leave DETECTED or unlock START_NETWORK/bearer.
CP/USIM path still publishes PIN by first GET. Goal incomplete (no rmnet/IPv4).

## 2026-09-30: ATU OB2 RO discovery ? no MAIN PA

SET#2 stock bytes @file `0x14fb404` = `02 20 96 f0 64 f1 6e 78`. Systematic
OB2 RO retarget scan (see MODEM-06 ? ?OB2 systematic RO sig-scan?): all
probed windows DUMP `ff`; **no** `02 20` HIT. **No poke.** Goal incomplete.

## 2026-09-30: SHMEM_IPC RO ? only FMT ring copies; no Present poke

### Layout (DT + CPIF)

| region | AP PA | size | role vs SIM |
| --- | --- | --- | --- |
| SHMEM_IPC (`cp_rmem`) | **`0xea400000`** | 8MiB | FMT/RAW rings, united_status, srinfo |
| MSI | `0xf6200000` | 4KiB | boot MSI regs |
| united_status | IPC+`0x4000` (ap2cp) | word | **ds_det / PDA / BTL bits only** ? not app_state |
| srinfo | IPC+`0x400000` | crash/sr blob | **IOCTL_GET/SET_SRINFO**; no SIM Present in CPIF |
| `sim/ds_detect` | sysfs | ? | Dual-SIM **module param** only |

Factory/CPIF (`s5300-pantah`): **no** SHM mirror of PresentObj/`+0xBF6`/GET_APP.
START_NETWORK still reads CP heap via SIT.

### Live RO (`saaios_shm_ro.ko` via `cp_shmem_get_region(0,3)`)

ONLINE + PIN(2)/pin1=3. Pattern search (no AID/ICCID):
`card=1 apps=1 type=2 state?{1,2,5} pin1?{0,1,3}`.

| window | hits | note |
| --- | --- | --- |
| fmt+ctrl | **2** @`+0x2968`,`+0x2c88` state=2 pin1=3 | **stale FMT RX** `0x0200` copies |
| srinfo | **0** | not a SIM mirror |
| raw_rx_head | **0** | |

Poking FMT RX does **not** change CP GET_APP / PresentObj. **No write.**

### Tray path

`os/targets/panther/diagnostics/tray-bearer-chase.sh` (on phone:
`/tmp/tray-bearer-chase.sh`, log `/data/saaios/var/tray-bearer.log`):
`WATCH_ROUNDS=12` (~36 min) loops `tray-watch` ? on DETECTED/READY/
ABSENT?PRESENT chase Radio/LTE/auto/reg/rmnet IPv4.

**Live 2026-09-30:** stock ONLINE PIN+DISABLED; background
`nohup WATCH_ROUNDS=12 /tmp/tray-bearer-chase.sh` started (pid logged).
Await physical reseat. Goal incomplete until rmnet/IPv4.

Goal incomplete (no rmnet/IPv4).

## 2026-09-30: SET SC lock enable (factory 0x020a) candidate A ? no VerifyPin

Factory `BuildSimSetFacilityLock` is id **`0x020a`**, length 72 (GET sibling
is `0x0209`/71). Layout: SC index 3 @12, lock mode 1 @13, password length @14,
digits @15+, service class 7 @54; optional AID length @55. Stock
`setIccLockEnabled(true)` uses SC + class 7 + PIN digits. Candidate A only;
digits never printed.

**Live (clean SIT):** pre app PIN, **pin1=2**, remain 3. One SET enable+A+class7
? length 16, **error_raw=2**. Post: pin1 still 2, remain 3. pin1 ? ENABLED(1)
? VerifyPin not sent; no candidate B. rmnet 0/0. ONLINE.

Do not spray further SET on this path. Fixed false-EDGE tray chase redeployed
and restarted (pid on device). Goal incomplete.

## 2026-09-30: pin1=2 is AOSP ENABLED_VERIFIED ? chase under PIN

Fresh `0x0200` after the SC SET attempt: card PRESENT, apps=1, **app=PIN(2)**,
**pin1=2**, remain=3. This is **not** a misread of DISABLED(3).

Factory `GetPinState` returns packet byte 72 with no rename. AOSP
`RIL_PinState` mapping used by the card-status path:

| raw | AOSP name |
| --- | --- |
| 0 | UNKNOWN |
| 1 | ENABLED_NOT_VERIFIED (only value that opens CheckAndAutoVerifyPin) |
| **2** | **ENABLED_VERIFIED** |
| 3 | DISABLED |

Factory libs still have no `RIL_PINSTATE_*` strings; only the compare-to-1
path is proven in HAL. Skipping VerifyPin when pin1?1 remains correct
(verify is for NOT_VERIFIED). pin1=2 after SET is real progress vs prior
pin1=3, but **app_state stayed PIN** so START_NETWORK/READY still blocked.

**Chase anyway (PIN+pin1=2):** radio=10 ON; preferred LTE SET ok; network
selection auto error_raw=2; data reg=0 tech=UMTS; **voice reg=3** tech=UMTS;
operator plmn_present=1; AllowData `0x0710` allow=1 re-sent; rmnet up but
**no IPv4**, rx=0. Soft-lock holds for PS bearer. Goal incomplete.

## 2026-09-30: RO pin1=2 ? READY; CardPower?pin1=1; VerifyPin A timeout

**RO (STATUS/SET_APP):** Exhaustive MAIN BL?SET_APP table already proves
**READY(#5) only at `0x14fb5c6` behind Present==2 (`+0xBF6`)**. No branch
maps AOSP pin1==2 (`ENABLED_VERIFIED`) or SIT pin1 byte to READY without
that Present gate. Pin1Verified (obj+20 / `+0xBF5`) is set only by
successful VerifyPin path ? not by pin1 field alone, not by SET#6
GapMeasure. `#6`/`#7` BLs exist without Present gate but are not pin1=2
aliases to READY.

**Live CardPower DOWN(4)?UP(1) then Radio OFF?ON:** both card/radio cycles
error 0. After card UP: **pin1 2?1** (ENABLED_NOT_VERIFIED), app still PIN,
remain 3. Radio pulse kept pin1=1 briefly. Final settle: **pin1=2** again,
app PIN, voice reg flapped 3?0?3, data reg=0, rmnet no IPv4.

**One proven step with pin1=1 window:** factory `0x0201` VerifyPin candidate
A (RFS-aware one-shot). **Timeout / no matching response**; remain stayed 3
(no attempt burn). No candidate B. App never left PIN(2).

Goal incomplete.

## 2026-09-30: VerifyPin A completes (err 0) with AID+long wait; app stays PIN

**Hang fix:** prior `0x0201` without status AID (+ short/no-RFS wait) saw
`observed_frames=0`. Live status has **aid_len=12**. Working one-shot:
RFS-aware exchange, 45s budget, match `0x0201` (any token), copy AID from
last `0x0200` into verify (AID never printed).

**Live:** CardPower ? confirm **pin1=1** remain=3 ? Radio ON ? ONE VerifyPin
**A** ? length=16 **error_raw=0** remain_raw=3. After: **pin1=2**, **app
still PIN(2)**. B skipped (pin1?1). Data/voice reg 0, rmnet no IPv4.

VerifyPin success ? READY; Present==2 gate still holds. Goal incomplete.

## 2026-09-30: Pin1Verified remote; follow-up chase still PIN / no PS

**Factory after `0x0201` OK:** AP stock is `DoVerifyPin` then **GetSimStatus
`0x0200`** (OnGetSimStatusDone ? RIL notify). No extra SIT (no SimRefresh
builder from this path). CP FirstPIN (`0x18e`) sets Pin1Verified obj+20;
SET_APP#7 only if **GET_APP==6** (SET#6 is SADR_GAP_MEASURE_PAUSE, not SIT).

**STATUS after Pin1Verified:** GET_APP still PIN(2) ? skip Present copy /
SET#5. CardPower would re-STATUS the same skip and previously reset pin1
2?1. **Not sent.**

**Live chase (RFS-aware, no CardPower):** `0x0200` app=PIN pin1=2 remain=3;
radio=10; LTE_ONLY (`preferred_raw=11`); AllowData `0x0710` error 0.
Poll ~45s: data reg=0 tech=UMTS; voice reg=3; rmnet tx=432 rx=0 **no IPv4**.
Pin1Verified achieved remotely; START_NETWORK still denied for app?{1,4,5}.
Goal incomplete.

## 2026-09-30: pin1=2 vs SET#6 / START_NETWORK ? Radio OFF?ON still PIN

**RO SET#6 gate `0x18c2acc`:** after helper `0x18db24e`, **CMP r0,#2** (first
accept) then **CMP r4,#3**. pin1=2 (ENABLED_VERIFIED) is the primary pass;
DISABLED(3) is the fallback. Gate does **not** require Pin1Verified byte
`+0xBF5`. **Sole** BL?SET#6 entry `0x14c643c` remains `0x14b7cbc` (SADR
PAUSE). FirstPIN/Pin1Verified BFS still **MISS** SET#6.

**RO START_NETWORK `0x18e831a`:** GET_APP **CMP #1/#4/#5** only. PIN(2) still
denied. No `LDRB +0xBF5` in that window ? Pin1Verified does **not** open
camp/search while app=PIN.

**Live (no CardPower, no VerifyPin):** Radio OFF?ON `0x0800` err0; pin1 stayed
**2**; LTE_ONLY; operator plmn_present=1; auto `0x0704` err2; AllowData err0.
Poll ~45s: app **PIN**, data=0, voice=3, rmnet no IPv4. GET_APP never 6/7/5.

Chicken-egg holds: SET#6 gate **would** accept pin1=2, but PAUSE_REQ still
needs LTE camp ? START_NETWORK ? app?{1,4,5}. Goal incomplete.

## 2026-09-30: START_NETWORK/camp RO + GetPsService ? still PIN

**RO GET_APP after BL (NET/SIM windows):** START_NETWORK `0x18e831a` still
allow **only {1,4,5}**. PIN(2) denied. No Pin1Verified (`+0xBF5`) in that
CMP list.

Nearby `0x18e81e0` (after SET#6 gate `0x18c2acc`): GET_APP **CMP #2** then
`LDRB.W +0xBF5` (Pin1Verified) ? this is the **SADR/SET#6 helper**, not
camp/START_NETWORK. STATUS `0x14fb340` CMPs {1,4} then #2 (PIN store).
SET#7 site CMPs #6. **No camp/attach entry ignores GET_APP or treats PIN
as START_NETWORK-ready.**

IMS SetupDataCall / InitialAttachApn need APN ? not sent. EmergencyMode /
StartNetworkScan need extra args ? not sent.

**Live ONE:** empty GET `BuildGetPsService` **`0x0711`** len12 ? length 13
**error 0 byte12=1**. App still PIN pin1=2; data=0 voice=3; rmnet no IPv4.

**MODEM-06 now:** Pin1Verified **achieved** (VerifyPin A err0). Remaining
gate: GET_APP still PIN ?{1,4,5} so START_NETWORK/camp/SET#6 chicken-egg.
**Next remote ? tray:** RO-map Present/`GET_APP` in CP shm, or a proven
empty camp SIT not found. Physical tray-reseat remains operator unblock.
Goal incomplete.

## 2026-09-30: MODEM-06 docs + tray EDGE VerifyPin A+AID

**Docs:** MODEM-BLOCKER / ROADMAP ? Pin1Verified remote OK; remaining
chicken-egg GET_APP PIN vs START_NETWORK `{1,4,5}` / SET#6 camp. No SHM
poke; ATU MAIN invisible.

**`tray-bearer-chase.sh`:** on EDGE/ABSENT?PRESENT (or pin1 left PIN soft-
lock), if pin1?2 and remain>1 ? `VERIFY_TOOL` CardPower?VerifyPin A+AID
(proven), then Radio/LTE/AllowData/reg/rmnet. Skip CardPower when pin1=2.
remain?1 ? stop. No PIN/AID in logs.

Goal incomplete without live rmnet/IPv4.

## 2026-09-30: RO emergency/NAS without {1,4,5}; SET#1 vs Pin1Verified

**Tray peek:** watch RUNNING; `saw_ready=0 saw_detected=0 saw_absent=0`;
TRANSITION app=PIN pin1=2; rmnet no IPv4. No real EDGE ? no chase.

**RO emergency / limited / CSFB / NAS start:** CP strings exist
(`EmergencyMode*`, `EMERGENCY_SCAN_*`, `LimitedService*`, CSFB timers).
GET_APP windows that accept PIN(#2) are STATUS/SET#6-adjacent, **not**
camp/START_NETWORK. Sole START_NETWORK gate `0x18e831a` still CMP
**{1,4,5} only**. Post-gate `GET_APP` @ `0x18e8348` is ignore/fallthrough,
not an alternate attach. **No SIT-proven empty emergency/limited/CSFB
opcode** that bypasses the gate ? **no live attempt**.

**RO SET_APP#1:** sole store via SET1FN `0x146a99c` ? SET_APP; callers
still only `0x14538e4`, `0x1a2abf8` (internal). SET1FN body/nearby callees
show **no** STRB Pin1Verified clear (`+20`/`+0xBF5`). **Not AP-triggerable**
while soft-lock PIN+pin1=2 (no SIT?SET#1). STATUS still overwrites #1?#2
when Present?{1,2,3}. **No live SET#1.**

Goal incomplete.

## 2026-09-30: BTL/SHMEM RO for live PIN+pin1=2 ? no unique PresentObj

**Tray:** no EDGE (`saw_ready/detected/absent=0`); pin1=2 app=PIN; rmnet no IPv4.

**Factory layout cites:**
| surface | offsets |
| --- | --- |
| SIT `0x0200` body | card@0 apps@2 type@3 state@5 pin1@60 |
| STATUS base | `+0xBF4` app_state, `+0xBF5` Pin1Verified, `+0xBF6` Present |
| PresentObj | `getobj` size `#0x636c`; FirstPIN Pin1Verified @obj+20 |
| BTL map | CP `0x47200000` ? ATU AP `0x87200000` (`cp_btl.c`) |

**Live RO (`saaios_sim_ro.ko`):**
| window | result |
| --- | --- |
| SHMEM FMT | **12? WIRE** app=2 pin1=2 @`+0x2000`? ? **FMT RX `0x0200` copies** |
| SHMEM STATUS scan | coincidental `+0xBF4/5/6` in ring ? **not** 0x636c PresentObj |
| srinfo / raw | 0 |
| BTL `0x87200000`+0..7MiB | ATU ok or -32; readable windows **all `0xff`** |

**Verdict:** no unique AP-visible PresentObj / GET_APP cell. FMT poke would
not move CP heap. **No write.** Watch kept. Goal incomplete.

## 2026-09-30: umts_dm0 / Shannon DM ? no proven mem poke

**Tray:** watch alive; no EDGE; PIN/pin1=2; no rmnet IPv4.

**s5300-src:**
| surface | role | CP RAM poke? |
| --- | --- | --- |
| `direct_dm.c` | CP?AP/USB **RX log DMA** (desc/buff SHMEM) | **N** ? no TX mem API |
| `ipc_io_device` `umts_dm0` | SIPC misc ch; `ipc_write` queues to CP | TX pipe only; **no** MEM_READ/WRITE framing in CPIF |
| IOCTLs | power/boot/dump/srinfo/crash | none = arbitrary CP VA write |

**MAIN strings (false positives):** `MemoryRead/Write` = Pigweed stream RTTI; `MEM_WRITE` = LTE MTM DPD cal; `RegWrite` = ATI NV/cal ? **not** DM heap poke.

**Live:** `mknod umts_dm0` 493:4; RX **0 bytes** (`NO data in RXQ`); write of 5B accepted by iod (**no** response, **no** proven effect on app/Present). Prior listen: EOF without DM-enable (protocol **unknown**, not invented).

**Verdict:** DM = **RX-log / unknown enable**; **no** proven write of PresentObj/`+0xBF4` at known VA. PresentObj still heap-unreachable. **No spray / no DM poke.** Watch kept. Goal incomplete.

**Tray:** watch alive (chase+tray-watch); no EDGE; PIN/pin1=2; no rmnet IPv4.

**RO PresentObj allocation (`getobj` `0x20ea040`):**
| fact | evidence |
| --- | --- |
| Size arg | `MOVW r1,#0x636c` + `MOVS r0,#4` then `BL getobj` |
| Call sites | **7** (FN_A `0x14f6956`, `0x1551d0a`, `0x1551fa0`, `0x155342c`, `0x1a17600`, `0x1a347f0`, `0x1a5521a`) |
| Kind | **dynamic heap** ? getobj ? allocator path (`BL 0x20e90e0`); **not** static BSS in MAIN image |
| Debug tag | `r0/r2` file/line (`0x43d7f6a0` / `0x410c18ee`) ? not object VA |
| Singleton slot (candidate) | stores after getobj toward **`0x48b87b7c`** (data RAM VA, **outside** MAIN file) |
| Object VA/PA | **runtime-only** heap pointer; no fixed PA in TOC/BTL |

**AP reachability:** BTL `0x47200000`?heap. Hyp ATU `0x48b87b7c+0x40000000=0x88b87b7c` live RO **`0xff`** (open-bus, same class as MAIN hyp). SHMEM has only FMT copies. **PresentObj not AP-reachable ? no poke.**

Watch kept. Goal incomplete.

## 2026-09-30: umts_dm0 / Shannon DM ? no proven mem poke

**Tray:** watch alive; no EDGE; PIN/pin1=2; no rmnet IPv4.

**s5300-src:**
| surface | role | CP RAM poke? |
| --- | --- | --- |
| `direct_dm.c` | CP?AP/USB **RX log DMA** (desc/buff SHMEM) | **N** ? no TX mem API |
| `ipc_io_device` `umts_dm0` | SIPC misc ch; `ipc_write` queues to CP | TX pipe only; **no** MEM_READ/WRITE framing in CPIF |
| IOCTLs | power/boot/dump/srinfo/crash | none = arbitrary CP VA write |

**MAIN strings (false positives):** `MemoryRead/Write` = Pigweed stream RTTI; `MEM_WRITE` = LTE MTM DPD cal; `RegWrite` = ATI NV/cal; `DM_ENABLE` = RF MM API ? **not** DM heap poke.

**Live:** `mknod umts_dm0` 493:4; RX **0 bytes** (`NO data in RXQ`); 5B write accepted by iod (**no** response, **no** proven effect on app/Present). Prior listen: EOF without DM-enable (protocol **unknown**, not invented).

**Verdict:** DM = **RX-log / unknown enable**; **no** proven write of PresentObj/`+0xBF4` at known VA. PresentObj still heap-unreachable. **No spray / no DM poke.** Watch kept. Goal incomplete.

## 2026-09-30: lab/test/force-camp SIT hunt ? none bypass GET_APP

**Tray peek:** watch alive; no EDGE; app=PIN pin1=2; rmnet no IPv4.

**Factory HAL (`sit-stream` / `libsitril`):**
| candidate | layout | starts NET w/o app?{1,4,5}? |
| --- | --- | --- |
| `ProtocolMiscDebugBuilder::SetEngMode` | **proven** id **`0x0908`** len **13**, mode `@12` (Ehh: len 14) | **unproven** ? CP register site only; **no** BL?START_NETWORK/`GET_APP` |
| `BuildSetImsTestMode` | id `0x0614` len 13 | IMS test ? not camp/attach |
| `FakeSimPresence` | `DbEccInfoLoader` local ECC DB | **not a SIT** |
| VirtualSim | CP `sitVirtualSim*` APDU/event ch | needs VSIM backend; not force-camp |
| `SIT_SET_EMC_MODE` | name; logs ?isn't supported? | no |
| TEST_SIM / CampOn / NS_ATTACH | flash/NAS internal strings | no AP SIT builder |

**MAIN:** sole START_NETWORK gate still `0x18e831a` CMP **{1,4,5}**. **No live send** (layout?bypass proof). Watch kept. Goal incomplete.



## 2026-09-30: lab/test/force-camp SIT hunt ? none bypass GET_APP

**Tray peek:** watch alive; no EDGE; app=PIN pin1=2; rmnet no IPv4.

**Factory HAL (`sit-stream` / `libsitril`):**
| candidate | layout | starts NET w/o app?{1,4,5}? |
| --- | --- | --- |
| `ProtocolMiscDebugBuilder::SetEngMode` | **proven** id **`0x0908`** len **13**, mode `@12` (Ehh: len 14) | **unproven** ? CP register site only; **no** BL?START_NETWORK/`GET_APP` |
| `BuildSetImsTestMode` | id `0x0614` len 13 | IMS test ? not camp/attach |
| `FakeSimPresence` | `DbEccInfoLoader` local ECC DB | **not a SIT** |
| VirtualSim | CP `sitVirtualSim*` APDU/event ch | needs VSIM backend; not force-camp |
| `SIT_SET_EMC_MODE` | name; logs "isn't supported" | no |
| TEST_SIM / CampOn / NS_ATTACH | flash/NAS internal strings | no AP SIT builder |

**MAIN:** sole START_NETWORK gate still `0x18e831a` CMP **{1,4,5}**. **No live send** (layout?bypass proof). Watch kept. Goal incomplete.

## 2026-09-30: +0xBF4 object identity + ONE SetEngMode live

**Tray:** was alive (no EDGE); paused for SIT lock; restarted after.

**RO +0xBF4 holder:** **same PresentObj** `getobj(#0x636c)` (size fits `0xBF4<0x636c`). STATUS uses one base for `+0xBF4`/`+0xBF5`/`+0xBF6`; SET_APP sole `STRB.W [obj,#0xBF4]` @`0x1991734`; GET_APP `LDRB` same field. Heap runtime VA; **not AP-reachable** (prior ATU/BTL/SHMEM). **No +0xBF4 poke.**

**Live ONE SetEngMode:** id `0x0908` len 13 mode@12=`1` (factory `SetEngMode(h)`).
| probe | result |
| --- | --- |
| RSP | n=12 err=0 (accept) |
| SIM before/after?4 | card=1 apps=1 **app=2 PIN** pin1=2 unchanged |
| reg `0x0701` | err=0 tech=0 |
| rmnet0/1/2 | rx=0 no IPv4 |

**Verdict:** EngMode accepted, **no** app/Present/reg/rmnet change. Watch restarted. Goal incomplete.

## 2026-09-30 follow-up: EngMode/+0xBF4 closed; wait on tray

**Live peek (soft-lock only):** CP `ONLINE`; `0x0200` **app=PIN** pin1=2
remain=3; data reg=0; rmnet no IPv4. `tray-bearer-chase` **alive** (pid
logged); round-1 fired a chase without `saw_detected`/`saw_ready` (still
PIN) ? not a verified physical EDGE. No new remote lever tried (EngMode
already one-shot; +0xBF4 AP-unreachable).

**Closed:** remote soft-lock cannot force READY/Present via EngMode or
AP poke of `+0xBF4`. **Next:** physical SIM tray reseat while watch armed
(VerifyPin-on-EDGE). Goal incomplete.

## 2026-09-30: live peek + false EDGE fix in tray-bearer-chase

**Live (COM13 / USB NCM):** CP `ONLINE`; soft-lock holds ? app=PIN(2)
pin1=2 remain=3; `present_infer=notin_1_2_3`; data reg=0; `rmnet*` rx=0
(tx from prior false chase only); **no IPv4**. Bearer **not** verified.

**Bug:** `sim_left_pin` used BusyBox BRE `grep ? \| ?`. Alternation never
matched PIN, so every soft-lock round fell through to `return 0` ? false
`EDGE observed` ? Radio/LTE/AllowData chase while still PIN. Round-1/2/3
logs confirmed chase without `saw_detected`/`saw_ready`.

**Fix (deployed `/tmp/tray-bearer-chase.sh`):** numeric `app_raw=2` ? not
EDGE; `grep -qE` alternation; ASCII `ABSENT.PRESENT`; `sed` for IPv4
(no `awk` on image). Killed duplicate watches; **one** instance re-armed
(`WATCH_ROUNDS=12`, VerifyPin-on-EDGE).

**Remote path:** unchanged ? EngMode/+0xBF4/ATU/PresentObj poke closed;
no `saaios_cp_poke` write (no proven Present/+0xBF4 PA). **Operator:**
physical tray reseat while watch armed. Goal incomplete.

## 2026-09-30: live peek + soft-lock pin1=2 detect + CPIF caps

**Live (COM13):** CP `ONLINE`; soft-lock holds ? app=PIN(2) pin1=2
remain=3; `present_infer=notin_1_2_3`; data reg idle; `rmnet*` rx=0
(tx residual from prior false chase); **no IPv4**. Bearer **not** verified.
`tray-bearer-chase` was mid round 3/12 (healthy; not hung).

**Remote hunt (no new poke / no new opcode):**
| path | result |
| --- | --- |
| PresentObj / `+0xBF4`/`+0xBF6` PA | still heap `#636c`; singleton candidate ATU open-bus; no AP write |
| Signed SIT beyond EngMode | no proven PIN>READY / Present=2 builder left untried |
| CPIF capabilities | AP part0=`0x3`, CP part0=`0x7` (bits PKTPROC_UL + CH_EXT + 36BIT on CP); `pktproc`/`pktproc_ul`/`toe` sysfs present ? negotiated at INIT_START; **not** a SIM READY lever |

**Automation:** `saai-modemd soft-lock` treats pin1=2 (ENABLED_VERIFIED) as
MODEM-06 soft-lock alongside pin1=3. `tray-bearer-chase` logs caps, refuses
concurrent instances, heartbeats `uptime_s` per round, longer bearer poll,
optional `get-ps-service` after EDGE. Deployed + single watch re-armed.

**Next:** physical tray reseat while watch armed. Do not re-run EngMode /
MAIN patch / invented opcodes. Goal incomplete.

## 2026-09-30: sysrq reboot + clean soft bring-up (still chicken-egg)

**Pre-reboot:** ONLINE; app=PIN pin1=2; tray-chase armed; rmnet rx=0
tx residual; no IPv4.

**Action:** killed watches; `sysrq` b; USB NCM/COM13 back (~24s);
`/data/saaios/bin/reboot-soft-bringup.sh` (CPIF then handover then
`probe-no0200`). Stock `probe-handover` **polluted** hold with many
`0x0200` SIM lines (prefer `probe-handover-clean` ? now installed on
device and preferred by script). Killed contaminated hold; modem already
ONLINE COMPLETE.

**no-0x0200 NET:** Radio already ON (state=10); PrefLTE err0; AllowData
err=2; voice reg=3; data reg=0; rmnet 0/0 no IPv4.

**VerifyPin:** boot pin1=**1** (not 2); CardPower+VerifyPin A+AID ?
err0, pin1 **1?2**, app stayed **PIN**. GetPsService `0x0711` err0
byte12=1 ? still PIN, no bearer.

**Post:** soft-lock restored (PIN+pin1=2). One `tray-bearer-chase`
re-armed (VerifyPin-on-EDGE). CPIF caps AP=3 CP=7 unchanged.

**Changed vs pre-reboot:** only transient pin1=1 window before same
Pin1Verified chicken-egg; no READY/Present/reg/bearer change.

**Automation:** one command after next sysrq:
`sh /data/saaios/bin/reboot-soft-bringup.sh` then arm chase. Repo copy:
`os/targets/panther/diagnostics/reboot-soft-bringup.sh`.

**Bearer verified?** no. **Next:** physical tray reseat while watch armed.
Goal incomplete.

## 2026-09-30: CardPower tray-analog ? physical reseat

**Hypothesis:** signed `BuildSetSimCardPower` `0x024c` DOWN(4)?UP(1) might
produce ABSENT?PRESENT / EDGE without a physical tray pull.

**Live before:** ONLINE; app=PIN pin1=2 remain=3 card=PRESENT; rmnet rx=0;
one `tray-bearer-chase` armed (round 3).

**ONE cycle:** `/tmp/cardpower-reseat` (proven opcode only; no VerifyPin in
helper). Both DOWN/UP **error 0**. Mid-after-DOWN: transient odd app byte,
card stayed **PRESENT** (never ABSENT). Post-UP: **pin1 2?1** NOT_VERIFIED,
app still PIN, remain=3, card still PRESENT.

**Chase:** updated `sim_left_pin` treats PIN+pin1?{0,1} as EDGE (VerifyPin
window); immediate start check. Caught EDGE ? `card-then-verify-a` ?
VerifyPin A+AID **err 0**, pin1 **1?2**, app **still PIN**. Bearer chase:
data reg=0, rmnet tx residual / **rx=0**, no IPv4.

**Verdict:** CardPower **?** physical reseat. It re-opens the pin1=1
VerifyPin window only; no ABSENT?PRESENT, no DETECTED/READY, no Present=2.
Same chicken-egg after Pin1Verified. Helper for repeat: `cardpower-reseat.c`
+ `cardpower-reseat.sh` next to `reboot-soft-bringup.sh`.

**Bearer verified?** no. **Next:** physical tray reseat while chase armed.
Do not spam CardPower. Goal incomplete.


## 2026-09-30: RE PIN?READY + post-EDGE automation

**Live one-liner:** ONLINE; soft-lock app=PIN(2) pin1=2 present_infer=notin;
rmnet rx=0 tx~384; no IPv4. Watch re-armed (`tray-bearer-chase` rounds=12)
with strengthened post-EDGE tools (`allow-data-once` / `get-ps` / `radio`
resolved from `/data/saaios/bin`).

**RE (what sets READY / Present=2):** Sole `SET_APP` #5 @ `0x14fb5c6` gated
`LDRB +0xBF6` CMP#2. `+0xBF6` = sole STATUS copy of PresentObj[0]. Sole
reachable Present=2 = FN_A (CDMA TIMING_LATCH/MEAS arg==3). EU image
`No CDMA in SupportedRatMap` ? FN_A never fires. PresentObj heap ?
AP ATU/SHMEM unreachable. **No signed SIT / no AP poke** forces READY.
Physical tray re-enumerate remains the only proven EDGE (rild/unsigned
banned).

**Live try this turn:** none (RE exhausted; no new opcode/PA evidence).

**Automation:** `saai-modemd post-edge` plans
VerifyPin?START_NETWORK{1,4,5}?LTE_ONLY?AllowData?GetPsService?reg?rmnet;
`tray-bearer-chase.sh` executes on EDGE (tests 38/38). Deployed + re-armed.

**Bearer verified?** no. **Next:** physical tray reseat while watch armed.
Goal incomplete.

## 2026-09-30: FN_A / RatMap remote lever closed (no live CDMA preferred)

**Live one-liner:** ONLINE (~uptime 1980s); soft-lock app=PIN(2) pin1=2
remain=3 present_infer=notin_1_2_3; rmnet rx=0 tx=384 residual; no IPv4.
`tray-bearer-chase` armed (pid watch round 1/12) ? reseat anytime.

**RE (SupportedRatMap / FN_A):**
- Map arrives inbound on `QM_MM_INIT` from GMC/RRM; TCS `CDMA_SUPPORT` from
  **reg/NV** (`@[TCS] GV updated from reg`). No `SetRapMap` / SIT writer.
- Sole reachable Present=2 = FN_A arg==3 from CDMA TIMING_LATCH / MEAS only.
- `0x070a` preferred 11 (LTE_ONLY) and 12 (LTE_WCDMA) already live ? neither
  adds CDMA to RatMap nor fires FN_A. LTE_ONLY is **not** a unique FN_A
  block; RatMap absence is.
- CDMA preferred enums exist in factory sit-stream but are **not** RatMap
  writers > **no strong evidence** for a live try; skipped (not EFS RW, but
  cargo-cult preferred would not unlock Present=2).
- Flipping TCS/RatMap = **EFS RW** > banned.

**Live try this turn:** **none** (RatMap lever blocked; EngMode/CardPower/
Present poke / unsigned MAIN not repeated).

**Watch:** confirmed armed single chase + tray-watch; do not duplicate.

**Docs/code:** MODEM-BLOCKER ? FN_A/RatMap + operator reseat steps;
`saai-modemd` soft-lock/post-edge advice notes RatMap blocked; SetupDataCall
still deferred until READY+real APN.

**Bearer verified?** no. **Next:** physical tray reseat while watch armed.
Goal incomplete.

## 2026-09-30: SetupDataCall soft-lock + persistent tray watch

**Live one-liner:** ONLINE; soft-lock app=PIN(2) pin1=2 remain=3; rmnet rx=0
tx=432 residual; no IPv4. Persistent `tray-bearer-chase` (PERSIST=1) armed
batch=1 ? reseat anytime. Heartbeat `/data/saaios/var/tray-bearer.alive`.

**Remote levers:** closed (EngMode / Present poke / ATU / CardPower / FN_A
RatMap). Operator must physical SIM tray reseat for EDGE/Present.

**SetupDataCall (RE + soft-lock):**
- sit-stream simple `BuildSetupDataCall` `0x78c80`: id `0x0600`, length
  **246**; TD overload length 983 unused.
- APN at packet offset 16 (max 100); factory default proto IPV4V6=3 when DB
  empty. `isValidPdpApn` requires non-NULL context field.
- `saai-modemd post-edge`: SetupDataCall armed only with usable APN; soft-lock
  PIN always `setup_data_call=blocked_soft_lock`; no APN ->
  `deferred_no_apn`. Never invent carrier string ? write
  `/data/saaios/etc/apn`.
- Tools: `setup-data-call`, `get-data-call-list` (`0x0602` empty GET).
- Chase wires GetDataCallList + SetupDataCall after GetPs/reg when START_NETWORK
  open and APN file present.

**Watch persistence:** `PERSIST=1` (default) re-arms batches; flock single
instance; `tray-bearer.alive` heartbeat. Confirm:
`cat /data/saaios/var/tray-bearer.alive`; log lines `batch=` /
`PERSIST re-arm`.

**Tests:** `cargo test -p saai-modemd` 42 passed.

**Bearer verified?** no. **Next:** physical tray reseat while watch armed;
optionally place carrier APN in `/data/saaios/etc/apn` before EDGE so
SetupDataCall fires on READY. Goal incomplete.

## 2026-09-30: Life APN seed + tray-GPIO skip + chase re-arm

**Live one-liner:** ONLINE; soft-lock app=PIN(2) pin1=2 remain=3; rmnet0
rx=0 tx residual only; no IPv4. `tray-bearer-chase` PERSIST=1 re-armed so
TOOLS pick up `/data/saaios/bin/{setup-data-call,get-data-call-list}` and
`/data/saaios/etc/apn` (were missing at prior start). Heartbeat
`/data/saaios/var/tray-bearer.alive`.

**APN seeded?** yes ? Life public default hostname only to
`/data/saaios/etc/apn` (8 bytes); tree `diagnostics/apn.example-life`.
Contents not logged. SetupDataCall still gated on READY (soft-lock).

**GPIO/tray-detect try?** searched live sysfs/DT/`gpioinfo`/`debug/gpio` ?
no SIM tray detect/hotplug export; only `cpif/sim/ds_detect` (dual-SIM
param). **No pulse** (unsafe/unknown). Physical reseat remains EDGE.

**Bearer verified?** no. **Next:** physical SIM tray reseat while watch
runs; on EDGE/READY chase should VerifyPin-if-needed ? Radio/LTE ?
SetupDataCall(APN) ? verify rmnet IPv4 or rx+tx. Goal incomplete.

## 2026-09-30: ds_detect inspect + ONE 2->1->2 pulse (inert)

**Live one-liner:** ONLINE; soft-lock app=PIN(2) pin1=2 remain=3 card=1;
rmnet0-3 rx=0 tx=480 residual; no IPv4. `tray-bearer-chase` PERSIST=1
alive (batch=1 advanced round 2->3/12 during ~3 min poll).

**Path:** `/sys/devices/platform/cpif/sim/ds_detect` (`-rw-r--r--`), twin
`/sys/module/cpif/parameters/ds_detect` (`-rw-rw-r--`). Live value **2**.

**Kernel (s5300 `modem_ctrl_s5100.c`):** `ds_detect_store` only assigns the
module static + `mif_info`; **does not** call `update_ctrl_msg`. Mailbox
`ds_det` (`get_ds_detect()` = `ds_detect-1`) is written only in
`init_control_messages` at bring-up. Not Present enum / tray hotplug.

**ONE experiment (chase watching):** read 2 -> write 1 -> read 1 -> write 2 ->
read 2. Both writes rc=0; dmesg `set ds_detect: 1` then `: 2`. modem_state
stayed ONLINE. Chase TRANSITION unchanged: app=2(PIN) pin1=2
`present_infer=notin_1_2_3`. No ABSENT/PRESENT/READY. sit query lock-busy
under tray-watch (expected); snapshot from chase log.

**Watch (~3 min):** heartbeat solid; rounds advanced; no physical EDGE.
APN file still present (8 B); SetupDataCall tools on path.

**Bearer verified?** no. **Next:** physical SIM tray reseat while PERSIST
watch runs (remote ds_detect lead closed -- bring-up mailbox param only).
Goal incomplete.

## 2026-09-30: SET_APP is CP-internal only (not AP SIT)

**Live one-liner:** phone unreachable (`192.168.0.104` ping/22/5555 down;
adb empty; LAN `.100`/`.101` ping-only, no ssh/adb). Bearer/watch unknown
this turn ? cannot re-arm PERSIST until host is back.

### Challenge: is `SET_APP` (READY#5 after Present/+0xBF6 CMP#2) an AP SIT opcode?

**Verdict: internal-only CP function.** No AP-facing SIT opcode / builder /
payload layout to call SET_APP or force READY(5). **No live send.**

| Evidence | Result |
| --- | --- |
| HAL strings `SET_APP` / `GET_APP` / `FORCE_READY` / `SetSimApp` / `APP_STATUS` | **absent** in `sit-stream.so` / `sit-base.so`; libsitril has only `GetAppStateEv` + `APPSTATE_*` RIL enum strings (read path) |
| `ProtocolSimBuilder::Build*` | status/PIN/PUK/IO/ATR/channel/auth/facility/slotmap/Uicc/CardPower only ? **no** SetApp / ForceReady / SetSimStatus |
| sit-base `SIT_*` name table (SIM/APP/READY filter) | `SIT_GET_SIM_STATUS` exists; **no** `SIT_SET_APP` / `SIT_SET_SIM_APP` / `SIT_FORCE_READY`. Closest SETs already exhausted: `SIT_SET_SIM_CARD_POWER`, `SIT_SET_UICC_SUBSCRIPTION`, `SIT_SET_PIN_CONTROL` |
| sit-stream aarch64 MOVZ SIM-family ids | `0x0200`?`0x0212`, `0x0247`/`0x0249`/`0x024c`/`0x024d`/`0x0250`, ? ? **GET status is `0x0200`**; no SET sibling that writes app_state |
| MAIN BL?SET_APP `0x19916d2` | **14** sites (imm 0/1/2/3/4/5/6/7); **none** have MOVW `#0x02xx` within ?0x100 |
| Sole `STRB.W #+0xBF4` | `0x1991734` inside SET_APP body only |
| Sole `STRB.W #+0xBF6` | `0x14fb380` STATUS Present copy only (re-confirmed) |
| READY(#5) | still sole `0x14fb5c6` after `LDRB +0xBF6` `CMP #2` |

**Naming:** project label `SET_APP`/`GET_APP` = CP Thumb helpers at
`0x19916d2` / `0x18ec8c0` (no ASCII symbol in MAIN). AP sees only the
**mirror** via `SIT_GET_SIM_STATUS` (`0x0200` app_state@17 = `+0xBF4`).

**Alternate Present writer (optional this turn):** no second `+0xBF6`
store; Present=2 path still FN_A-only (prior). Do not invent opcodes.

**Live try:** none (no AP opcode + layout). **Watch:** unknown (host down).
**Bearer verified?** no. **Next:** restore phone link ? confirm/
re-arm `PERSIST` tray watch ? physical SIM tray reseat. Goal incomplete.

## 2026-09-30: host restore via USB NCM + PERSIST re-arm

**Reachability:** Wi?Fi `192.168.0.104` still down (wlan0 no IPv4; adb empty).
Recovered on **USB NCM** `172.31.7.1` (ping + TCP 22) and **COM13** console.
SSH pubkey still denied from this host; shell via `com13.ps1`.

**Live one-liner:** ONLINE; soft-lock app=PIN(2) pin1=2 remain=3 card=1;
rmnet0-3 rx=0 tx residual (~480-528); no IPv4. Prior Wi?Fi outage did not
drop CP ONLINE; old PERSIST chase had stacked duplicates + sit lock-busy
after partial killall ? cleaned by PID kill, flock cleared.

**Watch re-armed?** yes ? single `tray-bearer-chase` pid, `PERSIST=1`
`WATCH_ROUNDS=12`, heartbeat `/data/saaios/var/tray-bearer.alive`
(batch=1 round=1/12), APN `/data/saaios/etc/apn` still present (8 B),
SetupDataCall/tools on path. Chase script size matched local (15651) ?
no redeploy needed.

**Bearer verified?** no. **Next:** physical SIM tray reseat while PERSIST
watch runs. Goal incomplete.
## 2026-09-30: live PresentObj DRAM locate ? SCAN NEGATIVE (no poke)

**Live one-liner:** ONLINE; soft-lock app=PIN(2) pin1=2; rmnet rx=0 tx~528;
no IPv4. `tray-bearer-chase` PERSIST=1 alive (batch=1 round~10?11/12).
COM13 + USB NCM `172.31.7.1` (SSH pubkey still denied).

**Hypothesis:** static PresentObj PA unknown, but live CP DRAM RO via
`saaios_sim_ro` / `saaios_shm_ro` / `saaios_cp_poke` / `cp-dram-mmap-probe`
might locate `#636c` / `+0xBF4`/`+0xBF6` as a kernel-reachable PA.

### Scan (RO only; modem stayed ONLINE)

| surface | result |
| --- | --- |
| `saaios_sim_ro` SHMEM WIRE | 12 hits @FMT ? **0x0200 ring copies** (app=2 pin1=2); not heap |
| `saaios_sim_ro` SHMEM STATUS | coincidental `+0xBF4/5/6` in FMT bytes ? **not** unique `#636c` PresentObj |
| `saaios_sim_ro` BTL 8?1MiB | ATU -32 or **all-ff** empty; no SIM STATUS |
| `saaios_shm_ro` | TOTAL_HITS=8 FMT copies only; srinfo/raw 0 |
| ATU RO singleton hyp `0x88b87b7c` (VA`0x48b87b7c`+`0x40000000`) | ATU ok, **RO=0xff** open-bus |
| ATU RO `+0xBF4`/`+0xBF6` hyp `0x88b88770`/`0x88b88772` | **RO=0xff** |
| ATU RO raw VA `0x48b87b7c` | **RO=0xff** |
| BTL base `0x87200000` | **RO=0xff** |
| `cp-dram-mmap-probe` iod mmap | all **ENODEV** (boot0/ipc0/rfs0); no userspace CP DRAM |

**Writable PA found?** no. **Poke tried?** no (confidence gate failed;
do not write open-bus / ring copies). Log: `/data/saaios/var/presentobj-scan.log`.

**Bearer verified?** no. **Watch status:** still armed PERSIST (left running).
**Next:** physical SIM tray reseat while watch runs. PresentObj heap remains
AP-unmapped (ATU/SHMEM/mmap exhausted for this object). Goal incomplete.

## 2026-09-30: stock EU Present/READY path ? STATUS on SADR, not FN_A

**Live one-liner:** ONLINE; soft-lock app=PIN(2) pin1=2 remain=3
`present_infer=notin_1_2_3`; rmnet0?3 rx=0 tx=528 residual; no IPv4.
`tray-bearer-chase` PERSIST=1 alive **batch=2** round~6/12 (re-armed after
batch=1 expiry). COM13; USB NCM `172.31.7.1` up (SSH pubkey denied).

### What we missed (paradox split)

| Piece | Prior claim | Correction |
| --- | --- | --- |
| READY *trigger* | implied via FN_A/CDMA path | STATUS_WRAP **only** from **`MMCIF_L1LC_DSL1C_SADR_MEASURE_RSP`** (`0x14b7ca8`) + L1TUNNEL LTE RF (`0x1a3e6cc`) ? **LTE** |
| Present=2 *latch* | sole FN_A | **unchanged** ? FN_A CDMA LATCH/MEAS only; FN_B dead; EU RatMap no CDMA |
| `+0xBF6` writers | sole STRB `0x14fb380` | still sole **Present** STRB; STRH `#bf6` sites are **other** structs (L1LC pack / `0xffff` clear) |
| SET#1 `STRB #2` @`0x146ac48` | candidate Present=2 | **false** ? `getobj(#0x10)` + helper `0x1991838` (L1LC msg), not `#636c` |
| pin1=VERIFIED(2)?READY | maybe blocked mid-branch | **no branch** ? STATUS READY only `+0xBF6==2` after GET?{6,7} |
| CardPower vs tray | CardPower ? ABSENT?PRESENT | HotSwap INSERT strings (`SIM_HOT_SWAP` / `HotSwapInsertTimer`); **no** signed SIT/mailbox to fake; `ds_detect` inert |

**Stock EU READY** = HotSwap/cold INSERT ? DETECTED ? (Present latch) ? LTE
camp ? **SADR_MEASURE_RSP ? STATUS ? SET#5** when `+0xBF6==2`. FN_A is the
Present latch, **not** the READY trigger. EU No-CDMA still blocks the latch
on this MAIN ? that hole remains; the miss was treating FN_A as the only
READY-related path.

### Live try

**None.** No signed SIT/mailbox reproduces HotSwap INSERT or injects
SADR_MEASURE_RSP (SADR needs camp; camp denied while PIN).

### Does physical reseat still suffice on EU No-CDMA?

**Required** for ABSENT?PRESENT / HotSwap re-enumerate (CardPower cannot).
**Not proven sufficient** for Present=2 latch without FN_A ? chase must
observe READY/`rmnet` after reseat. Leave PERSIST armed.

**Bearer verified?** no. **Next:** physical SIM tray reseat while PERSIST
watch runs. Goal incomplete.

## 2026-09-30: HotSwap INSERT chain + getobj#0x10 + RatMap RO (stock-EU gap)

**Live one-liner:** ONLINE; soft-lock app=PIN(2) pin1=2 remain=3
`present_infer=notin_1_2_3`; rmnet* rx=0 tx?576; no IPv4.
`tray-bearer-chase` PERSIST=1 **alive batch=3** round~2/12 (re-armed after
batch=2). COM13; USB NCM `172.31.7.1` up.

### HotSwap INSERT vs Present=2 / FN_A

| Probe | Result |
| --- | --- |
| `[HOTSWAP] HotSwapInsertTimer*` / `A[SIM_HOT_SWAP] SIM insertion*` @`0x44xxxxxx` | **DBT-only** ? zero MOVW/MOVT/ADR/literal-pool code xrefs (unlike `0x40d0?` TCS strings) |
| SET#1 callers | still exactly `0x14538e4`, `0x1a2abf8` ? neither BL FN_A / STATUS_WRAP / STRB Present=2 / `+0xBF6` |
| `getobj(#0x636c)` sites | still **7**; Present=2 STRB still **only** FN_A `0x14f6a16` |
| CardPower `0x024c` | unchanged ? no ABSENT?PRESENT; not HotSwap |

**Can Present=2 on EU without FN_A?** **Not found in RO.** HotSwap HW can still
drive ABSENT?PRESENT?SET#1 DETECTED (physical EDGE), but the INSERT handler
chain does **not** latch PresentObj`[0]=2`. Stock-EU READY therefore still
requires either an unseen Present=2 writer or a non-FN_A meaning of the gate ?
gap remains; tray reseat is the live falsifier.

### getobj `#0x10` (SET#1 @`0x146ac48`)

| Fact | Detail |
| --- | --- |
| Site | SET#1 fn: `MOVS r1,#0x10` ? `getobj` ? `MOVS #2; STRB [obj,#0]` ? `BL 0x1991838` |
| Helper strings | `L1LC_INT_SYNC/MEAS/IRAT_MEAS/?_TIMER_EXPIRED` ? **L1LC MsgHandler** object |
| Path tag | `../../../LTESAE/LteL1/L1LC/Code/src/L1LC_MsgHandler.c` |
| PresentObj? | **no** (not `#636c`; no FN_A/STATUS/`+0xBF6`) |
| Signed SIT lever? | **no** standalone ? only as SET#1 side-effect after INSERT/DETECT |
| Live try | **none** (no READY evidence) |

### RatMap RO (no EFS RW)

| Evidence | CDMA? |
| --- | --- |
| Log polarity | only `No CDMA in InitRapMap!` / `No CDMA in SupportedRatMap` (positive `CDMA in InitRapMap!` is the `No ` prefix of the same string) |
| `SetSupportedRatMap` / `EnableCdmaRat` / `Found CDMA` | **absent** |
| `TCS_CDMA_SUPPORT` / `DS_TCS_GV_CDMA_SUPPORT` | NV/reg GV id `0x127` ? flip = **EFS RW banned** |
| Preferred `0x070a` | not a RatMap writer (already live-neg) |
| Live hex `SupportedRatMap(0x?)` dump in tree logs | **not present** this turn; image/behavior still **CDMA = no** |

### SADR signed AP path?

`STATUS_WRAP` still **only** 2 callers (SADR_MEASURE_RSP + L1TUNNEL).
`SADR_MEASURE_REQ` strings are L1?RRC internal. No `BuildSadr` / `sitTxSadr`
SIT. Camp denied while PIN ? **no live try**.

### Live tries this turn

**None** (getobj`#0x10` / SADR / CDMA preferred all lack strong signed evidence).

**Bearer verified?** no. **PERSIST:** left armed. **Next:** physical tray
reseat; chase must prove READY/`rmnet` ? RE predicts Present=2 still missing
unless HotSwap hits an unseen writer. Goal incomplete.

## 2026-09-30: SET#5 / Present gate re-verification (fresh eyes)

**Image:** MAIN B `saaios-probe-b-modem.bin` sha256 `449eeab3?` (stock, not
the patched-ready sibling).

### Method

Whole-MAIN Thumb scan for `BL ? SET_APP (0x19916d2)` with preceding
`MOVS r0,#imm`; inventory `LDRB.W`/`STRB.W` `+0xBF6`; STATUS decode
`0x14fb322..0x14fb620`. Script:
`os/targets/panther/diagnostics/tmp-reverify-set5-gate.py`.

### Results

| Claim | Result |
| --- | --- |
| How many `SET_APP#5`? | **exactly 1** @`0x14fb5c6` (14 BL total; imms 0/1/2/3/4/5/6/7 each sole except #0?7) |
| Gate before `#5` | `LDRB.W [r0,#0xBF6]` @`0x14fb5bc` ? **`CMP r0,#2`** @`0x14fb5c0` ? **`BNE 0x14fb632`** ? `MOVS r0,#5` ? BL SET_APP |
| Other Present values ? `#5`? | **No.** `==0` CBZ?SET#2 PIN; `==1`?SET#3 PUK; `==3`?SET#4 PERSO; only `==2`?READY |
| Second STATUS/SET_APP entry skipping Present? | **No.** Extra `LDRB.W +0xBF6` @`0x14fb672` feeds notify `BL 0x20e184e` (not SET_APP) |
| Misidentified object? | Sole `STRB.W +0xBF6` still @`0x14fb380` (STATUS copy PresentObj[0]). No alternate READY store |
| STATUS entry while PIN | `GET_APP` CMP#1 / CMP#4 then **BNE.W ? `0x14fb4ca`** (APP 6/7 only). Live PIN skips Present copy / SET#3/#2/#4/#5 |

**Verdict:** gate is still **Present/`+0xBF6 == 2` only**. Not weaker.
No signed sequence can satisfy it without Present=2 ? **no live try**.

### Live / chase this turn

ONLINE; soft-lock app=PIN(2) pin1=2 `present_infer=notin_1_2_3`; rmnet*
rx=0 tx residual; no IPv4. `tray-bearer-chase` hardened: on TRANSITION /
ABSENT?PRESENT / soft-lock hold logs
`EDGE_DECISION ? app=? pin1=? present_infer=? post_edge=yes|no` (no secrets).
PERSIST re-armed batch=1. Physical reseat remains the falsifier.

**Bearer verified?** no. **Next:** physical tray reseat while watch runs;
EDGE_DECISION lines decide whether post-edge ran.


## 2026-09-30: Physical tray HotSwap (ABSENT?PRESENT) ? Present=2 falsifier

**Live:** USB NCM `172.31.7.1` + COM13; modem ONLINE; PERSIST `tray-bearer-chase` armed (batch=1 round=6).

### Timeline
1. Tray out: `card=ABSENT` apps=0; chase `saw_absent=1`; ABSENT-edge Radio/LTE/AllowData/GetPs with `START_NETWORK_ALLOWED=no` (no app); rmnet rx=0 tx=576.
2. Reinsert during round 6: `tray-watch: ABSENT?PRESENT`; `TRANSITION card=1 apps=1 app=2(PIN) pin1=1 present_infer=notin_1_2_3`.
3. EDGE: `pin_verify_window` pin1=1 remain=3 ? VERIFY_TOOL CardPower+VerifyPin A.

### Post-EDGE (signed only)
- VerifyPin A: `verify_err=0`, remain=3, **pin1 1?2** (ENABLED_VERIFIED).
- **app stayed PIN(2)** ? never DETECTED/READY (`saw_detected=0` `saw_ready=0`).
- `chase_gate START_NETWORK_ALLOWED=no` (app?{1,4,5}); SetupDataCall gated.
- Radio/LTE_ONLY/AllowData/GetPs still ran; reg=`registration_raw=0` radio_tech=UMTS.

### Decisive final snap (post-VerifyPin)
`card=PRESENT` apps=1 **app=PIN(2)** **pin1=2** remain=3; `present_infer=notin_1_2_3`; rmnet0?3 **no IPv4**, rx=0 tx=576.

### Verdict
Physical HotSwap **does** produce ABSENT?PRESENT and a real pin1=1 VerifyPin window. VerifyPin **succeeds** but **does not** latch Present=2 / READY. Confirms EU Present=+0xBF6==2 gate: reseat alone is insufficient for START_NETWORK/bearer.

**Bearer verified?** no.

## 2026-09-30: present_infer validation + HotSwap hard blocker

**Live one-liner:** ONLINE (~uptime 12.7ks); soft-lock app=PIN(2) pin1=2
`present_infer=notin_1_2_3`; reg=0; rmnet* rx=0 tx residual; **no IPv4**.
`tray-bearer-chase` PERSIST alive batch=1 round~8/12 (observability only).

### present_infer vs GET_APP / MAIN +0xBF6 ? **valid**

| Check | Result |
| --- | --- |
| `0x0200` offsets | card@12 apps@14 type@15 **app@17** pin1@72 remain@74 (factory HAL / `note_sim`) |
| GET_APP `0x18ec8c0` | sole `LDRB.W [obj,#0xBF4]` @`0x18ec8ec` ? byte17 mirrors **app_state**, not Present |
| +0xBF6 on wire? | **never** ? Present only via STATUS STRB @`0x14fb380` then SET_APP |
| Inference table | Present 0?PIN(#2)=`notin_1_2_3`; 1?PUK=`was_1`; 2?READY=`was_2`; 3?PERSO=`was_3` |
| SET#5 | still sole @`0x14fb5c6` after `LDRB +0xBF6` **CMP #2** |
| STRB +0xBF6 | still sole @`0x14fb380` |
| STATUS_WRAP callers | still exactly 2 (`0x14b7ca8` SADR_MEASURE_RSP, `0x1a3e6cc` L1TUNNEL) |
| Tool bug? | **No** ? HotSwap landed PIN with Present=0 decision; CP does not secretly hold Present=2 while publishing PIN under EU No-CDMA |

Script: `os/targets/panther/diagnostics/tmp-validate-present-infer.py`.

Caveat (documented, not a parse bug): while GET_APP==PIN, STATUS head
CMP#1/#4 skips Present re-eval. Infer = last published STATUS?SET decision.
EU FN_A never writes Present=2, so sticky-PIN cannot hide Present=2 under bans.

### Bypass hunt ? none under bans

| Candidate | Verdict |
| --- | --- |
| SET#5 skip Present==2 | **no** ? sole site gated CMP #2 |
| Alternate STATUS entry / wrong object | **no** ? sole STRB +0xBF6; getobj#0x10 = L1LC not Present |
| L1 SADR_MEASURE_RSP inject | **no signed SIT**; STATUS_WRAP only from L1; needs camp (denied while PIN) |
| HotSwap / SET#1 ? Present=2 | RO DBT-only; **live falsified** |
| FN_A without CDMA | EU RatMap No-CDMA; EFS TCS banned |

**Live try this turn:** **none** (no strong signed evidence after HotSwap falsifier).

### Hard blocker

EU No-CDMA + Present=2 FN_A-only + HotSwap insufficient.
Remaining options only: **EFS TCS** (banned), **unsigned MAIN** (rejected),
**stock rild** (banned). Post-READY CPIF (SetupDataCall/GetDataCallList/APN)
already on device ? unreachable until READY.

**Bearer verified?** no. Goal incomplete. See MODEM-BLOCKER.

## 2026-09-30: stock libsitril radio-on?READY RE + missing empties live

### Live soft-lock (COM13 / UsbNcm)

**Live one-liner:** ONLINE; soft-lock app=PIN(2) pin1=2 remain=3
`present_infer=notin_1_2_3`; rmnet0-3 rx=0 tx=720; **no IPv4**.
`tray-bearer-chase` PERSIST re-armed batch=1 after stock-missing experiment.
SSH pubkey still denied; shell via `com13.ps1`.

### Stock vs ours (sit-stream / libsitril dynsym + prior RUNTIME)

Radio-on ? SIM READY stock SIT order:

1. `0x0801` GetRadioState
2. `0x0800` RadioPower ON (`DoRadioPower`)
3. `0x0200` GetSimStatus (`DoGetSimStatus` / `OnGetSimStatusDone`)
4. `0x070b` / `0x070a` preferred get/set
5. `0x0704` network selection auto
6. `0x0700`/`0x0701` registration poll
7. `0x0201` VerifyPin **only if** app=PIN and pin1 enabled
   (`CheckAndAutoVerifyPin` / `DoVerifyPin`)

Already covered on soft-lock (incl. VerifyPin A+AID ? pin1=2). Adjacent
stock builders also already live: `0x0209`, `0x0212`, `0x020d`/`0x0247`,
`0x024d`, `0x024c`, `0x0249`, `0x0710`, `0x0908`.

**Missing signed cmds on the READY critical path:** **none.** No builder
for Present poke / `SET_APP` / `SIM_INIT` / `START_STACK`. Property-gated
unsent (`0x0250` slotmap, `0x4603` encrypted PIN) are not READY writers.

**Never-sent empty GETs (builder-proven, not READY-path):**

| Builder | id | len | Stock caller |
| --- | --- | --- | --- |
| `BuildSim3GPbCapa` | `0x0245` | 12 | `UiccPhonebookHandler::DoGet3GPbCapa` |
| `BuildGetPreferredCallCapability` | `0x0930` | 12 | `MiscService::DoGetPreferredCallCapability` |

Scripts: `diagnostics/tmp-stock-sim-bringup-re.py`,
`tmp-post-verify-path.py`, `tmp-decode-unsent-stock.py`,
`diagnostics/stock-missing-gets.c`.

### ONE live replay

`diagnostics/stock-missing-gets` (static aarch64): baseline `0x0200` ?
`0x0801`/`0x070b` ? **`0x0245`** ? **`0x0930`** ? post `0x0200` + rmnet.

| Step | Result |
| --- | --- |
| pre | app=PIN(2) pin1=2 remain=3 `present_infer=notin_1_2_3` |
| radio | state=10 (ON); preferred=11 |
| `0x0245` | length=13 **error_raw=0** |
| `0x0930` | length=16 **error_raw=0** |
| post | **still** PIN(2) pin1=2 `notin_1_2_3` |
| rmnet | rx=0 tx residual; **no IPv4** |

**Verdict:** stock AP SIT gap for Present/READY **closed** under bans.
Difference vs retail EU READY is **not** a missing signed opcode we can
replay ? remaining gap is CP Present=2 (FN_A/CDMA RatMap) / NV-EFS /
rild-private. Do **not** start rild or EFS RW.

**Bearer verified?** no.

**Next:** under bans, no further signed SIT aimed at Present/READY;
document-only unless policy allows EFS TCS or unsigned MAIN (both
previously rejected). Post-edge pipeline remains ready for `{1,4,5}`.

## 2026-09-30: SIT NV stubs + no alt CDMA-signed CP

### Live (no extra SIT ? chase holds lock)

**Live one-liner:** ONLINE (~uptime 21.8ks); soft-lock app=PIN(2) pin1=2
`present_infer=notin_1_2_3`; rmnet0-3 rx=0 tx=720; **no IPv4**.
`tray-bearer-chase` PERSIST batch=1 (observability). Image B
`449eeab3?` / `g5300q-260317-260505-B-15346003` still embeds
`No CDMA in SupportedRatMap`.

### SIT NV / TCS write (libsitril + sit-stream RO)

| Symbol | Verdict |
| --- | --- |
| `ProtocolMiscBuilder::BuildNvReadItem(int)` | **size 8 stub** `MOV X0,XZR; RET` |
| `ProtocolMiscBuilder::BuildNvWriteItem(int,char*)` | **same stub** ? only MiscBuilder stubs |
| `ProtocolMiscNvReadItemAdapter::GetValue` | **stub** |
| sit-base Nv symbols | **none** |
| `MiscService::DoNvReadItem` / `DoNvWriteItem` | present in libsitril; cannot emit SIT without builders |
| Prior mis-read of `0x90c`/`0x90d` | those MOVZ belong to **ModemActivityInfo** / **SetOpenCarierInfo**, not NV |

Ban nuance honored: mounting original EFS RW is banned; a working SIT NV
path would have been in-policy to consider. **No usable path** ? no item-id
table, no RatMap/TCS payload layout ? **no live NV try**.

Adjacent (skipped ? not evidenced RatMap/CDMA enable): `BuildCdmaSubscription`
`0x90f`, `SendSGCValue` `0x404`, `BuildSetCpCarrierConfig` (`0x214` blob),
`BuildRadioConfigReset` `0x907`.

Scripts: `diagnostics/tmp-nv-sit-hunt.py`, `tmp-nv-decode.py`,
`tmp-nv-stubcheck.py`, `tmp-nv-carrier.py`, `tmp-sgc-ab-cdma.py`.

### Alt signed CP / MAIN / NV defaults

| Image | Role |
| --- | --- |
| B live / `fw/saaios-probe-b-modem.bin` | EU No-CDMA; Present=2 FN_A-only |
| A `fw/saaios-probe-a-modem.bin` | same No-CDMA strings + same gate (prior RO) |
| `fw-saaios-probe-b-modem-PATCHED-ready.bin` | unsigned ? UDL DONE reject (prior) |
| US/CDMA / other SKU signed MAIN | **not found** under Projects/worktrees |

**Not booted** alternate: nothing loadable that adds CDMA to RatMap under
existing CPIF UDL without unsigned patch.

### Impossible under bans vs newly actionable

| Still impossible | Newly actionable |
| --- | --- |
| EFS RW TCS flip; unsigned MAIN; rild/cbd; SADR invent; Present poke; reseat; stock empties; **SIT NV write (stubs)** | **none** on device/tree ? need external signed CDMA-RatMap CP or policy change |

**Bearer verified?** no.

**Next:** source a **signed** panther/close-SKU CP image with CDMA in
SupportedRatMap (or NV defaults that set `TCS_CDMA_SUPPORT` at load without
EFS RW), then ONE UDL boot; else policy exception. Do not spam SIT.

## 2026-09-30: FN_A re-analysis (LTE/IRAT/SADR) + US image hunt

### Live (brief)

COM13 up; `tray-bearer-chase` PERSIST holding `sit-sim-status tray-watch`
(IPC busy for one-shot queries). Soft-lock from chase log:

| field | value |
| --- | --- |
| app | **PIN (2)** |
| pin1 | **2** (ENABLED_VERIFIED) |
| `present_infer` | **notin_1_2_3** |
| rmnet0 | rx=0 tx residual; **no IPv4** |
| READY | never |

### FN_A call graph (MAIN B `449eeab3?`, whole-image BL scan)

Script: `diagnostics/tmp-fna-lte-irat-re.py`, `tmp-fna-alias-check.py`.

| edge | count / site | note |
| --- | --- | --- |
| **BL ? FN_A `0x14f692c`** | **exactly 2** | `0x14c3872`, `0x14f6d7a` |
| BL ? WRAP_A `0x14c380e` | **1** @`0x14b7766` | MMC case lo16 **`0xec0b`** @`0x14b775a` = `MMC_LTEL1_CDMA_MEAS_RESULT_IND` |
| BL ? LATCH `0x14c388e` | **1** @`0x14b79e2` | MMC case lo16 **`0xefb3`** @`0x14b79d6` = `MMC_LTEL1_CDMA_TIMING_LATCH_CNF` |
| BL ? WRAP_SIM | **1** (from LATCH path) | feeds FN_A with `[msg+8]` rat_mode |
| BL ? STATUS_WRAP | **2** | `0x14b7ca8` SADR_MEASURE_RSP; `0x1a3e6cc` L1TUNNEL ? **no** nearby BL FN_A |
| UMTS_MEASURE / UMTS_TDD cases | lo16 `0xeee0` / `0xef38` | **different** handlers; **not** WRAP_A/FN_A |
| FN_A Present=2 | `MOVS #2; STRB [obj,#0]` @`0x14f6a14` | gated **`CMP arg,#3`** (`arg==3` ? 2 else 3) |

**SADR_MEASURE_RSP / L1 ? Present store?**  
SADR/L1TUNNEL ? **STATUS_WRAP only** (READY *evaluation* / `+0xBF6` copy).  
They do **not** call FN_A and do **not** write PresentObj[0]=2.

**Can LTE/NR IRAT feed the same latch?**  
**No on this MAIN.** Sole FN_A callers are CDMA MEAS IND + CDMA TIMING_LATCH.  
`LTE_MEAS_RESULT` / `NR_MEAS_RESULT` strings exist but have **no** literal/BL edge into WRAP_A/FN_A. UMTS measure cases do not BL FN_A.

**Verdict:** FN_A Present=2 latch is **CDMA-only**. LTE scan/radio-on/SADR cannot elicit Present=2 under signed SIT. **No live LTE?Present=2 sequence** (no lever).

**Stock paradox (unchanged):** retail EU READY with CDMA-only Present=2 writer remains unexplained (FN_A mischaracterization **rejected** by this re-scan; other Present writer / different stock path still open).

### US/CDMA signed image hunt

| source | result |
| --- | --- |
| Host `diagnostics/fw/` A+B | both embed `No CDMA in SupportedRatMap`; same FN_A gate (prior) |
| Projects / Downloads / caches | **no** US/Verizon/TMO-named panther radio/modem |
| Local factory | `saaios-s01/.../image-panther-cp2a.260705.006.zip` ? `modem.img` |
| Factory `modem.img` SHA-256 | `491993b01530f3ddd6d100bf9f2af7a21934de4bbf7469724ef8e38fc55e5487` |
| Factory version | `g5300q-260317-260505-B-15346003` (same family as live B) |
| Factory CDMA | **`No CDMA in SupportedRatMap` n=1** ? **not** a CDMA-RatMap SKU |
| Google factory page | panther is **single-SKU** per build; no separate US/CDMA radio package listed |
| Boot experiment | **not tried** ? candidate is same No-CDMA B family; UDL would not change FN_A reachability |

**Signature OK for CDMA RatMap?** N/A ? no CDMA-capable signed candidate found.  
**Tried?** no.

**Bearer verified?** no.

**Next:** paradox remains; under bans need either (1) external signed MAIN/NV-defaults that actually set CDMA in SupportedRatMap (not another EU factory zip), or (2) policy exception (EFS TCS / unsigned). Do not re-run LTE preferred / SADR hopes / factory-same radio boot.

## 2026-09-30: Present init default ?2; clean-boot present_infer

### Hypothesis under test

EU image has FN_A Present=2 latch CDMA-only, yet stock EU READY works.
Hypothesis: **Present defaults to 2 at object init**, and our path
(HotSwap SET#1 / CardPower / 0x0200 / EngMode / soft-lock) **clears it to 0**;
FN_A only restores on CDMA.

### RE ? Present / +0xBF6 writers (MAIN B `449eeab3?`)

Script: `diagnostics/tmp-present-init-clear-re.py` ? `tmp-present-init-clear-re.out`.

| Probe | Result |
| --- | --- |
| Explicit ctor `[PresentObj,#0]` @`0x1a552a4` | **`MOVS #0; STRB [r6,#0]`** ? **Present init = 0**, not 2 |
| Early STRB after getobj`#636c` @`0x1a5521a` | lookback imm **0** @`0x1a552a6` |
| Present=2 near `#636c` | **sole** FN_A `0x14f6a16` |
| Present=0 near `#636c` | **6** sites: ctor `0x1a552a6` + clears `0x13bd58e`, `0x147dbd2`, `0x158be66`, `0x1990fe8`, `0x1991064` |
| STRB `+0xBF6` | **sole** STATUS mirror `0x14fb380` (copies PresentObj[0]) |

**Verdict:** init-default=2 hypothesis is **FALSE**. Present starts **0**; FN_A is still the only Present=2 latch. Clears exist but they are not ?undoing a 2 default.?

### Live ? clean reboot soft-bringup (no CardPower / EngMode / HotSwap)

Tool: `diagnostics/clean-present-snap.sh` (also `/data/saaios/bin/?`).

1. sysrq `echo b`; shell back ~uptime 87s (cpif unloaded).
2. `probe-handover-clean` ? ONLINE (probe_rc=0; handover log has no SIM/0x0200).
3. **Immediate** first `0x0200` via `sit-sim-status query-sim-status` ? **before** any tray cycle / CardPower / EngMode / RadioPower.

| Field | Clean first GET |
| --- | --- |
| card | PRESENT (1) |
| apps | 1 |
| app_state | **PIN (2)** |
| pin1 | **1** (ENABLED, not verified) |
| remain | 3 |
| `present_infer` (from app?STATUS table) | **`notin_1_2_3`** (Present decision 0?SET#2) |
| rmnet* | rx=0 tx=0; **no IPv4** |

**Compare to post-HotSwap soft-lock:** same `present_infer=notin_1_2_3` / app=PIN; HotSwap only moved pin1 1?2. Clean boot never held Present=2 for us to clear.

**Bearer verified?** no.

### Stock EU Present setter (still open)

Who sets Present=2 on retail EU without CDMA remains unexplained. This turn
rules out: (a) ctor default=2, (b) our bring-up uniquely clearing a prior 2.
Still true: sole Present=2 writer on this MAIN is FN_A (CDMA). Google
developers.google.com panther factory remains **single-SKU** per build; no
separate US/CDMA radio package (re-checked; local factory modem already
No-CDMA).

**Next:** hunt non-STRB / aliased PresentObj[0]=2 writers, or stock EU
STATUS path that treats Present!=2 as READY; else external CDMA-RatMap signed
CP / policy exception. Do not re-reseat / EngMode / CardPower hoping to
"restore" Present=2.

### SET#5 / Present bypass hunt (2026-09-30 evening) ? **none**

Whole-MAIN B (`449eeab3?`): sole SET#5 still `0x14fb5c6` + `LDRB +0xBF6` CMP#2;
sole +0xBF6 STRB STATUS copy; sole PresentObj(#636c) Present=2 = FN_A;
wide/STM/BF4 stores are ctor/enum noise; mid-STATUS BL hits are false friends.
Scripts: `tmp-set5-bypass-hunt.py`, `tmp-set5-alt-entries.py`. See MODEM-BLOCKER.

**Live (COM13):** ONLINE; `app=PIN pin1=2 present_infer=notin`; rmnet 0/0; no IPv4.

**Stock-EU paradox:** unresolved under current MAIN RE. No signed-elicitable
bypass ? **no live try**. Next = external signed CDMA-RatMap CP **or** policy
exception (EFS TCS / rild / unsigned MAIN). Do not reseat.

## 2026-09-30: live MAIN SET#5 CMP DRAM patch -- **not writable**

In-policy lever: patch **running** MAIN in DRAM (not UDL) at sole READY gate
`LDRB +0xBF6` / `CMP #2` near BL SET#5.

| item | value |
| --- | --- |
| MAIN B SHA | `449eeab3...` (live) |
| file LDRB / CMP / BL#5 | `0x14fb5bc` / `0x14fb5c0` / `0x14fb5c6` |
| stock Thumb | `90 f8 f6 0b 02 28 36 d1 05 20...` |
| VA LDRB / CMP | `0x414f49ac` / `0x414f49b0` (= `0x40010000+(file-0x16c10)`) |
| hyp ATU PA | `0x814f49ac` / `0x814f49b0` (VA+`0x40000000`, BTL formula extension) |
| intended patch | NOP BNE or force EQ->READY; **not applied** |

### Live (COM13 / USB NCM, ONLINE soft-lock)

| check | result |
| --- | --- |
| soft-lock | `app=PIN(2)` pin1=1 `present_infer=notin`; rmnet0 rx=0 tx=0; **no IPv4** |
| PCIe warm | SIT burst -> `current_link_width=2`, poke `link_status=1` |
| ATU RO `@0x814f49ac` (SET#5 LDRB hyp) | ATU **ret=0**, DUMP **16x `ff`**, WIN all-ff -- not stock Thumb |
| ATU RO `@0x814f49b0` (CMP) | same **`ff`** |
| ATU RO `@0x414f49ac` (VA) | same **`ff`** |
| BTL control `@0x87200000` | ATU 0, **`ff`** (empty log window) |
| `cp-dram-mmap-probe` | iod **ENODEV** -- no userspace CP DRAM map |
| `/dev/mem` | absent |

**Writable?** **No** -- hyp/VA code PA is open-bus on OB2, same class as SET#2 /
PresentObj. **Patch tried?** **No** (would brick/noise on open bus). **app_state
after:** still PIN. **Bearer verified?** **no** (rmnet rx=0, no IPv4).

Script on device: `/data/saaios/var/set5-ram-ro2.sh` -> `set5-ram-ro2.txt`.
Host: `diagnostics/set5-ram-ro.sh`, `set5-ram-ro2.sh`. Restore bytes: n/a.

**Next:** live RAM MAIN patch via modem ATU is **dead** (SET#2 + SET#5). Need
external signed CDMA-RatMap CP/MAIN **or** policy lift (EFS TCS / rild /
unsigned UDL). Do not random-poke further ATU windows.

## 2026-09-30: A) host soft-lock vs CP START_NETWORK gate + B) CDMA image hunt

### A) Force signed START_NETWORK while PIN (COM13 / USB NCM) ? **CP-enforced**

Pre: ONLINE; pin1=1. VerifyPin A+AID ? pin1=2, app stayed PIN.
Then one `set-preferred-lte` + **`set-network-selection-auto` (`0x0704`)**:

| Field | After `0x0704` | +8s poll |
| --- | --- | --- |
| `0x0704` | **error_raw=2** (len 12) | ? |
| app / pin1 | PIN / 2 | PIN / 2 |
| voice / data reg | 0 / 0 | 0 / 0 |
| rmnet0?5 | rx=0 tx=0 | rx=0 tx=0 |
| IPv4 on rmnet* | none | none |

AllowData err0; GetPsService err0. **CP rejects camp start while PIN** ? host
`app?{1,4,5}` gate matches CP. No bearer. Do not re-spam `0x0704` under PIN.

### B) External CDMA-RatMap hunt ? **no soft UDL**

| Image | Result |
| --- | --- |
| CP2A.260705.006 modem/radio (Desktop factory zip) | `B-15346003`; `No CDMA in SupportedRatMap`=1; `EnableCdmaRat`=0; SHA modem `491993b0?` ? **identical EU** |
| TD1A.221105.001 radio (downloaded; zip SHA `10a338fe?`) | **`g5300g-?-B-9040061`**; no SupportedRatMap/CDMA/EnableCdmaRat strings; FBPK ? live g5300q TOC ? **not UDL-compatible** |
| cheetah/lynx CP2A (~3.9GB each) | HEAD 200; DL aborted after panther CP2A proved identical EU |

**Soft UDL attempted?** **No** (no CDMA-RatMap + CPIF-compatible candidate).
**Bearer verified?** **no**. Goal incomplete.

**Next:** policy exception (EFS `TCS_CDMA_SUPPORT` / stock rild / unsigned MAIN)
or a proven signed CDMA-RatMap `g5300q` image. Do not ATU-poke MAIN; do not
BAR ioremap `0x81400000`; do not reseat hoping for Present=2.

## 2026-09-30: RE `0x0704` preconditions + live brief (no 0x0704 spam)

### Live brief (COM13 / USB NCM)

| Field | Value |
| --- | --- |
| modem_state | ONLINE |
| app / pin1 / remain | PIN(2) / 2 / 3 |
| radio / preferred | 10 ON / 11 LTE_ONLY |
| **selection `0x0703`** | **`selection_mode_raw=0` already auto** (`fix-selection` skipped set) |
| data reg | 0 (UMTS tech raw) |
| rmnet0?5 | rx=0 tx=0; **no IPv4** |

### `0x0704` error_raw=2 ? refined

Prior forced `0x0704` under PIN returned `error_raw=2`. Same-session
`0x0703` now proves **already auto** ? err=2 is the documented
`RCM_E_GENERIC_FAILURE` **already-auto** path (see earlier
`fix-selection` / ?Redundant auto?). **Not** a clean SIM-not-ready
fingerprint by itself.

Camp still blocked independently: START_NETWORK `@0x18e8028` main gate
`@0x18e831a` `GET_APP` CMP **#1 / #4 / #5** else ignore
(`START_NETWORK Ignored: SIM is not ready` @ file `0x4c18afc`). PIN(2)
denied. Present `+0xBF6` is **not** in this CMP list.

SIT `0x0704` register site `@0x1150b14` ? `BL SIT_REG 0x20d1afa` with
descriptor VA `0x407BE4A0` (BSS in flash). Scripts:
`diagnostics/tmp-re-0704-handler*.py`, `tmp-0704-precond-matrix.py`.

### Live try?

**No new satiable signed SIT.** All SIT-satiable preconditions (radio,
pin1, card, preferred, selection) already met; hard unmet is P3
`GET_APP?{1,4,5}` (needs Present=2/FN_A). `fix-selection` only queried;
**did not** re-send `0x0704`.

### g5300q CDMA image hunt (continued)

| Candidate | Result |
| --- | --- |
| cheetah-cp2a.260705.006-factory-**23d564ad** | HEAD 200 ~3.9GB; same-era EU twin expected; **not downloaded** this turn |
| panther-ota-**td1a.221105.003**-32ef0dee (Verizon) | **DL complete** SHA `32ef0dee?` ok; modem carve **`g5300g-?-B-9144834`** ? wrong chip |
| panther-ota-**ap1a.240505.005.a1**-83fca43d (latest panther Verizon) | **DL complete** SHA `83fca43d?` ok; PIXELMODEM?`modem.bin.gz`?TOC **`g5300q-231218-240405-B-11675365`** inner `809d9695?`; **`No CDMA in SupportedRatMap`=1**, EnableCdmaRat=0 |
| Panther Verizon OTA catalog | **ends at AP1A May 2024**; no 2025?2026 Verizon panther SKU on developers OTA page |

**Soft UDL?** **no** ? no g5300q image without `No CDMA in SupportedRatMap`.
**Bearer verified?** **no**.

**Next:** external non-Google signed CDMA-RatMap g5300q, or policy exception
(EFS TCS / rild / unsigned MAIN). Do not blind-UDL AP1A/TD1A; no `0x0704`
spam under auto+PIN; no ATU/BAR poke.
## 2026-10-01: No-CDMA USIM READY model (Verizon proof) ? FN_A framing retracted

### Live brief (COM13 / USB NCM)

| Field | Value |
| --- | --- |
| modem_state | ONLINE |
| app / pin1 / remain | PIN(2) / 2 / 3 |
| present_infer | notin_1_2_3 |
| radio / preferred | 10 ON / 11 LTE_ONLY |
| data reg | 0 |
| rmnet0-5 | rx=0 tx=0; **no IPv4** |
| peek-present-surfaces | PresentObj AP-unmapped; ds_detect!=Present |

### RE: SET_APP / GET_APP / Present (MAIN B + Verizon AP1A)

Scripts: `diagnostics/tmp-usim-nocdma-ready-re.py`, `tmp-usim-present-init-re.py`,
`tmp-simrefresh-sit-re.py`.

Both images: `No CDMA in SupportedRatMap`; sole READY = SET_APP#5 behind
`+0xBF6` CMP #2; sole PresentObj[0]=2 = FN_A-class; SET#1 DETECTED has no
Present gate; STATUS_WRAP = SADR+L1TUNNEL; sit-stream has no SIM_INIT /
START_STACK builders (SimRefresh = STK adapter only).

Stock sitril after VerifyPin: re-poll `0x0200` only ? **no** second READY SIT.
GetImsi exists but is post-READY / secret-bearing ? **not sent**.

### Corrected model

FN_A/CDMA L1 is **not** the stock USIM READY path (Verizon No-CDMA reaches
READY without RatMap CDMA). Present=2/SET#5 machine still exists on No-CDMA
SKUs; the Present=2 latch for stock USIM remains unidentified. Soft-lock =
STATUS Present=0?PIN while DETECTED, then stuck (PIN skips Present re-eval).

### Live try

**None** ? no missing signed READY SIT found this turn. RO peek only.

### Bearer verified?

**no**.

### Next

Hunt non-FN_A Present=2 / USIM boot-init writers (internal SIM_INIT, not
sit-stream), or policy (cbd/rild). Stop CDMA-RatMap/FN_A-as-USIM framing.
No blind-UDL AP1A; no ATU MAIN poke; no BAR `0x81400000`; no `0x0704` spam.

## 2026-10-01: non-FN_A Present=2 + SIM_INIT/USIM boot-init hunt

### Live brief (COM13 / USB NCM)

| Field | Value |
| --- | --- |
| modem_state | ONLINE |
| app / pin1 / remain | PIN(2) / 2 / 3 |
| present_infer | notin_1_2_3 |
| radio / preferred | 10 ON / 11 LTE_ONLY |
| data reg | 0 (UMTS tech) |
| rmnet0?5 | rx=0 tx=0; **no IPv4** |
| peek-present-surfaces | PresentObj AP-unmapped; ds_detect?Present |

### RE: Present=2 writers (MAIN B `449eeab3?`)

Scripts: `tmp-sim-init-present2-hunt.py`, `tmp-trace-sim-init-handlers.py`,
`tmp-sim-init-msgtable.py`, `tmp-dump-sim-init-handler.py`,
`tmp-trace-2f50-senders.py`.

| Probe | Result |
| --- | --- |
| `#636c` + MOVS#2 + STRB/#0 (Thumb16 **or** STRB.W) | **only FN_A** `0x14f6a14`/`0x14f6a16` (`CMP r9,#3` IT EQ ? `#2`) |
| BL?FN_A | still **2** (CDMA MEAS + TIMING_LATCH) |
| BL?STATUS_WRAP | still **2** (SADR + L1TUNNEL) |
| STR*.W `+0xBF6` | STATUS STRB.W `0x14fb380` + 4? STRH.W false-friends (no `#636c`/SET_APP nearby) |
| SET_APP imm inventory | sole `#5` still `0x14fb5c6`; `#1` DETECTED `0x146aaba` (no Present) |

**Non-FN_A Present=2 path:** **not found** (aliased STRH/`+0xBF6`, SIM_INIT
handlers, START_STACK handlers, bare-msgid consumers in USIM band ? none store
Present=2).

### RE: SIM_INIT / USIM boot-init (internal, not sit-stream)

| Item | Evidence |
| --- | --- |
| State | `USIM_WAIT_FOR_INIT_REQ`; log `USIM_NOT_INITIALISED. Waiting for SIM_INIT_REQ` |
| Timer name | `USIM_SCHEDULE_SIM_INIT_TIMER` (name table only; no new AP lever) |
| Msg name | `USIM <== SIM_INIT_REQ` / `USIM <== SIM_START_STACK_SERVICES_REQ` |
| Table id word | `0x2f50` (INIT), `0x2f58` (START_STACK) beside name ptrs @`0x10fb07c` / `0x10fb88c` |
| Table ?handler? `0x43909d49` / `0x4390ecf5` | **codec/pack** at file `0x3910958` / `0x3915904` ? **zero** `#636c` / SET_APP / FN_A / `+0xBF6` |
| `sitInformSimInit()` | CP log string only; **0** MOVW/MOVT or ptr xrefs found |
| sit-stream / libsitril | **no** `SIM_INIT` / `START_STACK` builders; SimRefresh = STK only |
| Bare MOVW `#0x2f50` (no MOVT) | 24 sites (IPC/log ids); **0** in USIM band `0x14f0000..0x1600000` |
| Many MOVW `#0x2f50`+MOVT | false friends (pointer assemble e.g. `0x44a82f50`) |

**Soft-path skip (hypothesis, refined):** stock bring-up eventually delivers
internal `SIM_INIT_REQ` (msgid `0x2f50`) into USIM while
`USIM_NOT_INITIALISED` / `WAIT_FOR_INIT_REQ`; our soft CPIF path has no
sit-stream opcode for that IPC and does not start cbd/rild producers. **But**
the traced INIT/START_STACK table handlers do **not** latch Present=2, so
forcing INIT alone is **not** a proven READY fix even if an AP path existed.

### Live try

**None** ? no signed SIT/mailbox evidence to emit `SIM_INIT_REQ` / force
Present=2 without cbd/rild / EFS / unsigned MAIN. Avoided: AP1A UDL, ATU MAIN
poke, BAR `0x81400000`, `0x0704` spam, CDMA-RatMap framing.

### Bearer verified?

**no**.

### Next

1. Trace **who sends** bare msgid `0x2f50` into USIM (GMC/NS/boot task) and
   whether that producer is gated on a soft-CPIF-skippable boot IPC ? still
   expect **no Present=2** from INIT itself.
2. Keep hunting **non-STRB PresentObj[0]=2** (memcpy/template/NV default into
   `#636c` object) ? sole remaining MAIN hole under Verizon No-CDMA proof.
3. Policy: cbd/rild SIM bring-up, or EFS ? only if bans lift.

No blind-UDL AP1A; no ATU/BAR poke; no invent SIM_INIT SIT.

## 2026-10-01: who sends `0x2f50` + non-STRB Present=2

Scripts: `tmp-classify-2f50-senders.py`, `tmp-2f50-producer-deep.py`,
`tmp-2f50-descriptor-hunt.py`, `tmp-2f50-catalog-owner.py` on MAIN B.

### Live brief

**No ADB** this turn (Pixel USB = MTP/charging phantoms only). Prior sticky
state assumed: ONLINE / PIN+pin1=2 / `present_infer=notin` / no bearer ? **not
re-verified**.

### Who sends `SIM_INIT_REQ` (`0x2f50`)?

**AP OEM IPC host (cbd/rild), not an internal CP producer.**

- Catalog entry `@0x6de740`: `msgid=0x2f50` `SIM_INIT_REQ` beside VerifyPin/Info
  in an **[OEM][IPC]/[OEM][SIT]** data bank.
- GMC sends START_STACK (`0x2f58` @`0x19f7dc` + `GMC ==> SIM__ [START_STACK?]`)
  but **never** SIM_INIT (`GMC ==> SIM__` list has no INIT; GMC-band `0x2f50`
  descriptors = **0**).
- `USIM_SCHEDULE_SIM_INIT_TIMER` / `USIM_WAIT_FOR_INIT_REQ` = consumer wait,
  not send.
- Soft CPIF / sit-stream: still **no** INIT builder; `sitInformSimInit()` = log.

**Soft elicit without cbd/rild?** **No.** Would need OEM-IPC emit (banned /
unsigned relative to our SIT builders). No mailbox/SIT alias evidenced.

### Non-STRB PresentObj[0]=2

**None.** `#636c` + Present=2 store patterns still **only FN_A**
`0x14f6a14`/`0x14f6a16`. No memcpy/template/wide-store latch found.

### Live try / bearer

**None** (no device ADB + no signed lever). **Bearer verified? no.**

### Next

1. Restore ADB ? live brief.
2. Stock No-CDMA Present=2 path still unidentified under bans (INIT ? Present=2).
3. Policy only for cbd SIM_INIT / EFS; no invent OEM-IPC INIT; no UDL/ATU/BAR.

## 2026-10-01: OEM IPC `0x2f50` framing vs soft VerifyPin

Scripts: `tmp-oem-ipc-2f50-framing.py`, `tmp-oem-ipc-2f50-wire.py`,
`tmp-oem-ipc-table-decode.py`, `tmp-disasm-2f50-ril.py`,
`tmp-oem-ipc-header-hunt.py`.

### Live brief

COM13 + USB NCM `172.31.7.1` restored (SSH pubkey still denied; shell via
`com13.ps1`). CP **ONLINE**; radio_state=10; SIM **PIN** pin1=2 remain=3;
`present_infer=notin`; data reg=0; rmnet 0; **no** cbd/rild processes.

### Soft VerifyPin transport (reuse check)

Soft-lock VerifyPin is **SIT** on `/dev/umts_ipc0`:

- type `0`, id **`0x0201`**, length 38, token? (factory `BuildSimVerifyPin`)
- OEM catalog sibling **`SIM_VERIFYPIN_REQ` = `0x2f52`** (`flags=0xa`) is a
  **parallel** msgid space ? **not** what soft sends.
- MAIN: **no** MOVW duals `0x0201`?`0x2f52` or `0x0200`?`0x2f50`.

Therefore soft CPIF/SIT cannot be assumed to carry OEM `0x2f50` by swapping
the SIT id field.

### Catalog `@0x6de740` layout (evidenced)

- Record stride **28** (`flags:u16`, `msgid:u16`, `name_va:u32`, `meta:u32`, ?).
- `SIM_INIT_REQ`: `flags=2`, `msgid=0x2f50`, `meta=0x10104`, response id **0**.
- `flags=2` matches other empty-ish REQs (`SIM_INFO_REQ`, `SIM_STOP_REQ`) ?
  plausible **OEM-encoded body size**, not a full host frame.
- Outer OEM header (magic / length / seq / channel) **not** recovered from
  encode strings or sit-stream builders.

### Channel

- `oem_ipc0..7` present under `/sys/class/cpif` (`oem_ipc0` = 493:12).
- mknod node: open ? **Permission denied** (chmod 666 does not help).
- Prior RO listen: immediate EOF / no ASCII without enable protocol.
- sit-stream: **no** `SIM_INIT` builder; u16 `0x2f50` count **0**.
- libsitril: `MOVZ #0x2f50` only in a multi-msgid table init; string
  `SIM_INIT or SIM_RESET` is eSIM refresh log text, not a sender.

### Live try / bearer

**None** ? framing incomplete; did not invent host bytes. Sticky state
unchanged (not re-sent). **Bearer verified? no.**

### Next

1. Recover OEM host frame from `cbd` / `ipc_message_server` encode (or stock
   capture on `oem_ipc*` once openable) ? then one soft `SIM_INIT_REQ`.
2. Treat INIT?Present=2 as still unproven even if send succeeds.
3. No invent bytes; no start cbd/rild; no EFS RW / ATU MAIN / BAR / UDL.

## 2026-10-01: OEM host encode RE + oem_ipc0 open (no send)

Scripts: `tmp-oem-ipc-host-encode.py`, `tmp-oem-ipc-handler-frame.py`,
`tmp-oem-ipc-missing-fields.py` (+ prior framing scripts).

### Live brief

COM13. CP **ONLINE**; radio=10; SIM **PIN** pin1=2 remain=3; data reg=0
radio_tech=UMTS; rmnet rx=0; **no** cbd/rild.

### Encode path RE

| Source | Finding |
| --- | --- |
| `cbd` (`raw-cbd`) | `/dev/umts_boot0`, `/dev/umts_ramdump0` only ? **not** OEM IPC |
| CPIF SIT write (`ipc_io_device.c`) | Kernel prepends **EXYNOS 12B** when `iod->link_header`; userspace writes **app payload only** |
| MAIN `ipc_message_server` | Encode/decode log strings + `kMessageId<>` typed assert; **app header layout unrecovered** |
| libsitril `@0x95660` | msgid registry stride24 `id\|0x23\|0\|0x402\|idx`; MOVZ table-init only; **no** `/dev/oem_ipc`; **no** builder |
| sit-stream / sit-base | **no** `oem_ipc` / **no** `0x2f50` builder |
| Catalog `@0x6de740` | unchanged: `flags=2` body-size hint, meta `0x10104`, rsp=0 |

### Exact missing fields (no invent)

1. OEM **app-layer header** layout (SIT-12B reuse / `sipc_fmt_hdr` / other ? all unproven)
2. **2-byte body** for `flags=2` (zeros unproven)
3. Token/seq/transaction rules
4. Which `oem_ipcN` stock uses for SIM_INIT

### oem_ipc0 permissions

Prior EACCES: node often **absent** until `mknod` from sysfs `493:12`. This turn:
`mknod` + `chmod 666` ? `exec 3<>/dev/oem_ipc0` ? **OEM_RDWR_OK** as root.
**Usable for open**; still **not** sendable without app frame.

### Live try / bearer

**No SIM_INIT send** (framing incomplete). **Bearer verified? no.**

### Next

1. Stock capture or host OEM writer binary (rild-side OEM path ? not cbd boot).
2. Prove app header + body from encode/decode, then ONE soft INIT on `oem_ipc0`.
3. Same bans: no start cbd/rild; no invent bytes; no EFS RW / ATU / BAR / UDL.

## 2026-10-01: host OEM encoder hunt (no SIM_INIT send)

Scripts: `tmp-oem-host-encoder-hunt.py` / `hunt2.py`, `tmp-disasm-build-oem-sim.py`,
`tmp-catalog-sim-oem.py`.

### Live brief

COM13 + USB NCM `172.31.7.1`. modem_state=**ONLINE**; radio=10; SIM **PIN** pin1=2
remain=3; present_infer=notin; data reg=0 tech=UMTS; preferred=LTE_ONLY(11);
oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild; SaaiOS image has no `/vendor/lib64/*ril*`.

### Host encoder search

| Binary / symbol | Finding |
| --- | --- |
| sit-stream `BuildOemSimRequest` `@0x806d0` | SIT ids **0x208/0x20c/0x20f/0x247**, hdr **+12** ? umts_ipc OEM-over-SIT, **not** `0x2f50` |
| sit-stream `BuildSimGetStatus` | SIT `0x0200` len12 (known soft path) |
| sit-base `SIT_OEM_*` | carrier/PIN_ENC/CQI/SVN? ? **no** SIM_INIT_REQ |
| libsitril IoChannel | `/dev/umts_ipc0` only; registry `@0x95660` has `0x2f50` but **no** builder / **no** oem_ipc path |
| libsec-ril OemIpc / IpcTxSimInitMessage | classic Samsung SIPC/STK; **0** `0x2f50`/`0x2f52` immediates |
| raw-cbd | boot/ramdump only |
| MAIN oem_ipc_message_* | encode strings only; app header unrecovered |

**Encoder found?** no. **Frame layout?** unrecovered (kernel still: userspace=app
payload; EXYNOS 12B if `link_header`). **Live SIM_INIT?** none. **Bearer?** no.

### Missing fields + offset hints

1. App header layout ? candidate RE: MAIN near encode fail `@0x6dfcf3`, not-REQUEST
   `@0x6dfe0d` (lit `@0x6dff00`)
2. Body 2B (`flags=2`) ? need typed encode or stock capture
3. Token/seq rules
4. `oem_ipcN` channel selection
5. Host binary still missing: stock vendor radio/oemhook that `open("/dev/oem_ipc*")`
   (pull from factory vendor.img ? do **not** start rild)

## 2026-10-01: MAIN encode island + factory SitOem oem_ipc0 (no SIM_INIT send)

Scripts: `tmp-oem-encode-re-2f50.py`, `tmp-oem-encode-movw2.py`,
`tmp-oem-encode-codefind.py`, `tmp-oem-base-xref.py`, `tmp-oem-factory-scan.py`,
`tmp-carve-vendor-oemipc.py`, `tmp-re-sitoem-write.py`.
Vendor extract: `diagnostics/fw/cdma-hunt/factory-td1a-vendor/vendor.img` +
`carved-oemipc-25e7b000.so` (oem_ipc0 SitOem), `carved-oemipc-cf64000.so` (oem_ipc1).

### Live brief

COM13 + USB NCM `172.31.7.1`. modem_state=**ONLINE**; query-sim-status:
card=PRESENT apps=1 **app=PIN** pin1=2 remain=3; oem_ipc0 **OEM_RDWR_OK**;
**no** cbd/rild; rmnet rx=0; **no bearer**.

### MAIN encode path at hinted addresses

| Item | Result |
| --- | --- |
| `@0x6dfcf3` / `@0x6dfe0d` | **rodata strings** only (encode-fail / not-REQUEST) |
| Band `0x6de000..0x6e1000` | catalog + strings + ptr table; **0** PUSH (not code) |
| MOVW+MOVT / LDR.W to encode strings / utils.c | **0** whole-image |
| Catalog `@0x6de740` | body=**2** msgid=`0x2f50` meta=`0x10104` rsp=0 `SIM_INIT_REQ` |
| Catalog `@0x6de874` | body=**10** msgid=`0x2f52` rsp=`0x2fa1` `SIM_VERIFYPIN_REQ` |
| Catalog code xrefs | object helpers via `BL 0xca893e`; **no** proven wire header/body stores |
| `MOVS #12` near catalog xref | error-path return value ? **not** hdr-len proof |

**App header + 2B body for `0x2f50`:** still **unrecovered**.

### Factory vendor.img `/dev/oem_ipc*`

| Binary | Opens | Protocol | SIM_INIT / `0x2f50` |
| --- | --- | --- | --- |
| carved SitOem (`?25e7b000.so`) | **`/dev/oem_ipc0`** | protobuf `sit_ipc_message::IpcMessage` via `ModemData::{initialMessageHeader,protobufSerialDataLen,sendMessageData}` ? `writeModemData` | **absent** (Ping/Config/Thermal/? only) |
| carved log helper (`?cf64000.so`) | `/dev/oem_ipc1` | extended log / modemstat | **absent** |
| whole vendor.img | ? | ? | `SIM_INIT_REQ` count **0**; `SIM_INIT` only STK ?SIM_INIT or SIM_RESET? text |

**Stock oem_ipc0 opener found, but it is not a catalog-`0x2f50` encoder.**

### Missing for sendable soft SIM_INIT (unchanged + refined)

1. Frame that CP accepts as OEM catalog **`0x2f50`** (SitOem protobuf ? proven alias)
2. **2-byte body** (`flags=2`)
3. Token/seq rules
4. Host emitter of catalog `0x2f50` (if any) ? **not** carved SitOem `.so`

### Live try / bearer

**No SIM_INIT send** (frame incomplete; no invent). **Bearer verified? no.**

### Next

1. Map CP OEM decode: does SitOem protobuf ever become internal `0x2f50`, or is
   `SIM_INIT_REQ` a different AP path entirely?
2. Hunt other vendor ELFs / MAIN consumers for bare catalog send of `0x2f50`.
3. Only then ONE soft write on openable `oem_ipc0`. Same bans.

## 2026-10-01: SitOem protobuf ? catalog `0x2f50` (demux falsified)

Scripts: `tmp-sitoem-demux-2f50.py` / `.out`, `tmp-sitoem-demux-2f50b.py` / `.out`,
`tmp-cbd-2f50-hunt.py` / `.out`.

### Live brief

COM13 + USB NCM `172.31.7.1`. modem_state=**ONLINE**; query-sim-status:
card=PRESENT apps=1 **app=PIN** pin1=2 remain=3; oem_ipc0 **OEM_RDWR_OK**;
**no** cbd/rild (not present under `/vendor` on SaaiOS); rmnet rx=0 all;
wlan0 has LAN IPv4 only ? **no rmnet bearer**.

### SitOem ? catalog `0x2f50`?

| Check | Result |
| --- | --- |
| SitOem `sit_ipc_message` types | Ping/Config/Thermal/Metrics/DeviceState/Traffic/Txas/Scone/Coex/Debug/DataFlow/DataValidation/Mch ? **no SimInit** |
| SitOem `MOVZ #0x2f50` / u16 `0x2f50` | **0** / **0** |
| MAIN `[OEM][IPC]` + protobuf encode | PERCALLSTATSKPI_IND / duration/audio lists ? **0** SIM_* in those strings |
| Catalog SIM_* names ? protobuf OEM strings | **[]** |
| Catalog `SIM_INIT_REQ` | still `@file 0x6de740` msgid=`0x2f50` flags=2 meta=`0x10104` |

**Verdict: NO demux.** SitOem protobuf and OEM SIM catalog are separate dialects.
Kernel CPIF does not invent a mapping; MAIN protobuf OEM path never names SIM_*.

### Alternate emitter of catalog `0x2f50`

| Candidate | Result |
| --- | --- |
| vendor `/dev/oem_ipc0` | SitOem only |
| vendor `/dev/oem_ipc1` | log/modemstat helper |
| vendor `SIM_INIT_REQ` / `IpcTxSimInit` / `SimInitMessage` | **0** |
| libsitril | registry MOVZ table-init; **umts_ipc0** only; no builder |
| factory cbd/rild | init.rc refs; **absent** on live SaaiOS; **do not start** |

**No soft-sendable emitter found under bans.**

### Live try / bearer

**No SIM_INIT send** (frame incomplete; no invent Ping-as-INIT). **Bearer verified? no.**

### Next

1. MAIN binary-catalog encode RE (`oem_ipc_message_dispatcher/utils`) for app
   header + 2B body of `0x2f50` ? not SitOem protobuf.
2. Proper offline carve of factory `cbd` ELF (not via init.rc string) and RE ?
   still **no start**.
3. Only then ONE soft write on `oem_ipc0`. Same bans.

## 2026-10-01: MAIN encode island + factory cbd carve (no SIM_INIT send)

Scripts: `tmp-oem-frame-2f50-re.py` / `.out`, `tmp-oem-encode-island.py` / `.out`,
`tmp-carve-cbd-2f50.py` / `.out`.
Carve dir: `diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-cbd/`
(notably `carved-cbd-a45e000.elf` = real cbd).

### Live brief

COM13 + USB NCM. modem_state=**ONLINE**; query-sim-status: card=PRESENT
apps=1 **app=PIN** pin1=2 remain=3; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild;
rmnet0?2 rx=0; **no bearer**.

### MAIN catalog / encode path

| Item | Result |
| --- | --- |
| `@0x6de740` `SIM_INIT_REQ` | body=**2** msgid=`0x2f50` meta=`0x10104` rsp=0 `+0x18=4` |
| `@0x6de874` `SIM_VERIFYPIN_REQ` | body=**10** msgid=`0x2f52` rsp=`0x2fa1` |
| Encode-island ptr table | pairs of log-string + `oem_ipc_message_utils.c` / `dispatcher.c` / `oem_sit_main.c` ? **not** wire serializers |
| Island CODE slots | object handlers; **no** proven STR of msgid/len/token/2B body to TX buffer |
| Catalog xref `@0x32e4494` | message-object helper; mutates fields +8/+9 ? **not** app wire |
| Bare `MOVW #0x2f50` | log/name-reg banks ? **not** emitters |
| USIM `@0x10fb078` | handler `0x43909d49`; internal size `0x10000` ? catalog body |

**App header + 2B body:** still **unrecovered**. **SENDABLE? NO.**

### Factory cbd (offline carve; not started)

| ELF | Role | `0x2f50` / SIM_INIT_REQ / oem_ipc |
| --- | --- | --- |
| `carved-cbd-a45e000.elf` | **real cbd** (S5300, `umts_boot0`/`umts_ramdump0`, many `cbd:`) | **0** / **0** / **0** |
| other boot0-near carves | helpers / prior oem_ipc1 log ELF | no catalog INIT encoder |

`/vendor/bin/cbd` path strings are init.rc text only (no adjacent ELF).

### Live try / bearer

**No SIM_INIT send** (frame incomplete; no invent). **Bearer verified? no.**

### Next

1. Stock OEM catalog wire capture on `oem_ipc*` (or another host encoder binary
   that is not cbd / SitOem protobuf).
2. Only with proven app header + 2B body: ONE soft write on `oem_ipc0` ? poll
   READY ? bearer chase. Same bans.

## 2026-10-01: BuildOemSimRequest RE + live SIM_IO 0x0208 (still PIN)

Scripts: `tmp-oemsim-layout-re.py`, `tmp-buildsimio-layout.py`,
`tmp-oemsim-simio-once.c`.

### Live brief

COM13 + USB NCM `172.31.7.1`. modem_state=**ONLINE**; radio=10; SIM
**PIN** pin1=2 remain=3; data reg=0 tech=UMTS; rmnet rx=0; **no** cbd/rild.

### BuildOemSimRequest map (sit-stream `@0x806d0`)

| RIL `w1` | SIT id | Builder | Notes |
| --- | --- | --- | --- |
| 28 | `0x0208` | `BuildSimIO` | SIM_IO; len `0x23c` |
| 114 | `0x020c` | `BuildSimTransmitApduBasic` | prior live SELECT/STATUS |
| 115 | `0x0247` | `BuildSimOpenChannelWithP2` | prior live OpenChannel+STATUS |
| 117 | `0x020f` | `BuildSimTransmitApduChannel` | prior live channel STATUS |

Passthrough builder (hdr+12 memcpy). **Not** OEM catalog `0x2f50`.

`BuildSimIO` layout (factory): cmd@12, fileid_lo@13, u16@14, path_len@16,
path@17, p1@29 p2@30 p3@31, data_len@32 data@34, pin2@546/547, aid@555/556.
Accepted cmds include `0xF2` STATUS.

### Live try

ONE empty-default `0x0208` STATUS (`0xF2`, zero body). rsp length=528
error=0 SW=`9000`. Post-status: app=2 pin1=2 remain=3 -- **unchanged**.
No VerifyPin (not EDGE). No SetupDataCall. **Bearer verified? no.**

### Next

OemSim SIT quartet exhausted for soft-lock exit. OEM `0x2f50` framing still
unrecovered -- stock `oem_ipc*` capture or non-cbd encoder only.

## 2026-10-01: post-VerifyPin OpenChannel+SELECT+SIM_IO+AID (still PIN)

Scripts: `tmp-post-verify-och-re.py` / `.out`, `tmp-och-fields-re.py`,
`tmp-post-verify-och-once.c`.

### Live brief

COM13 + USB NCM `172.31.7.1`. modem_state=**ONLINE**; radio=10; SIM
**PIN** pin1=2 remain=3; data reg=0 tech=UMTS; rmnet rx=0; **no** cbd/rild.

### RE (sitril / sit-stream)

| Finding | Result |
| --- | --- |
| `OnVerifyPinDone` | **no** BL to OpenChannel / TransmitApdu / SimIO (not a literal post-PIN SIT emitter) |
| `BuildSimOpenChannelWithP2` | id `0x0247` len 30; aid_len@12 aid@13 p2@29 |
| OpenChannel rsp adapter | session u32@12; SW1@16 SW2@17 |
| `BuildSimTransmitApduChannel` | id `0x020f`; session/CLA/INS/P1/P2/P3 words @12..32; data@38 |
| `BuildSimIO` switch | accepts `0xB0/B2/C0/D6/DC/F2` ? **not** `0xA4`; ADF SELECT is via `0x0247`/`0x020f` |

### Live try (ONE, AID from GetSimStatus, never printed)

Opcodes: `0x0200` ? `0x0247`(P2=0) ? `0x020f`(SELECT A4/04) ?
`0x0208`(STATUS F2 **+AID**) ? `0x0200` ? `0x020e`.

| Step | Result |
| --- | --- |
| OpenChannel | len 276 err0 session=1 SW=`9000` |
| Channel SELECT | err0 SW=`9000` apdu_hint=70 |
| SIM_IO STATUS+AID | len 528 err0 SW=`9000` |
| Post GetSimStatus | app=**2** pin1=**2** remain=3 |

**No** EDGE READY. **No** SetupDataCall. **Bearer verified? no.**

### Next

AID-filled OemSim OpenChannel/SELECT/STATUS path closed under soft-lock.
Resume OEM catalog `0x2f50` wire recovery only (no invent). Same bans.

## 2026-10-01: CP RX consumer RE for catalog `0x2f50` / `0x2f52` (no send)

Scripts: `tmp-rx-2f50-consumer.py`, `tmp-rx-2f50-demux-deep.py`.

### Live brief

COM13. modem_state=**ONLINE**; `sit-sim-status`: PRESENT apps=1 **app=PIN**
pin1=2 remain=3; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild; rmnet rx=0;
**no bearer**.

### RX consumer / dispatch

| Check | Result |
| --- | --- |
| Catalog `SIM_INIT_REQ` | body_hint=2 msgid=`0x2f50` meta=`0x10104` rsp=0 |
| Catalog `SIM_VERIFYPIN_REQ` | body_hint=10 msgid=`0x2f52` meta=`0x10104` rsp=`0x2fa1` |
| USIM `<== SIM_INIT_REQ` table | handler `0x43909d49` size_word=`0x10000` (codec / internal) |
| USIM `<== SIM_VERIFYPIN_REQ` table | msgid=`0x2f52` rsp=`0x2fa1` size_word=`0xb` ? catalog 10 |
| Wire header parse from RX | **not recovered** (no SIT-like LDR cluster; MOVW msgid sites are false-friends) |
| Soft SIT `0x0201` vs OEM `0x2f52` | body sizes differ ? do not reuse soft VerifyPin as OEM frame |

### Kernel oem_ipc path (live DT)

`oem_ipc`: attrs=`0x2000` ch=`0x81` fmt=0 ch_count=8. No `ATTR_NO_LINK_HEADER`
? kernel prepends EXYNOS 12B; userspace writes **app payload only**. Same
attrs/fmt pattern as `umts_ipc` (different ch).

### Frame layout / live try

**RX-derived full app frame?** no. **SIM_INIT sent?** no. **Bearer?** no.

### Exact missing

1. OEM app-layer header layout
2. 2-byte body for `flags=2`
3. Token/seq rules
4. Which `oem_ipcN` for catalog SIM_INIT

### Next

Stock catalog OEM capture or deeper RX preprocess/`Message ID not found` code
path ? then ONE soft INIT. Same bans.

## 2026-10-01: OEM IPC preprocess RX + `Message ID not found` (no send)

Scripts: `tmp-oem-preprocess-rx-re.py`, `tmp-oem-preprocess-rx-re2.py`.

### Live brief (COM13)

modem_state=**ONLINE**; `sit-sim-status`: PRESENT apps=1 **app=PIN** pin1=2
remain=3; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild; rmnet0-5 rx=0 tx=0;
**no IPv4 / no bearer**.

### DBT attribution (preprocess / msgid path -- not litpool-alone)

Classic MOVW+MOVT / LDR.W to log-string VAs: **0** (confirmed again). Logs are
Shannon **DBT** records (`magic=0xfecdba98`), not direct string loads.

| Log | DBT line | File path VA | Source file |
| --- | --- | --- | --- |
| `[OEM][IPC] Message is not a REQUEST` | `0x51` | `0x41067caa` | `oem_ipc_message_dispatcher.c` |
| `[OEM][IPC] Message ID not found` | `0x57` | `0x41067caa` | `oem_ipc_message_dispatcher.c` |
| `[OEM][IPC] Failed to preprocess id %d` | `0x62` | `0x41067caa` | `oem_ipc_message_dispatcher.c` |
| `[OEM][IPC] Invalid message` / encode-size / decode | `0x95` / ... | `0x41067cff` | `oem_ipc_message_utils.c` |
| `[OEM][SIT] Received packet from channel %u, size %u` | `0xda` | `0x41068655` | `oem_sit_main.c` |

**RX pipeline (evidenced):** `oem_sit_main.c` receives SIT/oem channel packet
then `oem_ipc_message_dispatcher.c` looks up catalog msgid + **preprocess**;
fail logs above. Encode path siblings live in `oem_ipc_message_utils.c`.

**False friend:** ASCII `preprocess_cb` sites are **gmetrics** clients only --
**not** OEM catalog preprocess.

### Catalog entry layout (refined; still not wire hdr)

28B stride; SIM bank examples:

| off | body | msgid | meta | rsp | +0x14 | +0x18 | name |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `@0x6de740` | **2** | `0x2f50` | `0x10104` | `0` | `0` | **`4`** | `SIM_INIT_REQ` |
| `@0x6de874` | 10 | `0x2f52` | `0x10104` | `0x2fa1` | `0` | `0` | `SIM_VERIFYPIN_REQ` |
| `@0x6de724` | 2 | `0x2f57` | `0x10104` | `0x2fa8` | `0` | `0` | `SIM_INFO_REQ` |

Fields: `u16 body_hint`, `u16 msgid`, `u32 name_va`, `u32 meta`, `u16 rsp`, ...
`SIM_INIT` uniquely has `+0x18=4` in this bank (meaning still **unknown** --
not proven as wire token length).

### Code-xref gap

MOVW+MOVT to DBT record VAs (`va_of(0x6dff18)` etc.): **0**. LDR pc-rel to
msgid/preprocess/sit_recv string VAs outside the DBT island: **0**. Catalog
`SIM_INIT` VA consumer `@0x32e4494` and bank touch `@0x3388318` are **internal
state / other msgid (`0x2f79`)** builders -- **not** proven app-wire serializers
for `0x2f50`.

### Kernel (unchanged)

`oem_ipc` DT: attrs=`0x2000` ch=`0x81` -- no `ATTR_NO_LINK_HEADER` -- EXYNOS 12B
wrap; userspace = app payload only.

### Frame / live try

**App header + 2B body recovered?** **no**. **SIM_INIT sent?** **no** (no invent).
**Bearer verified?** **no**.

### Exact missing (unchanged blockers)

1. OEM **app-layer header** field order/size (dispatcher preprocess still opaque
   without DBT-emit / nanopb descriptor xrefs)
2. **2-byte body** contents for catalog `flags/body_hint=2`
3. Token/seq rules (catalog `+0x18=4` on INIT is a hint only -- **not** wire proof)
4. Which `oem_ipcN` carries **catalog** SIM_INIT (SitOem protobuf already owns
   `oem_ipc0` for a different dialect)

### Next

Stock catalog OEM capture on `oem_ipc*`, or recover dispatcher preprocess by
non-string xref (nanopb / catalog binary-search emit path / host encoder). Then
**ONE** soft `SIM_INIT`. Same bans.

## 2026-10-01: non-string dispatcher / catalog / nanopb RE (no send)

Scripts: `tmp-oem-dispatcher-nonstr-re.py` / `.out`,
`tmp-oem-dispatcher-nonstr-re2.py` / `.out`.

### Live brief (COM13 / USB NCM `172.31.7.1`)

modem_state=**ONLINE**; `sit-sim-status`: PRESENT apps=1 **app=PIN** pin1=2
remain=3; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild; rmnet* rx=0; wlan0 LAN
IPv4 only; usb0=`172.31.7.1` -- **no rmnet bearer**. ADB absent; SSH pubkey
denied; brief via COM13.

### Catalog walk (28B stride; non-string)

| Item | Result |
| --- | --- |
| SIM `*_REQ` bank | 43 entries; stride **0x1c** confirmed |
| `flags/body_hint==2` cohort | **3**: `SIM_INFO_REQ` (`0x2f57`, `+18=0`), `SIM_INIT_REQ` (`0x2f50`, `+18=4`), `SIM_STOP_REQ` (`0x2f51`, `+18=4`) |
| `@0x6de740` raw | `02 00 50 2f ... 04 00 00 00` -- body=2 msgid=`0x2f50` meta=`0x10104` rsp=0 **`+0x18=4`** |
| meta `0x10104` | lo16=`0x104` hi16=`1` -- still opaque enum; common across SIM REQ bank |

### Catalog `+0x18` refined (was "maybe token len")

Cross-domain OEM catalog REQs with `+0x18!=0`:

| `+0x18` | Examples |
| --- | --- |
| **4** | `SIM_INIT_REQ` / `SIM_STOP_REQ`, `CC_INIT_REQ`, `SMS_INIT_REQ`, `SS_INIT_REQ`, `SMREG_INIT_REQ`, `NS_EMM_STOP_NETWORK_REQ`, `NS_MM_STOP_NETWORK_REQ` |
| **5** | Phonebook `PB_*_REQ` bank |
| **1** | Various CC/IMS disconnect/setup REQs |

**Verdict:** `+0x18` behaves as a **class / family tag** (INIT-family=`4`),
**not** proven wire token/seq length. Do not treat `+18=4` as "4-byte token".

### Catalog code consumers (MOVW to entry VA only; litpool=0)

| Entry | Site | Behavior |
| --- | --- | --- |
| `SIM_INIT` `@0x6de740` | `@0x32e4494` | `BL 0x2ca893e` object helper; **STRB #5 to obj+8**, STRB from other to obj+9 |
| `SIM_STOP` `@0x6de9c4` | `@0x32e4508` | sibling; **STRB #1 to obj+8** |
| `SIM_INFO` `@0x6de724` | `@0x3388338` | same helper pattern; builds **other** msgid `0x2f79` into local `{u16,u16,u32}` |
| `SIM_VERIFYPIN` | `@0x331af14` | object helper; `MOVS #12` = error return (not hdr len) |

**No** proven STR of wire msgid/len/token/2B body to TX buffer. msgid-to-fn
tables in catalog island: **0**.

### Nanopb / `[OEM][PB]`

| Check | Result |
| --- | --- |
| `pb_decode` / OEM island | present; logs `[OEM][PB] pb_decode_varint_cb...` |
| `[OEM][PB]` string count | **3** -- decode/varint/buf only |
| SIM / INIT / `0x2f50` in OEM[PB] | **0** |

Nanopb OEM[PB] is not the SIM catalog dialect (consistent with SitOem demux falsified).

### DBT / preprocess (reconfirmed non-string)

MOVW/MOVT/litpool to DBT **record** VAs for msgid_nf / not-REQUEST /
preprocess: **0**. Logs remain DBT-indexed; preprocess emit path still opaque.

### Frame / live try

**App header recovered?** **no**. **2B body contents?** **no** (size hint only).
**SIM_INIT sent?** **no** (no invent). **Bearer verified?** **no**.

### Exact missing (unchanged + refined)

1. OEM **app-layer header** field order/size (dispatcher preprocess still
   opaque without stock capture or proven encode emit)
2. **2-byte body** contents for `body_hint=2` (zeros unproven)
3. Token/seq rules -- catalog `+0x18` is **class tag**, not token proof
4. Which `oem_ipcN` carries **catalog** SIM_INIT (SitOem/protobuf owns a
   different dialect on `oem_ipc0`)

### Next

Stock catalog OEM capture on `oem_ipc*`, or host encoder binary that is not
cbd / SitOem protobuf / internal object-helper. Then **ONE** soft `SIM_INIT`.
Same bans.

## 2026-10-01: GET_APP parse confirmed PIN; SitOem Ping alive

### Live

COM13 + USB NCM. modem_state=**ONLINE**; oem_ipc0 RDWR OK; no cbd/rild.
`0x0200` dump (no AID/IMSI): len=143 card=1 apps=1 **type=2 state=2** pin1=2
remain1=3. stock-missing-gets: app=PIN(2) pin1=2 present_infer=notin; rmnet
rx=0; **no bearer**.

### libsitril / sit-stream re-validation

- `ProtocolSimStatusAdapter::GetPinState`: arg1 -> adapter `#0x58` (=pkt 72),
  arg2 -> `#0x59` (=pkt 73).
- `GetPinRemainCount`: PIN1 -> `#0x5a` (=pkt 74).
- `Init`: apps LDRB `#14`; app records from `#15`.
- `BuildRilCardStatusApplications`: per-app **ADD #63**.
- `covertAppStateToString`: values 1..5 match DETECTED..READY; **2=PIN**.

Live type@15 and state@17 are **both 2** (USIM + PIN) ? parser is correct;
this is not READY misread as PIN. Multi-app N/A (`apps=1`).

### SitOem Ping (oem_ipc0)

Factory evidence: REQUEST=1, PayloadCase ping=5, PingRequest string field=1.
ONE userspace write (raw protobuf, 11B): tags `08 01 | 10 01 | 2a 05/0a 03/0a 01`
+ 1B payload. write_ok; RX 9B starting `08 02` (RESPONSE). Transport proven.
**Not** catalog `SIM_INIT_REQ` / `0x2f50`.

Scripts: `tmp-revalidate-appstate-offsets.py`, `tmp-revalidate-appstate-deep.py`,
`tmp-re-sitoem-ping.py`, `tmp-dump-sim-layout.c`, `tmp-sitoem-ping-once.c`.
Tool: `sit-sim-status.c` now prints type/state/pin1/remain.

## 2026-10-01: SitOem schema exhaust; external 0x2f50 hunt empty

### Live brief

COM13 + USB NCM `172.31.7.1`. modem_state=**ONLINE**; oem_ipc0 **OEM_RDWR_OK**;
no cbd/rild. GET_APP: PRESENT apps=1 **app=PIN** pin1=2 remain=3. rmnet rx=0;
wlan/usb IPv4 only; **no bearer**. ADB empty (Pixel as MTP); SSH pubkey denied.

### SitOem protobuf exhaust (factory carve)

`tmp-sitoem-schema-exhaust.py` on `carved-oemipc-25e7b000.so`:

- **68** `sit_ipc_message::*` mangled types; **22** Arena `CreateMaybeMessage`
  types; wrappers Ping/Config/Thermal/Metrics/DeviceState/Traffic/Txas/Scone/
  Coex/Debug/DataFlow/DataValidation/Mch/KPI inds ? full list in
  `tmp-sitoem-schema-exhaust.out` and MODEM-BLOCKER ? schema-exhaust.
- **SIM/init/card/uicc-related with encode?** **NONE** (Ping / sitInitModem /
  StatsAtom are false friends, not catalog SIM).
- `carved-oemipc-cf64000.so` = log helper; **0** protobuf schema.

**Live SitOem SIM try?** **SKIP** ? nothing to send without inventing.

### External hunt

github / XDA / paste / ShannonBaseband / libsamsung-ipc / pixel-mainline modem
(`cbd-lite`, `sit-smoke`): **no** complete catalog frame with msgid `0x2f50`
and body size **2**. In-tree RE still has catalog meta only.

**Soft catalog write?** **no**. **Bearer?** **no**.

### Next

Stock `oem_ipc*` catalog capture or non-SitOem host encoder for app header +
2B body. Then ONE soft `SIM_INIT`. Same bans.

## 2026-10-01: public FMT header research vs catalog `0x2f50` (no send)

Scripts: `tmp-public-fmt-2f50-map.py` / `.out`; live brief helper
`tmp-run-live-brief-com13.ps1`.

### Live brief (COM13 / USB NCM `172.31.7.1`)

modem_state=**ONLINE**; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild.
`sit-sim-status`: PRESENT apps=1 **app=PIN** pin1=2 remain=3.
rmnet* rx=0; wlan0 LAN IPv4 + usb0=`172.31.7.1` only ? **no rmnet bearer**.
SSH pubkey denied; brief via COM13.

### Public layouts researched

| Dialect | Source | Layout | Maps to catalog `0x2f50`? |
| --- | --- | --- | --- |
| Classic samsung-ipc FMT | morphis `radio.h`; Replicant/Wireshark `ipc_fmt_header` | 7B: `len,mseq,aseq,group,index,type` | **Closest structural guess only** ? `group/index=0x2f/0x50`; public SEC is **group `0x05`** (`IPC_SEC_SIM_STATUS=0x0501`); **no** `SIM_INIT_REQ` / `0x2f50` in `sec.h` |
| Soft SIT (Pixel) | factory sit-stream + live `sit-sim-status` | 12B: `type,pad,id,len,token` | **no** ? id space `0x02xx`; twin `0x0201` vs OEM `0x2f52` body sizes contradict |
| Kernel EXYNOS wrap | public `exynos_build_header` + live Ping kprobe | 12B: sync `ABCD`, seq, cfg `C000`, len, ch `0x81` | **outer only** ? msgid not in header; userspace = app passthrough |
| SitOem protobuf | factory carve + live Ping | protobuf tags | **no** ? schema exhaust has no SIM/INIT |
| Comsecuris / Hardwear / ShannonBaseband | slides + shannonRE | SHM FMT rings + internal `qitem_header` | **no** catalog app header / 2B body |
| AOSP sepolicy | gs201 `file_contexts` | `/dev/oem_ipc[0-7]` | node only; **no** header layout |

### Catalog stride-28 cross-check (unchanged)

| Entry | body_hint | msgid | meta | rsp | +0x18 |
| --- | --- | --- | --- | --- | --- |
| `SIM_INIT_REQ` `@0x6de740` | **2** | `0x2f50` | `0x10104` | 0 | **4** (class tag) |
| `SIM_VERIFYPIN_REQ` `@0x6de874` | 10 | `0x2f52` | `0x10104` | `0x2fa1` | 0 |

`meta=0x10104` does **not** decode to classic sipc `type`, SIT `type`, or EXYNOS
`frag_cfg`. `+0x18=4` remains INIT-family class tag ? **not** wire token length.

### Gaps blocking soft send

1. OEM **app-layer header** field order/size (classic 7B binding unproven; SIT
   reuse falsified; EXYNOS is not app).
2. **2-byte body CONTENTS** for `body_hint=2` (size known; zeros unproven).
3. Token/seq/type rules for catalog REQUEST.
4. Which `oem_ipcN` stock uses for catalog SIM_INIT (SitOem owns a different
   dialect on `oem_ipc0`).

### Live try / bearer

**Fully evidenced frame?** **no**. **SIM_INIT sent?** **no** (no invent).
**Bearer verified?** **no**.

### Next

Stock `oem_ipc*` catalog capture (policy-gated rild one-shot per
`OEM-IPC-CAPTURE.md`) or external dump of catalog app header + 2B body ?
`oem-ipc-inject` + `post-init-chase`. Same bans.

## 2026-10-01: deep vendor const-build for `0x2f50` (MOVZ+MOVK/ORR/rodata)

Scripts: `tmp-vendor-deep-2f50-constbuild.py`,
`tmp-vendor-deep-2f50-constbuild-vendor.py` (+ `.out`). Prior hunts used bare
`MOVZ #0x2f50` only.

### Live brief (COM13 / USB NCM `172.31.7.1`)

modem_state=**ONLINE**; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild.
`sit-sim-status`: PRESENT apps=1 **app=PIN** pin1=2 remain=3.
rmnet* rx=0; wlan0 + usb0 IPv4 only -- **no rmnet bearer**. Injector armed
(`oem-ipc-inject` / `post-init-chase`); EXYNOS wrap known; public FMT incomplete.

### Carved factory-td1a-vendor (11 ELFs/SOs)

| Pattern | Result |
| --- | --- |
| `MOVZ` W/X `#0x2f50` | **0** in all carves (carved `libsitril` is **truncated** ~40KB; full research `libsitril.so` has **1** MOVZ registry site) |
| `MOVZ`+`MOVK` exact build `0x2f50` | **0** |
| `MOVN` `#~0x2f50` (`#0xd0af`) | **0** in carves |
| `ORR` imm (=`0x2f50` / lo16) near IPC/SIM | **0** |
| LE/BE `50 2f` / `2f 50` rodata near oem/SIM/IPC strings | **0** useful (no encoder binding) |
| `/dev/oem_ipc*` openers | SitOem `?25e7b000.so` (`oem_ipc0`) + log helper `?cf64000.so` (`oem_ipc1`) -- **no** catalog msgid / **no** `SIM_INIT` |
| False-friend "msgid table" | sitril-builder `@0xd96c0` u32s like `0x12f50` (relocs/ids), **not** catalog `0x2f50` |

### Full vendor.img + research libsitril

| Check | Result |
| --- | --- |
| vendor `MOVZ_W #0x2f50` | **29** sites; **all** islands `oem_ipc=False` |
| `MOVZ`+`MOVK` exact `0x2f50` | **0** (325 pointer-ish MOVZ+MOVK on lo16 only) |
| `ORR` near oem_ipc / `SIM_INIT_REQ` / `oem_ipc_message` seeds | **0** |
| Cross: oem_ipc ELF islands ? MOVZ `#0x2f50` | **0** (`@0xcf64000`, `@0x25e7b000`) |
| aligned u32 literal `0x00002f50` in oem islands / libsitril | **0** |
| libsitril (2.3MB) | 1? `MOVZ #0x2f50` `@0x11f300` table-init; `umts_ipc` only; **no** builder |

### Frame / live try

**Constant-build encoder for catalog `0x2f50`?** **no**.
**App header + 2B body recovered?** **no**. **SIM_INIT sent?** **no** (no invent).
**Bearer verified?** **no**.

### Miss patterns (document)

1. Bare-MOVZ-only hunts were incomplete in method -- this pass closed MOVZ+MOVK /
   MOVN / ORR / rodata / oem?msgid cross; still empty for a sendable encoder.
2. Carved `libsitril` / small ELFs can omit the registry MOVZ; full SO still has
   **no** oem_ipc encode path.
3. Arithmetic u16/u32 sequences containing `?2f50` are reloc/false friends.

### Policy need (restated)

Under current bans (no cbd/rild, no invent frame bytes), soft catalog
`SIM_INIT_REQ` remains blocked. Need **policy-gated** stock `oem_ipc*` capture
(`OEM-IPC-CAPTURE.md`) or an external evidenced app header + 2B body, then
one-shot `oem-ipc-inject` + `post-init-chase`. Same bans otherwise.

## 2026-10-01: CP USIM self-init hypothesis — FALSIFIED

Hypothesis: stock has no AP encoder for catalog `0x2f50`; maybe CP
self-inits USIM to READY without AP `SIM_INIT`, and prior soft-lock only
failed because SIT floods / VerifyPin-before-READY trapped PIN.

### Method (live)

1. Soft sysrq reboot to CPIF OFFLINE.
2. `diagnostics/passive-selfinit-wait.sh`: `probe-handover-clean` only
   (no `probe-no0200`, EngMode, CardPower, OemSim, `0x0704`, VerifyPin).
3. Passive poll ~5 min: `0x0200` + data-reg + rmnet every ~20s only.
4. Cont helper finished window after waiter loop-var bug (`for i` in snap
   clobbered outer `i`); script fixed to use `rif`.

### Result

| Field | First SNAP t0 | Final cont7 | Changed? |
| --- | --- | --- | --- |
| modem_state | ONLINE | ONLINE | no |
| app_state | **PIN (2)** | **PIN (2)** | no |
| pin1 | **1** (ENABLED_NOT_VERIFIED) | **1** | no |
| present_infer | **0** | **0** | no |
| remain | 3 | 3 | no |
| data reg / tech | 0 / none | 0 / none | no |
| rmnet* rx/tx | 0 / 0 | 0 / 0 | no |

16/16 PARSE lines identical. Handover log: **ONLINE clean** (no early
`0x0200`). **No** READY/{1,4,5}. **No** VerifyPin sent. **No** cbd/rild.

**Self-init observed?** **No.** CP does **not** leave PIN to READY on a
minimal soft ONLINE + passive wait. Soft-lock floods are not the sole
blocker; catalog `0x2f50` / policy path still required.

**Bearer verified?** **no.**

### Next

Policy-gated stock `oem_ipc*` capture for `SIM_INIT_REQ` (`0x2f50`) or
external evidenced frame — same bans; do not invent.

## 2026-10-01: VerifyPin A+AID (pin1=1, no CardPower) > READY

Overnight live was ONLINE / app=PIN(2) / pin1=**1** / reg=0 / no IPv4.
Careful soft-lock path: **VerifyPin A+AID only** (skip CardPower because
pin1 already NOT_VERIFIED). Tool:
`diagnostics/tmp-verifypin-a-uns-watch.c` (RFS-aware; logs type/id/len/err
only — never PIN/AID/IMSI/ICCID).

### Result (reproduced twice this turn)

| Field | Pre | Post VerifyPin |
| --- | --- | --- |
| modem_state | ONLINE | ONLINE |
| app_state | PIN (2) | **READY (5)** |
| pin1 | 1 | **2** (ENABLED_VERIFIED) |
| remain | 3 | 3 |
| data/voice reg | 0 | 0 |
| rmnet IPv4 | none | none |

Direct `0x0201` match often **times out** (45–90s) while RFS `cmd=7` is
seen as `rfs ignored` (byte pattern ? stock unprotect). Response then
appears as **late UNSOL** when a follow-up GET is sent:

- `0x0201` tok=7 len=16 **err=0** (VerifyPin OK, delayed)
- `0x0200` > UNSOL_SIM **app=5 pin1=2**
- Also: `0x0210` (len=8), `0x4604`, `0x4602`, `0x0303`, `0x000d`
  (ids/lens only; no payload decode / no invented OEM frames)

Independent `sit-sim-status` confirmed `app0_state_raw=5` /
`pin1_state_raw=2` / `layout=app16-perso17-v1`.

### Post-edge chase (READY held)

`diagnostics/tmp-ready-bearer-chase.c` (skip Radio ON — already
`radio_state_raw=10`):

| Cmd | Result |
| --- | --- |
| preferred LTE `0x070a` | err **0** |
| selection auto `0x0704` | err **0** (was err=2 under PIN) |
| AllowData `0x0710` | err 0 |
| GetPs `0x0711` | err 0 |
| SetupDataCall | **deferred_no_apn** — `/data/saaios/etc/apn` is 8-byte
  `internet` with **no dot**; `apn_usable()` requires `.` (do not invent
  carrier APN) |
| ~60s camp poll | registration_raw=**0**, tech=0, rmnet rx=0 |

One AP soft-reboot occurred mid-session (uptime>0); cause not attributed
to `do_cp_crash` (never written). Soft handover not needed — CP returned
ONLINE; SIM reset to PIN+pin1=1; VerifyPin>READY **reproduced**.

**Soft-lock chicken-egg for GET_APP>READY: broken** on this pin1=1 path.
**Bearer verified?** **no** — still no camp / rmnet IPv4.

### Next

1. Fix RFS unprotect match so `0x0201` lands inside the wait window.
2. Operator-supplied dotted APN in `/data/saaios/etc/apn` for SetupDataCall
   after camp.
3. Debug why READY + `0x0704` err0 still yields reg=0 (RF/PLMN/antenna /
   further signed NET inds — no invent `0x2f50`).

## 2026-10-01: first passive no-gap IPC/RFS owner boot

The guarded `owner-handoff-bringup.sh` was run after a fresh AP reboot. It
verified the existing NV copies and reviewed modem image, opened IPC0/RFS0
before FIN, forked a detached single reader, then completed the modem boot.
The probe returned 0 and CP remained ONLINE. The owner (PID 594 in this
run) held both descriptors and sent only four read-only SIT snapshot GETs;
it sent **no RFS replies**, no PIN/APN/network mutations and no NV writes.
The original EFS was not mounted.

The initial snapshot returned SIM card raw 0/apps 0, radio raw 1, voice/data
registration raw 0. Header-only RFS trace from the owner's READY point:

| Elapsed | Command | Sequence | Payload length |
| --- | ---: | ---: | ---: |
| 7.282 s | 7 | 0 | 4 |
| 17.282 s | 6 | 1 | 16 |

The first 60.169 s window counted two RFS frames and nine other IPC
indications beyond the four GET replies. `rmnet0` RX remained 0. No command 3
was observed in this no-reply run. This proves that the RFS traffic occurs
early while the single owner holds both endpoints; it does **not** prove
which unserved request prevents SIM or network registration. The earlier
already-ONLINE attach could not observe this boot window. Device logs:
`/data/saaios/var/owner-handoff-bringup.log` and
`/data/saaios/var/modem-channel-owner.log`.

Factory TD1A `sit-base.so` message-name dispatch labels the first-minute
unsolicited IDs `0x024e` as SIM slot status, `0x0210` as SIM status (three
frames), `0x0803` as radio ready, `0x0802` as radio state changed, `0x0246`
as SIM phonebook ready and `0x000d` as emergency-call list. Factory
`libsitril.so` maps `0x0248` to ICCID information. Only type/id/length was
logged; **do not log these payloads**, which may contain ICCID/EID or other
identifiers. `NetworkService::OnRadioReady`/`OnRadioStateChanged` update
radio state and broadcast internal system event `0x101`, rather than
directly sending a CP request. The passive owner consumes these events but
does not yet implement that AP-side dispatch. Its absence is a candidate
missing prerequisite, not an established cause of failed registration.

Before the successful run, the first script attempt stopped with `sh: missing
]` during device-node identity checking. CP stayed OFFLINE and no modem stage
was sent. That one-line shell error was fixed in commit `40c2b7e`; the two
short failed-run logs and script were retained with `.syntax-failed-20261001`
suffixes on the phone.

## 2026-10-01: event-driven SIM refresh confirms the early-read race

After another explicit AP reboot, the same guarded no-gap handoff returned
`probe_rc=0`, CP `ONLINE`, and one owner held IPC0/RFS0 (PID 545). The new
owner build was SHA-256
`f2db2b815273530ea8ea5699ac0a33903ac75b61e75c22d207650f6c25a11591`.
It sent the original four read-only status GETs, then one coalesced read-only
`0x0200` after `0x0210` SIM-status-change indications. It sent no RFS reply,
PIN, APN, or network-configuration command and did not write NV/EFS.

The initial SIM GET again returned card 0/apps 0. The event-driven refresh
returned card 1/apps 1, app state 2, PIN1 state 1, error 0. Thus **the initial
absent result was a startup timing artifact**, not evidence that the SIM is
missing. Factory wire offsets in `sit-sim-layout.h` place app state at packet
byte 16; older notes using byte 17 for app state are incorrect. The
[AOSP `ril.h`](https://android.googlesource.com/platform/hardware/ril/+/refs/heads/main/include/telephony/ril.h#1051)
`RIL_AppState` enum names 2 `PIN` and `RIL_PinState` names 1
`ENABLED_NOT_VERIFIED`, but these are *modem-reported states*, not proof that
the user's actual SIM still requests a PIN. The user states PIN is disabled;
no guessed PIN should be sent. A later read-only status check and, ideally,
confirmation on another handset are needed to resolve this discrepancy.

RFS metadata repeated: cmd7/seq0 at +7.270 s, then cmd6/seq1 at +17.270 s;
no cmd3 with the current no-reply owner. At +60.169 s the owner remained alive,
CP was `ONLINE`, registration remained 0 in the initial snapshot, and `rmnet0`
RX remained 0. The live log is
`/data/saaios/var/modem-channel-owner.log`; the prior boot logs were retained
under `.pre-refresh-20261001.log` names. This diagnostic still does not
isolate the cause of failed network registration.

## 2026-10-01: 60-second settled control, SIM still absent on third boot

A third explicit no-gap owner boot used the bounded 60-second follow-up
build (SHA-256
`ff065132dfbfd63baf274fc253bea669c361625f7c0a36d89a78949237e84c76`).
The probe again returned 0/CP `ONLINE`; the owner had one IPC0/RFS0 reader,
sent only four initial GETs, one SIM-indication GET, and one settled sweep of
four read-only GETs. No RFS reply, PIN, APN, radio-power or NV/EFS write was
sent. The initial and indication-driven SIM replies both had card 0/apps 0.
Unlike the preceding boot, the settled +60-second SIM reply **still** had
card 0/apps 0. The settled radio reply was raw 3; voice/data registration
remained 0 and `rmnet0` RX remained 0. RFS headers again appeared at
+7.272 s (cmd7/seq0) and +17.272 s (cmd6/seq1). Thus SIM visibility is
not deterministic across otherwise similar no-reply boots. The operator
later confirmed the physical SIM was removed before the *following* boot,
but its removal time relative to this third boot is unknown. Therefore this
run cannot establish nondeterminism or an RFS-induced SIM failure; neither
the earlier card-present/PIN result nor this card-absent result alone is a
stable diagnosis.

Factory TD1A `sit-stream.so` SHA-256
`cef8756461c74102f9a78f91177d1baff80fb9af11c14994497fb8854e0f530a`
maps `0x0801` raw 3 via `ProtocolNetRadioStateRespAdapter::GetRadioState`
(`0x48770`, string table `0x2a9c8`) to
`SIT_PWR_RADIO_SIM_STATE_SIM_LOCK_OR_ABSENT`; it is **not** raw 10 `ON`, but
is in the radio-powered/available class mapped to external RIL `ON` by the
adapter (`0x48818-0x4882c`). Together with card 0/apps 0 this identifies a
SIM-absent-or-locked *report*, not whether the physical card, RFS exchange,
or CP-internal initialization caused it. Replaying `0x0800` ON without a
new isolated hypothesis is not justified; a prior READY/ON test acknowledged
that command yet remained unregistered.

## 2026-10-01: slot-status indication with the physical tray removed

One final no-gap control boot for this series used owner build SHA-256
`3a604367b622e9012714ad83851949de94c22e3595146fe56e91dc4be942b9ea`.
It observed only redacted scalars from the already arriving `0x024e`
indication: at owner +8.008 s, `slot_count=2` and
`slot0_card_state_raw=1` (PRESENT by the factory slot adapter). No extra
SIT request was sent for this observation. Initial and event-refresh
`0x0200` for IPC0 both returned card 0/apps 0; the +60-second settled
`0x0200` was also card 0/apps 0. The settled radio was raw 3
(`SIM_LOCK_OR_ABSENT`), voice/data registration 0, `rmnet0` RX 0. CP stayed
`ONLINE`; RFS again showed cmd7/seq0 at +7.271 s and cmd6/seq1 at
+17.272 s with no replies. No PIN, radio-power, APN or NV/EFS write occurred.

The factory type-2 slot-status adapter accepts length at least 429, reads
slot count at packet byte 8 and fixed 105-byte slot records from byte 9.
This build logged only the first record's card byte. **After this run, the
operator clarified that the physical SIM and its tray had been removed before
this reboot** for a PIN check in another phone. Thus IPC0 card 0/apps 0 is
consistent with the removed physical SIM and is not evidence of a newly
failed SIM initialization. `slot0_card_state_raw=1` may describe an embedded SIM
(eUICC) or another slot/port representation; it does **not** establish that
the physical tray was present or that an eSIM profile was active. The current
boot did not retain slot 1 or logical-port mapping scalars, so even the exact
slot identity remains unproven. Earlier factory-slot diagnostics showed a
crossed logical/physical mapping, but cannot be assumed to describe this
boot. The timing of physical removal relative to the preceding third boot
was not established, so its card-absent result is not a controlled comparison
with the card-present boot either.

Next compare a clearly documented physical-SIM-insertion state (after the
operator confirms the PIN check) using only read-only status and validated
card/port scalars for both slot records through the **same** IPC owner, without
logging ATR/ICCID/EID or opening a competing reader. The two-slot logger is
host-built but was **not** deployed. Do not send `0x0250` slot mapping, PIN,
radio-power, or RFS grants based on this absent-tray run. No further phone
reboot was made in this series.

## 2026-10-01: SIM-in, PIN-disabled no-gap control reaches READY but not camp

The operator tested the physical SIM in another phone, found that its PIN
request had in fact been enabled, disabled that request, returned the SIM to
the Pixel, and reported that the account had been topped up. The numeric PIN
was not sent to the modem or retained in this diagnostic record. While the
previous passive owner still held IPC0/RFS0, the hot-insertion caused two
bounded `0x0210`-triggered read-only SIM GETs: card 1/apps 1 first had app
state 0, then app state **5 (READY)** with PIN1 state **3 (DISABLED)** and
three attempts remaining. CP stayed `ONLINE`; `rmnet0` had no IPv4 address
or RX/TX packets. This is direct evidence that the physical SIM can reach
READY without a VerifyPin command from SaaiOS on the now-disabled card.

The new two-slot logger was rebuilt from the committed source, SHA-256
`afd32e81756a3eb73e88f508ddfcedd21d8e0cd98a14cdfa49e8da43f9a5ca6f`,
passed its ARM `self-test`, and was installed with the preceding binary
preserved as `.pre-two-slot-20261001`. The first ordinary `reboot` request
did not restart this native init; after confirming the old PID and uptime,
one `sync; reboot -f` restarted the AP. The prior logs were retained under
`.pre-sim-in-20261001` names. The guarded owner handoff completed with
`probe_rc=0`; CP was `ONLINE` and a single owner (PID 549) held IPC0/RFS0
before FIN. No factory daemon, PIN, APN, radio SET, slot mapping SET or RFS
reply ran; the verified NV/EFS baseline was not modified.

At +9.768 s the redacted factory `0x024e` indication reported two slot
records: slot 0 card 1/port 0 logical 1/state 1, slot 1 card 1/port 0
logical 0/state 1. This crossed mapping is now observed on the **current**
SIM-in boot. Together with the preceding physical-tray-out control, it is
consistent with slot 0 describing eUICC and slot 1 the removable SIM, but
neither record establishes an active eSIM profile. The indication-driven
`0x0200` for IPC0 returned card 1/apps 1, app state READY(5), PIN1
DISABLED(3), error 0. At +60 s the settled sweep repeated READY/DISABLED;
radio state was **ON(10)**, but voice and data registration were both 0,
reject 0, data tech 0. `rmnet0` still had no IPv4 address and RX/TX were 0.
RFS again showed file-3 command 7 at +7.271 s and command 6/sequence 1 at
+17.272 s without a reply. Thus PIN and absence of the physical SIM no
longer explain this boot's failure to register; the missing network-search
prerequisite versus incomplete RFS transaction remains to be isolated.

## 2026-10-01: extended network-status control under PIN-free READY

A second guarded SIM-in AP boot deployed the reviewed read-only owner build
SHA-256 `b5e9fa744e6bb47a1055043926057897902f78796d506d2d393efa03f5b5e04e`
(source commit `ecc32f8`). Its ARM self-test passed before deployment; the
previous binary and logs were retained under `.pre-network-20261001` names.
The no-gap handoff returned `probe_rc=0`; CP was `ONLINE` and one owner
(PID 554) held IPC0/RFS0. It sent no PIN, radio SET, network-selection SET,
active scan, RFS reply or NV/EFS write.

The +9.768-second two-slot metadata again had slot 0 card 1/port 0 logical 1
and slot 1 card 1/port 0 logical 0. Indication and +60-second SIM GETs
reported card 1/apps 1, READY(5), PIN1 DISABLED(3). The +60-second radio
was ON(10), while voice/data registration remained 0, reject 0, data tech
0; `rmnet0` had no IPv4 address and RX 0. The new one-at-a-time GET series
then returned selection mode **0 (automatic)**, preferred network type
**raw 16**, operator response success/length 119, and signal response
success/length 210. Operator names, PLMN and signal payloads were neither
parsed nor logged. Factory TD1A `sit-stream.so` SHA-256
`cef8756461c74102f9a78f91177d1baff80fb9af11c14994497fb8854e0f530a`
names SIT raw 16 `SIT_NET_PREF_NET_TYPE_NR_LTE_GSM_WCDMA` (name-table
entry at file offset `0x8d048`, string at `0x27676`). Its conversion
table at `0x2a8fc` maps this to Android network mode 26, **not** Android
mode 16. The current preferred mode already permits NR/LTE/GSM/WCDMA;
a prior boot also defaulted to raw 16 before the manual LTE-only tests.
RFS again showed cmd7/seq0 at +7.271 seconds
and cmd6/seq1 at +17.273 seconds without replies. These status GETs confirm
that manual network selection alone is not the blocker, but a successful
operator/signal response does **not** prove RF search or network camp.

The next discriminating diagnostic candidate is one factory-shaped
`0x0706` available-network query under READY/ON, not a blind replay of
radio/allow-data SETs. Unlike the four status GETs, this can actively occupy
RF and has a factory cancel path on timeout. Factory code sends a 16-byte
request with zero at payload offset 12 and allows up to 300 seconds; on
timeout it sends `0x0707` cancel. The current owner cannot verify the
opposite SIM stack's RF-idle state or cancel a scan. Its bounded response/count
parser, RF arbitration and cancel behavior must be reviewed in a separate
opt-in single-owner build before any live send.

## 2026-10-01: one guarded active network scan is rejected immediately

The operator clarified that an earlier attempt to provision an eSIM profile
never completed and that its data need not be preserved. This does not prove
the embedded-SIM RF stack idle; no eSIM profile was deleted or switched.
The physical SIM also registered and carried mobile data in another phone
after the account top-up, so this control is specific to the Pixel/SaaiOS
path rather than evidence of an unusable SIM or blocked account.

The separately named scan owner (ARM64 SHA-256
`9d501aa68a83caa85abc83549621115dd9d668aaa0b61a40788c38d191a01078`)
and probe (`82fd4ff3edd3dc308592b99eaa99b9a597cda5d0fbb04e9841da9f8d4fe0b6d3`)
were built from commit `9f9371c`, checked on the phone by SHA-256, mode,
probe path and self-test, and installed under new names without replacing
the default binaries. The new guarded script used its own log paths and
started only after an AP reboot. Its NV-copy verification and pre-FIN owner
handoff returned 0; CP remained ONLINE, with one owner holding IPC0/RFS0.
No PIN, APN, slot mapping, original EFS or NV write was made.

At +9.770 s, two-slot metadata repeated the slot 0/logical 1 and slot
1/logical 0 mapping. The indication-driven SIM GET and +60 s settled GET
both reported card 1/apps 1, READY(5), PIN1 DISABLED(3). At +60 s radio was
ON(10), voice/data registration 0/reject 0/tech 0, automatic selection 0,
preferred raw 16, operator GET success and signal GET success. The signal
presence mask's low seven bits were **0**; this is not itself a calibrated
signal-strength or RF-power measurement. Once the owner observed a quiet
receive queue and no competing endpoint opener, it sent exactly one
factory-shaped 16-byte `0x0706` request. CP promptly returned a matching
12-byte response with a nonzero 16-bit result; the owner recorded
`network_scan result=remote-error` without logging the numeric result or
any network/PLMN payload. No timeout, cancellation, second scan, or scan
retry occurred. The owner stayed alive and CP ONLINE, but `rmnet0` remained
down. The numeric cause cannot be reconstructed from this redacted log.

The control rules out a missing scan command as the only reason no networks
appeared: CP accepted the transport transaction but rejected the operation.
It does **not** distinguish a missing factory AP/RFS prerequisite from CP RF
state or other modem-side refusal. The earlier `0x0706` error 2 was under a
PIN-locked state; assigning that same number to this new response would be
an inference, not an observation. Do not repeat the active scan blindly.

## 2026-10-01: guarded result-scalar follow-up confirms error 2

Commit `160941b` added logging of only the full 16-bit `0x0706` result
field, with a synthetic high-byte fixture. The new static ARM64 scan owner
SHA-256 was
`cb8653530881118bd7ededea4bfced0593bce3672b1abfcdaf99f9a41690d750`.
It passed its phone self-test and replaced only the separate diagnostic
owner; the first owner's binary and both first-run logs were retained with
`.first-20261001` names. The default owner/probe/script stayed untouched.

A fresh AP reboot and guarded no-gap handoff again returned 0 with CP
ONLINE. The SIM became READY(5)/PIN1 DISABLED(3), the +60-second settled
radio was ON(10), voice/data registration remained 0/reject 0/tech 0,
selection was automatic 0, preferred raw 16, and the signal technology
presence mask's low seven bits remained 0. One 16-byte `0x0706` request
then received a matching 12-byte response immediately:
`network_scan result=remote-error error_raw=2 late_scan_reply=0`.
There was no timeout, cancel or repeat in this boot. CP stayed ONLINE,
but `rmnet0` remained down. This matches the numeric error seen in an
earlier PIN-locked scan, and now shows the PIN lock was **not** its sole
cause. Factory-facing error 2 is a generic refusal; it does not identify
which AP/RFS startup or CP radio precondition is missing. No more scan
replays are planned without a new isolated hypothesis.

## 2026-10-01: first quarantined RFS grant

The reviewed separate owner/probe were installed with matching ARM64 static
hashes; the passive binaries were unchanged. A first cold-boot attempt
refused before original EFS access because Toybox `/bin/sh` could not find
the BusyBox-only `awk` applet. The explicit BusyBox path fixed that. A
second attempt was interrupted by a serial-console Ctrl-C during MAIN
firmware transfer, before the RFS owner started. Both failure logs were
retained; neither sent an RFS response.

The next fresh AP boot used a non-interrupting console. The wrapper verified
original EFS read-only, matched the four NV/sidecar files to the existing
userdata copies, unmounted EFS, and created a volatile protected-NV pin.
The owner consumed that pin before READY; CP reached `ONLINE`. It logged
`grant_attempted=1 first_chunk_stored=1` and terminalized without a second
grant or final ACK. A 524288-byte root-only candidate with `NO_PROMOTION`
was retained; a silent independent check found its tail equal to the source
and its first 2012 bytes different. The source still matched the consumed
pin and EFS was unmounted. No file bytes or digests were logged.

A controlled AP reboot ended this partial experiment. A new read-only EFS
check again matched all four userdata files, and the passive no-reply owner
was restored with CP `ONLINE`. Its SIM refresh reported card=1/apps=1,
app_state=READY(5), while the initial registration snapshot was 0 and
`rmnet0` had no RX or IPv4. This does not prove RFS caused READY or that a
bearer works. Because reboot ended the one-grant process, its pre-reboot
terminal log is not a post-OFFLINE exit-code PASS. Full RFS transfer remains
unimplemented and no candidate was promoted.

## 2026-10-02: early SGC accepted on the radio edge, still no registration

Interactive access was restored over the USB serial console (`COM13`, root
shell). The SSH key the operator placed on "server 110"/R620 was unreachable
from this host (the home LAN is not routable from here and every local key is
refused by the phone's dropbear); it was not needed once the serial console
gave a root shell. File transfer used the existing USB-NCM push
(`serve-once` → `nc`). No ADB and no persistent cbd/rild.

The separately named early-SGC owner (ARM64 SHA-256
`322ac00d2e6e50bf7e5502ebf260caee80c1bad0fbad0937fe564836b22af524`) and probe
(`435602ea456a2369f10a6b3f41a2730cadf57cdaf121e4859cf6ff2d3ee3fde6`) were
rebuilt from the tree, checked on the phone by SHA-256, `--mode`, probe owner
path and self-test, and installed under new names without replacing the default
binaries. A dedicated `owner-handoff-sgc-early.sh` used its own log paths, the
same CP-OFFLINE and log guards as the `sgc-once` wrapper, and mounted persist
read-only only for the `cpsha`; the original EFS was never mounted.

The BusyBox `reboot` applet does nothing here — native-init reboots only via
the `reboot()` syscall — so a forced `reboot -f` (after `sync`) was used. The
device returned at uptime 1 min with cpif modules unloaded, no owner, and no
`modem_state` node. A read-only preflight confirmed the original EFS (`sda5`)
unmounted and `rmnet0` rx/tx 0/0 with no IPv4. The guarded handoff then loaded
the cpif modules (CP OFFLINE), booted the reviewed B firmware
(`PROBE END result=0`, `probe_rc=0`, CP OFFLINE→ONLINE), unmounted persist, and
handed the fresh channels to the early-SGC owner.

The owner latched the early radio edge — `cp_ind 0x0803` len 8 then
`cp_ind 0x0802` len 12 `radio_state_raw=0` (INITIALIZED) at +9.817 s — and sent
the single factory 24-byte `0x0404` SGC at +10.323 s, within the 2 s deadline
(trigger `0x0803-0x0802-raw0`, target `europen-400`). CP returned
`response=yes error_raw=0 status=accepted observation=released`. This is the
first run where the factory carrier SET landed on the early radio edge instead
of the +60 s settled baseline. SIM then refreshed to card 1/apps 1,
app_state READY(5), PIN1 DISABLED(3).

The +60 s settled snapshot matched every prior boot: radio ON(10), voice
registration 0/reject 0, data registration 0/reject 0/tech 0, automatic
selection 0, preferred raw 16, operator response success length 119, signal
response success length 210 with the low seven presence bits 0, modem_stack
enabled. Registration-state indications arrived (`0x0700` len 88, `0x0701`
len 86, `0x0703`, `0x070b`, `0x0702`, `0x0900`, `0x0810`) but registration
never moved off 0. Over three more minutes `rmnet0` rx/tx stayed 0/0 with no
IPv4 and CP stayed ONLINE. No PLMN, operator, signal, or SIM-secret payload was
logged.

Applying the exact factory SGC at the exact factory early stage is a real
mechanical advance — the CP now accepts the carrier config on the
radio-available edge — but it does **not** by itself produce camp, registration
or a bearer. Stage-1 carrier/region timing is therefore not the sole missing
precondition. The next isolated candidates at the same radio-edge trigger, one
at a time, are early `SetModemsConfig 0x093f` and early camp-on `0x0800`
(`TrySetRadioPower(10)`); each needs its own guarded mutually-exclusive build
and self-test before a device run. No NV, APN, PIN, CardPower or EFS write was
made.

## 2026-10-02: registration triggers in the owner + PCIe link-drop blocker

The unified owner now runs a SIM-READY-gated, one-shot registration sequence:
confirm radio ON (`0x0801`), read selection (`0x0703`) and force automatic
(`0x0704`) unless already auto, read preferred RAT (`0x070b`) and set
`LTE_WCDMA(12)` (`0x070a`) unless it already reads 12, then `AllowData(1)`
(`0x0710`). GETs retry a bounded number of times then advance; SETs fire once.
Builders are the recovered/self-tested factory shapes; owner `b5ac7592…`,
host + on-device self-test PASS (`test_camp_reg`), `-Werror`.

Hypothesis under test: preferred RAT excludes WCDMA while the only present signal
is UMTS (`mask_low7=2`, data `tech_raw=3`), so broadening to `LTE_WCDMA(12)`
should let PS search the present cell. The settled `preferred raw 16` seen on
earlier boots is exactly why the broaden fires for any value other than an
explicit `12`.

Live status: one clean boot showed the sequence engage (radio `radio_raw=10`,
then selection GET — which did not reply; the bounded-retry fix now advances past
it). The broaden itself was **not** observed live: the following eight
`sysrq`-reboot → handoff cycles all came up with the modem **PCIe endpoint
dropping right after RadioPower-ON** — `cpif: pcie_send_ap2cp_irq: Reserve
doorbell interrupt: PCI not powered on`. The link replies for the first ~9 s then
dies after `0x0800`; SIM never initializes (`card_raw=0`) and every IPC GET times
out, so the sequence never runs. CP boots cleanly (`complete_normal_boot`,
`CP2AP_WAKEUP=1`, no `cp_crash`; 100 % / 33 °C). Disabling EP L1.2 ASPM
(`l1_2_aspm`/`l1_2_pcipm`=0), pinning RC `power/control=on`, long modem-off soak,
and repeated reboots did not clear it — the drop is cpif-managed CP runtime-PM
below ASPM, and warm `sysrq b` does not reset the modem power rail; a cold power
cycle is the likely requirement. No NV/APN/PIN/CardPower/EFS write was made.

## 2026-10-02 (later): op-mode SET hunt — no constraint-safe SET; FLASH-NV boundary

Follow-up to the RFS CP-STORE verdict. The last allowed avenue was a live SIT
operational-mode / attach-enable **SET command** (not an NV write). Static RE of
the vendor SIT/RIL carves + CP image found untried operational SETs **by name**
(`SIT_SET_PS_SERVICE_DOMAIN`, `SIT_SET_DEVICE_SERVICE`, `SIT_SET_INTPS_SERVICE`,
`SIT_SET_VOICE_OPERATION`, `SIT_SET_MODEM_CONFIG`, `SIT_NS_NETWORK_NORMAL_START`,
`SIT_SET_DUAL_NTW_AND_PS_TYPE`, + GETs) but their **wire opcode ids were not
recoverable**: the CP dispatches by numeric id and does not code-reference the
name strings; the vendor RIL id→name table (`…sitril-builder` @ `0x217fa4`, 557 ×
12-byte stubs) is indexed by an internal enum (non-linear vs wire id: 279→`0x800`,
175→`0x600`, 88→`0x208`), and the truncated carve has no clean wire table that
validates on anchors. Issuing them would require inventing bytes (forbidden). All
wire-mappable operational SETs already known (EngMode `0x0908`, SGC `0x0404`,
`0x093f`, `0x0800`, `0x0704`, `0x0710`, `0x070a`) were tried live and are
ineffective. The gate value `SAE_UE_OPERATION_MODE` is mutated via the CP's
internal registry and persisted in `SAE_FLASH_UE_OPERATION_MODE`; no SIT handler
writes it without a FLASH-NV persist. **Verdict: no confirmed command-only path
flips the gate; the only known mutation is a FLASH-NV write (out of scope, no
operator authorization) — none performed.** Device unchanged: owner pid 571
(`69f9b62d…`), CP ONLINE, SIM READY, data `NOT_SEARCHING(0)`, voice
`REG_DENIED(3)`. **Bearer? no.**

