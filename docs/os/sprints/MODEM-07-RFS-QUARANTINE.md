# MODEM-07: quarantined protected-NV RFS experiment

Status: **one-grant and full-transfer diagnostics exercised on phone;
scalar, logical-stack and isolated late-SGC observations also completed**.
Registration and a cellular bearer remain absent. These are manual diagnostic
boots, not a deployed modem service; see the dated live results below.
Target: Pixel 7 `panther` S5300, explicit diagnostic boots only.

**2026-10-03 ALL SCAN FORMS REFUSED — recovered operator-control levers exhausted (VERDICT 17).**
Tested the last recovered-but-untried lever: the len-16 scan variant
`ProtocolNetworkBuilder::BuildQueryAvailableNetwork(int)` (@0x236950) — same opcode 0x0706 as the
len-12 header-only scan but total len 16, with an explicit scanType int32 at payload[12]. The
stock clamps the arg (`sub w8,arg,#1; cmp #5; csel` ⇒ scanType = (1≤arg≤5) ? arg : 0), so the
distinct accepted values are 0,1,2,3,4,5 (HAL `DoQueryBplmnSearch` passes 0, `DoQueryAvailableNetwork`
passes a framework "scanType=%d"). Recovered byte-for-byte, implemented guarded/one-shot/self-tested
(-Werror, on-device RC=0). One boot: bring-up to allow_data (error 0), camped `25501#` UMTS, then
swept all six scanType values, each once — every mode returned `error_raw=2` (RIL_E_GENERIC_FAILURE),
no result list. Combined with VERDICT 14 (len-12 scan + manual-select refused camped) and VERDICT 15
(len-12 scan refused deregistered), every recovered form of operator-controlled scan is refused in
every state. Conclusive terminal firmware-state + environmental boundary. Owner 3320c305, config
cleared, proven backup 90f403df intact. Bearer NOT achieved; no scan, no reselection reachable.

**2026-10-03 NEW-SIM TEST, CARD READY but CS-DENIED (VERDICT 16).**
A second SIM was swapped in (intended Kyivstar 255-03, to be activated by one outgoing voice
call to +380953444757). Recovered the stock voice/PIN command set byte-for-byte from
`libsitril.so` (efcca0d5): PIN1 verify (opcode 0x0201, len 38, `[12]`=PIN len/`[13..]`=PIN
ASCII, AID omitted), DIAL (0x0001, len 104, `[12]`=type/`[14]`=numlen/`[15..]`=number/`[97]`=TOA/
`[99]`=CLIR), GET_CALL_LIST (0x0000, len 12; reply count@`[12]`, per-call stride 327, state@entry
`[0]`, index@`[1..4]`), HANGUP (0x0008, len 20, `[12]`=index/`[16]`=1). Implemented guarded/one-shot/
self-tested (-Werror, on-device self-test RC=0). One fresh boot: card present (`card_raw=1
apps=1`), SIM READY (`app_state_raw=5`) with PIN reported DISABLED (`pin1_raw=3`) ⇒ PIN-verify
correctly never fired (contradicts the expected PIN-enabled card — physical reseat worth
checking). Camped only on foreign `25501#` UMTS; voice REG_DENIED(3), data not-registered(0),
stable ~234 s. CS registration never reached home/roaming, so the activation call was correctly
gated off and never dialed (`camp_call=sent` count 0). Voice/PIN host stack validated but not
network-exercised. Owner 661ac0ab, secret PIN config written transiently then deleted (never
logged/committed), proven backup 90f403df intact. No bearer, no call.

**2026-10-03 TERMINAL FIRMWARE-STATE BOUNDARY: scan refused even deregistered (VERDICT 15).**
Tested whether the VERDICT-14 scan refusal was only "can't scan while camped": recovered
RADIO_POWER (0x0800) power word from `ProtocolNetworkBuilder::BuildRadioPower` (payload[12] =
arg1?2:1 ⇒ OFF=1/ON=2, len 18) and added a one-shot deregister-then-scan sub-sequence
(/data/saaios/etc/dereg_scan; guarded, self-tested 195-210, -Werror; owner 6fe99aa6). After
full bring-up (camped on foreign 25501 3G), the owner cycled RADIO_POWER OFF→ON (both ACK
error_raw=0), CONFIRMED radio read off (radio_off_confirmed=1), and fired the scan ~2 s after
radio-ON in the pre-camp window — yet 0x0706 STILL returned error_raw=2 (RIL_E_GENERIC_FAILURE).
So the modem/firmware does not service GET_AVAILABLE_NETWORKS in any tested state, registered
or deregistered; manual-select correctly did not fire (home PLMN never visible). The OFF→ON
cycle was fully reversible (re-camped on 25501, CP ONLINE). Operator-reselection lever
exhausted; no bearer. Device known-good: config cleared (default LTE+WCDMA, automatic
selection), proven backup 90f403df intact; recovered-only bytes, no NV/EFS write, no
IOCTL_POWER_OFF, no do_cp_crash.

**2026-10-03 TERMINAL BOUNDARY: modem refuses operator reselection — scan + manual-select both GENERIC_FAILURE (VERDICT 14).**
Recovered GET_AVAILABLE_NETWORKS (SIT 0x0706: header-only req len 12 via
`BuildQueryAvailableNetwork()`; response count=int32@payload[12], 14-byte per-PLMN entries
@payload[16] = RAT[0..3]/PLMN-ASCII[4..9]/status[10..13], scan-RAT remap @.rodata 0xd8ac8)
and SET_NETWORK_SELECTION_MANUAL (SIT 0x0705: len 22, RAT int32@payload+0 [0=any],
PLMN-ASCII@payload+4, '#' filler) from `libsitril.so` (efcca0d5)
`ProtocolNetAvailableNetworkAdapter` + `BuildSetNetworkSelectionManual`. Added a
read-only scan GET + parser and a file-gated manual-select (/data/saaios/etc/do_scan,
/data/saaios/etc/manual_plmn; guarded, self-tested 184-194, -Werror; owner 5834fd01). Boot A
(scan): modem camped at 57 s (voice REG_DENIED(3), UMTS, 25501) — i.e. RF ready — yet the
65 s scan returned `error_raw=2` (protocol 2 → RIL_E_GENERIC_FAILURE, identity map @0xd7bd0),
no async result; the "scan failed only because RF wasn't ready" premise is FALSIFIED. Boot B
(manual-select to home lifecell 25506): `error_raw=2` again, while same-path
preferred/initial_attach_apn/allow_data all ACKed error_raw=0; serving PLMN unchanged
(25501 Vodafone UA, voice REG_DENIED(3), data NOT_SEARCHING(0), UMTS). The host command path
is correct; the modem specifically declines operator control, so the home PLMN (lifecell
255-06) could be neither confirmed reachable nor selected. Terminal modem/environmental
boundary; no bearer. Device known-good: CP ONLINE, config cleared (default LTE+WCDMA,
automatic selection), proven backup 90f403df intact; recovered-only bytes, no NV/EFS write.

**2026-10-03 TERMINAL BOUNDARY: foreign 3G PLMN + no LTE acquisition (VERDICT 13).**
Recovered 0x0702 GET_OPERATOR (header-only req len 12; response PLMN numeric MCC/MNC = 6
ASCII at payload[12..17], '#'=2-digit MNC; short name [18], long name [50]) from
`ProtocolNetOperatorAdapter::Init`, and the LTE-only RAT value from
`BuildSetPreferredNetworkType` table @.rodata 0xd8c5c (RIL 11 → SIT 0x0b=11; identity low
range). Added a read-only 0x0702 probe + a file-selectable preferred-RAT override
(/data/saaios/etc/pref_rat; guarded, self-tested 178-183, -Werror; owner 2a4e07ed). Boot A
(default LTE+WCDMA): serving PLMN = 25501 (MCC 255 Ukraine / MNC 01 = Vodafone Ukraine) on
UMTS/3G — FOREIGN vs the SIM's home lifecell (255-06, "Life"/internet APN) — voice
REG_DENIED(3), data NOT_SEARCHING(0), reject 0. Boot B (forced LTE-only, set ACKed
error_raw=0): still camped 25501 UMTS; tech tally UMTS(3)×36, none(0)×2, LTE(14)×0 — the CP
never acquired any LTE cell even when restricted to LTE-only. Verdict: host-side SIT replay
cannot reach a bearer — the modem can only find a foreign 3G PLMN that denies the SIM and
cannot acquire home (lifecell) LTE, whose RF/band enablement lives in NV/RF-cal we must not
modify (and/or no reachable home LTE here). Environmental/subscription + RF-cal reality, not
a SaaiOS host defect. Bearer NOT achieved. Device known-good (CP ONLINE, default restored).
See [MODEM-BLOCKER](../targets/panther/MODEM-BLOCKER.md) VERDICT 13.

**2026-10-03 TRUE reject-cause decoded = genuinely 0; modem camped on UMTS/3G only.**
Recovered the exact 0x0700/0x0701 response layout from `libsitril.so` (efcca0d5)
`ProtocolNet{Voice,Data}RegStateAdapter` fixed-offset accessors and extended the owner
to decode every field read-only. Stock reads reject_cause at **offset 13** — exactly
where our owner already read it, so the "wrong offset" premise is FALSIFIED; reject is
genuinely 0. Field→offset (frame-relative): reg_state[12], reject[13], voice tech[14]/
lac[15]/cid[19]/psc[23]; data MaxSDC[14]/tech[15]/lac[16]/cid[20]/psc[24]. RAT map
(.rodata@0xd8afc, idx=raw-1): 3→UMTS, 14→LTE, 16→GSM, 20→NR. One boot (owner 69d3d1c2,
full sequence ACKed): data `registration_raw=0 reject_raw=0 tech_raw=3 rat_mapped=3
lac=36291 cid=85793345 psc=187`; voice `registration_raw=3 reject_raw=0 tech_raw=3
rat_mapped=3 lac=36291 cid=85793345 psc=187`. So the modem SEES a real cell but only on
UMTS(3) (PSC=187 is a WCDMA scrambling code): CS=DENIED(3) with no cause, PS=NOT_SEARCHING.
Not a cause-coded auth reject and not an empty scan — it is a RAT/coverage situation
(stuck on 3G despite LTE+WCDMA preferred; LTE RF/band lives in NV we won't touch).
Serving PLMN (MCC/MNC) is not in the reg-state frame (it is 0x0702). Device known-good
(CP ONLINE). Next (read-only, one boot each): query operator 0x0702 for the serving PLMN
(home vs foreign) and/or try LTE-only preferred RAT. See
[MODEM-BLOCKER](../targets/panther/MODEM-BLOCKER.md) VERDICT 12.

**2026-10-03 SET_INITIAL_ATTACH_APN recovered + replayed (accepted) but NOT the gate.**
Recovered the stock rild→libsitril attach chain from the factory `libsitril.so` (efcca0d5):
all builders go through `ProtocolBuilder::InitRequestHeader(hdr, opcode, len)`. Wire IDs:
RADIO_POWER `0x0800`, SET_NETWORK_SELECTION_AUTO `0x0704`, SET_PREFERRED_NETWORK_TYPE
`0x070a`, **SET_INITIAL_ATTACH_APN `0x0603` (250-byte `sit_pdp_set_initial_attach_apn_req`)**,
ALLOW_DATA `0x0710`, DETACH `0x0608`, (DATA/VOICE)_REG_STATE `0x0701`/`0x0700`, GET_PS_SERVICE
`0x0711`. The one command we were missing was `0x0603`. Body (frame-relative, from
`BuildSetInitialAttachApn`+`FillApnInfo`): `[12]`=attach cid, `[13]`=0x0e, `[14]`=dataProfileId,
`[15]`=apnType, `[16..115]`=APN, `[117]`/`[167]`=user/pass, `[217]`=auth, `[218]`=pdpType
(`GetPdpType("IP")=1`), `[219]`=pcscf. Implemented `make_initial_attach_apn_request` (guarded,
`_Static_assert`, byte-exact self-test 165–172, `-Werror`), inserted before ALLOW_DATA. One
boot: `camp_apn=loaded len=8`; `set_initial_attach_apn response=yes error_raw=0` (ACCEPTED),
then `allow_data error_raw=0`. Registration UNCHANGED: data `registration_raw=0` NOT_SEARCHING
tech_raw=3, voice `registration_raw=3` REG_DENIED, `reject_raw=0`. So the attach-APN precondition
is accepted but is not the blocker — the full accepted stock host sequence does not register the
modem. Device known-good (CP ONLINE, owner `bb9398f2`), READ-ONLY, nothing invented. Next
(read-only): decode the full reg-state response for the TRUE reject cause. See
[MODEM-BLOCKER](../targets/panther/MODEM-BLOCKER.md) VERDICT 11.

**2026-10-03 CP normal-NV self-downgrade FALSIFIED (read-only capture + diff).**
Extended the quarantine owner with a guarded, self-tested, `-Werror`-clean
`SAAIOS_RFS_NORMAL_CAPTURE` block that answers the CP's handle-1 (normal-NV)
open/grant-request and streams the chunks to a quarantine-only `normal-candidate.bin`
(never real EFS/sda5/nv_normal; no payload logged). First attempt got 0 bytes — the CP
refused the grant (status-6) because the handle-1 request uses sequence 2 while the proven
handle-3 grant echoes sequence 1; fixed by echoing the request's sequence in the grant's
`w0` high-16. Second boot captured the full blob intact (476454 bytes,
`NORMAL_CAPTURE done received=476454 grants=237`), CP ONLINE/SIM-READY. A fast anchored
diff vs the fed-in `nv_normal.bin` (byte-identical to real sda5 per V7) shows 476361 equal
bytes / 10 tiny edit regions; the entire static config body is byte-identical (zero
mismatches over 4000 random samples). All edits are write-gen counters, per-record
checksums, one timestamp-shaped 9-byte value, and a non-persisted ~47 KB tail — no
registration-relevant NV item (op-mode, service-domain, limited-service, PLMN-sel, RAT/band,
GCFMODE, attach) changed. The CP does not self-downgrade normal-NV, matching protected-NV
(V7). Device left known-good: proven owner restored on disk (`90f403df`), CP ONLINE;
READ-ONLY throughout. With V7/V9/V10 all falsified, the single best remaining hypothesis is
a missing host-side RIL/SIT bring-up sequence (radio online + automatic PLMN selection / PS
attach) rather than NV/secure-boot. See
[MODEM-BLOCKER](../targets/panther/MODEM-BLOCKER.md) VERDICT 10.

**2026-10-03 secure-boot / `IOCTL_REQ_SECURITY` FALSIFIED as the gate (live-tested, one controlled boot).**
We implemented the GENUINE handshake and issued it. First pinned that the params are
kernel-ignored (the handler reads only `mode`; all SMC args are kernel-derived from
`cp_shmem_get_base/size` regions 7/8 + fixed SIP FIDs `0x82001011`/`0x82000700`), so
AP params `0` give a byte-identical SMC — genuine, not forged. Added a guarded, self-
tested, non-fatal `PROBE_SECURITY` block to `cp-boot-probe.c` (modes 2→0→1 after the
handover), `-Werror`; the proven rebuild is byte-identical (`e32538e8…`) to the deployed
binary. On the controlled boot all three calls returned `EINVAL` and dmesg showed
`cpif: bootdump_ioctl: umts_boot0: security_req is null` — the EL3/ldfw SMC never ran.
Root cause: cpif `create_link_device@0x9708` writes the `security_request` pointer (io-
device offset 976) only when arg2==0 and a DT link-attr bit is set; panther's modem link
config leaves it NULL. Same stock `cpif.ko` + DT ⇒ stock cbd's `REQ_SECURITY` also
`EINVAL`/non-fatal, so it is vestigial here and not the gate. MAIN DONE still passed, CP
ONLINE, registration unchanged (CS REG_DENIED, PS NOT_SEARCHING). Reverted to the proven
probe (`e32538e8…`), CP ONLINE; no NV/EFS write, nothing forged. Next (RO): capture and
diff the ~476 KB normal-NV write-out vs fed-in `nv_normal.bin`. See
[MODEM-BLOCKER](../targets/panther/MODEM-BLOCKER.md) VERDICT 9.

**2026-10-03 secure-boot path characterized — genuine `IOCTL_REQ_SECURITY` we omit; legitimately reproducible (EL3/ldfw), not a GSA-secret boundary a priori.**
Pursuing the post-VERDICT-7 lead (registration gate may be the secure-boot/auth state).
Factory `cbd` (pulled RO) issues `ioctl(boot_fd, 0x40106f53 /*REQ_SECURITY*/, &{mode,p2,p3,0})`
**three times** in normal boot (mode 2 flag; mode 0 main-auth with `p2=[cfg+0x260]`
`p3=[cfg+0x28c]`; mode 1), logging `Request security : non-secure mode` /
`ERR! IOCTL_CHECK_SECURITY fail`. The kernel `shmem_security_request` copies the struct,
switches on mode, maps the CP shmem region, and `__arm_smccc_smc`s to **EL3/ldfw**. Our
probe performs the handover (`0x6f57`, genuine cpsha+IMEI+CDT) + signed MAIN (integrity-
validated at UDL DONE) but issues **none** of the three REQ_SECURITY calls. **Feasibility:
legitimately reproducible from SaaiOS** — genuine kernel→EL3 SMC, device-fused keys do the
crypto, AP supplies only mode+layout params (no secret, no forging); this is the EL3/ldfw
path, **not** the ADR-092 GSA mailbox. Open question (live-only): whether EL3 accepts the
SMC in our boot context; a `security check fail` would be the terminal boundary (no forge).
Caveat: the CP already runs integrity-validated MAIN with RF-rx + SIM READY, so REQ_SECURITY
may only map secure DRAM rather than gate MM — unproven. NV write-out diff (RO): CP-written
protected-NV == fed-in except 2 bytes (off 20/189444, each +4 = write-gen counter) → no
registration-relevant protected-NV downgrade; normal-NV write-out isn't captured (needs an
owner extension). No reboot/NV write this session; device on safe owner, CP ONLINE, bearer
not established. Next: pin the two layout params (or confirm kernel derives them), add the
three genuine REQ_SECURITY calls at the cbd-matched order, one controlled boot + registration
re-check. See [MODEM-BLOCKER](../targets/panther/MODEM-BLOCKER.md) VERDICT 8.

**2026-10-03 NV-starvation disproven — op-mode-NV FALSIFIED as the registration gate; NV write is moot.**
Operator reframe (resolve the (c) contradiction): traced exactly how the CP gets NV at
boot and whether our boot starves it. Finding: the probe **pushes** `NV_NORM` (TOC
idx5, `0x80000`) + `NV_PROT` (idx6, `0x80000`) from `/data/saaios/var/efs-copy/`
straight into the CP as SIT boot stages (like stock cbd), during `BOOTING` before the
owner attaches — the CP does **not** read op-mode from EFS via RFS. The RO verifier
(`verify-original-efs-readonly.sh`) **PASS**es: those pushed blobs are **byte-identical**
to the real EFS partition **sda5** (`PARTNAME=efs`). The owner logs every RFS frame
(incl. a post-terminal drain through the MM registration window); across boot + 42 min
there are **zero RFS reads** — all 232 frames are NV **write-OUTs** (protected-NV 189446 B
handle 3; normal-NV ~476 KB handle 1, repeated). So the CP is fed the exact real stock
NV (op-mode included) that the phone registers with as stock, never reads NV via RFS,
and still denies (voice `REG_DENIED(3)`/data `NOT_SEARCHING(0)`). Firmware is **stock B**
(`449eeab3…`), boot completes cleanly (`COMPLETE rc=0`), SIM READY, signal present.
**Conclusion: `SAE_UE_OPERATION_MODE` FLASH-NV is NOT the gate — the earlier (c) verdict
is corrected and the NV-write NO-GO is moot.** Remaining non-NV divergences to chase:
the secure-boot/authentication path (we use `HANDOVER_RAM_ONLY 0x6f57` + preamble, never
`IOCTL_REQ_SECURITY`; stock uses GSA secure boot — cf. ADR-092), and a structural diff
of the CP's self-written normal-NV vs the fed-in blob. No stock run, no NV/EFS write;
device on safe owner, CP ONLINE; bearer not established. See
[MODEM-BLOCKER](../targets/panther/MODEM-BLOCKER.md) VERDICT 7.

**2026-10-03 stock-registration capture — INFEASIBLE under SaaiOS; pivotal verdict (c): the delta is FLASH-NV, not a command.**
Operator asked to watch the stock stack register live and extract the minimal delta.
A live stock registration is **not runnable under SaaiOS**: no `/vendor/bin`, no
`rild` present; `cbd` (Android-dynamic, at `/data/saaios/var/vextract/cbd`) only
boots the already-ONLINE CP; `rild`/libsitril issue radio-on/network-select SIT
commands **only when driven by the Android telephony framework** (binder/
`hwservicemanager`/`system_server`), which SaaiOS lacks (`servicemanager` only,
empty `/apex`, no `/dev/socket/rild`); and the `cpif` driver exposes no
`dynamic_debug`/ftrace frame logger (pstore empty) to capture frames without the HAL.
Booting full stock Android would register but removes our COM13/SIT capture harness
and a persistent cbd/rild is forbidden. Instead, the equivalent evidence was pinned:
our owner already has **full command-parity** with stock's known stage-1 trio
(`0x093f`→`0x0404` europen `0x0101,0,0`→`0x0800`, all ACK `error_raw=0`, signal
present `mask_low7=1`) yet stays voice `REG_DENIED(3)` / data `NOT_SEARCHING(0)`.
Pivotal verdict **(c)**: (a) ruled out by command parity + the exhausted command
surface; (b) ruled out because the RAM op-mode SETs ACK and the GETs already report
the target values (voice=3/stack=1/devsvc=1) with no registration change (the gate
reads FLASH-NV, not the settable RAM value); (c) the delta is CP-internal state
seeded from FLASH-NV `SAE_UE_OPERATION_MODE` — and stock registering on this phone
confirms that value is normally "normal". Minimal delta = the single operator-gated
NV write (VERDICT 5, NO-GO); no constraint-safe command/RAM-init substitutes. No
stock stack run, no NV/EFS write; device on the safe quarantine owner, CP ONLINE;
bearer not established. See [MODEM-BLOCKER](../targets/panther/MODEM-BLOCKER.md)
VERDICT 6.

**2026-10-02 de-risk (READ-ONLY, no NV write) — NV gate characterized; RECOMMENDATION NO-GO; tested backup/revert harness built.**
Per operator decision, characterized the hypothetical `SAE_UE_OPERATION_MODE` NV
write and built a reversible safety harness, then stopped for go/no-go. Findings:
the gate + siblings (`SAE_FLASH_GCFMODE`, `SAE_FLASH_PLMN_SEL_MODE`) are **name-keyed**
SAE-L3 flash NV items (aliases `!SAEL3.`/`!SAEL3_DS.`/`SAE_FLASH_`/`SAECOMM_FLASH_`);
the CP accessor (Thumb-2 @≈`0x3DBF0xx`) loads them **by name string**, so the
physical slot is assigned at CP runtime and there is **no static name→offset map**.
The on-disk protected-NV blob (512 KB quarantine copy) is **plaintext/structured
flash** (entropy 3.28, 81/128 blocks `0xFF`-erased, not encrypted) but contains
**none** of the name keys → the target byte offset is **not determinable**. The CP
integrity validator (checksum/coverage/recompute-vs-reject) is **unconfirmed**
(dominant brick risk). The CP **writes NV OUT** to the AP and (prior data) never
reads it back in the observed window → the authoritative store is **CP-side**;
editing the AP copy is not confirmed to propagate, and a real persist would need the
forbidden `sda5`/`nv_protected` partition write. Current/target enum **not readable**
read-only. Built + tested on copies only: `nv-edit-harness.py` (selftest PASS:
backup/narrow/dry-run/apply-copy/diff/restore), on-device `nv-backup.sh` (RO backup,
sha match, source untouched) and guarded `nv-revert.sh` (refuses without
`SAAIOS_NV_REVERT_CONFIRM=yes`). **RECOMMENDATION: NO-GO** — offset unpinned,
validator unconfirmed, no propagating AP write path. No NV/EFS write performed; see
[MODEM-BLOCKER](../targets/panther/MODEM-BLOCKER.md) VERDICT 5 for the full writeup.

**2026-10-02 final — last command-only lever `0x072B` recovered, confirmed safe, live-tested INEFFECTIVE; command-only avenue FULLY exhausted; NV boundary is the only path left (operator-gated, NOT done).**
Recovered `0x072B` (SET_DUAL_NETWORK_AND_ALLOW_DATA) from
`ProtocolNetworkBuilder::BuildSetDualNetworkAndAllowData` @ `0x2375a0`: a 28-byte
frame, 4 × int32 = `[translate(primaryNet), translate(secondaryNet),
primaryAllowData, secondaryAllowData]` (caller log string
`"Dual Network Type : Primary(%d,%d), Secondary(%d,%d)"`). `translateNetworktype`
@ `0x236790` is the SAME table `0x070a` uses and `translate(12)=12`, so the body
was filled with already-proven wire values — net type `12` (= our `0x070a`
LTE/WCDMA) and allow-data `1` (= our `0x0710`) for both stacks; nothing invented.
No GET counterpart exists. The caller
`NetworkService::DoSetDualNetworkTypeAndAllowData` @ `0x19fe00` ends in
`Service::SendRequest(…,0x7530,0xff3,…)` with **no NvWrite path** → command-safe,
not FLASH-NV; it is effectively the union of `0x070a`+`0x0710` (both already
ineffective). Live (owner `90f403df`, step=`dual`, one guarded boot): SET acked
`error_raw=0` (accepted) but **registration did not move** — voice REG_DENIED(3)
/reject 0, data NOT_SEARCHING(0)/tech 3, mask UMTS(2), and **no `rmnet` IPv4**.
→ Command-only avenue is fully exhausted; every constraint-safe AP→CP command has
been issued and the modem is already in the target operational state. The only
remaining lever is a scoped, backed-up, reversible **FLASH-NV write of
`SAE_UE_OPERATION_MODE`** (persisted `SAE_FLASH_UE_OPERATION_MODE`; siblings
`SAE_FLASH_GCFMODE`/`SAE_FLASH_PLMN_SEL_MODE`/`CalDone`), which is CP-internal NV
behind the hard-constraint boundary — **operator-gated, high brick risk, NOT
authorized, NOT performed.** See the [MODEM-BLOCKER one-pager](../targets/panther/MODEM-BLOCKER.md) VERDICT 4 for the full NV operator-decision writeup.

**2026-10-02 latest — full `libsitril.so` recovered; all constraint-safe op-mode SETs issued LIVE; all ACK clean, NONE move registration; command-only avenue EXHAUSTED; NO NV write.**
The prior entry's blocker (wire ids unrecoverable from the truncated carve) was
resolved by extracting the **full** `/lib64/libsitril.so` READ-ONLY from
`vendor.img` with `debugfs` (no mount, no sudo). SHA-256 `efcca0d5…2d5b1` — exact
match to the operator's expected TD1A. Wire ids + body shapes were lifted from the
builder disassembly (`InitRequestHeader` immediates), not guessed, and
cross-checked against the open `sitdef.h`. `BuildNvWriteItem` is a no-op stub and
`DoOemSetPsService`→`BuildAllowData(0x0710)`, confirming these SETs are
`SendRequest` commands, not NV writes.

Four SETs were issued live, one per guarded warm-reboot handoff, from the unified
owner after SIM READY / radio ON / allow_data (single `umts_ipc0` lock):
- `voice` `0x091A` int32 `mode=3` (len16): GET read voice_operation=**3** already;
  ack `error_raw=0`; no reg change.
- `intps` `0x0933` int32 `mode=1` (len16): ack `error_raw=0` (**not** `2` → live,
  not removed from firmware); no reg change.
- `stack` `0x080F` byte `mode=1` (len13): GET read stack_status=**1** already
  enabled; ack `error_raw=0`; no reg change.
- `devsvc` `0x0956` int32 `mode=2` data-centric (len16): GET read
  device_service=**1** (voice-centric); ack `error_raw=0`; no reg change.

Across every boot and ~65s post-SET: voice `registration_raw=3` REG_DENIED /
`reject_raw=0`; data `registration_raw=0` NOT_SEARCHING / `tech_raw=3`;
`mask_low7=2` UMTS. The GETs show the modem is already in the target operational
state, so these levers are no-ops — the denial is below/outside the AP→CP
operational-SET surface. `0x072B` (dual-ntw/PS-type) body not fully pinned → not
sent per operator; `0x0937`/`POWER_OFF(3)`/NV all hard-barred. Owner hash this run
`ecdf874f…`; `opx-step` cleared to GET-only afterwards. Also fixed a reg-sequence
bug: the short `allow_data` (`0x0710`) ACK was being swallowed by the generic
length guard before the `REG_ALLOW_DATA` branch, so `reg_complete` (the opx gate)
never fired; SET-ack branches now precede the guard.

**2026-10-02 later — op-mode SET hunt: no constraint-safe SET confirmed; boundary stands; NO NV write.**
Chasing the one remaining allowed avenue from the prior entry (a live SIT
operational-mode / attach-enable *SET command*, not an NV write). Static RE of
the vendor SIT/RIL carves + CP image:
- Untried operational SETs exist **by name** (`SIT_SET_PS_SERVICE_DOMAIN`,
  `SIT_SET_DEVICE_SERVICE`, `SIT_SET_INTPS_SERVICE`, `SIT_SET_VOICE_OPERATION`,
  `SIT_SET_MODEM_CONFIG`, `SIT_NS_NETWORK_NORMAL_START`,
  `SIT_SET_DUAL_NTW_AND_PS_TYPE`), with GET counterparts → handlers likely
  present in this build.
- Their **wire opcode ids were not recoverable**, so they cannot be issued
  without inventing bytes (forbidden). CP dispatches by numeric id only and does
  not code-reference the SIT name strings (handler-ptr xref = 0). The vendor RIL
  id→name table (`…sitril-builder` @ file off `0x217fa4`, 557 × 12-byte
  `adrp/add/ret` stubs) is indexed by an **internal enum**, not the wire opcode;
  index→wire is non-linear (idx 279→`0x800`, 175→`0x600`, 88→`0x208`) and no
  clean wire-opcode table in the truncated carve validates against those anchors.
- Every wire-mappable operational SET we *do* know was already tried live and is
  ineffective (EngMode `0x0908`, SGC `0x0404`, cfg `0x093f`, radio `0x0800`,
  net-sel `0x0704`, AllowData `0x0710`, pref `0x070a` — all ACKed, no reg change).
- The gate value `SAE_UE_OPERATION_MODE` is read/written via the CP's internal
  registry (`MMC_GET/SET`, `PlmnSimDataAcc`) and persisted in
  `SAE_FLASH_UE_OPERATION_MODE`; the untried service-SETs mutate *different*
  state, and no SIT handler was found that writes the op-mode parameter without a
  FLASH-NV persist.

**Verdict: no confirmed command-only path flips the op-mode gate; the only known
mutation path is a FLASH-NV write (out of scope, no operator authorization).
Per constraint, no real NV/EFS was written. Device unchanged.** Next RE step if
the operator wants to keep chasing the command avenue (not an NV write): recover
the untried SETs' wire ids from the **full `libsitril.so`** (extract from
`vendor.img`, ext4) or the **live CP RX-dispatch** (`cmp wire_id,#imm; beq
sitRxSet…`); both multi-hour. **Bearer? no.**

**2026-10-02 late — DECISIVE: gate value is CP-STORE; read-divert ruled out.**
Instrumented the owner to log every RFS frame (header fields only) and to keep
reading `umts_rfs0` **after** the write-out completes (the stock owner stopped
polling RFS at TERMINAL, so the RadioPower-ON / MM-gate window had never been
observed). Over a fresh guarded boot the CP's **entire** RFS traffic is
OPEN(7)/STAT(3)/WRITE(6): the early-boot protected-NV write-out (handle 3, size
`0x0002e406`=189446 bytes — the field is the transfer *size*, correcting the
earlier "NV offset" wording) followed by four post-RadioPower-ON `cmd6` **write**
attempts of a ~476 KB file (handle 1). **Zero read-expecting-data requests exist
anywhere** — the CP never asks the AP for file data; it only writes its own
NV/EFS out. Static RE agrees: `RfsRead` exists only in the USIM **PERSO** path
(did not fire — SIM READY), while the gate parameters (`SAE_FLASH_UE_OPERATION_MODE`,
`SAE_FLASH_GCFMODE`, `SAE_FLASH_PLMN_SEL_MODE`, `SAE_FLASH_MOBILE_CLASS_MODE`,
RF-cal `CalDone`) are CP-internal FLASH-NV read via internal accessors.
**Verdict: the gate value is served from the CP's own store, not the AP → no
allowed read-divert can satisfy it; this is the real-EFS/NV boundary.** Owner
instrumented+rebuilt (`69f9b62d…`, self-test PASS, `-Werror`), a one-time
data-chunk payload mis-log was fixed and the on-phone log scrubbed. **Bearer? no.**

Remaining constraint-compliant avenues to raise with the operator:
- **(Allowed) live SIT operational-mode / attach-enable SET** — a *command*,
  not a file write: if an un-issued SIT provisioning opcode can set the CP's
  operational mode to normal at runtime (parallel to the accepted 0x0704/0x0800
  camp SETs), the gate could flip without touching real EFS. Needs opcode
  recovery from the vendor SIT/RIL (not yet confirmed to exist for op-mode).
- **(Boundary — operator decision) authorize a scoped real-NV edit** of the
  handle-1 normal-NV `SAE_FLASH_*` item(s) on a backed-up copy. This crosses the
  standing "never write real EFS/nv" constraint and must not be done without an
  explicit operator go-ahead; document which item and keep a reversible backup.
- **(Allowed) factory/provisioned SIM or factory NV reprovision** via the
  vendor path, out of scope for this quarantine-only owner.

**2026-10-02 pm — MM gate narrowed; PCIe mitigation baked.** The registration
blocker is isolated to a CP-internal **pre-PLMN local MM gate** (voice/data reg
GET `error_raw=0`, `registration_raw=3`/`0`, `reject_cause=0`, no PLMN latched),
**not** forbidden-PLMN and **not** the RFS quarantine (`request_6` writes-out NV
`0x0002e406`; the CP is the data source and keeps its NV in RAM, write ACKed).
`0x20d1afa` is the SIT handler **registrar**; `error_raw=2` is a **generic SIT
refusal** shared by RF scan (0x0706) and SIM_IO ADF ops (both SELECT 0xA4 and
READ_BINARY blocked; only READ_RECORD/MF EFdir works). PCIe mitigation is now in
`owner-handoff-rfs-camp.sh` (RC `power/control=on` pre-boot +
`pcie-stabilize-cp.sh` bounded re-apply of EP L1.2 disable across 0x0800),
validated on a fresh warm reboot: link recovers, IPC functional, SIM READY(5),
registration reproduces the local deny. New one-shot `simdiag-once.c` reads the
settled registration scalars + attempts the USIM EF reads. Full write-up:
[MODEM-BLOCKER one-pager](../targets/panther/MODEM-BLOCKER.md). **Bearer? no.**

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

## Same-owner radio/network event comparison (2026-10-01)

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
its final ACK; the passive owner traces the same ID ranges without answering
RFS. Synthetic host fixtures, Linux ASan/UBSan, static ARM64 builds and
on-device self-tests passed before the phone comparison.

Two fresh, pre-FIN, single-owner boots used the same physical SIM and the
same configured, previously reviewed firmware/NV paths. This table reports
**owner receipt times**, not CP emission times:

| Observation | Passive, RFS unanswered | Full RFS quarantine |
| --- | --- | --- |
| RFS | cmd 7/6 observed, no reply | 95/95 chunks and 189446 bytes stored; final ACK local write returned at +7753 ms |
| `0x0803`, length 8 | +9817 ms | +9813 ms |
| `0x0802`, length 12 | +9818 ms | +9813 ms |
| First 60-second trace window | 2 indications, 0 overflow; no `0x07xx` seen | 2 indications, 0 overflow; no `0x07xx` seen |
| Owner +60-second status | SIM READY/PIN disabled; radio raw 10; voice/data registration 0, reject 0, data technology 0 | Same raw statuses |
| `rmnet0` | down, RX/TX 0 | down, RX/TX 0 |

The same two radio-range headers arrived at nearly the same owner-relative
times even when RFS was unanswered. This narrows the early header sequence,
not the undecoded payloads: the completed RFS exchange was not
required for those two observed headers, and it did not establish camp. RFS
is serviced before IPC and candidate
finalization is synchronous, so an indication queued before an ACK can be
read afterward; the local ACK marker does **not** establish CP emission or
causal order. No `0x07xx` indication reached either owner in its first minute,
but that does not prove CP never searched or RF was idle.

The full wrapper's fresh original-EFS read-only pin passed. After controlled
AP reboot, the four-file read-only postflight comparison passed and EFS was
unmounted. The new root-only candidate, SHA sidecar and `NO_PROMOTION` marker
remain quarantined; no source was promoted. The original owner binaries were
restored from hash-checked backups, and the default passive owner again holds
an ONLINE CP. Logs for both compared boots were preserved under distinct
names. MODEM-06 camp and bearer remain unresolved; further work must isolate
a separate startup/registration prerequisite without guessing a SET. The
next bounded step was host-only review of the exact TD1A `0x0802`
indication adapter and radio-state callbacks; the result is below. The
recorded headers still do not reveal the early radio-state value.

## Factory `0x0802` radio-state indication decode (2026-10-01)

The exact TD1A `vendor.img` `/lib64/libsitril.so` (SHA-256
`efcca0d5fa5a3eb3a09d8c9f68fc35f8b194bb511379987fd4a353f12ed2d5b1`)
has `ProtocolRadioStateAdapter::GetRadioState` at `0x230a20`. It checks the
16-bit indication ID at frame offset `+2` for `0x0802`, then reads a
little-endian 32-bit scalar at frame offset `+8` (`0x230a3c-0x230a48`). The
same TD1A `sit-stream.so` (SHA-256
`cef8756461c74102f9a78f91177d1baff80fb9af11c14994497fb8854e0f530a`)
independently checks `0x0802` and reads `+8` at `0x4890c-0x48918`. Its
factory strings identify the raw values:

| `0x0802` raw scalar | Factory label | Factory RIL state |
| --- | --- | --- |
| 0 | `INITIALIZED` | 0 = OFF |
| 1 | `STOP_NETWORK` | 0 = OFF |
| 2 | `START_NETWORK` | 10 = ON |
| 3 | `POWER_OFF` | 1 = UNAVAILABLE |
| 4 | `RESET` | 1 = UNAVAILABLE |

The exact `libsitril.so` conversion table at `0xd2e80` is `[0, 0, 10, 1]`
for raw 0-3 and defaults to 1 for higher values; `sit-stream.so` uses
`[0, 0, 10]` plus the same default. The OFF/UNAVAILABLE/ON names for RIL
states 0/1/10 are defined by the
[AOSP RIL enum](https://android.googlesource.com/platform/hardware/ril/+/461ada9c41b674fb3e178bd91d656439228644fe/include/telephony/ril.h).
The observed 12-byte `0x0802` frame fits an 8-byte indication header plus
this four-byte body; the factory adapter itself does not enforce an exact
12-byte total. Do not confuse this with the solicited `0x0801` GET response:
its radio-state scalar is at `+12` (`ProtocolNetRadioStateRespAdapter`,
`0x230890-0x2308b8`).

`NetworkService::OnRadioStateChanged(Message*)` (`0x1943d0-0x194440`)
passes the converted value to `UpdateRadioState(value, true)`. The latter
stores a changed state and broadcasts system event `0x101`
(`0x192784-0x19285c`). The distinct `0x0803` radio-ready path calls
`UpdateRadioState(1, true)` (`0x1937c0-0x193844`), i.e. UNAVAILABLE, not
ON. The notifier's available-hook gate checks old state 1 and new state
different from 1 (`0x1561e8-0x1561f4`); its radio-on hook separately checks
new state 10 (`0x1562d0-0x1562dc`). Thus `0x0802` raw 2 can establish an
ON transition, but the two previous boots logged **only IDs and lengths**.
Their `0x0802` values, and whether the early event was ON, remain unknown.
The later solicited raw-10 ON observations do not retrospectively decode
that earlier indication or prove network registration.

The user-supplied `s5300_sit` kernel-driver sketch is not the implementation
in the linked public source. In the published
[S5300 CPIF `modem_main.c`](https://android.googlesource.com/kernel/google-modules/radio/samsung/s5300/+/refs/heads/android-gs-akita-6.1-android15-qpr2/modem_main.c),
the OF compatible is `samsung,exynos-cp` and the platform driver is
`cp_interface`; it parses mailbox, shared-memory and IO-device properties.
That source is from a later branch and does not replace the exact TD1A
factory evidence above. The sketch's `google,s5300-sit`/`link_up` names and
log-only interrupt handler cannot implement the SIT radio-state callback.
Do not install it as a modem fix.

If another controlled phone observation is warranted, its minimal observable
is only the `0x0802` scalar on the existing exclusive owner, with exact ID,
length and bounded enum checks; log no other body bytes, identifiers or
subscriber data. First validate it on a passive fresh boot. Any later
matched passive/full-RFS comparison requires the same trace in both owners
and the original-EFS read-only/postflight gates. This decode alone is not a
reason to send a radio-power, carrier, SIM or network SET.

The default passive `modem-channel-owner.c` now has a host-tested, opt-in
diagnostic build that appends the scalar and factory label to its existing
bounded indication line only for an exact 12-byte type-2 `0x0802` frame with
matching declared length and raw value 0-4. Invalid enum values are not
printed as numbers; other bodies and `0x0803` remain header-only. This adds
no endpoint, GET, SET or RFS reply. GCC `-Werror`, host self-test,
ASan/UBSan, scan-variant self-test and static ARM64 compilation passed.
At that host-validation checkpoint the source had not been run on the phone;
the later passive observation is recorded below. The full-RFS owner
still logs headers only, so a new matched RFS A/B would first require a
separately reviewed identical scalar trace in that owner. The eight-event
per-minute cap remains: absence of a printed scalar after overflow is
inconclusive.

The earlier active `0x0706` scan rejection and later full-RFS completion
were different AP boots. There is no same-boot `dmesg` interval between
them, and the existing wrappers did not preserve a complete kernel event
timeline. Do not assign an IRQ/PCIe cause from older CPIF snapshots. A
future bounded kernel capture should be designed separately only if the
scalar/other evidence points to a transport transition; it must not add a
second IPC reader or an unbounded packet/kernel log. One early OFF or
UNAVAILABLE value is not proof of an RF reset; one ON value is not proof of
camp, completed host initialization or an eSIM/carrier cause.

## Native radio-service boundary and logical-stack check (2026-10-01)

SaaiOS needs the responsibilities of the factory radio service: one request
dispatcher, explicit SIM/logical-stack/radio/registration/data states, startup
callbacks, and bounded recovery. Porting the Android application framework or
Binder interface alone does not supply the vendor SIT initialization. The
factory early-camp path above can request radio power before a framework
client connects. Keep the native `saai-modemd` direction; its current host
models are not yet a working runtime service.

The [AOSP Radio 1.3 contract](https://android.googlesource.com/platform/hardware/interfaces/+/3e9d442/radio/1.3/IRadio.hal)
separates `enableModem` from `setRadioPower`; SIM access can remain available
while a logical modem is disabled. This is an interface contract, not proof
of the Panther's current stack state. The exact TD1A `libsitril.so` SHA-256
listed above independently confirms a distinct SIT route:

| Factory evidence | Wire operation |
| --- | --- |
| `EnableModemHandler::OnRequest` `0x1d1290`, call `0x1d1344`; `ProtocolMiscBuilder::BuildSetStatckStatus` `0x22d0c0` (factory spelling) | SET `0x080f`, length 13, normalized boolean at +12. **Not part of the planned observation.** |
| `GetModemStackStatusHandler::OnRequest` `0x1d1600`, call `0x1d169c`; `BuildGetStatckStatus` `0x22d140` | GET `0x0810`, length 12, no body; factory timeout 5000 ms. |
| `ProtocolMiscGetStackStatusAdapter::GetMode` `0x2297e0`, load `0x2297fc` | Checks response ID `0x0810`, reads byte +12, converts nonzero to true. |

The response handler checks error before reading mode (`0x1d17d8`,
`0x1d1848`). Its default true value on an error is not an observation and
must not be copied into SaaiOS state. Factory code proves a minimum response
length of 13, not an exact length. SaaiOS will additionally require matched
type/token/ID, full-width zero error, bounded framing and mode 0 or 1;
anything else remains unknown. These stricter bounds are SaaiOS policy.

The next isolated change after the passive scalar observation is one
`0x0810` GET through the existing owner after its settled status pass.
An enabled response excludes a disabled logical stack only at that instant.
A disabled response supports a separately reviewed enable experiment; it
does not itself authorize automatic recovery or prove the cause of no camp.
No `0x080f`, carrier SET, slot remap or new IPC reader is added to this GET.

Commit `2d5c28a` implements that GET after the four existing factory network
queries, with a five-second deadline and no retry. The optional scan variant
also requires fresh stack-enabled evidence; unknown/disabled cannot arm it.
An adjacent diagnostic correction reads the entire 16-bit response error
for initial, SIM-refresh and settled status logging, so error `0x0100` is
not displayed as success. Host GCC `-Werror`, default and scan-variant
ASan/UBSan fixtures passed for the stack change; the final logging correction
passed the host fixture and ARM64 build. The on-device passive self-test and
hash check also passed before activation. The same ARM64 GCC toolchain as
the scalar control produced SHA-256
`5683e67228643af682e3dd08bd8ad6705088f6a913e36eb2276ebe751da68276`;
the separate candidate is `/data/saaios/bin/modem-channel-owner.stack-2d5c28a`.

### Concrete rollback for the scalar observation

Before this run, the phone's known-good passive pair and wrapper were copied
to `/data/saaios/var/rollback-0802-b5ade9e/`; its `SHA256SUMS` check passed.
The recorded baseline hashes are:

- `modem-channel-owner`: `b5e9fa744e6bb47a1055043926057897902f78796d506d2d393efa03f5b5e04e`
- `probe-handover-owner`: `a4e15c4420e6fb8608fa04b6f986b1c6b2841016fdafc60aa61a50ed9e9047c5`
- `owner-handoff-bringup.sh`: `33ae6dcfb02f8e2c57fbcfcbaf7088b8040a2fef759999fc15ba10cbe8645007`

The separately staged passive scalar owner, built from `b5ade9e`, is
`/data/saaios/bin/modem-channel-owner.scalar-b5ade9e`, SHA-256
`3b044a28fa59047578a7b1a06f8a285d1125f06e418289afa2aa80a7b4319fac`.
Its on-device synthetic self-test passed and mode is `passive`. The legacy
baseline owner lacks `--mode`, and its probe has no usable
`--owner-exec`/`--owner-log` introspection; preserve their reviewed hashes,
owner self-test and previously validated default handoff.

Before activation, reboot AP, require CP OFFLINE (or unloaded CPIF followed
by the wrapper's OFFLINE check), and preserve both current logs under unused
names. Install the candidate only in this offline interval and use the
default one-shot wrapper. On failure, stop the experiment without an in-place
retry. For return, reboot AP, verify the saved manifest, restore the passive
pair/wrapper while offline, preserve experiment logs, and run the default
wrapper once. Confirm CP ONLINE and owner continuity. No original EFS access
or write is required for this passive instrumentation change.

### First scalar phone observation

A fresh AP boot with the scalar-only candidate reached CP ONLINE and retained
the sole owner. At owner-relative +9815 ms it received `0x0803`, length 8,
then `0x0802`, length 12, raw state **0 (`INITIALIZED`)**. The first minute
had two traced indications and zero overflow. At +60 seconds the same owner
reported SIM READY/PIN disabled, solicited radio raw 10, voice/data
registration 0, selection automatic, preferred SIT type 16 and signal
technology-presence mask 0. `rmnet0` was down with RX/TX 0. This observes an
early INITIALIZED state followed by a later ON query result; it does not
establish an RF reset, a missing host callback or an eSIM cause. No new SET,
RFS reply or EFS access occurred. Preserve these logs as
`modem-channel-owner.scalar-b5ade9e.log` and
`owner-handoff-bringup.scalar-b5ade9e.log` before the stack-status run.

### Logical-stack status phone observation

A separate fresh AP boot with the `2d5c28a` passive owner again reached
ONLINE with one IPC/RFS owner. It received `0x0803` and `0x0802` at
+9807 ms; the latter was again raw 0 (`INITIALIZED`). The first trace
window had two indications and zero overflow. After the same successful
+60-second SIM/radio/registration and four factory-network GETs, the new
`0x0810` request returned a **13-byte success response, error 0,
enabled=yes**. SIM remained READY/PIN disabled, radio raw 10, voice/data
registration 0, automatic selection, preferred SIT 16, signal-presence mask
0 and `rmnet0` down with RX/TX 0.

This excludes a disabled logical modem as the explanation at the measured
instant. Do not issue `0x080f` just because the new command exists. The
observation does not establish RF activity or registration. No SET, active
scan, RFS reply, or original-EFS access was part of either run. Preserve
the second boot's logs as `modem-channel-owner.stack-2d5c28a.log` and
`owner-handoff-bringup.stack-2d5c28a.log`; the next research target is a
specific remaining factory startup prerequisite, not another enable request.

After another AP reboot, both experiment log pairs were preserved under
those names. The original owner/probe/wrapper were restored from the saved
manifest and rechecked at their installed paths; all three hashes matched.
The baseline owner self-test passed, default guarded bringup returned CP
ONLINE, and the passive owner remained alive. Both experimental candidates
and the rollback copies remain available; no candidate became an init service.

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
invokes its available hook when the old RIL state is 1 (UNAVAILABLE) and
the new state is not 1. On RIL socket 0,
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

## SGC configuration provenance correction (2026-10-01)

Absence of `vendor.ril.app.target_carrier` in `build.prop` is not evidence
that the factory RIL lacks an input for SGC. Exact TD1A
`RilApplication::LoadConfigToRilProperty` (`0x12ecc0`) computes and writes
that **internal RilProperty** before the radio-available callback:

1. Read `persist.vendor.radio.sgc`, default empty (`0x12ed9c-0x12edb8`);
   a valid numeric override supplies the target.
2. Otherwise, `CarrierLoader::GetTargetOperator` (`0x11cc10`) uses
   `ro.carrier`, default `unknown`.
3. If unresolved, `GetVendorTargetOperator` (`0x11cd30`) uses
   `ro.vendor.config.build_carrier`, default `unknown`; an unresolved
   vendor lookup falls back to target 400 (`0x11cdbc-0x11cdc8`).
4. Store the resulting internal property (`0x12ee38` or `0x12eec0`) and
   mirror it to `persist.vendor.radio.target_oper` (`0x12eec4-0x12eed0`).

The offline TD1A Panther `vendor.img` SHA-256
`09dd17f863b84601f5b14ed9a282dccd0113c290a8201a44bf4a29c10f0b113e`
has `/build.prop` fingerprint
`google/panther/panther:13/TD1A.221105.001/9104446:user/release-keys`,
`ro.carrier=unknown` and `ro.vendor.config.build_carrier=europen`, with no
SGC override there. The exact library's constructor creates the `europen`
key at `0x11c690-0x11c6bc` and assigns 400 at `0x11c73c-0x11c744`.
`MappingSGCValue` table `0xd86c4` maps 400 to `0x0101` (entry `0xd871c`);
target 0 instead maps to `0x2001`. Never substitute zero for missing config.

The factory `DoSendSGC` supplies `(target, 0, 0)` to the builder
(`0x17fa08-0x17fa1c`), so the static `europen` profile yields the three
body words `0x0101, 0, 0`. This establishes payload provenance, not a causal
link to camp. Image inspection cannot establish a saved runtime override.
On the current SaaiOS phone, `/data/property/persistent_properties` and
`/data/property/persist.vendor.radio.sgc` were absent and no `rild`, `cbd`
or `modem_svc` process was found. That observation is not a reconstruction
of a previous Android session. No SGC GET was identified in the exported
factory symbols; do not invent `0x0405` as a getter.

This corrects the earlier "property absent, payload unknown" research
conclusion. Do not import an entire Android runtime merely to reproduce this
small property-resolution and startup-dispatch responsibility.

Independent review also establishes initialization order. Decoded APS2
relocations put `LoadConfigToRilProperty` in vtable slot +64 and
`OnInitialize` in slot +48 (address point `0x295358`). `InitInstance`
calls them in that order at `0x12daa4` and `0x12dab4`, before the contexts
are created. Thus the factory target property exists before radio callbacks.
The observed `0x0803` UNAVAILABLE followed by `0x0802` raw 0/OFF qualifies
for the factory radio-available hook; an ON indication is not required.

### Isolated delayed SGC experiment

Hypothesis: the missing factory carrier-configuration command alone can
explain the currently unregistered state. This is unproven. The experiment
uses the verified static TD1A `europen` profile, not a claim about an earlier
Android runtime's overrides. Both known persistent-property file paths were
absent on the current SaaiOS filesystem. No Android vendor daemon is run.

Commit `c44ead4` adds a separate compile-time `sgc-once` owner/probe/wrapper
mode and unique logs; passive and scan defaults do not send SGC. First collect the normal
settled baseline and require fresh same-boot SIM READY/PIN disabled, radio
ON, logical stack enabled, unregistered voice/data, automatic selection and
the already observed broad preferred RAT. Also require sole ownership,
CP ONLINE, empty receive buffers and no pending AP transaction. Those are
AP-request gates, not proof of CP-autonomous RF idleness.

The sole active request is factory `0x0404`: 24 bytes, type 0, reserved
bytes 1/10/11 zero, fresh token at +6, LE32 words `0x0101, 0, 0` at +12,
+16 and +20. The auxiliary words' meanings are not established; zero comes
from the exact caller, not an invented default. Use the factory 2000 ms
deadline (`DoSendSGC`, `0x17fa28`). The reply needs type 1, matching ID/token,
declared length and at least 12 bytes, with full 16-bit zero error for
acceptance; the factory generic adapter does not prove exact length 12.

Record the attempt before its single write. Never retry or cancel it. A
well-formed reply permits one independent read-only SIM/radio/voice/data/
logical-stack sweep ten seconds later. Timeout, ambiguous write or lost
framing stops further IPC writes while retaining ownership until OFFLINE;
recover through AP reboot. No RFS reply or original-EFS write is added.
The AP completion callback (`0x17fae0`) has no follow-up SET/reset/file
write, but CP-side persistence and reset behavior are not established.

Success requires actual registration, not merely an ACK. A negative result
only excludes this **late single-command** intervention in that boot; it
does not exclude the earlier factory timing, other startup dependencies or
their ordering. Preserve logs and return to the manifest-pinned passive
baseline after the controlled run. This experiment does not promote a new
boot service or an RFS candidate.

### Late SGC phone result (2026-10-01)

The `c44ead4` owner/probe were built as static ARM64 binaries, with host
default/scan/SGC fixtures, actual-dispatch injected-I/O tests, ASan/UBSan,
CLI/refusal tests and independent review passing before installation.
Distinct phone paths left the manifest-pinned passive recovery files intact:

| Artifact under `/data/saaios/bin/` | Installed SHA-256 |
| --- | --- |
| `modem-channel-owner-sgc-once` | `5955d5fd926391f8898f474053755aaa8948a81d4a8236f2503a0d324335bbc0` |
| `probe-handover-sgc-once` | `201e94c3adca98c2341acabe7920effcb6665019886951d8fab91c9f952f547d` |
| `owner-handoff-sgc-once.sh` | `6c1b46b75ab5b8880092fdc25d539dc06d44fca1190997711663658249ef7dc2` |

On-device `self-test`, mode, probe owner/log identity and shell syntax checks
passed. An initial `--self-test` invocation only printed usage; the correct
`self-test` verb was then run and passed before CP boot. Its simulated
request lines are fixture output, not live modem traffic.

After AP reboot CPIF was unloaded. The original-EFS verifier first refused
the absent `/dev/block` parent before mounting anything. Creating that fresh
root-owned mode-0700 directory allowed the unchanged verifier to compare all
four original/userdata NV files and unmount successfully. The explicit
`sgc-once` wrapper then booted CP ONLINE with one pre-FIN IPC/RFS owner.

The early indications were `0x0803` and `0x0802` raw 0/INITIALIZED at
owner-relative +9808 ms. At the settled baseline, SIM was READY/PIN disabled,
radio raw 10, voice/data registration 0 with reject 0, automatic selection 0,
preferred SIT 16, signal mask 0 and logical stack enabled. No RFS reply was
sent. The single active request and separate post-observation were:

| Owner-relative time | Observation |
| --- | --- |
| +60785 ms | One factory `0x0404` request sent; 2000 ms deadline. |
| +60830 ms | Matched response `error_raw=0`, accepted; no retry. |
| +70912 ms | Post-SIM READY/PIN disabled, error 0. |
| +70916 ms | Post-radio raw 10, error 0. |
| +70919 ms | Post-voice registration 0/reject 0, error 0. |
| +70922 ms | Post-data registration 0/reject 0/technology 0, error 0. |
| +70931 ms | Post-logical-stack enabled, error 0; observation complete. |

The independent sysfs observation still had `rmnet0` down and RX/TX 0.
Thus the **late single SGC request was accepted but did not establish
registration in the measured window**. This does not prove the early factory
sequence unnecessary, nor identify a firmware/carrier/eSIM root cause.
It also does not prove the carrier setting was applied merely from ACK0.

Logs remain on the phone at
`/data/saaios/var/modem-channel-owner-sgc-once.log` and
`/data/saaios/var/owner-handoff-sgc-once.log`. The successful preflight is
`/data/saaios/var/sgc-c44ead4-efs-preflight.log`. No raw subscriber payload,
PIN, EID/ICCID or NV content was printed.

After preserving those logs, AP was rebooted again; CPIF was unloaded.
The unchanged verifier again returned PASS for all four original/userdata
files and confirmed EFS unmounted. Its result is
`/data/saaios/var/sgc-c44ead4-efs-postflight.log`. The original three-file
rollback manifest and passive owner's on-device `self-test` passed.
Previous baseline logs were preserved with `.before-sgc-c44ead4.log`
suffixes. The original default wrapper then reached CP ONLINE; PID 643 was
the sole observed modem-channel owner, with READY/PIN-disabled SIM from its
post-indication query. Its fd 6/8 still held IPC0/RFS0, and its subsequent
+60-second sweep confirmed READY/PIN disabled, radio 10, registration 0,
automatic selection 0 and preferred SIT 16; `rmnet0` remained down with
RX/TX 0. The legacy passive baseline has no logical-stack GET, so its
restoration is not a new stack-status measurement. Separate SGC files
remain inactive, not autostarted.

After reboot the preserved owner log (3744 bytes) had SHA-256
`dbba07a72fe76a672156ee191c9a804d7c5246bc41930a1b0fa8d7b8658920c3`;
the wrapper log (25843 bytes) had SHA-256
`d6d315a8e10d839d0bf48785de0ab3327c1bf2d494fac90b60fc7c69b7f964b1`.

The next isolated hypothesis is **timing**, not another command: the exact
factory callback runs on the early UNAVAILABLE-to-OFF transition, before
our settled +60-second intervention. A future early-SGC variant needs its
own reviewed eligibility/timeout state machine at `0x0803` followed by
`0x0802` raw 0, with the same payload and no added SET/RFS behavior. The
late variant's READY/ON gate must not simply be removed. An early callback
comparison is not yet implemented, installed or authorized as an autostart
service. Porting Android wholesale would not answer this timing question.

### Early SGC comparison contract (2026-10-02)

The next manual experiment changes the dispatch opportunity, not the payload
or the set of active commands. Keep late-SGC, scan and passive binaries
unchanged, and build a mutually exclusive `sgc-early-once` mode with separate
owner/probe/wrapper paths and logs. This section is the pre-implementation
contract, not a phone result or a claim of network service.

Preserve the ordinary four initial read-only GETs. A dedicated state machine
sees every framed IPC indication independently of the capped trace logger.
Require type 2, exact `0x0803` length 8, followed in order by exact `0x0802`
length 12 with raw scalar 0, including valid declared lengths. At that raw-0
event, all four initial GETs must have completed with structurally valid,
zero-error responses and no pending GET or already-sent SIM refresh. If not,
skip the active experiment for that boot rather than moving it later.
SIM READY, radio ON and stack-enabled are not early eligibility requirements:
the reviewed factory hook runs before those later observations.

The event pair must arrive within the first 30 seconds of owner lifetime,
with at most 1000 ms between its two events and at most 1000 ms from raw 0
to dispatch. These are conservative **SaaiOS experimental bounds**, not
constants recovered from the factory RIL. Duplicate, reversed, malformed or
superseding radio events invalidate eligibility; no later event rearms it.
Absence of the pair or failure to dispatch inside that window means
`not-attempted`, not evidence that early SGC cannot work.

Do not write from a frame callback. Parse the entire available batch first;
the early dispatcher then takes priority over queued-but-unsent SIM refresh.
Keep that queue entry for later, require empty IPC/RFS userspace buffers,
no backoff or unread kernel input, and recheck CP ONLINE and sole ownership
immediately before the one write. An already-sent refresh at the trigger
causes a skip; no request is cancelled or intercepted. Do not import the
late mode's 500 ms quiet delay into this early dispatch opportunity.

Use the same factory 24-byte `0x0404` request and 2000 ms reply deadline.
Consume the attempt before writing. A matched well-formed ACK, including a
remote-error ACK, releases the ordinary bounded GET schedule; preserve the
original owner-relative +60-second SIM/radio/registration and factory GETs.
There is no extra ten-second post-SGC sweep in this early variant. Thus a
queued SIM refresh can move by the bounded SGC response interval; the test
is an exploratory timing comparison, not a perfectly schedule-matched A/B.
Ambiguous writes, timeout, malformed reply, framing loss or CP reset retain
both channels without further IPC writes until OFFLINE. Never retry SGC.

Before installation require actual callback/dispatcher fixtures for the
coalesced and fragmented event pair, pending/queued GET ordering, successful
initial-response mask, conflicts, expiry, short writes and late ACKs; also
test all old modes, ASan/UBSan, ARM64 build and independent review. On the
phone require installed hashes, on-device self-test, a fresh AP boot and
read-only original-EFS preflight. Preserve the one run's logs, reboot,
repeat the four-file read-only comparison and return to the pinned passive
baseline. Original EFS is never writable or exposed to RFS. An ACK alone
still does not establish application of carrier configuration or camp.

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

## Early SGC implementation + factory-order consolidation (2026-10-02)

This turn implements the `sgc-early-once` mode from the contract above and
consolidates the factory RF/network bring-up order recovered from the stock
TD1A `libsitril.so` (`efcca0d5…`) and `sit-stream.so` (`cef87564…`). No new
phone run was performed: the diagnostic phone is reachable only over USB NCM
(`172.31.7.1`) with no interactive shell (ADB is an MTP phantom, SSH pubkey
denied), and the contract requires independent review, a fresh AP boot and a
read-only original-EFS preflight before any early-SGC device run. Bearer is
**not** verified.

### Factory ordered bring-up (recovered), with stage deltas vs ours

Order is by stock stage, not a replay list. IDs are from named factory
builders (see the OnRadioAvailable and SGC-provenance sections above).

| Stage | Factory action | SIT id | Our soft stack | Delta |
| --- | --- | --- | --- | --- |
| 0 pre-context (`InitInstance`: `LoadConfigToRilProperty` slot +64, then `OnInitialize` slot +48) | Resolve carrier/region: `persist.vendor.radio.sgc` → `ro.carrier` → `ro.vendor.config.build_carrier=europen` → target **400** → SGC body `0x0101,0,0`; mirror to `persist.vendor.radio.target_oper` | (property only) | not resolved on SaaiOS (no rild); value is the static `europen` profile | **region config exists before radio callbacks**; we never stage it |
| 1 radio-available (RIL UNAVAILABLE→not; wire = `0x0803` then `0x0802` raw 0) `MiscService::OnRadioAvailable` | a. `SetDebugTraceOffOnBoot` | `0x090b` | never | conditional debug; not a camp input |
| 1 | b. `SetModemsConfig` (modem count from `IsMultiSimEnabled`) | `0x093f` | **never** | untested; single-SIM necessity unproven |
| 1 | c. **`SendSGCValue`** (carrier/region config) | **`0x0404`** | late (+60 s) or absent | **primary timing delta — addressed by `sgc-early-once`** |
| 1 | d. `SendSvnInfo` | `0x4605` | never | version metadata; unlikely camp dep |
| 1 | `NetworkService::OnRadioAvailable` → `TrySetRadioPower(10)` under `mUseCampOnEarlier`/`mFirstRunOnBoot`/airplane-off | `0x0800` | radio-on sent **late** at settled baseline | factory may request camp-on **at this early stage**; we only power radio on later |
| 2 post radio-on | GetSimStatus / Get-Set preferred / SetNetworkSelectionAuto / reg polls / VerifyPin-if-enabled | `0x0200`,`0x070b`/`0x070a`,`0x0704`,`0x0700`/`0x0701`,`0x0201` | all sent (READY via VerifyPin A+AID; auto; LTE/broad RAT; AllowData `0x0710`) | **parity** — no missing SIT on the post-radio path |

Net: the post-radio SIT set is at parity; the deltas are all at **stage 0/1**
— carrier/region (SGC `0x0404`) applied too late, and `0x093f`/early camp-on
never applied. `SendSGCValue` is the one stage-1 action with a plausible camp
dependency and a verified static payload, so it is the first isolated timing
test.

### Region / regulatory findings

The "regional" configuration on this No-CDMA SKU is the carrier target
`europen` → 400 → SGC `0x0101`, resolved AP-side by
`LoadConfigToRilProperty` before the radio callbacks and pushed to CP by
`SendSGCValue` at stage 1. No separate OPLMN / regulatory / band-table
builder distinct from SGC was found in `sit-stream` for this path; preferred
RAT (`0x070a`) and selection mode (`0x0704`) are already live and are not the
regional carrier input. So the region delta is specifically the **SGC stage/
timing**, not a missing separate regulatory SIT.

### Firmware / NV load-order finding (2b)

No evidence the modem firmware/NV loads differently from factory: the live
MAIN B (`449eeab3…`) matches the factory signed image family, UDL reaches CP
ONLINE at parity, and the carrier/region selection is an **AP-side RilProperty
→ SGC** runtime step, not a distinct TOC/NV/region partition section applied
at a different firmware stage. The gap is the runtime AP→CP carrier-config SIT
**ordering**, not firmware section order. Do not blind-UDL an alternate image
to chase this.

### Root cause of `0x0706` error 2 (hypothesis, grounded)

`0x0706` is `DoQueryAvailableNetwork` — an **AP-initiated active PLMN scan**.
Its CP reply `error_raw=2` is a generic RF refusal (`RCM_E_GENERIC_FAILURE`
class), not a unique fingerprint. After READY, radio ON, automatic selection
and broad preferred RAT are all satisfied (live-confirmed), the remaining
unmet factory precondition is **stage-1 carrier/region config and/or early
camp-on**: the stock stack applies SGC `0x0404` (and may request camp-on) on
`OnRadioAvailable` *before* any scan, whereas our stack reaches the scan with
carrier config either unsent or applied only +60 s after the settled baseline
(late-SGC ACK alone did not register). The most likely precondition the signed
SGC satisfies is the CP carrier/regulatory profile that an active scan
requires; `0x093f` and early camp-on remain secondary untested candidates.
This is a testable hypothesis, not proof — the `sgc-early-once` mode exists to
apply the exact factory payload at the exact factory stage and re-measure
registration before any further `0x0706`.

### What was implemented and tested

`modem-channel-owner.c` gains a mutually exclusive `SAAIOS_SGC_EARLY_ONCE`
build (`--mode` → `sgc-early-once`), alongside the unchanged late-SGC, scan
and passive modes (a 3-way build guard rejects combining them). It adds an
independent radio-event observer fed every framed IPC indication, separate
from the capped trace logger, that latches the exact `0x0803` (len 8) →
`0x0802` raw 0 (len 12) trigger pair and invalidates on any duplicate,
reversed, malformed, out-of-window or superseding radio event. At that
trigger, within a 30 s owner window, ≤1000 ms inter-event gap and ≤1000 ms
raw-0-to-dispatch bound, and only if the four initial GETs completed with no
pending GET and no already-sent SIM refresh (a queued-but-unsent refresh does
not block — early dispatch takes priority and keeps that entry), it sends the
**one** factory 24-byte `0x0404` (`europen` body `0x0101,0,0`) with the 2000 ms
reply deadline. A matched ACK (including a remote-error ACK) releases the
ordinary bounded GET schedule with **no** post sweep and **no** retry;
ambiguous write, timeout, malformed reply, framing loss or CP reset hold the
owner without further IPC writes until OFFLINE. SIM READY / radio ON / stack
enabled are intentionally not early eligibility requirements.

Build + host fixtures passed under WSL native gcc 15.2 and under
ASan/UBSan, and all four modes (passive, scan, late-SGC, early-SGC)
cross-compile static ARM64 with `-Wall -Wextra -Werror`. New fixtures cover:
the exact request bytes; the early ACK → DONE transition (no post sweep) for
success and remote-error ACKs; the ownership matrix (IDLE/DONE release the
GET slot, WAIT_ACK/HOLD retain it); the radio observer (coalesced and
fragmented pair, reversed/duplicate/non-zero/wide-gap/out-of-window/malformed
invalidation, superseding); and the dispatch gate (happy path, queued refresh
priority, already-sent-refresh skip, incomplete initial GETs, invalidated
pair, no-pair window expiry, short-write and ACK-timeout holds, offline/
another-opener not-attempted, busy transport and recent-RX deferral). Device
artifacts build via `build-owner-sgc-early-once.sh` (owner +
`probe-handover-sgc-early-once` with `probe-sgc-early-once-config.h`).

**Bearer verified? no.** Next: operator-gated early-SGC run (fresh AP boot,
read-only EFS preflight, independent review, installed-hash + on-device
self-test), then watch `0x0802`/registration; if registered, AllowData /
SetupDataCall and verify `rmnet` IPv4 or rx/tx.

## Early SGC live run (2026-10-02)

The operator-gated early-SGC run was executed. Phone interactive access was
restored over the **USB serial console** (`COM13`, root `/ #`); the
"server 110"/R620 SSH key named by the operator was unreachable from this host
and proved unnecessary once the serial console gave a root shell. All device
I/O went through that console plus the existing USB-NCM file push
(`serve-once` → `nc`); no ADB, no persistent cbd/rild.

**Install + self-checks.** The ARM64-static `sgc-early-once` owner
(SHA-256 `322ac00d2e6e50bf7e5502ebf260caee80c1bad0fbad0937fe564836b22af524`)
and probe
(`435602ea456a2369f10a6b3f41a2730cadf57cdaf121e4859cf6ff2d3ee3fde6`) were
rebuilt reproducibly from this tree, pushed, and verified on-device by SHA-256,
`--mode` (`sgc-early-once`), probe `--owner-exec`/`--owner-log` match, and the
on-device `self-test` (PASS). They were installed under their own names; the
default passive binaries, scan and late-SGC owners were untouched. A dedicated
`owner-handoff-sgc-early.sh` (same guards as the `sgc-once` wrapper: CP-OFFLINE
gates, log-symlink/existence checks, **read-only** persist mount only, original
EFS never mounted) was installed and `sh -n`-clean.

**Fresh boot.** The BusyBox `reboot` applet is a no-op here (native-init does
not catch init signals; it reboots only via the `reboot()` syscall), so a
forced `reboot -f` was used after `sync`. The device came back at uptime 1 min
with the cpif modules unloaded, no channel owner, and no `modem_state` node —
a clean pre-bring-up state. Read-only preflight confirmed the original EFS
(`sda5`) was not mounted and `rmnet0` rx/tx were 0/0 with no IPv4.

**Guarded bring-up.** The handoff loaded the cpif modules (CP → OFFLINE),
mounted persist read-only for the `cpsha`, and the probe booted the reviewed B
firmware: `PROBE END result=0`, `probe_rc=0`, CP `OFFLINE`→`ONLINE`. Persist
was unmounted; the early-SGC owner took the fresh channels
(`--ipc-fd 5 --rfs-fd 7 --ready-fd 8`).

**Result — early SGC accepted, no camp/registration.** The owner latched the
early radio edge: `cp_ind 0x0803` (len 8) then `cp_ind 0x0802` (len 12)
`radio_state_raw=0` = INITIALIZED at **+9.817 s**. It then dispatched the
single factory `0x0404` SGC at **+10.323 s**, inside its 2 s deadline
(`trigger=0x0803-0x0802-raw0 target=europen-400`), and the CP **accepted** it:
`response=yes error_raw=0 status=accepted observation=released no-retry`. This
is the first run in which the factory carrier SET landed on the early radio
edge rather than the +60 s settled baseline. SIM then refreshed to
card 1 / apps 1, app_state READY(5), PIN1 DISABLED(3).

The +60 s settled snapshot was nonetheless identical to every prior boot:
radio ON(10); voice `registration_raw=0 reject_raw=0`; data
`registration_raw=0 reject_raw=0 tech_raw=0`; selection automatic (mode 0);
preferred raw 16; operator response success len 119; **signal response success
len 210 with `mask_low7=0`**; modem_stack enabled. Registration-state
indications arrived (`0x0700` len 88, `0x0701` len 86, `0x0703`, `0x070b`,
`0x0702`, `0x0900`, `0x0810`) but registration never left 0. Over three more
minutes `rmnet0` rx/tx stayed **0/0 with no IPv4** and CP stayed ONLINE. No
PLMN, operator, signal or SIM-secret payload was logged.

**Conclusion.** Applying the exact factory SGC at the exact factory early
stage is a genuine mechanical advance — the CP now accepts the carrier SET on
the radio-available edge — but it is **not** sufficient to make the modem camp
or register, and no bearer appeared. **Bearer verified? no.** The stage-1
carrier/region timing is therefore not the sole missing precondition. The next
isolated candidates at the same `0x0803`→`0x0802`-raw-0 trigger, one at a time
with evidence, are early `SetModemsConfig 0x093f` and early camp-on `0x0800`
(`TrySetRadioPower(10)`); each needs its own guarded, mutually exclusive build
mode built and self-tested the same way before any device run. No secrets, NV,
APN, PIN, CardPower or EFS writes were made in this run.

## Stock stage-1 RE from the vendor RIL, and the `sgc-seq-once` build

**Vendor library recovery.** With root on the live phone (build
`CP2A.260705.006`), the stock cbd was pulled read-only by loop-mounting the
logical `vendor_a` partition inside `super` (parsed from the LP metadata;
`losetup -r -o <offset> /dev/loop0 /dev/block/super`, `mount -t ext4 -o ro`).
The live `/vendor/lib64` RIL/SIT libraries sit in an **unmerged Virtual-A/B COW
overlay** (`vendor_a-cow`), so the base partition reports "Structure needs
cleaning" on `lib64` and is not directly readable. The reverse-engineering
therefore used this tree's complete TD1A `vendor.img`
(`libsitril.so` SHA-256 `efcca0d5…`, identical to prior RE); the SIT command
IDs seen in every live boot log match it, so the ordered protocol is the same.
EFS was never mounted read-write; no secrets were logged.

**The real stock ordered sequence (socket 0, radio-available).** Disassembling
the `OnRadioAvailable` handlers:

- `MiscService::OnRadioAvailable` (gated on `GetRilSocketId()==0`):
  `SetDebugTraceOffOnBoot` (**0x090b**) → `SetModemsConfig` (**0x093f**) →
  `SendSGCValue` (**0x0404**) → `SendSvnInfo` (**0x4605**, tail call).
- `NetworkService::OnRadioAvailable`: `TrySetRadioPower(10)`, which issues an
  internal `OnRequest(RIL_REQUEST_RADIO_POWER=23, …, 4)` whose downstream
  handler builds **0x0800** (conditional on a camp-early `SystemProperty`).
- `PsService::OnRadioAvailable`: a conditional `OnRequest(10161/0x27b1)` only
  when a `RilContextProperty` string is non-empty.

We were sending **only 0x0404**. The command that stock issues immediately
*before* the SGC, and which we omitted, is **`SetModemsConfig` 0x093f**; the
command stock issues *after* (at the Network stage) is the early camp-on
**0x0800**. `0x090b` (debug-trace) and `0x4605` (SVN/version) are non-camp.

**Recovered bodies.** `0x093f` = a 13-byte request
(`InitRequestHeader(id=0x93f,len=13)`) whose single payload byte at offset 12
is `(modem_count!=1)?1:0` = **0 on this single-SIM SKU**. `0x0800` = an 18-byte
request with the power word at offset 12 = `(on)?2:1` and flag bytes at offsets
16/17; its exact `arg2/arg3` come from `DoRadioPower`, not yet fully recovered,
so `0x0800` is **not** implemented this turn (no guessed body).

**`0x0706` error 2 decoded.** In `rcmErrorToString` (vaddr `0xd7b50`, codes
0–31) and `ConvertProtocolErrorCodeToRilErrorCode` (`0xd7bd0`), code `2` is
**`RCM_E_GENERIC_FAILURE`** → RIL error 2. It is the generic catch-all refusal,
**not** the reg-ordering code 9 (`OP_NOT_ALLOWED_BEFORE_REG_TO_NW`) nor a
PLMN-search-specific code — i.e. a fundamental precondition is missing, not a
simple ordering slip.

**Implementation — `sgc-seq-once`.** A fourth mutually-exclusive diagnostic
build mode `SAAIOS_SGC_SEQ_ONCE` was added to
`os/targets/panther/diagnostics/modem-channel-owner.c`. It reuses the early
radio observer, the early dispatch gate, the ACK→DONE-with-no-post-sweep
transition and the early ownership/reporting (now shared under a new
`SAAIOS_SGC_EDGE` umbrella), and on the `0x0803`→`0x0802`-raw-0 edge it writes
the stock pair **in order**: `make_setmodemsconfig_request` (0x093f, 13 bytes,
payload 0) back-to-back, then the existing `0x0404` SGC. The config ACK is
matched and logged separately (`sgc_seq_config … status=accepted`) while the
SGC ACK still drives DONE. The build guard is now 4-way; host self-test adds
cases 179–185 (exact 0x093f body, config-ACK consume without disturbing the SGC
phase, no duplicate consume, remote-error config ACK) and the live fixtures
assert the two-write dispatch. All five modes (seq, early, late, scan, passive)
compile with `-Wall -Wextra -Werror` and pass `self-test`.

Artifacts: `build-owner-sgc-seq-once.sh`, `probe-sgc-seq-once-config.h`,
`owner-handoff-sgc-seq.sh`. ARM64-static reproducible SHA-256 — owner
`12c07414d3f7875f97a59254a3c4534420a160788f5a87525ea940bcfd322ebb`, probe
`651515c8887f81f86fd0b9aa7ace7a5eaf620242ccb1a6cd277a356457593f2d`.

**Live run — `0x093f` is a confirmed no-op; still no camp/registration.**
Pushed the owner/probe/handoff over USB-NCM (`serve-once`→`nc`), verified
on-device by SHA-256, `--mode` (`sgc-seq-once`), probe `--owner-exec`/`--owner-log`
match and `self-test` (PASS); `sh -n` clean. Forced a fresh boot (`reboot -f`
after `sync`); the device came back at uptime 0 with cpif unloaded, no
`modem_state`, EFS unmounted and no rmnet IPv4. The guarded handoff loaded the
modules, mounted persist read-only for the cpsha, booted the reviewed B
firmware (`PROBE END result=0`, CP OFFLINE→ONLINE), and the owner took the
fresh channels.

The owner latched the early edge (`0x0803` len 8 then `0x0802` len 12
`radio_state_raw=0` at +9.817 s) and dispatched the stock pair in order at
+10.322 s: `sgc_seq_config … cmd=0x093f modems=1` then the `0x0404` SGC. The CP
**accepted the SGC** (`error_raw=0 status=accepted`) exactly as before, but
returned **no response at all to `0x093f`** (no `sgc_seq_config … response`
line) — the CP silently ignores a single-SIM `SetModemsConfig`, consistent with
it being a no-op on this SKU. The +60 s settled snapshot was byte-for-byte the
same story as the early-SGC run: radio ON (`radio_raw=10`), voice/data
`registration_raw=0 reject_raw=0`, selection mode 0, preferred 16, operator len
119, signal len 210 `mask_low7=0`, modem_stack enabled. All `rmnet0`–`rmnet29`
stayed `rx=0 tx=0` with no IPv4 on any rmnet. **Bearer verified? no.**

**Conclusion.** Prepending the stock `0x093f` in correct order does not move
the CP toward camp or registration, and the CP does not even acknowledge it —
so `SetModemsConfig` is not the missing precondition on this single-SIM SKU.
With the SGC and `0x093f` both ruled out as sufficient, the remaining stage-1
delta is the **early camp-on `0x0800`** that stock issues at
`NetworkService::OnRadioAvailable` via `TrySetRadioPower(10)`
(`OnRequest(RIL_REQUEST_RADIO_POWER=23, …, 4)`). The next step is to recover the
exact `0x0800` body from `DoRadioPower`/`BuildRadioPower` (power word, `arg2`,
`arg3`) — no guessed body — then add a `sgc-camp-once` (or extend
`sgc-seq-once`) mode that issues `0x093f`→`0x0404`→`0x0800` on the same edge,
self-test it, and live-run it one candidate at a time. No secrets, NV, APN,
PIN, CardPower or EFS writes were made in this run.

## `0x0800` RE + `sgc-camp-once`: camp-on reaches START_NETWORK, zero signal

**Exact `0x0800` body (derived, not guessed).** From stock TD1A `libsitril`:
`NetworkService::TrySetRadioPower(int)` (`0x1930f0`) issues
`ServiceInterface::OnRequest(RIL_REQUEST_RADIO_POWER=23, &v, 4)` with the single
int `v = (arg==10)?1:0 = 1`. `NetworkService::DoRadioPower(Message*)`
(`0x193860`) reads `ints[0]` as the power value and only reads `ints[1]/ints[2]`
when the request carries **≥3 ints**; with one int both are 0. It then calls
`ProtocolNetworkBuilder::BuildRadioPower(1, 0, 0)` (`0x236350`), which emits an
18-byte message via `InitRequestHeader(id=0x800, len=18)` with the power word at
offset 12 = `(arg1!=0)?2:1 = 2` and the two flag bytes at offsets 16/17 =
`arg2`/`arg3` = 0. So the camp-on `0x0800` body is: id `0x0800`, len 18, payload
`+12 = LE32 2` (radio ON), `+16 = 0`, `+17 = 0`.

**Implementation — `sgc-camp-once`.** A fifth mutually-exclusive mode
`SAAIOS_SGC_CAMP_ONCE` (under the shared `SAAIOS_SGC_EDGE` umbrella) sends the
full stock stage-1 in order on the `0x0803`→`0x0802`-raw-0 edge:
`0x093f`→`0x0404`→`0x0800`. The SEQ config-step blocks were generalized to a
`SAAIOS_SGC_CFG_STEP` macro and a new `SAAIOS_SGC_CAMP_STEP` adds the `0x0800`
builder, back-to-back send, and ACK matching (moved above the phase gate so the
post-SGC `0x0800` reply is logged even after the SGC ACK reaches DONE). Host
self-test adds cases 186–190; all six modes build `-Wall -Wextra -Werror` and
pass `self-test`; the guard rejects any combination. Artifacts:
`build-owner-sgc-camp-once.sh`, `probe-sgc-camp-once-config.h`,
`owner-handoff-sgc-camp.sh`; ARM64 reproducible SHA-256 — owner
`0597553d7d0924b7ca33a3d499adfa4d1d20ea19de22dab50b695b4be3deb7cd`, probe
`7a2914d454845336f9da32e1e7e9efebcc625ffa88fc956fde812c04e02904e9`.

**Live run — `0x0800` ACCEPTED, CP enters START_NETWORK (new), still zero
signal.** Fresh boot → RO-persist guarded handoff → CP ONLINE; verified on
device (SHA-256, `--mode sgc-camp-once`, probe flags, `self-test` PASS). On the
edge (`0x0803` len 8 then `0x0802` raw 0 at +9.818 s) the owner dispatched all
three at +10.323 s. The SGC was accepted (`error_raw=0`) and — new — the
**`0x0800` was accepted** (`sgc_camp_power … error_raw=0 status=accepted` at
+10.386 s), immediately driving `cp_ind 0x0802 radio_state_raw=2 =
START_NETWORK`. The CP then actively searched: the owner's heartbeat recorded
`ipc_2:0x0906` **×47** (PLMN/network-search indications), `0x074b`, `0x070e`,
`0x0810` — activity never seen in the earlier runs, where the radio only sat at
INITIALIZED→ON. `0x093f` even drew a (very late, +80.8 s) accept this time.

But the settled result was unchanged where it matters: radio `radio_raw=10`,
data `registration_raw=0`, `net_factory signal … mask_low7=0` (no serving
cell), and some settled GETs timed out because the CP was busy searching. All
`rmnet0`–`rmnet29` stayed `rx=0 tx=0`, no rmnet IPv4. **Bearer verified? no.**

## RF-precondition pivot: the unserved protected-NV RFS write

The camp-on moved the modem from idle to **active network search yet it detects
zero signal** — this is an RF/NV precondition, not command ordering. The
evidence converges on the protected-NV RFS transaction this sprint is named
for:

- Throughout every camp boot the modem drives the RFS channel
  (`umts_rfs0`) and our camp owner answers none of it (`rfs_responses=none`):
  the recorded sequence is **cmd 7 unprotect (state 3) → cmd 3 op-status → cmd
  6 io_write to protected NV at offset `0x02e406`, len 2** (the exact bytes are
  pinned in `rfs-error-probe.c`), retried across the whole run (seq 0→3 over
  162 s).
- The stock RIL gates the radio/RF on NV readiness: `libsitril` contains
  `RCM_E_NO_RF_CALIBRATION_INFO`, `SIT_PWR_RADIO_SIM_STATE_NV_NOT_READY` /
  `…NV_READY`, `ProtocolMiscVersionAdapter::GetRfCalDate`, `GetRfState`, and
  `NvReadItem`/`NvWriteItem` handlers. The modem must complete its protected-NV
  write/readiness handshake before the RF front-end yields valid measurements.
- So even though camp-on now reaches START_NETWORK and scans, the modem cannot
  finish the NV handshake (its `cmd 6` write is never granted), RF stays
  un-ready, and `mask_low7` stays 0.

**Most probable RF precondition:** the modem's protected-NV RFS write (`cmd 6`
to `nv_protected.bin` region, offset `0x02e406`) must be granted for the modem
to mark NV/RF-cal ready and produce signal. This is exactly the transaction the
MODEM-07 quarantine owners (`modem-rfs-one-grant-owner.c`,
`modem-rfs-full-quarantine-owner.c`) already serve **to a quarantined copy,
never to original EFS**.

**Concrete, constraint-safe next action:** run the early camp-on IPC sequence
and the quarantined RFS write-grant **together** — an owner that holds both
`umts_ipc0` and `umts_rfs0`, serves the modem's `cmd 7/3/6` protected-NV
sequence into the existing quarantine (RAM/userdata copy, with the fresh
read-only original-EFS provenance check first, never an EFS RW mount or a write
to `sda5`), and issues `0x093f`→`0x0404`→`0x0800` on the radio edge. Then watch
whether `mask_low7` becomes non-zero and registration (`0x0700`/`0x0701`)
advances. The prerequisite is the factory `cmd-6`-after-state-3 success reply
shape (which `rfs-error-probe.c` refuses to invent) recovered from the stock
`rfsd` (SHA `58d7f885…`). No opcodes are invented; no RF-cal/NV is written to
original EFS; EFS is never mounted RW.

## Combined owner (`SAAIOS_RFS_CAMP`): reply-shape recovery + live result (2026-10-02)

**Factory reply shapes recovered from `rfsd-cp2a` (SHA `58d7f885…`).** The
protected-NV responder at `0x0f230` builds a 16-byte reply: `strh #3,[buf+0]`
(response command = `3`), `strh seq,[buf+2]` (the request's echoed sequence,
loaded from `[obj+64]`), `stur d0,[buf+4]` (an 8-byte `.rodata` constant =
`{len=8, status}`), and `str [obj+8],[buf+12]` (the protected-file state = `3`).
The success constant is `{8,0}` (len 8, status 0); the error variant at
`0x5138` is `{8,6}`. This confirms, directly from the disassembly (no invented
bytes), the shapes already encoded and self-tested in
`modem-rfs-full-quarantine-owner.c`:

- cmd7 unprotect / cmd3 op-status reply `status_7` = `{3,0,0,0, 8,0,0,0, 0,0,0,0, 3,0,0,0}` (seq 0).
- **cmd6-after-state-3 success** `final_status` = `{3,0,1,0, 8,0,0,0, 0,0,0,0, 3,0,0,0}` (seq 1): response cmd `3`, echoed seq `1`, len `8`, status `0`, state `3`.

**Combined owner implemented.** `modem-rfs-full-quarantine-owner.c` gains an
opt-in `SAAIOS_RFS_CAMP` feature (mutually exclusive with the SGC owner modes):
an isolated active camp dispatcher with its own bounded framer and token that
arms on the exact `0x0803`→`0x0802`-raw0 radio edge and issues the stock
stage-1 `0x093f`→`0x0404`→`0x0800` once, while the unchanged RFS machinery
serves the `cmd 7/3/6` protected-NV sequence into the quarantine copy (fresh
read-only original-EFS provenance gate first; original EFS never mounted RW,
`sda5`/`nv_protected.bin` original never written). Host + on-device `self-test`
PASS (`-Wall -Wextra -Werror`); non-camp build unregressed. Artifacts:
`build-rfs-camp-combined-owner.sh`, `probe-rfs-camp-combined-config.h`,
`build-probe-rfs-camp-combined.sh`, `owner-handoff-rfs-camp.sh`. Reproducible
ARM64 SHA-256 — owner `7a88e30b9a5fbb72eb84f8482b55dde9b3dac1da4feddc8724bc1a50892e1d38`,
probe `e32538e86d06040aba71f120cd7002080a44a762b219b306b38c413986898c6f`.

**Live run — the protected-NV handshake now COMPLETES; still no registration.**
Fresh boot → guarded RO-persist handoff → CP OFFLINE→ONLINE; on-device SHA-256,
`--mode rfs-camp-combined`, probe `--owner-exec`/`--owner-log`, and `self-test`
(PASS) all verified. The combined owner:

- Served the CP's full protected-NV transfer to quarantine and **sent the gated
  final success ACK**: `rfs_full_quarantine=complete_quarantined_ack
  grants_attempted=95 chunks_stored=95 bytes_stored=189446 final_ack_sent=1
  failure_stage=none` at +7.7 s. This is the `cmd 7/3/6` + 189446-byte write
  handshake that was never completed before — the RFS-level blocker is cleared.
- Dispatched the camp trio at the radio edge (+9.8 s): `0x0404` accepted
  (`error_raw=0`) and `0x0800` radio-power-on accepted (`error_raw=0`); `0x093f`
  stays a silent no-op on this single-SIM SKU (no ACK), as previously found.
  (First boot the camp was blocked because the successful RFS terminal gated all
  modem writes; the gate was corrected to forbid writes only after a *failed*
  terminal, and a self-test case was added.)

**But registration and bearer did not follow.** No `0x0700`/`0x0701`
registration indications appeared in the observable window; `rmnet0` rx/tx =
`0/0` with no IPv4 at +180 s. The passive SIT snapshot read `card_raw=0 apps=0`
at +1.5 s and its settled (+60 s) SIM GET **timed out**, which self-poisons the
observer (`sit_observer=reply_timeout no_more_gets=1`), so later radio/network
indications are no longer traced and this owner cannot read signal strength.

**Where the handshake now stalls / next action.** The protected-NV RFS write is
no longer the stall — it completes and is ACKed. The modem accepts radio-power
but produces no registration and no bearer, and two observability gaps block a
definitive signal verdict: (1) this owner never issues a signal-strength GET, so
`mask_low7` is unread; (2) its SIT observer self-poisons on a single settled-GET
timeout, and the online-pass `card_raw=0 apps=0` suggests the card is not driven
to READY by this passive owner (unlike `modem-channel-owner`, which performs SIM
refresh/queries). The single most probable remaining cause is that NV-write
completion alone is insufficient — the modem additionally needs the SIM driven
to READY and/or a signal query to confirm RF — so the constraint-safe next step
is to extend the combined owner with a signal-strength GET and sustained
(non-self-poisoning) radio/registration observation, plus the SIM-readiness
drive, to see whether `mask_low7` becomes non-zero after the now-completed NV
handshake. No opcodes or reply bytes were invented; EFS was never mounted RW.

## Combined owner + active prober: SIM READY, signal, registration (2026-10-02)

The combined owner (`SAAIOS_RFS_CAMP`) was extended with an **active prober**
that replaces the passive SIT observer in camp mode. It drives the SIM by
re-GETting `0x0200` on every `0x0210` (SIM status changed) and keeps a
single-outstanding, round-robin GET over `{0x0200 sim, 0x0900 signal,
SIT_NET_VOICE_REG, SIT_NET_DATA_REG}` across the whole settle window. A reply
timeout or ambiguous write only backs the prober off (`field=… status=timeout`);
it **never self-poisons**, never disables the dispatcher, and never touches RFS
quarantine. The signal mask is parsed exactly as `sit-stream.so`'s V4 reader
(`n>=210`, no error, `le16(buf+12)&0x7f`). The RFS quarantine machinery and the
one-shot stage-1 dispatch are unchanged. Host + on-device self-test PASS
(`PASS combined RFS quarantine + camp dispatch self-test`, mode
`rfs-camp-combined`), `-Wall -Wextra -Werror -pedantic`, reproducible ARM64:
owner `221cf0d222a70c985df4b90a072042ef3933bb4e65ab701f6b9abe0956390c96`,
probe `e32538e86d06040aba71f120cd7002080a44a762b219b306b38c413986898c6f`; the
non-camp full-quarantine owner is unregressed
(`7d4b6d6d251aa7ff93f129ef87781e683fdaa7f1b1e5701dd4aefedc0f3efdf9`).

**Live run (COM13, fresh `sysrq` boot → RO-persist guarded handoff).** Ordered
observations:

1. RFS protected-NV handshake **completes + ACKed** (`final_ack_sent=1`,
   95 grants, 189446 bytes quarantined; `no_promotion=1`, EFS never mounted RW).
2. Camp stage-1 on the `0x0803 → 0x0802-raw0` edge at +9.8 s: `0x0404` and
   `0x0800` **accepted** (`error_raw=0`); `0x093f` silent no-op.
3. A +10…+49 s GET-timeout storm (CP busy during radio-on/scan) — the prober
   **rode through it** instead of self-poisoning, and then captured the settled
   state the old self-poisoning owner never could:
   - SIM reached **READY**: `card_raw=1 apps=1 app_state_raw=5 pin1_raw=3`,
     held across 50 consecutive reads.
   - Signal **non-zero**: `mask_low7=2` (was 0), held.
   - Voice CS registration advanced to `registration_raw=3` = **REG_DENIED**,
     `reject_raw=0` = **local/internal deny** (no NAS cause IE), held (49 reads).
   - Data PS registration stayed `registration_raw=0` = **NOT_SEARCHING**,
     `tech_raw=3` (UMTS), held (49 reads).
4. Bearer: `rmnet0 rx=0 tx=0`, no IPv4. **Not attempted** — PS is not
   registered, so `SetupDataCall` cannot succeed.

**What this run newly establishes.** The 2026-09-30 open question — *does MM deny
CS because `app≠READY`?* — is now **refuted**: with the SIM **READY(5)**, the
NV handshake **landed**, and **signal present (mask=2)**, CS registration is
**still** `REG_DENIED` with `reject_cause=0` and PS **still** NOT_SEARCHING. A
`reject_cause=0` denial is the CP's MM refusing **internally**, before any
network NAS cause — so it is not a network rejection.

**Verdict: CP-config / missing protected-NV·RF-cal, not environmental.** Signal
presence (`mask_low7=2`) argues against a dead antenna or absent serving cell;
SIM READY removes SIM-init as the blocker; a local/internal CS deny plus PS
never searching points at CP-side provisioning/calibration the modem normally
reads from its **real** EFS — which the quarantine model deliberately never
writes, so "the NV handshake landing" gives the CP no config it lacked. **Next
unmet precondition:** successful CS/PS registration. **Constraint-safe next
action:** reverse-engineer the CP registration gate (`SIT_REG 0x20d1afa`;
PresentObj `#636c +0xBF6` in CP shm) to identify exactly which internal config
the MM validates before permitting registration — not EFS writes, not `0x0704`
spam. No opcodes or reply bytes were invented; EFS was never mounted RW.

## 2026-10-02 — registration triggers in the owner; blocked by PCIe link-drop

Rather than one-shot `ipc0` tools (which race the single owner for the channel
and never coordinate the sit-status lock), the registration triggers were
integrated into the unified owner (Option C): once the SIM reads READY it runs a
one-shot sequence — confirm radio ON (`0x0801`); read selection (`0x0703`) and
force **automatic** (`0x0704`) unless already auto; read preferred RAT (`0x070b`)
and **broaden `LTE_ONLY(11)` → `LTE_WCDMA(12)`** (`0x070a`); `AllowData(1)`
(`0x0710`). Builders are the recovered/self-tested factory shapes from
`ready-network-once.c`; SETs fire at most once, GETs retry a bounded number of
times then advance (so a slow/absent reply cannot stall the chain). Reproducible
owner `8ba92be2…`; host + on-device self-test PASS (adds `test_camp_reg`),
`-Werror`.

**Driving hypothesis:** preferred RAT `LTE_ONLY(11)` vs. only-UMTS-present
(`mask=2`, data `tech_raw=3`) is a RAT mismatch; broadening to `LTE_WCDMA(12)`
should let the modem register on the present UMTS cell — constraint-safe, no EFS
write.

**Live:** one clean boot showed the sequence engage (radio `radio_raw=10`, then
selection GET); the bounded-retry fix now carries it past a non-replying
`0x0703` to the broaden + AllowData. But the broaden was **not observed live**:
the next **8 consecutive** `sysrq`-reboot → handoff cycles came up with the modem
**PCIe endpoint dropping right after RadioPower-ON** (`cpif: pcie_send_ap2cp_irq:
… PCI not powered on`) — SIM never initialized (`card_raw=0`), every IPC GET
timed out, so the SIM-READY-gated sequence never ran. CP boots cleanly
(`complete_normal_boot`, `CP2AP_WAKEUP=1`, no `cp_crash`; 100 %/33 °C). Recovery
that did not help: long modem-off soak, RC `power/control=on`, disabling EP L1.2
ASPM (`l1_2_aspm`/`l1_2_pcipm`=0), repeated reboots — the drop is cpif-managed CP
runtime-PM below ASPM and warm `sysrq b` does not reset the modem rail; a **cold
power cycle** is the likely requirement. The real-EFS boundary is **not** reached
— the broaden experiment is ready and needs only one boot where the endpoint
stays powered. Bearer verified: **no**.
