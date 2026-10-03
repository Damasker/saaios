# Panther modem blocker (MODEM-06) — one pager

**VERDICT 23 — MECHANISM PINNED: the operator-control GENERIC_FAILURE is a nv_NORMAL write-stall caused by our quarantine NOT answering the modem's handle-1 commit grant — and it targets nv_NORMAL (handle=1), NOT nv_protected (handle=3). Read-only analysis of the V22 owner/RFS log + the owner source nails the exact path. When 0x074f (SetAllowedNetworkTypeBitmap) is sent, the CP (1) applies the bitmap to volatile RAM (GET-after reads `0x3fe`), then (2) tries to PERSIST it by issuing an RFS handle-1 (nv_NORMAL) `cmd=6` grant-request to flush its dirty nv_normal image. Our owner's `post_terminal_rfs_drain` only answers the exact protected-NV `request_7` (handle-3) unprotect; the handle-1 nv_normal grant path is compiled out (behind `SAAIOS_RFS_NORMAL_CAPTURE`, OFF in the shipped `-DSAAIOS_RFS_CAMP` build). So the CP's write-grant is never returned, the nv_normal commit stalls, and the CP returns `error_raw=2` (GENERIC_FAILURE) for the SET even though the RAM value already changed. V22 log proof (handle/field semantics from `rfs_trace`: cmd@+0 [low16 op | seq<<16], paylen@+4, handle@+8, off@+12, w4@+16, w5@+20; constants `request_7/3/6` all carry handle=3=nv_protected; source comment line 1773 confirms handle=1=normal-NV): boot quarantine = handle-3 cmd 7/3/6 + 94× handle-0 data chunks (189446 B) → `complete_quarantined_ack`; post-term the CP issued exactly 3 handle-1 `cmd=6` grant-requests (`off=0`, whole-file flush) with growing totals `w4=476552 → 476670 → 476714`, the last coincident with the 0x074f SET (`t=162446`, elapsed 55631 ms); `NORMAL_CAPTURE` count = 0 ⇒ the owner answered NONE of them. (1) MECHANISM: nv_normal commit grant unanswered ⇒ CP-side persist timeout ⇒ GENERIC_FAILURE; the RFS write is a WHOLE-FILE nv_normal flush (handle=1, off=0, len≈476714 B), so the specific allowed-RAT/selection byte offset is NOT exposed by the grant — it is recoverable only by capturing two quarantine nv_normal images and diffing (constraint-safe follow-up). (2) PROTECTED vs NORMAL: the record is **nv_NORMAL** (handle=1) — the mutable settings store — NOT nv_protected (handle=3, which carries RF-cal/IMEI/security and is only touched at boot). Both are off-limits for REAL writes per AGENTS.md, but nv_normal is the lower-risk settings partition — this corrects V22's loose "protected-NV" wording. (3) REAL-NV PATH: the GENERIC_FAILURE is a QUARANTINE ARTIFACT, not a modem limitation — on the device's genuine committed EFS the CP's handle-1 nv_normal writes land and the SET would ACK. But it is MOOT for LTE: V22 already read LTE as ALLOWED in committed/default NV (`wire=0x403fe`, LTE=1), so the RAT gate is already open and 0x074f never needed to succeed to permit LTE. No real write was performed or attempted this run (read-only log analysis only); device left pristine (rebooted post-V22: CP OFFLINE, config only `apn`, firmware `449eeab3` + NV backup intact) (2026-10-03):**

### 1. Exact commit mechanism (from V22 RFS log + owner source)
`post_terminal_rfs_drain` (owner `4427641b`) replies ONLY to `request_7` (handle-3, protected unprotect) with the observed `status_7`; the handle-1 nv_normal grant branch lives under `#ifdef SAAIOS_RFS_NORMAL_CAPTURE`, which is NOT defined in the shipped build. V22 post-term trace: 3× `handle=1 cmd=…0006` (nv_normal flush) grant-requests, `off=0`, `w4`=476552/476670/476714 (whole-file total, growing as records dirty), all UNANSWERED (`NORMAL_CAPTURE=0`). The 0x074f SET’s flush (`w4=476714`) stalls → CP returns `error_raw=2`.

### 2. Which NV, and which record
Handle map (from `request_7/3/6` constants + source): **handle=3 = nv_protected** (RF-cal/IMEI/security; boot-only, quarantined), **handle=1 = nv_NORMAL** (settings, incl. allowed-RAT / selection). The 0x074f commit is nv_NORMAL. The grant is a whole-image flush (off=0, len≈476714 B); the exact allowed-RAT byte offset within nv_normal is not in the grant and needs a capture+diff of two quarantine nv_normal images to localize.

### 3. Constraint-safe options (ranked; none performed)
- **(a1) No write needed — RAT gate already open (BEST, zero risk):** committed/default NV already has LTE allowed (`0x403fe`); skip 0x074f entirely. The allowed-RAT lever was never the blocker, so nothing needs to persist.
- **(a2) Quarantine-copy nv_normal capture (constraint-safe, policy-sanctioned):** enable `SAAIOS_RFS_NORMAL_CAPTURE` so the owner answers the handle-1 grant and stores the CP’s nv_normal write to a QUARANTINE COPY (`normal-candidate.bin`), acking locally — exactly "RFS writes served to a quarantine copy only." This may let operator-control SETs (e.g. 0x0705 manual-select) return `error_raw=0` WITHIN A SESSION with NO real EFS write; the change is volatile (reverts on reboot) so it is inherently revertible. Unproven (hypothesis — the protected-NV quarantine ack worked at boot, so the nv_normal ack plausibly satisfies the CP); it also enables the capture+diff that localizes the allowed-RAT record. **This is the recommended next experiment.**
- **(b) Real nv_normal write (FORBIDDEN — user policy decision only, NOT performed):** the ONLY way to PERSIST operator-control changes is to serve handle-1 nv_normal writes to the REAL EFS. Scope: nv_NORMAL (handle=1), whole-file ≈476714 B (Shannon flushes the entire nv_normal, not a single record; the allowed-RAT/selection bytes live within it — localizable via (a2) diff). This is NOT RF-cal/IMEI/security/SIM (those are nv_protected). A one-time, user-authorized write would require a byte-exact backup of the current real nv_normal + a verified restore path. Currently prohibited by AGENTS.md; described for the user’s decision only — do NOT perform.
- **(c) Modern StartNetworkScan 0x0734 (sidesteps NV entirely):** a scan is a transient query with NO nv_normal commit, so it avoids the write-stall; it may trigger LTE cell discovery/reselection with no persist. Remaining host lever; band/channel packing is the decode-confidence blocker (structure decoded in V22).

### 4. Decisive verdict
The operator-control GENERIC_FAILURE is NOT a protected-NV/security wall and NOT a modem limitation — it is our quarantine declining the CP’s **nv_NORMAL** (handle-1) commit grant, so the persist times out. For LTE specifically it is moot (LTE already allowed). The constraint-safe way to make SETs ACK in-session is a quarantine-copy nv_normal capture (a2); the only PERSISTENT unlock is a user-authorized real nv_normal write (b), which remains forbidden. **The non-NV host lever (modern StartNetworkScan, c) is the most promising path that needs neither a write nor a policy change.**

---

**VERDICT 22 — NO ENCODING DRIFT: the CP2A vendor RIL builds the SAME bytes as the TD1A RIL for every operator-control frame, the baseband-version read now succeeds once we send the stock selector, and the modem STILL refuses SetAllowedNetworkTypeBitmap and STILL camps foreign Vodafone-UA 3G. VERDICT 21 hypothesised a TD1A→CP2A opcode/body vintage drift and took the empty 0x0901 SW-version as evidence. We obtained the matching CP2A vendor RIL and re-derived the frames; the drift hypothesis is FALSIFIED. Source: the public Google factory image `panther-cp2a.260705.006` (`…-factory-ed94a24e.zip`); its `radio-…-g5300q-260317-260505-b-15346003.img` matches the running CP exactly. Extracted `vendor.img` (de-sparsed, debugfs read-only) → `lib64/libsitril.so` **sha256 `b488325d…`** (distinct from TD1A `efcca0d5`) and `lib64/vendor.radio.protocol.sit.stream.so` **sha256 `cef87564…`**. KEY STRUCTURAL FINDING: in CP2A, libsitril's `ProtocolNetworkBuilder::Build*` are UND imports — the actual frame builders live in the stream lib (`cef87564`), the SAME stream lib already referenced in V20. Per-frame TD1A↔CP2A diff (builders `@0x76620`/`@0x766f0`/`@0x74820`/`@0x74ad0`/`@0x74c60`/`@0x708c0`, reply adapters `GetRat@0x4bfc0`, `GetSwVer@0x41e90`): 0x074f (len 16, bitmap@12, RAF→wire transform bit-for-bit identical, verified against inverse GetRat), 0x0750 (len 12, reply bitmap@12), 0x0709 (len 12), 0x0706 (void len 12 / int-mode len 16 @12), and the 0x0901 reply offsets (SW@frame+13, HW@45, RfCal@77) are **ALL byte-identical** to TD1A — NOTHING drifted. The one real discrepancy was OURS, not the firmware's: stock calls `GetBaseBandVersion(h=0xFF)` (`BasebandVersionHandler::OnRequest` `@0x176e5c`, CP2A libsitril: `mov w1,#0xff`) — a field selector at payload[12]. V21 sent 0, so the CP returned an empty version string. Fixing the selector to 0xFF, the on-device read now returns `sw_version=g5300q-260317-260505-B-15346003` (`error_raw=0`) — proving the frame pipeline is byte-correct for CP2A and that V21's "empty = reply-body drift" was our bug. With the byte-identical CP2A 0x074f (LTE+WCDMA+GSM `0x3fe`): GET-before `wire=0x403fe` (LTE+WCDMA+GSM+NR already allowed), SET still `error_raw=2` (GENERIC_FAILURE), GET-after `wire=0x3fe` — i.e. the SET updated volatile RAT state but failed to COMMIT (consistent with protected-NV being non-writable in our quarantined bring-up; reverted by reboot). Modem stayed camped foreign **25501** with the Kyivstar SIM READY; no home registration; no bearer attempted. The modern band-specified StartNetworkScan (0x0734) was fully structurally decoded (input `RIL_RadioAccessSpecifier_V1_5` stride 172, SIT specifier stride 78, header fields payload[0..9], specifier array payload[10], per-RAT band jump-table + SIMD channel packing in `BuildStartNetworkScan@0x75870`) but the band/channel packing could not be reproduced byte-exact with full confidence, so per the no-invented-bytes rule it was NOT sent. CONCLUSION: CP2A frames accepted/byte-identical, pipeline proven correct — but still no home registration. The blocker is NOT RIL opcode/body vintage; it is (a) the modem declining to COMMIT operator-control SETs under our protected-NV quarantine (GENERIC_FAILURE with a volatile-only RAT change), and (b) the modem's own band/cell/PLMN selection staying on the foreign 3G cell while LTE is already allowed+preferred and home operators are in range (iPhone-confirmed, V19). Device left known-good: rebooted to pristine idle (CP OFFLINE, no owner running, config clean (only `apn`), volatile RAT-bitmap reverted via NV reload), firmware `449eeab3` intact, proven NV backup intact; recovered-only bytes, no NV/EFS/RF-cal/firmware write, no IOCTL_POWER_OFF, no do_cp_crash, no dial, no modern scan sent, vendor blob NOT committed (2026-10-03). Owner build sha16 `4427641b` (on-device self-test RC=0):**

### 1. CP2A vendor RIL obtained + provenance
`panther-cp2a.260705.006` factory zip → inner `image-…zip` → `vendor.img` (ext4, de-sparsed, debugfs ro). `lib64/libsitril.so` **`b488325d…`** (CP2A, 2026) vs in-repo TD1A `efcca0d5` (2022). `lib64/vendor.radio.protocol.sit.stream.so` **`cef87564…`** holds the real `ProtocolNetworkBuilder::Build*` (libsitril imports them). Radio image build `g5300q-260317-260505-B-15346003` = running CP. Blob kept under temp work dir, NOT committed.

### 2. Per-frame TD1A→CP2A drift = NONE (plus the one real bug)
| Frame | Opcode / len | Body / reply offset | TD1A vs CP2A |
|---|---|---|---|
| SetAllowedNetworkTypeBitmap | 0x074f / 16 | wire bitmap int32 @payload[12]; RAF→wire transform | identical (bit-for-bit) |
| GetAllowedNetworkTypeBitmap | 0x0750 / 12 | reply bitmap @payload[12] | identical |
| QueryAvailableBandMode | 0x0709 / 12 | header-only | identical |
| QueryAvailableNetwork | 0x0706 / 12 (void) or 16 (mode@12) | — | identical |
| SetNetworkSelectionManual | 0x0705 / 22 | [12]=radioTech, [16..]=PLMN ascii(6), '#'@+9 | structure recovered |
| GetBaseBandVersion | 0x0901 / 13 | **selector@payload[12] = 0xFF (stock)**; reply SW@+13, HW@+45, RfCal@+77 | reply offsets identical; **our bug was selector=0** |
| StartNetworkScan | 0x0734 / var | spec stride 172→78, band jump-table + SIMD channels | decode-confidence blocker — not sent |

### 3. On-device (Kyivstar SIM, CP2A-byte-correct owner `4427641b`)
Bring-up to allow_data `error_raw=0` ~49 s. **`camp_bbver sw_version=g5300q-260317-260505-B-15346003 error_raw=0`** (selector 0xFF ⇒ non-empty — fix validated). GET 0x750 before `wire=0x403fe` (LTE+WCDMA+GSM+NR already allowed); band mode 0x709 `error_raw=0 len=20`; SET 0x074f (`0x3fe`) `error_raw=2`; GET 0x750 after `wire=0x3fe` (volatile change, no NV commit). Operator `25501#`, SIM READY throughout; no home registration; no bearer attempted.

### 4. Decisive verdict — not vintage; it's NV-commit policy + modem band/cell selection
No opcode/body drift exists TD1A↔CP2A for the operator-control family; our frames are byte-correct for the running CP (proven by the now-parsing baseband version). The refusal is the modem declining to commit these SETs under protected-NV quarantine (GENERIC_FAILURE with volatile-only effect), while its internal band/cell/PLMN selection keeps the foreign 3G cell despite LTE being allowed+preferred and home operators in range. **Remaining host-side lever: the byte-exact modern band-specified StartNetworkScan (decode-confidence blocker).** Deeper levers (band NV / RF-cal / stored PLMN selection) are CP-internal and off-limits.

---

**VERDICT 21 — VERSION SKEW CONFIRMED, BUT INVERTED: the running CP firmware is NEWER (CP2A, 2026) than the libsitril we mine (TD1A, 2022). The hypothesis was that the running CP is OLDER than the mined `libsitril.so` (efcca0d5), so the modern operator-control opcodes (0x074f/0x0705/0x0706-modern/StartNetworkScan) would be unimplemented. Read-only investigation inverts that: (a) our bring-up loads `/data/saaios/bin/saaios-probe-b-modem.bin` (98,265,168 B, sha 449eeab3) whose embedded Shannon build id is `g5300q-260317-260505-B-15346003` — a 2026 / CP2A-era modem build (matches the live CP2A.260705.006), NOT the 2022 TD1A radio image; (b) the only `libsitril.so` in-repo (efcca0d5) is extracted from `factory-td1a-vendor/` ⇒ TD1A (td1a.221105.001, Nov 2022); no CP2A vendor libsitril is present. So the running CP is ~3.5 years NEWER than the mined RIL, falsifying "CP too old to implement these opcodes." SIT cross-check: GET_BASEBAND_VERSION was recovered byte-exact (`ProtocolMiscBuilder::GetBaseBandVersion(unsigned char)` @0x22b020 → opcode 0x0901, len 13, type byte at payload[12]; reply SW-version C-string at frame+13) and read on-device — the CP ACCEPTED 0x0901 (`error_raw=0`, so it DOES implement the baseband-version opcode), but the SW-version field came back EMPTY at the TD1A-expected frame+13 offset, i.e. the reply-body layout differs from the TD1A libsitril — a concrete marker that TD1A↔CP2A SIT wire definitions have diverged. CONCLUSION: the refused operator-control family is most consistent with opcode-number/body DRIFT between the TD1A libsitril we mine and the running CP2A firmware (the frames we build from the 2022 RIL do not match what the 2026 CP expects), while the legacy opcodes (0x070a/0x0404/0x0710/0x0800) still ACK because they are stable across vintages. Fix direction (constraint-safe, host-side only): mine the libsitril that PAIRS with the running CP2A firmware — extract `libsitril.so` from the CP2A (260705.006) vendor image and re-derive 0x074f/0x0705/0x0706/StartNetworkScan opcodes + body layouts from THAT, then retry; the repo currently has only the CP2A modem/radio CP images, not the CP2A vendor partition, so obtaining/extracting that vendor `libsitril.so` is the next action. (Residual possibility: a state/mode gate; but the demonstrated 0x0901 reply-layout drift is direct evidence of vintage divergence.) Strictly read-only this run: recovered bytes only, no writes, no NV/EFS/RF-cal/firmware change, no IOCTL_POWER_OFF, no do_cp_crash; device left known-good: CP ONLINE, config clean (only `apn`), owner `69a8397e` (on-device self-test RC=0), proven backup `90f403df` intact (2026-10-03):**
**[SUPERSEDED by VERDICT 22: the "empty 0x0901 SW-version = reply-body drift" inference was wrong — the reply offset is byte-identical TD1A↔CP2A; the empty string was our request selector (0 instead of the stock 0xFF). No TD1A→CP2A encoding drift exists for the operator-control family.]**

### 1. Running CP firmware build (authoritative, from the loaded image)
Our handoff (`owner-handoff-rfs-camp.sh`) boots `FIRMWARE=/data/saaios/bin/saaios-probe-b-modem.bin`. Its Shannon TOC is `BOOT / MAIN / NV_NORM / NV_PROT / REPLAY / INFO`; embedded build id **`g5300q-260317-260505-B-15346003`** (2026, CP2A-era). SIT read of 0x0901 ACKed `error_raw=0` (opcode implemented) but returned an empty SW string at the TD1A reply offset (layout drift).

### 2. Mined libsitril provenance
`os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so` (efcca0d5) — **TD1A (td1a.221105.001, 2022)**. Only libsitril in-repo; no CP2A vendor libsitril extracted.

### 3. Decisive verdict — inverted skew; match the host RIL to the running CP
Running CP (CP2A/2026) is newer than the mined RIL (TD1A/2022); the refused modern opcodes come from the older RIL vintage and the one accepted modern-misc opcode (0x0901) already shows reply-body drift. The refusals are therefore most likely an opcode/body vintage mismatch, not a too-old CP. **Next constraint-safe step: extract and mine the CP2A vendor `libsitril.so` and re-derive the operator-control frames from it.**

---

**VERDICT 20 — RAT-GATE HYPOTHESIS FALSIFIED BY DIRECT READ: LTE IS ALREADY ALLOWED; the modem still camps only on foreign Vodafone-UA 3G and refuses the modern SetAllowedNetworkTypeBitmap. A sibling static analysis suspected our bring-up gates LTE out because it only sends the legacy SetPreferredNetworkType (0x070a) and never the Android-13 SetAllowedNetworkTypeBitmap (0x074f). We recovered the modern RAT-gate command set byte-for-byte from the factory `libsitril.so` (efcca0d5) and read the gate directly on-device with a Kyivstar SIM inserted. DIAGNOSTIC (read-only): GetAllowedNetworkTypeBitmap (0x0750) returned wire bitmap `0x403fe` ⇒ **LTE=1, WCDMA=1, GSM=1, NR=1** — the modem ALREADY permits LTE at the host-visible gate, so the "LTE gated out" hypothesis is falsified. QueryAvailableBandMode (0x0709) returned error 0 (band-mode list present). The legacy preferred read (0x070b) = 16 and set_preferred_lte_wcdma (0x070a) ACKed error 0. SET (0x074f) with an LTE+WCDMA+GSM bitmap (`0x3fe`) was REFUSED with `error_raw=2` (RIL_E_GENERIC_FAILURE) — the same refusal class as the scan/manual-select levers. Despite LTE being allowed and preferred, the modem camped ONLY on foreign **Vodafone-UA 25501**, UMTS/3G (tech_raw=3), out to ~165 s: voice REG_DENIED(3, reject 0), data not-registered(0). Kyivstar card READY, PIN disabled (`card_raw=1 apps=1 app_state_raw=5 pin1_raw=3`). No LTE cell was ever attempted or acquired. The one remaining untried lever — the modern band-specified StartNetworkScan (@0x2376a0) + SetSystemSelectionChannels — is a DECODE BLOCKER: StartNetworkScan is a large frame (766-byte `RIL_RadioAccessSpecifier_V1_5` array, stride 172, band jump-tables) that could not be reproduced byte-exact with confidence, and the `BuildSystemSelectionChannels()` symbol found (@0x2385f0, opcode 0x074e) is a header-only variant carrying no band specifiers — so per the no-invented-bytes rule it was NOT sent. NET: the blocker is NOT the host-side RAT bitmap (LTE is allowed); it is the modem's own cell/band selection sitting on the foreign 3G cell and declining operator-control SETs — pointing at CP-internal band NV / RF-cal / stored PLMN selection (off-limits) and/or the undecoded modern scan. Combined with VERDICT 19 (working SIM reader) and the iPhone evidence (all three UA home operators present), the blocker is now localized inside the modem's band/cell selection. Device left known-good: rebooted to reload stock NV (the SET was rejected; a read-back NR delta is restored by the NV reload), ratbm experiment disarmed, CP ONLINE, config clean (only `apn`), owner `f41849c7` (on-device self-test RC=0), proven backup `90f403df` intact; recovered-only bytes, no NV/EFS/RF-cal write, no IOCTL_POWER_OFF, no do_cp_crash, no dial, no scan sent (2026-10-03):**

### 1. Recovered modern RAT-gate frames (`libsitril.so` efcca0d5)
- **SetAllowedNetworkTypeBitmap** (`ProtocolNetworkBuilder::BuildSetAllowedNetworkTypeBitmap(int)` @0x238670) → **opcode 0x074f, len 16**, SIT-wire RAT bitmap int32 at payload[12]. The int arg is an Android RadioAccessFamily (RAF) bitmap that the builder re-encodes bit-for-bit into the wire bitmap. Recovered transform (verified against the inverse in `ProtocolNetGetAllowNetworkAdapter::GetRat` @0x2340d0): **wire bit3 = UMTS/WCDMA, bit4–6/8 = HSDPA/HSUPA/HSPA/HSPAP, bit7 = LTE, bit9 = GSM, bit10 = TD-SCDMA, bit18 = NR**. LTE+WCDMA+GSM ⇒ RAF `0x1ce0e` ⇒ **wire `0x3fe`**.
- **GetAllowedNetworkTypeBitmap** (@0x2387a0) → **opcode 0x0750, len 12** (header-only); reply carries the wire bitmap int32 at payload[12].
- **QueryAvailableBandMode** (@0x236ae0) → **opcode 0x0709, len 12** (header-only); reply is a modem-defined band-mode list.
- **StartNetworkScan** (@0x2376a0) / **SetSystemSelectionChannels** (@0x2385f0) / **StopNetworkScan** (@0x237cd0): DECODE BLOCKER — not sent (see verdict). StopNetworkScan uses opcode 0x0734 len 22; StartNetworkScan shares 0x0734 with a large specifier body.

### 2. On-device diagnostic + SET (Kyivstar SIM, ratbm armed)
Bring-up to allow_data `error_raw=0` ~49 s. GET 0x750 (before) `wire=0x403fe lte=1 wcdma=1 gsm=1 nr=1 error_raw=0`; band mode 0x709 `error_raw=0 len=20`; SET 0x074f (`0x3fe`) `error_raw=2`; GET 0x750 (after) `wire=0x3fe`. Operator `25501#` throughout; voice REG_DENIED(3)/UMTS, data not-registered(0), to elapsed ~165 s. SIM READY, PIN disabled.

### 3. Decisive verdict — LTE allowed but not acquired; blocker is modem band/cell selection
The host-visible RAT gate already allows LTE, so not sending 0x074f never gated LTE out; the modern SET is itself refused (GENERIC_FAILURE); and the modem keeps selecting only the foreign Vodafone-UA 3G cell while the home LTE operators are in range (iPhone-confirmed) and the SIM reader works (V19). This localizes the blocker to the modem's own band/cell/PLMN selection (CP-internal NV / RF-cal, off-limits) and/or the undecoded modern band-specified StartNetworkScan. **No LTE camp and no bearer reachable here via the host-side levers decoded so far; the modern scan remains a decode blocker.**

---

**VERDICT 19 — ENVIRONMENTAL CONCLUSION FALSIFIED + SIM READER VALIDATED: an iPhone at the SAME location does a manual network search and sees ALL THREE Ukrainian home operators with good signal (UA-KYIVSTAR 255-03, VODAFONE 255-01, lifecell 255-06). This directly falsifies the VERDICT 13–18 "no home coverage / environmental boundary" reading: the home networks are physically present and strong right here, so the real problem is in OUR Pixel modem bring-up (it camps only on foreign Vodafone-UA 25501 3G, never acquires LTE, and refuses every network scan), NOT the RF environment. To rule out a stale/phantom-card confound (across VERDICTs 16/18 every inserted card read identically — READY, PIN-disabled, same serving cell), we ran a control test with the SIM tray physically EMPTY: a full handoff boot (fresh CP boot OFFLINE→ONLINE, genuine card re-interrogation) then one SIM-status read. Result (no secrets): `card_raw=0 apps=0` ⇒ no card, no application (so no `app_state_raw`/`pin1_raw` emitted). With no card the modem also does NOT camp at all: voice `registration_raw=0` and data `registration_raw=0`, `lac=0 cid=0 psc=0` — unlike every card-present run, which camps on 25501. The SIM reader is therefore WORKING: card absence is correctly detected and the fresh boot genuinely re-interrogates the tray, so the prior card reads were real (not a frozen phantom session) and the modem truly saw each inserted card. NET: the blocker is NOT environmental and NOT a dead card reader; it is our bring-up/selection logic settling on a foreign 3G cell and declining scans. Device left known-good: CP ONLINE, config clean (only `apn` in /data/saaios/etc), owner binary intact, proven backup `90f403df` intact; read-only control test, no registration/scan/dial/bearer, no NV/EFS write, no IOCTL_POWER_OFF, no do_cp_crash (2026-10-03):**

### 1. iPhone falsification of the environmental conclusion
Same physical location, iPhone manual network search → all three UA home operators visible with good signal: **UA-KYIVSTAR (255-03), VODAFONE (255-01), lifecell (255-06)**. Home coverage is present and strong, so the VERDICT 13–18 "environmental boundary / no home coverage" explanation is falsified. The Pixel modem's foreign-only Vodafone-UA 3G camp + scan refusals are a bring-up defect, not an RF-reachability limit.

### 2. Empty-tray control test (SIM reader validation)
Full handoff boot (fresh CP OFFLINE→ONLINE, fresh card interrogation), tray EMPTY. First and steady SIM-status read: `card_raw=0 apps=0` — no card, no application present, hence no `app_state_raw`/`pin1_raw`. Voice `registration_raw=0`, data `registration_raw=0`, `lac=0 cid=0 psc=0` — with no SIM the modem does not camp anywhere (contrast: every card-present run camps on 25501).

### 3. Verdict — reader OK, boundary is in our bring-up
`card_raw=0 / apps=0` on an empty tray = reader WORKING: absence correctly detected, fresh boot genuinely re-reads the tray ⇒ the identical prior per-card readings were genuine, and the modem really did see each card. This retires the "phantom/stale card" hypothesis for presence detection (the identical PIN-disabled app readings across cards remain a separate open question, but are not a frozen presence session). Combined with the iPhone result, the root cause is relocated from the environment to the SaaiOS modem bring-up: the CP acquires only a foreign Vodafone-UA 3G cell and refuses operator-controlled scan/selection, while the home networks are demonstrably in range. **Next focus: why bring-up settles on 25501 3G and never scans/selects the available home PLMNs.** No code changed this run.

---

**VERDICT 18 — THIRD CARD, SAME RESULT: another physically-swapped SIM reaches READY (PIN disabled) and is again CS-DENIED on foreign Vodafone-UA 25501. The user inserted a different SIM card (operator unspecified). One minimal observation run (no code change): bring-up to known-good, one SIM-status read, one registration observation (~2 min settle). Card status (no secrets): `card_raw=1 apps=1 app_state_raw=5 pin1_raw=3` ⇒ card present, **READY**, **PIN disabled** (no PIN required). The modem camped only on foreign **25501#** (Vodafone UA, UMTS/3G, LAC 36291); **CS/voice = REGISTRATION_DENIED (state 3, reject cause 0)**; **PS/data = not registered (state 0)**. Identical to VERDICTs 13–17, so the environmental/firmware-state boundary now holds across THREE distinct card insertions. The card's home operator cannot be named without reading a subscriber identifier (IMSI/ICCID), which is not logged; only the serving PLMN (25501, foreign) is observable. NOTABLE ANOMALY: every card read so far comes up **PIN-disabled and READY on the identical serving cell** (same LAC/CID/PSC), and the card the user previously stated had PIN enabled (VERDICT 16) ALSO read PIN-disabled — this suggests the SIM reader/tray may not be re-reading the physically swapped card (stale card session or poor tray contact), a hardware angle worth confirming physically. RF-path note: the user confirmed this target uses the **STOCK internal Pixel 7 antenna** (no external antenna/attenuator/booster), so an external-RF-hardware explanation for the foreign-only camp + CS-deny is ruled out. Device known-good: CP ONLINE, automatic selection, default LTE+WCDMA, owner `3320c305`, proven backup `90f403df` intact; read-only observation, no NV/EFS write, no IOCTL_POWER_OFF, no do_cp_crash, no scan, no dial (2026-10-03):**

### 1. Card status (raw, no secrets)
`card_raw=1 apps=1 app_state_raw=5 pin1_raw=3` — present, READY, PIN disabled. First read at ~2 s was `card_raw=0 apps=0` (pre-init), then steady READY. No PIN prompt needed.

### 2. Serving cell + registration
Operator `25501#` (Vodafone UA); voice `registration_raw=3` (DENIED), reject 0, UMTS(3), LAC 36291; data `registration_raw=0` (not registered), reject 0. Stable past the ~2-minute settle window.

### 3. Verdict — boundary holds across a third card; tray/reader anomaly flagged
Another card, same terminal outcome: foreign-only camp (25501) and CS deny. With the stock internal antenna confirmed (no external RF hardware), and three cards all reading PIN-disabled/READY on the same serving cell, the two live hypotheses are (a) an environmental/firmware-state boundary (the CP acquires only this foreign 3G cell and it denies the card), and (b) a SIM reader/tray contact issue causing a stale card session across physical swaps — the latter is worth a physical check. **No bearer reachable; no code changed this run.**

---

**VERDICT 17 — ALL SCAN FORMS REFUSED: the last recovered operator-control lever (the len-16 scanType scan variant) is rejected with RIL_E_GENERIC_FAILURE in every accepted scan-mode. The one remaining untried command was `ProtocolNetworkBuilder::BuildQueryAvailableNetwork(int)` (@0x236950) — same opcode 0x0706 as the len-12 header-only scan but total len 16, carrying an explicit scanType int32 at payload[12]. The stock clamps the arg (`sub w8,arg,#1; cmp #5; csel` ⇒ scanType = (1≤arg≤5) ? arg : 0), so the distinct accepted values are 0,1,2,3,4,5 (the HAL's `DoQueryBplmnSearch` passes 0; `DoQueryAvailableNetwork` passes a framework scanType logged as "scanType=%d"). We recovered the frame byte-for-byte, implemented it guarded/one-shot/self-tested (-Werror clean, on-device self-test RC=0), and after a clean bring-up (modem camped on foreign Vodafone-UA 25501 3G, allow_data ACKed error 0) swept all six distinct scanType values, each ONCE, from the camped state. EVERY mode returned `error_raw=2` (RIL_E_GENERIC_FAILURE) with no result list: mode 0→2, 1→2, 2→2, 3→2, 4→2, 5→2. No PLMN list was ever returned, so no home PLMN (Kyivstar 255-03, lifecell 255-06, or any non-25501) could be observed, and manual-select was not applicable. Combined with VERDICT 14 (len-12 scan + manual-select refused while camped) and VERDICT 15 (len-12 scan refused from a confirmed-deregistered window), this exhausts every recovered form of operator-controlled network scan across every radio state. The modem services every other host-side SIT step (config/SGC/radio-power/selection/preferred-RAT/initial-attach-APN/allow-data all ACK error 0) but categorically declines GET_AVAILABLE_NETWORKS in all lengths and scan-modes. This is a conclusive terminal firmware-state + environmental boundary, not a SaaiOS host-side defect, and not circumventable constraint-safe. No bearer, no scan, no reselection is reachable here via host-side SIT replay. Device known-good: CP ONLINE, automatic selection, default LTE+WCDMA, owner `3320c305`, proven backup `90f403df` intact; recovered-only bytes, no NV/EFS write, no IOCTL_POWER_OFF, no do_cp_crash (2026-10-03):**

### 1. Recovered len-16 scan frame (`BuildQueryAvailableNetwork(int)`, @0x236950)
- **Opcode 0x0706, total len 16** (InitRequestHeader(0x706, 16)); header + one int32 body.
- **scanType int32 at payload[12]**, clamped by the stock to `(1 ≤ arg ≤ 5) ? arg : 0`. Distinct accepted values **0, 1, 2, 3, 4, 5** (0 = default/clamp, 1–5 = explicit scan modes; the exact per-value semantics are not named symbolically in the binary — only the field name `scanType` is recovered from the HAL log string). This differs from the len-12 header-only variant (VERDICT 14) only by the explicit scanType field.

### 2. One controlled boot — full scanType sweep (scan16 armed)
`camp_scan16=armed`; bring-up to allow_data (`error_raw=0`) ~49 s; modem camped `25501#` (voice REG_DENIED(3), UMTS). Sweep from the camped state: `mode=0…5` each `response=yes error_raw=2` (elapsed 51–59 s), zero result lists. The whole recovered scanType enum is refused.

### 3. Decisive verdict — recovered operator-control levers exhausted (terminal boundary)
Across VERDICTs 14–17 the modem refuses network scan in **both** recovered frame forms (len-12 header-only and len-16 scanType) across **every** accepted scan-mode and **every** radio state (camped, registered, confirmed-deregistered), and refuses manual network selection (0x0705). No available-networks list is ever produced, so the home PLMN can be neither discovered nor selected, and the modem remains on a foreign Vodafone-UA 3G cell that CS-denies the card. **The recovered operator-control levers are now fully exhausted; no bearer is reachable here via host-side SIT replay.**

---

**VERDICT 16 — NEW-SIM TEST (Kyivstar activation attempt): CARD DETECTED + SIM READY, but STILL CS-DENIED on the only reachable cell — same environmental boundary now confirmed across two subscriptions; activation call correctly NOT placed. A second SIM was swapped in (intended Kyivstar, home PLMN 255-03, to be activated by one outgoing voice call to +380953444757). We recovered the full voice-call + PIN-unlock command set byte-for-byte from stock `libsitril.so` (efcca0d5) — SIM PIN1 verify, DIAL, GET_CALL_LIST, HANGUP — implemented them guarded/one-shot/self-tested (-Werror clean, on-device self-test RC=0), and ran one controlled fresh boot. Result: card present (`card_raw=1 apps=1`), SIM reached READY (`app_state_raw=5`) with PIN reported DISABLED (`pin1_raw=3`) — so the PIN-verify path correctly never fired (the card required no PIN; the card reports PIN disabled rather than the expected PIN-enabled state, so the physically-seated card is worth a re-check). Full bring-up ran (get_radio → selection → preferred LTE+WCDMA → initial_attach_apn → allow_data). The modem again camped ONLY on foreign Vodafone-UA `25501#` (UMTS/3G); CS/voice registration held `registration_raw=3` (REGISTRATION_DENIED, reject 0) steadily out to ~4 min; PS/data `registration_raw=0` (not registered). Because CS registration was never achieved (never 1=home / 5=roaming), the activation call was correctly gated off and NEVER dialed (`camp_call=sent` count 0) — exactly per the directive (place the call only IF CS-registered). No bearer, no call. The host-side voice stack is implemented and self-validated but could not be exercised end-to-end because the modem denies CS registration on the only acquirable cell. Device known-good: CP ONLINE, automatic selection, default LTE+WCDMA, owner `661ac0ab`, proven backup `90f403df` intact, secret PIN config file written transiently then deleted (never logged/committed); recovered-only bytes, no NV/EFS write, no IOCTL_POWER_OFF, no do_cp_crash (2026-10-03):**

### 1. Recovered voice-call + PIN command set (stock `libsitril.so`, efcca0d5)
- `ProtocolSimBuilderLegacy::BuildSimVerifyPin(0,…)` (@0x259bf0) → **PIN1 verify opcode 0x0201, len 38**: `[12]`=PIN length (strlen capped 8), `[13..]`=PIN ASCII, AID omitted. (which=1 ⇒ PIN2 opcode 0x0203.) The PIN is written only into the request frame and scrubbed immediately after the write — never printed, logged, or committed.
- `ProtocolCallBuilder::BuildDial(…)` (@0x222cd0) → **DIAL opcode 0x0001, len 104**: `[12]`=call type (voice=0), `[14]`=number length (≤82), `[15..]`=number ASCII, `[97]`=TOA (0x10 if leading '+', else 0x20), `[98]`=1, `[99]`=CLIR (default 0).
- `ProtocolCallBuilder::BuildGetCallList()` (@0x222c50) → **GET_CALL_LIST opcode 0x0000, len 12** (header-only). Reply: count=int32 at `[12]`; per-call list at `[16]`, stride 327 (v1_1); within an entry `[0]`=SIT call state, `[1..4]`=call index.
- `ProtocolCallBuilder::BuildHangup(int)` (@0x222f30) → **HANGUP opcode 0x0008, len 20**: `[12]`=call index int32, `[16]`=1.

### 2. One controlled boot (sim_pin + call_number armed)
Config loaded (`camp_sim_pin=loaded`, `camp_call_number=armed len=13`). SIM: first read `card_raw=0` (pre-init), then steadily `card_raw=1 apps=1 app_state_raw=5 pin1_raw=3` ⇒ card present, READY, PIN disabled ⇒ no `pin_required`, no `verify_pin1` sent. Reg sequence sent get_radio/get_selection/get_preferred/set_preferred_lte_wcdma/set_initial_attach_apn/allow_data. Operator `25501#` throughout. Voice `registration_raw=3` (DENIED), data `registration_raw=0`, tech UMTS(3), cell LAC 36291 — stable to elapsed ~234 s.

### 3. Decisive verdict — same boundary across two subscriptions
With CS registration denied on the only reachable cell, `camp_call_next` correctly refused to dial (gated on voice reg ∈ {1 home, 5 roaming}); no DIAL/HANGUP frame was ever emitted. The outcome is identical to VERDICTs 13–15 with the prior SIM: the modem acquires only a foreign Vodafone-UA 3G cell that CS-denies the card, and no home/roaming CS registration is attainable here. The voice/PIN host stack is now recovered, implemented, and self-validated; it is ready to place the activation call the moment a CS-registered (home or roaming) window exists, which this environment does not provide. **No bearer and no voice call are reachable here.** (Open item for the user: physically confirm the Kyivstar card is seated — the modem reports PIN disabled, not the expected PIN-enabled state.)

---

**VERDICT 15 — TERMINAL FIRMWARE-STATE BOUNDARY: the modem refuses the available-networks scan even from a genuinely DEREGISTERED state. VERDICT 14 left open whether the scan failed only because the modem was camped. We tested that directly: after full bring-up (modem camped on foreign Vodafone-UA 25501 3G), the owner issued a clean RADIO_POWER OFF→ON cycle (recovered `BuildRadioPower`: power word [12]=1 OFF / 2 ON, len 18) to force a limited-service window, CONFIRMED the radio read off (`radio_off_confirmed=1`), turned it back on, and fired the scan ~2 s after radio-ON — in the pre-camp window, before any re-camp. Both RADIO_POWER OFF and ON ACKed `error_raw=0` (so the modem honors radio control and the cycle is fully reversible — it re-camped on 25501 afterward, CP ONLINE), but the available-networks scan (0x0706) STILL returned `error_raw=2` (RIL_E_GENERIC_FAILURE). This is the stronger claim: the refusal is NOT a "can't scan while camped" timing artifact — the modem/firmware does not service GET_AVAILABLE_NETWORKS in this build/state at all, registered or deregistered. Manual-select (0x0705) correctly did not fire (no list ⇒ home PLMN never visible). Combined with VERDICT 14 (scan + manual-select both refused while camped) and VERDICT 13 (only foreign 3G acquirable, no home LTE), the operator-reselection lever is exhausted. Bearer NOT achieved. Device known-good: CP ONLINE, config cleared (default LTE+WCDMA, automatic selection), owner `6fe99aa6`, proven backup `90f403df` intact; recovered-only bytes, no NV/EFS write, no IOCTL_POWER_OFF, no do_cp_crash (2026-10-03):**

### 1. Recovered RADIO_POWER (0x0800) power word
`ProtocolNetworkBuilder::BuildRadioPower(arg1,arg2,arg3)` → InitRequestHeader(0x800, 18); power int32 at payload[12] = `arg1 ? 2 : 1` (so **OFF = 1, ON = 2**), arg2→[16], arg3→[17] (both 0). This is an airplane-mode-style radio cycle, not IOCTL_POWER_OFF and not do_cp_crash.

### 2. Deregister-then-scan run (dereg_scan armed, manual_plmn=25506)
Timeline (elapsed from owner start): bring-up to allow-data done ~72 s; `dereg_radio_off response=yes error_raw=0` @72.7 s; `radio_off_confirmed=1` @74.3 s (radio GET read not-ON — genuinely limited-service); `dereg_radio_on response=yes error_raw=0` @75.9 s; `dereg_query_available_networks` sent @77.5 s → `scan response=yes error_raw=2`. Post-cycle the modem re-camped on `25501#` (voice REG_DENIED(3), UMTS), CP ONLINE — the cycle is reversible and left the device camped exactly as before.

### 3. Decisive verdict — terminal firmware-state boundary
The modem honors radio power control (OFF/ON both error 0) but refuses operator-control (scan 0x0706 here = GENERIC_FAILURE; manual-select 0x0705 in VERDICT 14 = GENERIC_FAILURE) in every tested state — camped, and now proven also from a confirmed deregistered/limited-service window. The home PLMN (lifecell 255-06) can be neither scanned for nor selected, and only a foreign Vodafone-UA 3G cell that CS-denies the SIM is acquirable. This is a modem/firmware-state + environmental boundary, not a SaaiOS host-side defect, and not circumventable constraint-safe. **No bearer is reachable here via host-side SIT replay.** (One recovered-but-untried alternative remains: the len-16 scan variant `BuildQueryAvailableNetwork(int)` carrying an explicit scan-mode arg.)

---

**VERDICT 14 — TERMINAL BOUNDARY: the modem refuses operator reselection (scan + manual-select) with GENERIC_FAILURE. The remaining legitimate lever a real phone uses when stuck on a foreign network is an available-networks scan + MANUAL selection of the home PLMN. We recovered both wire formats byte-for-byte from stock `libsitril.so` (efcca0d5) — GET_AVAILABLE_NETWORKS (SIT 0x0706) and SET_NETWORK_SELECTION_MANUAL (SIT 0x0705) — implemented them read-only/guarded/self-tested in the owner, and ran two controlled boots. BOTH commands were rejected by the modem with `error_raw=2` (protocol code 2 → RIL_E_GENERIC_FAILURE, identity map at 0xd7bd0), issued while the modem was already camped on a live cell (voice REG_DENIED(3) on UMTS 25501 at 57 s, before the 65 s scan) — so the earlier "scan failed only because RF wasn't ready" premise is FALSIFIED: RF was ready and the scan still failed. On the same dispatch path, set_preferred_lte_wcdma / set_initial_attach_apn / allow_data all ACKed error_raw=0, so the host command path is correct; the modem specifically declines operator control. The manual-select to lifecell (25506) changed nothing: serving PLMN stayed 25501 (Vodafone UA), voice REG_DENIED(3), data NOT_SEARCHING(0), tech UMTS. The home lifecell (255-06) could not even be tested for reachability because the modem will neither scan nor manually select. Bearer NOT achieved. Device known-good: CP ONLINE, config cleared (default LTE+WCDMA, automatic selection), owner `5834fd01`, proven backup `90f403df` intact; recovered-only bytes, no NV/EFS write, nothing invented (2026-10-03):**

### 1. Recovered 0x0706 (GET_AVAILABLE_NETWORKS) layout
- Request: header-only, len 12 (`ProtocolNetworkBuilder::BuildQueryAvailableNetwork()` → InitRequestHeader(0x706, 12)). (A len-16 variant `BuildQueryAvailableNetwork(int)` also exists, writing a scan-mode/RAT int at [12]; untried.)
- Response (`ProtocolNetAvailableNetworkAdapter::{GetCount,GetNetwork}`, frame-relative): count = int32 at payload[12]; per-PLMN list at payload[16], 14-byte stride — `[0..3]`=RAT raw, `[4..9]`=PLMN numeric ASCII (`[9]`=='#' ⇒ 2-digit MNC), `[10..13]`=status (1=available, 2=current, 3=forbidden). Scan-RAT remap (raw 0x11..0x15 → {0,17,15,14,20} via .rodata@0xd8ac8; others pass through, 3=UMTS, 16=GSM).

### 2. Recovered 0x0705 (SET_NETWORK_SELECTION_MANUAL) layout
`ProtocolNetworkBuilder::BuildSetNetworkSelectionManual(int,char const*)`: total frame len 22; RAT int32 at payload+0 (**0 = any**; the builder maps an out-of-range RIL type to 0, letting the modem pick the RAT); PLMN numeric ASCII at payload+4 (5 or 6 chars, stock presets +21 to '#'). For lifecell → `"25506#"`. Nothing guessed.

### 3. Boot A — available-networks scan (do_scan armed)
`camp_scan=armed`; modem camped at 57 s (voice REG_DENIED(3), UMTS, 25501); scan sent at 65 s → `query_available_networks response=yes error_raw=2` (GENERIC_FAILURE). No async result frame ever followed. RF demonstrably ready, scan still refused.

### 4. Boot B — manual-select to home lifecell (manual_plmn=25506)
`camp_manual_plmn=25506`; `set_network_selection_manual response=yes error_raw=2` (GENERIC_FAILURE). Same-path SETs OK: preferred(0), initial_attach_apn(0), allow_data(0). Post-attempt state unchanged: operator still `25501#`, voice REG_DENIED(3), data NOT_SEARCHING(0), tech UMTS.

### 5. Decisive verdict — terminal boundary (modem refuses operator control)
Every host-side SIT step the modem will service is accepted, but the two commands that would move it off the foreign cell — scan (0x0706) and manual-select (0x0705) — are both refused with GENERIC_FAILURE while camped. The home PLMN (lifecell 255-06) could not be confirmed reachable (no scan list) nor selected (manual-select refused), and the modem stays on a foreign Vodafone-UA 3G cell that CS-denies the SIM. This is a modem/firmware-state + environmental boundary (the CP's automatic engine has locked the only acquirable cell and declines reselection via SIT), not a SaaiOS host-side defect, and not circumventable constraint-safe (forcing selection would require NV/band work we must not do). **No bearer is reachable here via host-side SIT replay.**

---

**VERDICT 13 — TERMINAL BOUNDARY: foreign 3G PLMN + no LTE acquisition. We queried the serving operator (0x0702, read-only) and forced LTE-only (recovered RAT value), each one controlled boot. Serving PLMN = 25501 (MCC 255 Ukraine, MNC 01 = Vodafone Ukraine) on UMTS/3G. The SIM's home is lifecell (255-06, the "Life"/internet APN), so the modem is camped on a FOREIGN Ukrainian 3G network where it is CS-denied. Forcing preferred RAT = LTE-only (SIT value 0x0b, ACKed error_raw=0) did NOT make the CP acquire any LTE cell — across the whole settle window it reported tech=UMTS(3) 36x and none(0) 2x, zero LTE(14) samples — it stayed on the denied Vodafone 3G cell. So host-side SIT replay cannot reach a bearer: the home (lifecell) LTE is not acquired (its RF/band enablement lives in NV/RF-cal we must not modify, and/or there is no reachable home LTE at this location), leaving only a foreign 3G cell that denies the SIM. This is an environmental/subscription + RF-calibration reality, not a SaaiOS host-side defect. Bearer NOT achieved. Device known-good: CP ONLINE, default LTE+WCDMA restored, owner 2a4e07ed; read-only + one recovered RAT SET, no NV, nothing invented (2026-10-03):**

### 1. Recovered 0x0702 (GET_OPERATOR) layout
- Request: header-only, len 12 (`ProtocolNetworkBuilder::BuildOperator` → InitRequestHeader(0x702, 12)).
- Response (`ProtocolNetOperatorAdapter::Init`, frame-relative): **PLMN numeric MCC/MNC = 6 ASCII bytes at payload offset 12..17** (trailing `#`/0x23 ⇒ 2-digit MNC); short name at [18..], long name at [50..]. Only the numeric PLMN is logged (names can carry branding/PII-ish text; MCC/MNC is sufficient and not a subscriber id).

### 2. Recovered LTE-only RAT value
`ProtocolNetworkBuilder::BuildSetPreferredNetworkType` maps the RIL preferred-type
through a `.rodata` table at 0xd8c5c and writes the result at frame+12 of the 0x070a
request. Identity in the low range: **LTE-only (RIL 11) → SIT 0x0b (11)**; LTE/WCDMA
(RIL 12) → 0x0c (12), which confirms our prior set. Nothing guessed.

### 3. Boot A — serving PLMN (default LTE+WCDMA)
`plmn_numeric=25501#` (MCC 255 / MNC 01 = Vodafone Ukraine), frame_len=119; voice
REG_DENIED(3), data NOT_SEARCHING(0), reject 0, tech UMTS(3), cell on LAC 36291.
Home is lifecell (255-06) ⇒ the camped network is FOREIGN.

### 4. Boot B — forced LTE-only
`camp_pref_target=11`, `set_preferred response=yes error_raw=0`. Result: operator
still 25501# (Vodafone UA 3G); voice REG_DENIED(3), data NOT_SEARCHING(0), reject 0;
**tech tally over the window: UMTS(3) ×36, none(0) ×2, LTE(14) ×0.** The CP never
acquired an LTE cell even when restricted to LTE-only; it remained on the foreign 3G
cell. Signal present throughout (mask_low7=2).

### 5. Decisive verdict — terminal boundary
Every host-side SIT step is accepted (SIM READY, radio on, auto-select, preferred RAT
incl. LTE-only, initial-attach APN, allow-data) yet the modem has no usable registrable
service: it can only find a foreign Vodafone-UA 3G cell that denies the lifecell SIM,
and cannot acquire home (lifecell) LTE. The missing piece is LTE RF/band acquisition,
which is governed by NV/RF-cal we must not modify (and/or genuine lack of reachable home
LTE at this location) — not a host-side defect and not forgeable constraint-safe. **No
bearer is reachable here via host-side SIT replay.** To actually get data, the realistic
paths are environmental/subscription, outside SaaiOS host code: (a) be in reachable
lifecell LTE coverage, and/or (b) the CP's LTE RF/band/NV provisioning (stock-owned) be
active. The SaaiOS host-side bring-up itself is complete and correct up to the network's
own refusal.

---

**VERDICT 12 — TRUE reject-cause decoded: it is genuinely 0, and the modem is camped on UMTS/3G only. We recovered the exact stock 0x0700/0x0701 response layout from `libsitril.so` (efcca0d5) `ProtocolNet{Voice,Data}RegStateAdapter` fixed-offset accessors, extended the owner to decode every field read-only, and captured one boot. The directive's premise — that we read `reject` at the wrong offset — is FALSIFIED: the stock adapter reads reject_cause at offset 13 (byte), exactly where our owner already read it. The reject cause is genuinely 0. The new decode shows the modem DOES see and camp on a real cell, but only on UMTS (3G): RAT=UMTS(3), LAC=36291, CID=85793345 (0x051D1A41 → RNC 1309 / cell 6721), PSC=187 (a WCDMA scrambling code) — identical in both the voice and data frames. CS voice = REG_DENIED(3) on that 3G cell with NO cause; PS data = NOT_SEARCHING(0) on the same cell. This is a RAT/coverage situation, not a cause-coded auth reject. Device known-good: CP ONLINE, owner `69d3d1c2`; read-only, no NV/EFS write, nothing invented (2026-10-03):**

### 1. Decoded 0x0700 (voice/CS) / 0x0701 (data/PS) response layout (field → offset)
Recovered from the stock fixed-offset accessors. Offsets are frame-relative (same
base as the 12-byte SIT header; `[2..3]`=opcode). Voice and data share
reg_state/reject; data inserts `MaxSDC` at [14], shifting the rest by one byte.

| field | voice 0x0700 | data 0x0701 | width |
|---|---|---|---|
| reg_state | 12 | 12 | u8 |
| **reject_cause** | **13** | **13** | **u8** |
| MaxSDC | — | 14 | u8 |
| radio_tech (raw; mapped via .rodata@0xd8afc, idx=raw-1) | 14 | 15 | u8 |
| LAC | 15 | 16 | u16 |
| cell_id | 19 | 20 | u32 |
| PSC | 23 | 24 | u8 |
| ECI (data) / StationLong (voice) | 52 | 33 | u32 |
| data: CSGID/TADV/ImsVops/EmcService | — | 37/41/45/46 | u32/u32/u8/u8 |
| voice: ConCurrent/SystemId/NetworkId/RoamingInd | 56/57/59/61 | — | u8/u16/u16/u8 |

RAT map (idx = raw-1): 1→GPRS 2→EDGE **3→UMTS** … 14→LTE 16→GSM 17→TD-SCDMA 20→NR.
Serving PLMN (MCC/MNC) is **not** in the reg-state frame — it is carried by the
operator response (0x0702), which the owner does not currently query.

### 2. Live one-boot decode (owner 69d3d1c2)
Full stock sequence ACKed (all error_raw=0): get_radio(raw 10 on), get_selection
(mode 0 auto), get_preferred(raw 16), set_preferred_lte_wcdma, set_initial_attach_apn,
allow_data. Registration readback (12 cycles, stable):
```
field=data  registration_raw=0 reject_raw=0 tech_raw=3 rat_mapped=3 lac=36291 cid=85793345 psc=187 frame_len=86
field=voice registration_raw=3 reject_raw=0 tech_raw=3 rat_mapped=3 lac=36291 cid=85793345 psc=187 frame_len=88
```
Signal present (mask_low7=2). So: CS = DENIED(3), PS = NOT_SEARCHING(0), **reject = 0
(confirmed at the correct offset)**, camped on a single UMTS(3) cell.

### 3. Decisive classification
- **Not a misread.** reject_cause lives at offset 13; it is genuinely 0. The CP is
  not returning any EMM/GMM cause (#3/#6/#7/#11/#13/#15 etc.).
- **Not a "no suitable cells" / empty-scan case either.** The modem reports a real
  serving cell (nonzero LAC/CID/PSC) on UMTS.
- **The real signal: the modem can only find/use UMTS (3G), and CS is actively
  denied there with no cause, while PS declines to search** — despite us setting
  automatic selection and LTE+WCDMA preferred RAT (the modem still reports UMTS).
  This is a RAT/coverage situation: either no permitted LTE cell is reachable for
  this SIM in this RF environment, or the RAM-handover boot leaves the CP's LTE
  RF/band state (which lives in NV we will not touch) unable to acquire LTE, so it
  falls back to a 3G cell it is not authorized on.

### 4. Honest boundary assessment + precise next step
Host-side SIT replay has now driven the CP through the entire accepted stock
bring-up and we can read the true network verdict: **camped on UMTS, CS-denied
without cause, PS idle.** This is at or very near the lawful-interoperability
boundary — the remaining variable is RAT/RF acquisition, and the LTE RF/band state
the CP needs lives in NV/RF-cal which we must not modify. Two constraint-safe reads
would make the verdict airtight (each is one read-only GET / one boot, no NV, no
forging): (a) **query the serving operator 0x0702** (read-only) to confirm whether
the camped PLMN is this SIM's home PLMN (home-denied ⇒ terminal auth boundary) or a
foreign 3G PLMN (⇒ the SIM's real service is LTE we are not acquiring); (b) **set
preferred RAT = LTE-only** (same 0x070a builder, recovered value) for one boot to
test whether any LTE cell is reachable at all. If LTE is reachable and attaches →
chase the bearer; if not, this is the terminal boundary for constraint-safe
host-side bring-up.

---

**VERDICT 11 — SET_INITIAL_ATTACH_APN recovered + replayed (accepted) but NOT the gate; the full stock host attach sequence is now replayed and every SIT SET is accepted, yet the modem still does not register. We recovered the stock rild→libsitril attach chain from the factory `libsitril.so` (efcca0d5), implemented the one missing command (`SIT_SET_INITIAL_ATTACH_APN` = opcode `0x0603`, 250-byte body) in the owner, self-tested it byte-exact, and live-tested one boot. The modem ACKed it with `error_raw=0`, but PS data stayed `registration_raw=0` (NOT_SEARCHING, tech_raw=3/UMTS) and CS voice stayed `registration_raw=3` (REG_DENIED), `reject_raw=0` — identical to before. So the attach-APN precondition is accepted but is not the blocker. Device left known-good: CP ONLINE, self-tested owner (`bb9398f2`); READ-ONLY throughout, no NV/EFS write, nothing invented (2026-10-03):**

### 1. Recovered stock attach sequence (opcode + body + order)
From `NetworkService`/`PsService` + `ProtocolNetworkBuilder`/`ProtocolPsBuilder` in
`libsitril.so`, every builder routes through `ProtocolBuilder::InitRequestHeader(hdr,
opcode, len)` then `ModemData`. Wire IDs (recovered, not guessed):

| RIL request | SIT opcode | len | notes |
|---|---|---|---|
| RADIO_POWER (on) | `0x0800` | 18 | == our camp-power; `[12]=2` on |
| QUERY_NETWORK_SELECTION_MODE | `0x0703` | 12 | header-only |
| SET_NETWORK_SELECTION_AUTOMATIC | `0x0704` | 12 | header-only |
| SET_PREFERRED_NETWORK_TYPE | `0x070a` | 16 | RAT int32 @ +12 |
| **SET_INITIAL_ATTACH_APN** | **`0x0603`** | **250** | `sit_pdp_set_initial_attach_apn_req` |
| SET_DATA_PROFILE | `0x06xx` | var | preload (optional) |
| ALLOW_DATA | `0x0710` | 13 | `[12]=1`; PS-attach trigger |
| DETACH | `0x0608` | 13 | |
| (DATA/VOICE)_REGISTRATION_STATE | `0x0701`/`0x0700` | 12 | poll |
| GET_PS_SERVICE | `0x0711` | 12 | |

`0x0603` body (frame-relative, from `BuildSetInitialAttachApn`+`FillApnInfo`): `[12]`=attach
pdp cid, `[13]`=`0x0e` (fixed), `[14]`=dataProfileId, `[15]`=apnType, `[16..115]`=APN
string (strlcpy 100), `[117..165]`=username, `[167..215]`=password, `[217]`=authType,
`[218]`=pdpType, `[219]`=pcscfReqType. For a plain IP APN ("internet", no auth) every
enum byte is a recovered constant: `GetPdpType("IP")=1`, `ConvertAuthTypeToProtocolAuthType(0)=0`,
default profile/apnType/pcscf = 0; attach cid = `RetrieveAttachPdpContext` profile-base+1
(base 0 ⇒ 1). Stock order: SET_INITIAL_ATTACH_APN precedes ALLOW_DATA (the attach trigger).

### 2. Delta vs our owner — what was missing
Our owner already issued stage-1 (`0x093f`/`0x0404`/`0x0800`), radio-on, auto-select
(`0x0704`), preferred RAT (`0x070a`), and ALLOW_DATA (`0x0710`). The one missing stock
command was **`SIT_SET_INITIAL_ATTACH_APN` (`0x0603`)** — the attach-APN precondition.
Implemented as `make_initial_attach_apn_request` (guarded, `_Static_assert`'d, byte-exact
self-test 165–172, `-Werror`), inserted between the preferred-RAT SET and ALLOW_DATA.

### 3. Live result — accepted, but registration unchanged
One controlled boot. `camp_apn=loaded len=8`; sequence all `error_raw=0`:
`set_preferred_lte_wcdma` → **`set_initial_attach_apn` response=yes error_raw=0** →
`allow_data` response=yes error_raw=0. Registration readback (unchanged from V9/V10):
data `registration_raw=0` (NOT_SEARCHING) `tech_raw=3` `reject_raw=0`; voice
`registration_raw=3` (REG_DENIED) `reject_raw=0`. Signal present (`mask_low7=2`), CP ONLINE.

### 4. Interpretation — the gate is not a host SIT command
The modem now accepts the COMPLETE stock bring-up (radio on → auto PLMN select →
preferred RAT → initial-attach APN → allow data), every command ACKed without error, yet
it does not register. **This falsifies "missing host attach command" as the blocker.** The
operative signal is CS voice REG_DENIED(3): the modem does attempt CS registration and is
denied; PS never leaves NOT_SEARCHING. `reject_raw=0` is read at a fixed offset and may not
be the true EMM/GMM reject cause.

### 5. Single best remaining hypothesis + honest boundary
Every host-side SIT SET is accepted, so the blocker lives at the network-registration /
attach-accept layer, not the host command layer. **Best next step (read-only, no new SITs,
one boot):** decode the FULL voice/data registration-state response (`0x0700`/`0x0701`) per
`ProtocolNetworkBuilder`/`OnVoiceRegistrationStateDone` field layout to recover the TRUE
reject cause + detailed reg sub-state. A concrete EMM/GMM cause would decide it: a
network-authorization denial (PLMN-not-allowed #11, illegal-ME #6, auth-failure) is the
lawful-interop boundary and cannot be worked around constraint-safe; "no suitable cells"
#15 / RAT would leave a band/RAT lever. **Honest assessment:** it is now likely that
host-side SIT replay ALONE cannot force registration — the full accepted stock sequence
does not move it — so either the denial is genuinely network-side (boundary), or it needs a
component we can't run constraint-safe (full Android telephony/IMS stack); the reject-cause
decode is the cheapest way to tell which.

---

**VERDICT 10 — CP NORMAL-NV SELF-DOWNGRADE FALSIFIED (read-only capture + structural diff). We extended the quarantine owner to capture the full ~476 KB handle-1 normal-NV write-out the CP emits during init (seq-matched grant; captured intact, 476454 bytes, `NORMAL_CAPTURE done received=476454 grants=237`) and diffed it against the fed-in `nv_normal.bin` (byte-identical to real sda5 EFS per V7). Result: the entire static NV config body is BYTE-IDENTICAL (4000 random samples across offsets ~1854→108784 and 108786→476439: zero mismatches). The only changes are write-generation bookkeeping: two header counters (off8 `07→10`, off24 `6b→6c`), one 9-byte record that grew with a CP-written timestamp/binary value (off790, before the `FKPSUZ` catalog token), a handful of single/double checksum bytes beside NV catalog strings (`GT-B3730 Ver 7.0`, `[-ALL-]`, `Specific #`), one 2-byte checksum before the `SINDX1` record (off108784 `3c19→bd0e`), and ~47 KB of trailing flash content the CP doesn't persist. NO op-mode / UE_OPERATION_MODE, service-domain, limited-service/emergency-only, PLMN-sel, RAT/band, GCFMODE, or attach field changed. The CP does NOT self-downgrade normal-NV at runtime — same as protected-NV (V7, only a +4 write-gen counter). With op-mode NV (V7), secure-boot/REQ_SECURITY (V9), and now CP NV self-downgrade (V10) all falsified, the deny is NOT a CP config/NV decision. Device left known-good: proven owner restored on disk (`90f403df`), CP ONLINE; READ-ONLY throughout (no NV/EFS write, nothing forged) (2026-10-03):**

### 1. What we did (read-only normal-NV capture)
The proven owner captures only the protected-NV (handle-3) write-out; handle-1 (normal-NV)
was observed but discarded. We added a guarded, self-tested, `-Werror`-clean
`SAAIOS_RFS_NORMAL_CAPTURE` block to `modem-rfs-full-quarantine-owner.c` that answers the
CP's handle-1 open/grant-request and streams the chunks into a quarantine-only
`normal-candidate.bin` (never the real EFS/sda5/nv_normal; no payload bytes logged). The
default build is unchanged and byte-identical to the deployed proven owner (`90f403df`);
the capture build is a separate macro variant.

### 2. The sequence-echo fix
First capture attempt got 0 bytes: the CP refused our grant with a status-6 frame
(`cmd=65538 off8=6 file=1 len=0`). RFS_TRACE showed the handle-1 grant-request carries
**sequence 2** (`w0=0x20006`) whereas the proven handle-3 grant echoes **sequence 1**.
Fix: echo the request's sequence in the grant's `w0` high-16 (`2u | (seq<<16)`). Second
boot captured the full blob: `NORMAL_CAPTURE open total=476454 seq=2`,
`NORMAL_CAPTURE done received=476454 grants=237`.

### 3. Structural diff vs fed-in `nv_normal.bin`
Raw `cmp` reported 96% bytes differing, but that is pure misalignment: a fast anchored
diff shows **476361 equal bytes** and only **10 edit regions**. The static configuration
body (offsets ~1854 through 476439, excepting one 2-byte checksum) is byte-for-byte
identical. All edits are write-generation counters, per-record checksums/CRCs, one
timestamp-shaped 9-byte value, and the non-persisted tail — classic NV bookkeeping, no
config field. No registration-relevant NV item (op-mode, service-domain, limited-service,
PLMN-sel, RAT/band, GCFMODE, attach) was altered by the CP.

### 4. Interpretation — no downgrade, no new lever
Because the CP writes back our config untouched, the local registration deny is **not** a
CP self-downgrade of NV, and the diff yields no downgraded field to re-set (so no new
constraint-safe lever from this path; op-mode RAM SETs were already ineffective). Combined
with V7 (op-mode NV) and V9 (secure-boot), the deny is not an NV/config or secure-boot
decision at all.

### 5. Device state & single best remaining hypothesis
Device left known-good: proven owner restored on disk (`90f403df`), CP ONLINE; the
`SAAIOS_RFS_NORMAL_CAPTURE` source block is inert in normal builds (build scripts don't
pass the macro) and kept as the reproducible characterization. **Shrinking possibility set:**
not op-mode NV (V7), not secure-boot (V9), not CP NV self-downgrade (V10). The CP reaches
ONLINE/SIM-READY with signal, yet PS stays `NOT_SEARCHING(0)` with `reject_cause=0` and CS
`REG_DENIED(3)`. `NOT_SEARCHING` + `reject_cause=0` means the CP is **not transmitting an
attach/registration request** — this is not a network reject, it is the CP declining to
search. **Best next hypothesis: the gate is a missing host-side control-plane (RIL/SIT)
bring-up sequence** — the stock `rild`/`libsit` steps that command the radio fully online
and trigger automatic PLMN selection / PS attach (e.g. RADIO_POWER on, set-network-selection
automatic, set preferred RAT, set attach/operator profile) that we have not replayed.
Next step: recover that SIT bring-up sequence from the stock `libsit`/`rild` binaries and
replay it (constraint-safe, no NV), then re-check CS/PS; if PS registers, bearer chase
(VerifyPin → GetPsService → SetupDataCall 0x0600 Life APN).

---

**VERDICT 9 — SECURE-BOOT / `IOCTL_REQ_SECURITY` FALSIFIED as the gate (live-tested). We implemented and issued the GENUINE handshake (ioctl `0x40106f53`, modes 2/0/1, byte-identical to stock cbd) on one controlled boot. All three were rejected by the kernel with `EINVAL` — dmesg: `cpif: bootdump_ioctl: umts_boot0: security_req is null` — so the EL3/ldfw SMC NEVER executed. Root cause: cpif's `create_link_device` installs the `security_request` handler (io-device offset 976) only when its arg2 is 0 AND a DT link-attribute bit is set; on panther's modem link config that slot is left NULL. Since SaaiOS loads the identical stock `cpif.ko` + the device's own DT, STOCK cbd hits the same NULL handler → its `REQ_SECURITY` also returns `EINVAL` and is non-fatal. REQ_SECURITY is therefore vestigial on panther and cannot be the registration gate. The real image authentication is the CP/PBL integrity check at UDL MAIN DONE, which we already pass. MAIN DONE still passed, CP reached ONLINE, registration UNCHANGED (CS REG_DENIED, PS NOT_SEARCHING). Device reverted to the proven probe; no NV/EFS write; nothing forged (2026-10-03):**

### 1. What we did (genuine handshake, one controlled boot)
Pinned the handshake first, then implemented it. The two `mode=0` params stock
passes (`ctx+0x260`/`ctx+0x28c`) are **kernel-ignored**: the kernel handler
`shmem_security_request` reads only the struct's first word (`mode`) and derives
every `__arm_smccc_smc` argument itself from `cp_shmem_get_base/size` (regions 7,8)
plus fixed SIP FIDs (`0x82001011`, `0x82000700`) — confirmed from both `cbd`
(`security()@0x12f80` builds `{mode,p2,p3,0}`, `ioctl 0x40106f53`) and `cpif.ko`
(`shmem_security_request@0xda00`). So AP params `0` yield a byte-identical SMC —
genuine, not forged. Added a guarded, self-tested (`_Static_assert` on struct size
and request number), non-fatal `PROBE_SECURITY` block to `cp-boot-probe.c` issuing
mode 2 → 0 → 1 right after the handover; built with `-Werror`; the proven rebuild is
byte-identical (`e32538e8…`) to the deployed binary, so the only change is this block.

### 2. Live result — rejected before EL3
On the controlled boot, every call logged:
`REQ_SECURITY: FAIL rc=-1 errno=22 (Invalid argument) req=0x40106f53`, and the kernel
logged `cpif: bootdump_ioctl: umts_boot0: security_req is null` three times. The ioctl
is rejected at the boot0 dispatcher: it loads a callback from io-device offset 976 and,
finding it NULL, returns `EINVAL` without calling the handler or the SMC. The boot was
otherwise healthy: MAIN stage transfer `result=0` (UDL MAIN DONE passed → genuine
signed image still trusted), CP reached **ONLINE**.

### 3. Root cause & why stock is identical
The `security_request` pointer (offset 976) is written in exactly one place in the
module — `create_link_device@0x9708`, at `0x9968` — and only when its second argument
(`w23`) is `0` and a DT-derived link-attribute bit (`w8` bit 6) is set. Both inputs
come from the kernel's modem/device-tree configuration, not from anything userspace
does. On this device's modem link config the slot is left NULL. SaaiOS uses the
**identical stock `cpif.ko`** and the device's own DT, so `create_link_device` runs
identically under stock → stock cbd's `REQ_SECURITY` hits the same NULL handler →
`EINVAL`, non-fatal. **REQ_SECURITY is vestigial on panther and is not the secure boot
that matters.** The operative authentication is the CP/PBL signature check at UDL MAIN
DONE (a one-byte MAIN patch is refused there; the stock signed MAIN passes) — which we
already satisfy. This is neither an EL3 "security check fail" (we never reached EL3) nor
a GSA-held-secret boundary.

### 4. Registration unchanged; secure-boot hypothesis falsified
Post-boot: CS voice `registration_raw=3` (REG_DENIED), PS data `registration_raw=0`
(NOT_SEARCHING), `reject_raw=0` — identical to the proven boot. The secure-boot /
`REQ_SECURITY` path is therefore **falsified** as the registration gate.

### 5. Device state & next step
Device left known-good: deployed probe restored to the proven `e32538e8…`, CP ONLINE,
owner running; no NV/EFS write, nothing forged. The guarded `PROBE_SECURITY` source
block is inert in normal builds (the build scripts do not pass `-DPROBE_SECURITY`) and
is kept as the documented, reproducible characterization of the (vestigial) handshake.
**Pinpointed next step (read-only follow-up):** extend the quarantine owner to capture
the ~476 KB normal-NV handle-1 write-out the CP emits during init and structurally diff
it against the fed-in `nv_normal.bin` for a registration-relevant field the CP downgrades
at runtime (op-mode / limited-service). This needs a small owner change + one boot.

---

**VERDICT 8 — SECURE-BOOT path characterized. Stock cbd issues `IOCTL_REQ_SECURITY` (ioctl `0x40106f53`) THREE times (mode 2/0/1) → kernel `shmem_security_request` → `__arm_smccc_smc` to EL3/ldfw; OUR boot issues NONE of them (only the handover `0x6f57`). FEASIBILITY: the genuine handshake is LEGITIMATELY REPRODUCIBLE from SaaiOS (genuine kernel→EL3 SMC with device-fused keys; we'd supply only `mode`+layout params, not secrets) — this is the EL3/ldfw path, NOT the ADR-092 GSA-mailbox boundary. Whether EL3 ACCEPTS it in our boot context is the open question, answerable only by a live in-boot issue. No forging. NV write-out diff: CP does NOT downgrade protected-NV at init. No NV/EFS write, no reboot this session (2026-10-03):**

### 1. CP security / boot-auth state (read-only)
Our probe brings the CP up with: the genuine **handover** (`IOCTL_HANDOVER_BLOCK_INFO`
= ioctl `0x6f57`, carrying the real `cpsha` signature + IMEI + CDT, 161-byte block)
+ the genuine **signed MAIN** (integrity-validated by the CP at **UDL MAIN DONE** —
a one-byte MAIN patch is rejected there, stock passes) + the real NV. The CP reaches
ONLINE, SIM READY, signal present. It does **not** reach registration
(`REG_DENIED(3)`/`NOT_SEARCHING(0)`/reject 0). We do **not** issue
`IOCTL_REQ_SECURITY`. No direct SIT "security-status" GET was found that proves the MM
layer gates registration on secure-boot state (honest limitation); the
`reject_cause=0` + `NOT_SEARCHING` pattern is *consistent with* a restricted/limited
mode but does not by itself prove it.

### 2. The stock secure-boot handshake we omit
From the factory `cbd` disassembly (pulled read-only): the security call is
`ioctl(boot_fd, 0x40106f53, &sec_req)` where `sec_req = {mode, param2, param3, 0}`
(logged `security_req: %x:%x:%x:%x`, `Request security : non-secure mode` /
`dump mode`, `ERR! IOCTL_CHECK_SECURITY fail`). `cbd`'s normal boot issues it **three
times**:
- `mode=2`, params `(0,0)` — a flag/stage check;
- `mode=0`, `param2=[cfg+0x260]`, `param3=[cfg+0x28c]` — the **main image auth**;
- `mode=1`, params from cfg — a second stage.

The kernel handler `shmem_security_request` does `copy_from_user` of the 16-byte
struct, `switch(mode 0..7)`, maps the CP shmem region (`cp_shmem_get_base`), and calls
`__arm_smccc_smc` to the EL3 secure monitor (ldfw). **Our probe performs the handover
but none of the three `REQ_SECURITY` calls**, so the EL3 secure-region authentication
is the omitted step.

### 3. Feasibility — legitimately reproducible; NOT a GSA-secret boundary a priori
- The handshake is a **genuine kernel ioctl → genuine EL3/ldfw SMC**. The actual
  cryptographic authentication is done by EL3 using **device-fused keys**; the AP side
  supplies only `mode` + CP-memory-layout params — **no secret we lack, nothing to
  forge**. Issuing it is the *genuine* handshake, exactly what stock does.
- This is the **EL3/ldfw** path (cmdline shows `fips140.load_sequential`,
  `kvm-arm.protected_modules=…`, EL3 present), **not** the GSA/Titan mailbox that
  blocked AoC in ADR-092. So the ADR-092 boundary does **not** directly transfer.
- **Open question (only a live in-boot issue resolves it):** whether EL3 *accepts* the
  SMC given SaaiOS's boot context (the CP region was set up by our handover/RAM path,
  not the stock LK/ABL secure flow). If EL3 returns `security check fail`, **that** is
  the terminal boundary and we stop — we do **not** forge/bypass it.
- Caveat on whether this is even the gate: the CP already runs the **integrity-
  validated** stock MAIN with working RF receive + SIM — evidence the image *is*
  trusted — so `REQ_SECURITY` may only map a secure DRAM region rather than gate MM.
  The hypothesis is plausible but unproven.

### 4. NV write-out diff (read-only secondary)
The owner captures only the **protected-NV** write-out (`candidate.bin`, 524288 B);
the ~476 KB **normal-NV** write-out (where op-mode lives) is observed but **not
stored**. Diffing the CP-written protected-NV vs the fed-in `nv_protected.bin`: they
are **identical except 2 bytes** (offset 20 and 189444, each **+4** — consistent with
a write-generation counter). So the CP does **not** downgrade any registration-relevant
field in protected-NV during init. A normal-NV diff would require a small read-only
owner extension to quarantine the handle-1 write-out.

### 5. Decision & pinpointed next step
A blind boot-probe change + reboot was **not** performed this session: the exact
`mode=0` params (`cfg+0x260`/`cfg+0x28c` = CP shmem base/size) are not yet pinned, and
issuing `REQ_SECURITY` with guessed params would violate "recover, don't invent" and
risk breaking the proven boot. **Next step (one controlled reboot):** (a) finish
recovering the two layout params from `cbd`'s boot config (or confirm the kernel
derives them itself via `cp_shmem_get_base`, in which case params can be 0); (b) add
the three genuine `REQ_SECURITY` calls (modes 2 → 0 → 1) to `cp-boot-probe.c` at the
cbd-matched ordering (after `START_CP_BOOTLOADER`, around the handover, before the MAIN
stage transfer), guarded/self-tested/-Werror/reproducible-hash, inventing nothing;
(c) one controlled boot → check CP security status + CS/PS registration; if PS
registers → bearer chase and verify rmnet rx/tx or IPv4. If the EL3 SMC returns
`security check fail`, record the terminal boundary.

Device unchanged: stock firmware `449eeab3…`, CP ONLINE, on the safe quarantine owner;
no NV/EFS write; bearer not established.

---

**VERDICT 7 — NV-STARVATION DISPROVEN / op-mode-NV FALSIFIED as the gate. The CP is fed its NV at boot by a BOOT-IMAGE STAGE PUSH of `NV_NORM`+`NV_PROT` that is BYTE-IDENTICAL to the real `sda5` EFS (RO verifier PASS), and it issues ZERO RFS reads across boot+42 min — so it is NOT starved of real NV. We already feed the exact real stock NV (op-mode included) that this phone registers with as stock, yet it still denies. Therefore the registration gate is NOT `SAE_UE_OPERATION_MODE` in FLASH-NV; the earlier verdict (c) is corrected, and the NV-write NO-GO is moot. No NV/EFS write (2026-10-03):**

Operator reframe: the (c) inference ("real NV op-mode is normally normal; our boot
starves the CP of it") creates a contradiction — if we fed the real normal NV the CP
would register. This session traced exactly how the CP gets NV at boot and measured
whether it is starved. It is not.

### How the CP obtains its NV / op-mode at boot — BOOT-IMAGE STAGE PUSH, not RFS
The boot probe (`cp-boot-probe.c`, `boot-b-with-verified-nv-handover`) reads the two
NV TOC stages from `/data/saaios/var/efs-copy/` and pushes them straight into the CP
over the SIT boot protocol, exactly like stock cbd:
- `NV_NORM` (TOC idx 5, `0x80000`) ← `nv_normal.bin`
- `NV_PROT` (TOC idx 6, `0x80000`) ← `nv_protected.bin`
These are sent via `sit_send_stage()` during `BOOTING`, **before** the owner attaches
to `umts_rfs0`. The CP therefore has its complete NV (op-mode is a `SAE_FLASH_*`/
SAEL3 item, which lives in these blobs) from the boot image — it does **not** read
op-mode from EFS via RFS.

### The pushed NV is byte-identical to the real EFS — RO-verified
`verify-original-efs-readonly.sh verify-read-only` → **PASS** ("original EFS matches
the four userdata files; EFS unmounted"). It mounts the real EFS partition **sda5**
(`PARTNAME=efs`, f2fs) **read-only** and `cmp`s `nv_normal.bin`/`nv_protected.bin`
against the pushed `efs-copy`. They are identical. Since we never write `sda5` and the
CP's NV backups are quarantined (never persisted to `sda5`), `sda5` still holds the
last stock-persisted, registering state — so the op-mode we feed is the stock-normal
value. (The exact op-mode bytes are not isolated — the store is name-keyed, offset
unknown — but whole-blob byte-identity to the stock-registering EFS is a strictly
stronger proof that the fed op-mode equals stock's.)

### The CP issues ZERO RFS reads — nothing to starve
The owner logs **every** RFS frame header, including a `post_terminal_rfs_drain` that
keeps reading through the RadioPower-ON / MM registration window. Across boot + 42 min
(t=60 s → 2554 s) there are **98 `note=serve` + 134 `note=post_term` frames, and every
one is a WRITE-OUT**: the CP backing up its protected-NV (handle 3, 189446 B) and its
normal-NV (handle 1, ~476 KB) *to* the AP, repeatedly. There is **not a single READ
request**. The CP never asks the AP for an EFS/NV file, so there is nothing our owner
could fail to serve. The "attach too late / miss early reads" concern is also moot:
even in the registration window (owner fully attached) there are zero reads.

### Boot is stock + clean, yet denies
Deployed firmware is **stock B** (`sha256 449eeab3…`, the non-patched accepted hash).
The probe log shows `HANDOVER_RAM_ONLY OK`, factory preamble (`c00b/c110/c11b/c11d`),
owner `READY`, `COMPLETE rc=0`, `PROBE END result=0`. The CP reaches ONLINE, SIM
READY, signal present (`mask_low7=1`), and all stage-1 commands ACK — yet voice
`REG_DENIED(3)` / data `NOT_SEARCHING(0)` / reject_cause 0.

### Verdict — (c) FALSIFIED; the gate is elsewhere
We feed the CP stock firmware + the exact real stock NV (op-mode normal), the CP never
reads NV via RFS, and it still denies. So **`SAE_UE_OPERATION_MODE` in FLASH-NV is not
the registration gate**, and **the operator-gated NV write (VERDICTs 5/6) would not
have helped** — it is moot. This is a correction of the earlier (c) conclusion.

### Pinpointed next step — identify the actual MM gate (NOT NV)
The remaining divergences from a true stock boot are narrow and non-NV:
1. **Secure-boot / authentication path (leading candidate).** We boot via
   `HANDOVER_RAM_ONLY` (ioctl `0x6f57`) + factory preamble and **never issue
   `IOCTL_REQ_SECURITY`**; stock boots under GSA-backed secure boot. The CP accepts
   our handover to ONLINE/SIM-READY, but the MM layer may hold registration in a
   restricted/unauthenticated-boot mode (analogous to the GSA authentication boundary
   documented in ADR-092 for AoC). This is unproven and needs a dedicated trace of the
   CP's MM state / security status (e.g., an EngMode/security-status SIT query), not an
   NV change.
2. **CP self-modified NV diff.** The CP writes a ~476 KB normal-NV *out* that differs
   in size/layout from the 512 KB blob we fed in; a structural diff of the written-out
   vs fed-in normal-NV could reveal whether the CP itself downgrades a
   registration-relevant field during init — a read-only diagnostic, separate from the
   NV-write boundary.

No stock stack run, no NV/EFS write; device on the safe quarantine owner, CP ONLINE;
bearer not established.

---

**VERDICT 6 — STOCK-REGISTRATION CAPTURE: a live stock-stack registration capture is NOT feasible under SaaiOS (no telephony framework to drive rild). But the attempt pinned the answer: our owner already has FULL command-parity with stock's known stage-1 sequence (`0x093f`→`0x0404`→`0x0800`, all ACK clean, signal present) and still denies, so the registration delta is NOT a missing/mis-ordered SIT command — it is CP-internal state seeded from FLASH-NV (`SAE_UE_OPERATION_MODE`). PIVOTAL VERDICT = (c). No stock stack run; no NV/EFS write (2026-10-03):**

Operator asked to watch the stock stack register live on this phone to find the
minimal delta. Feasibility was assessed end-to-end and the equivalent evidence was
obtained without running a persistent stock stack.

### Can the stock stack register live on SaaiOS? — NO (evidence)
- **No vendor userspace:** `/vendor` has only `firmware/` + `lib64/`; there is **no
  `/vendor/bin`, no `rild`, no `cbd`** on the running FS. (A `cbd` copy exists at
  `/data/saaios/var/vextract/cbd` — Android-37 dynamic arm64 ELF.)
- **Vendor binaries *can* be linked/run** under SaaiOS via a minimal bionic set
  (`/system/bin/linker64` + `libc.so` are present; proven by ADR-092 running stock
  `aocd`). But that only gets a process started.
- **`cbd` only boots the CP** — which is *already* ONLINE under our owner. It does
  not perform network registration.
- **`rild` cannot self-register:** rild/libsitril issue radio-power-on and
  network-selection SIT commands **only when driven by the Android telephony
  framework** (RIL_REQUEST_RADIO_POWER / network-select over binder, from
  `system_server`/telephony via `hwservicemanager`). SaaiOS has `servicemanager`
  only, no `hwservicemanager`, empty `/apex`, no telephony framework, no
  `/dev/socket/rild`. So rild would link and idle, never initiating registration.
- **No userspace-independent frame logger:** the `cpif` driver exposes **no**
  `dynamic_debug`/ftrace IPC tracepoints (only `google_cpm` charge-pump entries
  match); `/sys/fs/pstore` is empty. There is no in-kernel way to capture stock SIT
  frames without the HAL running.
- **Conclusion:** the only way to make the stock stack *register* is to boot full
  stock Android — which removes our COM13 root shell + SIT/RFS capture harness, and
  a persistent cbd/rild is forbidden. A literal "run stock once, capture a live
  registration" on SaaiOS is therefore not executable.

### Equivalent evidence obtained — command parity + live baseline
Our quarantine owner (`modem-rfs-camp-combined-owner`, PID 568) already issues the
**complete known stock stage-1 trio** on the exact `0x0803`→`0x0802`-raw-0 radio
edge, and all three ACK clean in the live boot:
```
camp_ack cmd=0x0404 response=yes error_raw=0   (SGC europen 0x0101,0,0)
camp_ack cmd=0x0800 response=yes error_raw=0   (RadioPower ON)
camp_ack cmd=0x093f response=yes error_raw=0
camp_probe field=signal mask_low7=1            (signal present)
camp_probe field=voice registration_raw=3 reject_raw=0   (REG_DENIED)
camp_probe field=data  registration_raw=0 reject_raw=0 tech_raw=2 (NOT_SEARCHING)
```

### Pivotal verdict — (c), with the evidence that rules out (a) and (b)
- **(a) "stock issues a command we don't" — RULED OUT.** We have full parity with
  stock's known stage-1 sequence, and the constraint-safe AP→CP command surface was
  already exhaustively enumerated and live-tested (`0x070a` pref-net, `0x0710`
  allow-data, `0x072B` dual-net+allow, op-mode SETs `0x091a`/`0x0933`/`0x080f`/
  `0x0956`) — none move registration (VERDICTs 3–4).
- **(b) "we skip a stock RAM-state init of op-mode" — RULED OUT as a replicable fix.**
  The RAM op-mode SETs ACK `error_raw=0` **and** the GET counterparts already report
  the *target* values (voice=3, stack=1, device_service=1) — i.e., the modem's RAM
  op-mode is already "normal" — yet registration does not change. So the CP's
  registration gate does **not** consult the RAM op-mode we can set; a RAM init
  cannot fix it.
- **(c) "stock relies on the CP already having a 'normal' op-mode in FLASH-NV" —
  STANDING CONCLUSION.** With command parity achieved, signal present, and RAM
  op-mode already normal, the only remaining divergence is below the AP→CP command
  surface: CP-internal state seeded from FLASH-NV `SAE_UE_OPERATION_MODE`. That the
  stock stack *does* register on this exact phone confirms the NV value is normally
  "normal" — our boot path leaves/has it wrong, and there is no command to correct
  it in RAM.

### Minimal delta
A **single FLASH-NV field** (`SAE_UE_OPERATION_MODE`) — i.e., exactly the
operator-gated NV write characterized in VERDICT 5 (NO-GO). There is **no
constraint-safe command or RAM-state init** that substitutes for it (proven by the
op-mode SET no-op). Nothing constraint-safe remains to implement.

### State / bearer
No stock stack was run (nothing to stop). Device left on the safe quarantine owner,
CP ONLINE. **Bearer NOT established** (data `NOT_SEARCHING`, no rmnet rx/tx) — goal
correctly not marked complete.

### Pinpointed next step (operator decision)
Two mutually exclusive avenues remain, both previously flagged:
1. **The operator-gated NV write** of `SAE_UE_OPERATION_MODE` (VERDICT 5): still
   NO-GO as characterized (offset not statically pinnable, validator unconfirmed, no
   proven AP-side propagation). The tested backup/revert harness is ready if the
   unknowns are resolved.
2. **An instrumented stock-Android boot** (outside SaaiOS) purely to *observe* the
   CP's op-mode NV handshake during a real registration — our tooling performing no
   NV/EFS write. This is the only way to get genuinely new live evidence, but it is
   out of the SaaiOS bring-up scope and sacrifices our capture harness.

---

**VERDICT 5 — NV de-risk (READ-ONLY): the `SAE_UE_OPERATION_MODE` gate is a NAME-KEYED CP NV item with a runtime-built layout; its on-disk byte offset is NOT statically determinable, the CP integrity validator is UNCONFIRMED, and there is NO confirmed AP-side write path that propagates to the CP. A safety harness was built + tested on COPIES. RECOMMENDATION: NO-GO for any NV write. No NV/EFS write performed (2026-10-02 de-risk):**

Per operator decision, this session only characterizes the hypothetical NV write
and builds a tested, reversible safety harness — then stops for go/no-go. All work
was read-only w.r.t. NV/EFS.

### 1. NV record location — NAME-KEYED, offset not statically determinable
In the live CP image (`factory-cp2a.260705.006-modem.img`) the gate and its
siblings exist as **named** SAE-L3 flash NV items, each with three aliases:
`!SAEL3.SAE_UE_OPERATION_MODE`, `!SAEL3_DS.SAE_UE_OPERATION_MODE`,
`SAE_FLASH_UE_OPERATION_MODE`; siblings `SAE_FLASH_GCFMODE`,
`SAE_FLASH_PLMN_SEL_MODE`, plus the wider family (`SAECOMM_FLASH_LTE_SET_PS_MODE_2`,
`…INTERNET_ATTACH_ENABLED`, `…VOLTE_CAPA`, `MOBILE_CLASS_MODE`, …). The accessor
is Thumb-2 code (≈ file offset `0x3DBF0xx`) that loads each item **by its name
string** (`movw/movt` → name) through the SAECOMM/SAEL3 NV manager. Consequences:
- The store is **name-keyed**; the physical slot for an item is assigned by the CP
  NV manager at runtime. There is **no static record-id→byte-offset table** in the
  image to validate against.
- The on-disk protected-NV blob (the quarantine `candidate.bin`, 512 KB) contains
  **none** of these name keys and no plaintext of the item (grep count = 0 for
  `OPERATION_MODE`/`SAEL3`/`GCFMODE`/`PLMN_SEL`/`SAE_FLASH`). So the target bytes
  **cannot be located** in the blob without the CP's runtime name→offset map.
- **Current vs target value: UNRESOLVED read-only.** The value is CP-internal (read
  via SAECOMM/SAEL3 accessors, not AP-served) and is not in plaintext in the blob.
  The enum family (`…PS_MODE_2`, `LTE_SET_PS_MODE_2`) indicates UE-operation-mode =
  3GPP CS/PS-mode-1|2 / PS-mode-1|2; "normal online" is a CS/PS-combined value, but
  the exact integer mapping and the live current value were **not** confirmable
  read-only this session.

### 2. Integrity / validation — UNCONFIRMED (the dominant risk)
The blob is **structured plaintext flash, not encrypted** (overall entropy 3.28;
81 of 128 4 KB blocks are `0xFF` erased flash; only 1 high-entropy block). Its
header has size-like words (`0x1e82e0` repeated at words 0 and 3) and a plausible
checksum word (`0x847886`), but the **exact CP validator is unconfirmed** —
algorithm, covered byte range, per-record vs per-blob, and crucially whether the
CP **recomputes on read** or **rejects a mismatched blob** (brick vs revert-to-
default). This logic lives inside the name-keyed NV manager and was not fully
reverse-engineered. Without it, any edited blob risks rejection → modem brick or a
full NV reset. **This is the single biggest blocker.**

### 3. Write path — no confirmed AP-side path that reaches the CP
The CP **writes its NV OUT** to the AP over RFS (observed); in the quarantine-owner
setup the AP does **not** serve NV back, and prior instrumentation saw **zero CP
read-expecting requests** in the post-write/MM-gate window. The authoritative NV is
**CP-side**. Therefore:
- Editing the AP-side quarantine/backup file is **not confirmed to propagate** to
  the CP (it is a backup the CP writes, not a source it reads).
- A real persist would require writing the CP's own NV region (the `sda5` /
  `nv_protected` EFS partition) — which is on the hard-constraint **NEVER-write**
  list, is not currently mounted, and has no `/dev/block` node exposed.
- **No confirmed non-EFS-RW injection path exists.**

### 4. Backup / revert / dry-run harness — BUILT + TESTED on copies only
- Host `nv-edit-harness.py` — `selftest` **PASS**: full backup+sha, narrow byte
  backup, dry-run (old→new without writing, source proven untouched), apply-to-copy
  (diff shows exactly the changed byte), restore round-trip (sha matches original).
- On-device `nv-backup.sh` — read-only full backup + sha; validated against the
  quarantine copy: backup sha `a26462bd…` equals the source sha and the source was
  untouched.
- On-device `nv-revert.sh` — destructive restore, **guarded**: refuses unless
  `SAAIOS_NV_REVERT_CONFIRM=yes` and the backup's recorded sha matches (refusal
  verified, exit 3, no action).
- Dry-run's checksum is a **CRC32 PLACEHOLDER** (clearly labeled) to exercise the
  mechanism; it is NOT the CP validator and must be replaced with the recovered
  algorithm before any write is considered. No secrets are printed by any tool.

### 5. Go / No-Go
**RECOMMENDATION: NO-GO for an NV write.** Three preconditions for a *safe,
reversible* edit are unmet: (1) the target byte offset is not pinnable (runtime
name-keyed layout; blob has no keys); (2) the CP integrity validator is unconfirmed
(brick/NV-reset risk); (3) no confirmed AP-side write path reaches the CP (the
authoritative store is CP-side; the only persist route is the forbidden modem NV
partition). The current/target enum values were also not confirmable read-only.
The harness is ready for the day (1)–(3) are resolved.

*If ever authorized AND after resolving (1)–(3):* the minimal write would be to set
`SAE_FLASH_UE_OPERATION_MODE` to the CS/PS-combined "normal" value (and confirm
`SAE_FLASH_GCFMODE` is disabled), preceded by a full `nv-backup.sh` of the exact
partition and a dry-run using the *recovered* checksum. **Awaiting explicit operator
go/no-go — no NV write performed.**

Device unchanged: CP ONLINE, SIM READY, data `NOT_SEARCHING(0)`, voice
`REG_DENIED(3)`.

---

**VERDICT 4 — the LAST command-only lever (`0x072B` dual-network + allow-data) was recovered from the full `.so`, confirmed command-safe, and live-tested: it ACKs clean but is INEFFECTIVE. COMMAND-ONLY AVENUE IS NOW FULLY EXHAUSTED. The only remaining path to registration is a scoped FLASH-NV write of `SAE_UE_OPERATION_MODE` — operator-gated, NOT authorized, NOT performed (2026-10-02 final):**

*Recovery (from `ProtocolNetworkBuilder::BuildSetDualNetworkAndAllowData` @ `0x2375a0`, decoded, not guessed):* `0x072B` is a **28-byte** frame — 12-byte header + **4 × int32** payload:

| offset | field | source | value sent |
|---|---|---|---|
| +12 | primary net type | `translateNetworktype(GetInt0)` | `12` (LTE/WCDMA wire) |
| +16 | secondary net type | `translateNetworktype(GetInt1)` | `12` |
| +20 | primary allow-data | `GetInt2` (raw) | `1` |
| +24 | secondary allow-data | `GetInt3` (raw) | `1` |

The caller `NetworkService::DoSetDualNetworkTypeAndAllowData` @ `0x19fe00` reads
those 4 ints from the RIL request and its log string is literally
`"Dual Network Type : Primary(%d,%d), Secondary(%d,%d)"`. `translateNetworktype`
@ `0x236790` is the **identical** table `BuildSetPreferredNetworkType (0x070a)`
uses, and `translate(12)=12`, so all four field values are **reused proven wire
values** (net type `12` = our already-applied `0x070a`; allow-data `1` = our
already-applied `0x0710`). There is **no GET counterpart** (no `GetDualNetwork`
symbol exists).

*Constraint-safety (confirmed):* the caller ends in
`Service::SendRequest(ModemData, 0x7530, 0xff3, …)` @ `0x1a0080` — an IPC command
with a 30 s timeout and response id `0xff3`. **No `NvWrite` path**; `BuildNvWriteItem`
remains a no-op stub. So `0x072B` is operational/RAM state, **not** a FLASH-NV
persist. Functionally it is the **union of `0x070a` + `0x0710`** across a
primary+secondary stack — and both of those were already tried live and are
ineffective on this single-stack device.

*Live result (owner `90f403df`, step=`dual`, one guarded boot):* GETs first read
stack_status=1, voice_operation=3, device_service=1 (unchanged from prior boots);
`0x072B` SET **acked `error_raw=0`** (accepted, not rejected by the dual-SIM /
stack-occupy guards). Across the settle window: voice `registration_raw=3`
(REG_DENIED) / `reject_raw=0`; data `registration_raw=0` (NOT_SEARCHING) /
`tech_raw=3`; `mask_low7=2` (UMTS). **No `rmnet` interface obtained an IPv4**
(only `lo`/`wlan0`/`usb0` have addresses). No movement whatsoever.

**→ COMMAND-ONLY AVENUE FULLY EXHAUSTED.** Every constraint-safe AP→CP operational
command has now been issued live and ACKed, and the operational GETs prove the
modem is already in the target state (voice-op on, stack on, pref RAT LTE/WCDMA,
allow-data on, dual-network+allow-data accepted). The registration denial is a
**CP-internal MM decision** (REG_DENIED with `reject_cause=0`, PS never searches),
not a missing command.

## Operator decision — the only remaining path is a scoped FLASH-NV write (NOT authorized)

- **NV item(s):** `SAE_UE_OPERATION_MODE` is the governing gate, persisted CP-side
  as `SAE_FLASH_UE_OPERATION_MODE`. Siblings in the same CP FLASH-NV store that
  shape the pre-PLMN MM gate: `SAE_FLASH_GCFMODE`, `SAE_FLASH_PLMN_SEL_MODE`, and
  the RF-cal `CalDone` flag. These are read via the CP's **internal** registry
  accessors (`MMC_GET/SET`, `PlmnSimDataAcc`), **not** over RFS/AP — so there is
  no AP-served read to divert and no SIT command that writes them (the RIL's
  nominal `BuildNvWriteItem` is a no-op stub in this build, confirming the AP RIL
  cannot write them either).
- **Write path:** the value lives in the modem's FLASH-NV / EFS region on the CP
  side (the `nv_protected` / modem NV area), mutated only by CP-internal MM/NV
  code during provisioning. Reaching it from the AP means a direct, offline edit
  of that NV region's bytes.
- **What a scoped, reversible edit would entail (design only, NOT done):**
  1. Identify the exact NV/EFS record + byte offset for `SAE_FLASH_UE_OPERATION_MODE`
     in the modem NV partition.
  2. **Back up** the full containing record (and a wider partition image) byte-for-
     byte before any write.
  3. Write **only** the single operation-mode value to its target.
  4. Keep the backup to restore the exact prior bytes on revert.
- **Risk / reversibility:** HIGH. The modem NV/EFS region also holds calibration,
  IMEI, and security material; a wrong offset or a CRC/signature the CP recomputes
  can brick the modem or its calibration. Reversibility depends on an exact byte
  backup **and** the CP not re-deriving/validating the record. This is firmly
  behind the hard-constraint NV/EFS boundary and requires **explicit operator
  authorization** with an accepted brick risk. **No NV/EFS write was performed.**

Device left unchanged: CP ONLINE, SIM READY, data `NOT_SEARCHING(0)`, voice
`REG_DENIED(3)`; `opx-step` cleared to GET-only.

---

**VERDICT 3 — the full `libsitril.so` was recovered and every constraint-safe operational-mode SET was issued LIVE; all ACK clean (`error_raw=0`) but NONE move registration; command-only avenue EXHAUSTED; no NV write performed (2026-10-02 latest):**
Supersedes the "wire ids not recoverable" blocker in VERDICT 2. The full
`/lib64/libsitril.so` was extracted READ-ONLY from `vendor.img` via `debugfs`
(no mount/sudo) — SHA-256 `efcca0d5…2d5b1`, an **exact match** to the operator's
expected TD1A image. The untried operational SET wire ids + body shapes were
then recovered from the builder disassembly (`InitRequestHeader` immediates), not
guessed, and cross-checked against the open `sitdef.h`. `BuildNvWriteItem` is a
no-op stub (`mov x0,xzr; ret`) and `DoOemSetPsService` just calls `BuildAllowData`
(`0x0710`), confirming these operational SETs go through `SendRequest`, **not** NV.

Each was issued live, one SET per guarded boot, from the unified owner after SIM
READY / radio ON / allow_data, with the owner holding the single `umts_ipc0`
lock. The GETs were read first each boot:

| step | SET opcode / body | GET readback before SET | SET ack | reg effect |
|---|---|---|---|---|
| voice | `0x091A` int32 `mode=3` (len16) | voice_operation = **3** (already enabled) | `error_raw=0` | none |
| intps | `0x0933` int32 `mode=1` (len16) | — (deprecated in sitdef) | `error_raw=0` (**not** `2`, so live/accepted, not removed) | none |
| stack | `0x080F` byte `mode=1` (len13) | stack_status = **1** (already enabled) | `error_raw=0` | none |
| devsvc | `0x0956` int32 `mode=2` data-centric (len16) | device_service = **1** (voice-centric) | `error_raw=0` | none |

After every SET, across ~65s of post-SET observation: voice `registration_raw=3`
(REG_DENIED), `reject_raw=0`; data `registration_raw=0` (NOT_SEARCHING),
`tech_raw=3`; `mask_low7=2` (UMTS). CS never left REG_DENIED(3); PS never left
NOT_SEARCHING(0). The GETs prove the modem is **already** in the target
operational configuration (voice-op enabled, stack enabled), so these levers are
no-ops here — the denial originates below/outside the AP→CP operational-SET
surface, not from a missing enable command.

**Remaining opcodes are off-limits by constraint:** `0x072B`
(`SET_DUAL_NTW_AND_PS_TYPE`) body params are not fully pinned from
`ProtocolNetBuilder` (operator: DO NOT SEND until lifted); `0x0937`
(`SET_RADIO_NODE`), `RadioPower POWER_OFF(3)`, and any NV/EFS write are hard-barred.
So the command-only avenue is exhausted. Owner code hash this run `ecdf874f…`;
`/data/saaios/etc/opx-step` cleared to GET-only after the sweep. Device left CP
ONLINE, SIM READY, data `NOT_SEARCHING(0)`, voice `REG_DENIED(3)` — unchanged.

A side bug was found + fixed during this work: the camp reg sequence's
`allow_data` ACK (`0x0710`) is a short frame that the generic length guard in
`camp_probe_match` swallowed before the `REG_ALLOW_DATA` branch, so `reg_complete`
(the opx gate) never flipped. The reg SET-ack branches now precede the length
guard; `allow_data` acks `error_raw=0` and `reg_complete` fires as intended.

---

**VERDICT 2 — no constraint-safe operational-mode SET opcode was confirmed; the op-mode gate sits behind the FLASH-NV boundary; NO NV write performed (2026-10-02 later):**
Follow-up to VERDICT 1 below. Having established the gate value is CP-store
FLASH-NV, the one remaining constraint-compliant avenue was a *live SIT
operational-mode / attach-enable SET command* (a command, not an NV/EFS file
write) — analogous to the accepted camp SETs `0x0704`/`0x0800` — *if* such an
opcode exists in the vendor SIT/RIL. We hunted it. Result:
- **Untried operational SETs exist by name** in the CP string table / vendor RIL
  name table: `SIT_SET_PS_SERVICE_DOMAIN`, `SIT_SET_DEVICE_SERVICE`,
  `SIT_SET_INTPS_SERVICE`, `SIT_SET_VOICE_OPERATION`, `SIT_SET_MODEM_CONFIG`,
  `SIT_NS_NETWORK_NORMAL_START`, `SIT_SET_DUAL_NTW_AND_PS_TYPE` (plus GET
  counterparts). Their presence implies handlers exist in this build.
- **Wire opcode ids could NOT be reliably recovered** (so they cannot be issued
  without inventing bytes — forbidden). The CP dispatches SIT frames by *numeric
  id only* and does **not** code-reference the SIT name strings (handler-pointer
  xref = 0). The vendor RIL's id→name table (`…sitril-builder` @ file off
  `0x217fa4`, 557 twelve-byte `adrp/add/ret` stubs) is indexed by an **internal
  enum**, not the wire opcode; the index→wire map is a non-linear grouped map
  (idx 279→`0x800`, 175→`0x600`, 88→`0x208`) and the truncated carve contains no
  clean wire-opcode table that validates against those anchors.
- **Every wire-mappable operational SET we already know was tried live and is
  ineffective** (prior art, all ACKed, no reg change): EngMode `0x0908`, SGC
  `0x0404`, SetModemsConfig `0x093f`, RadioPower `0x0800`, net-sel `0x0704`,
  AllowData `0x0710`, pref-RAT `0x070a`.
- **The gate parameter itself is FLASH-NV.** `SAE_UE_OPERATION_MODE` is read/
  written through the CP's *internal* registry (`MMC_GET/SET`, `PlmnSimDataAcc`)
  and persisted in `SAE_FLASH_UE_OPERATION_MODE`; the untried service-SETs mutate
  *different* state (service-domain / PS-enable / voice-op), and no SIT handler
  was found that writes the op-mode parameter without a FLASH-NV persist.

**Consequence / operator decision:** no confirmed command-only path flips the
op-mode gate. The only known mutation path for the gate value is a **FLASH-NV
write**, which is out of scope without explicit operator authorization. Per the
hard constraint we **did not** write real NV/EFS. If the operator wants to keep
chasing the command avenue, the next RE step is to recover the untried SETs'
wire ids from the **full `libsitril.so`** (extractable from `vendor.img`, ext4)
or from the **live CP RX-dispatch** `cmp wire_id,#imm; beq handler` chain near
`sitRxSet…` handlers — neither of which is an NV write, but both are
multi-hour RE. See `docs/os/sprints/MODEM-07-RFS-QUARANTINE.md` for the full
hunt log. Device unchanged (owner pid 571 `69f9b62d…`, CP ONLINE, SIM READY,
data `NOT_SEARCHING(0)`, voice `REG_DENIED(3)`).

---

**VERDICT 1 — the MM registration-gate value is CP-STORE, not AP-served; we are at the real-EFS/NV constraint boundary (2026-10-02 late):**
We instrumented the owner to log every RFS frame (header fields only — cmd,
numeric file handle, offset/size counters; never payload) and, critically, kept
reading `umts_rfs0` **after** the protected-NV write-out completes so the
RadioPower-ON / MM-gate window is observed (the stock owner stopped polling RFS
at that point, so this window had never been seen). Captured across a fresh
guarded boot (owner `69f9b62d…`):

- **Early boot (write-out, file handle 3):** `cmd7`(open/unprotect, handle 3) →
  `cmd3`(stat) → `cmd6`(write, size `0x0002e406` = **189446 bytes** — note this
  field is the transfer *size*, i.e. `RFS_TRANSFER_BYTES`, **not** an NV offset,
  correcting the earlier "NV @0x02e406" wording) → 95 data chunks. The CP is the
  **data source**; its very first RFS op this boot is a write-out, so it reads
  nothing from the AP before it.
- **Post-write / RadioPower-ON / MM-gate window (6 frames):** `cmd3`(stat
  handle 3), `cmd7`(open handle 3), and four `cmd6`(**write**, file handle 1,
  size ~476 KB each) attempts. The owner does not grant the handle-1 writes.
  **Every single frame is OPEN(7)/STAT(3)/WRITE(6) — there is ZERO
  read-expecting-data request anywhere in the entire boot→gate timeline.** The
  CP never asks the AP to return file data; it only writes its own NV/EFS out
  (AP is a backup *sink*, not a source the CP reads at boot).
- Registration settled at voice `REG_DENIED(3)`/`reject_cause=0`, data
  `NOT_SEARCHING(0)`/tech 3, with SIM READY and UMTS signal — same local deny.

**Static cross-reference (CP image) confirms it:** the firmware's RFS read op
(`RfsRead`) exists but appears **only** in the USIM **PERSO/SIM-lock** path
(`[PERSO]Read RFS Fail!, RfsReadResult=%d`, `[PERSO]Open RFS Fail!`), which did
not fire this boot (SIM reached READY, PIN1 enabled). The registration/
operational-mode gate parameters are a cluster of **CP-internal FLASH-NV items**
— `SAE_FLASH_UE_OPERATION_MODE` (`!SAEL3.SAE_UE_OPERATION_MODE`),
`SAE_FLASH_GCFMODE`, `SAE_FLASH_PLMN_SEL_MODE`, `SAE_FLASH_MOBILE_CLASS_MODE`,
and RF-cal `CalDone`/`RF_CAL`/`CAL.HEDGE.NV.*` — accessed through internal NV
accessors (`NvRead`, `PlmnSimDataAcc.SimState(MMC_GET,…)`), **never via an RFS
round-trip to the AP**.

**Consequence:** no allowed read-divert can satisfy the gate, because the CP
performs no AP-served read of the gate value — it reads it from its own in-RAM
copy of the factory FLASH NV (the same ~476 KB handle-1 image + 189 KB handle-3
`nv_protected` it writes out). Changing the gate value would require writing the
CP's **real** NV/EFS (`SAE_FLASH_*` lives in handle-1 normal NV), which is
exactly the forbidden real-EFS write. **This is the quarantine-vs-real-EFS
boundary, and we have reached it on the read-divert axis.** Remaining ALLOWED
avenues are live SIT provisioning SETs (operational-mode / attach-enable), not
file diverts — see MODEM-07 for the enumerated operator options.

**MM deny is a CP-internal pre-PLMN local gate, not quarantine, not forbidden-PLMN; PCIe mitigation now baked into handoff (2026-10-02 pm):**
Static RE + live confirmation narrowed the registration blocker decisively. (1)
`0x20d1afa` ("SIT_REG") is the **SIT command-handler registrar** — called
hundreds of times as `SIT_REG(opcode_id, descriptor_ptr)` to populate the SIT
dispatch table (0x6ff, 0x701-0x707, 0x741-0x747, SIM 0x2xx, etc.); it is the
index to the handlers, **not** the deny gate. (2) `error_raw=2` is a **generic
SIT "refused/precondition-failed" code** emitted by many handlers via
`MOVS r0,#2; STRB.W r0,[obj,#0x0a]` (SIT frame error field is at +0x0a); the RF
scan (0x0706) refusal and the SIM_IO ADF-operation refusals share this one
generic code, so neither is a PLMN/card verdict — both are the CP declining
operations in its current operational state. (3) The registration denial itself
is **not** a SIT error: the voice/data registration GETs return `error_raw=0`
with `registration_raw=3`/`0` and `reject_cause=0` in the payload, and **no
serving PLMN is latched** — a genuine MM-internal **local deny that precedes
PLMN evaluation**. Forbidden-PLMN is therefore ruled out behaviorally (the CP
never gets far enough to consult a PLMN). The two-step SIM_IO
(SELECT 0xA4 then READ_BINARY) to read EFfplmn/EFad directly is **blocked**: both
SELECT and READ_BINARY return `error_raw=2`; only READ_RECORD (0xB2) on the
MF-level EFdir works (USIM AID obtained, never logged).

**Quarantine vs real-EFS boundary:** the RFS flow the owner serves is the CP's
protected-NV **write-out** — `request_6` targets NV `0x0002e406` and the CP is
the **data source** of the 189 KB stream, which the owner captures into the
quarantine candidate and ACKs. The CP keeps its NV in RAM and gets its ACK, so
the local deny is **not** caused by the quarantine diverting that write. The MM
gate reads the CP's in-RAM NV/cal/operational state that originated from the
**genuine factory EFS** (read at early boot). **Verdict on satisfiability
without a real-EFS write: not yet definitive.** The deny is a CP operational-
mode/provisioning latch (the err=2 refusal family), reached *after* the owner's
accepted SGC `0x0404` / SetModemsConfig `0x093f` / RadioPower `0x0800` /
AllowData `0x0710` — so a SIT-only provisioning step may still be missing
(allowed path) *or* the latch reads a factory NV/cal value (which, if it must
change, is satisfiable by diverting the CP's early-boot RFS **read** into its RAM
— still no real-EFS write — or, only as a last resort, a real-EFS write =
constraint boundary). **Pinpointed next step:** instrument the CP's early-boot
RFS **read** sequence (the current owner only serves the cmd7/cmd3/cmd6
write-out) to determine whether the operational-mode/cal value the MM gate reads
is served from the AP (→ allowed read-divert) or from the CP's own store
(→ real-EFS boundary).

**PCIe mitigation baked + validated end-to-end:** `owner-handoff-rfs-camp.sh`
now sets RC `power/control=on` before CP boot and launches a bounded background
`pcie-stabilize-cp.sh` (120 s) after ONLINE that re-applies RC runtime-PM + EP
L1.2 ASPM/PCI-PM disable every second across the owner's `0x0800` dispatch. On a
fresh warm reboot this came up automatically: `power/control=on`, stabilizer +
owner running, and although `pcie_send_ap2cp_irq: PCI not powered on` still logs
transiently at 0x0800, **the link recovered and IPC stayed functional** — SIM
READY(5), registration sequence ran to the same settled result (preferred=12,
voice REG_DENIED(3)/reject=0, data NOT_SEARCHING(0)/tech=3). The owner's clean
stop is SIGTERM (handled → `stop_requested` → graceful exit; deferred, not
ignored, during critical RFS/IPC sections); SIGKILL is the hard fallback.
**Bearer verified? no.**

**RAT-broaden ruled out; deny is CP-local; PCIe wedge mitigated AP-side (2026-10-02):**
The operator cold-power-cycled the phone, but that alone did **not** clear the
modem PCIe endpoint wedge — the same signature recurred on the cold boot (cpif
`pcie_send_ap2cp_irq: PCI not powered on` ×9, SIM never initialized). Root
cause is confirmed AP↔EP: after `RadioPower-ON 0x0800` the modem EP cannot
complete the PCIe L2/L3 power-down handshake (kernel `logbuffer_pcie0: cannot
receive L23_READY DLLP packet`, LTSSM stuck/flapping at Detect(0)); the link
is healthy for the first ~9 s (RFS 189 KB transfer + camp `0x0404` succeed) and
dies only after `0x0800`. **Working mitigation (AP side, no CP changes):** set
`/sys/devices/platform/11920000.pcie/power/control`=`on` (disable RC runtime
suspend) **and** disable EP L1.2 ASPM
(`.../0000:01:00.0/link/l1_2_aspm`=0, `l1_2_pcipm`=0), applied **after** the EP
enumerates — this recovers even an already-wedged link to L0 and keeps IPC
alive through `0x0800`. With that applied the SIM reached **READY(5)** and the
full registration sequence ran.

On the live, stable link the owner's registration triggers all took and were
read back with `simdiag-once` (ARM64 `9e3c8584…`): **preferred_rat=12**
(LTE_WCDMA SET confirmed, not 16), selection **0 (automatic)**, radio **10
(ON)**, AllowData sent+ACKed — yet CS voice **`registration_raw=3` REG_DENIED
`reject_cause=0`** and PS data **`registration_raw=0` NOT_SEARCHING**,
`mask_low7=2` (UMTS present). **Broadening the preferred RAT to LTE_WCDMA does
not move registration.** The registration reply payloads carry
**`reject_cause=0`**, i.e. a **local / CP-internal deny, not a network NAS
reject** (a forbidden-PLMN / roaming-not-allowed rejection would carry a
nonzero cause). Direct EF confirmation is blocked: `SIM_IO 0x0208` READ_RECORD
on MF-level EFdir works (USIM AID obtained, never logged), but **READ_BINARY on
ADF_USIM EFs (EFad 0x6FAD, EFfplmn 0x6F7B) returns `error_raw=2` across all
four selection variants** (bare / AID-inline / ADF-path / AID+ADF-path) — this
modem rejects READ_BINARY on ADF EFs, so the forbidden-PLMN list could not be
read directly. **Bearer verified? no** (PS never searched, no SetupDataCall, no
rmnet rx/tx). **Pinpointed next unmet precondition:** the CP MM
local-registration gate (`reject_cause=0` = local deny) — RE the SIT_REG gate
(`0x20d1afa`; PresentObj #636c +0xBF6); secondary: a two-step SIM_IO
(SELECT 0xA4 then READ_BINARY) or manual-PLMN attempt to force a network-side
cause. Make the PCIe mitigation durable in the handoff path.

**Early-SGC live run — accepted early but no camp/registration (2026-10-02):**
Phone shell was restored over the **USB serial console** (`COM13` root shell);
the "server 110"/R620 key was unreachable from this host and turned out to be
unnecessary. The `sgc-early-once` owner (ARM64 SHA-256 `322ac00d…`) and probe
(`435602ea…`) passed on-device `--mode`/owner-path/self-test checks and were
installed without replacing the default binaries; a dedicated
`owner-handoff-sgc-early.sh` ran only after a forced AP reboot (`reboot -f` —
the BusyBox `reboot` applet is a no-op against native-init, which reboots only
through the `reboot()` syscall). On the fresh boot the guarded probe booted the
reviewed B firmware (`probe_rc=0`, CP `OFFLINE`→`ONLINE`); persist was mounted
**read-only** only to read `cpsha`, original EFS never mounted. The owner
latched the early radio edge — `cp_ind 0x0803` (len 8) then `0x0802` (len 12)
`radio_state_raw=0` INITIALIZED at **+9.817 s** — and dispatched the single
factory `0x0404` SGC at **+10.323 s**, inside its 2 s deadline (trigger
`0x0803-0x0802-raw0`, target `europen-400`); the CP **accepted** it:
`response=yes error_raw=0 status=accepted`. This is the first time the factory
carrier SET landed on the early radio edge rather than the +60 s settled
baseline. SIM then READY(5)/PIN1 DISABLED(3). But the +60 s settled snapshot
was identical to every prior boot: radio ON(10), **voice/data
`registration_raw=0` `reject_raw=0` `tech_raw=0`**, automatic selection,
preferred raw 16, operator len 119, **signal len 210 `mask_low7=0`**,
modem_stack enabled; `rmnet0` rx/tx **0/0, no IPv4**; CP stayed ONLINE.
**Bearer verified? no.** Early-SGC acceptance is a mechanical advance but is
**not** sufficient for camp/registration. Next isolated candidates at the same
stage-1 trigger, one at a time: early `SetModemsConfig 0x093f`, then early
camp-on `0x0800` (each needs a new guarded build mode). No secrets, NV, APN,
PIN, CardPower or EFS writes were made. Detail:
[MODEM-07 §Early SGC live run](../../sprints/MODEM-07-RFS-QUARANTINE.md#early-sgc-live-run-2026-10-02).

**Factory-order RE + early-SGC implementation (2026-10-02):** With the live
stack at SIM **READY(5)** / pin1 **DISABLED(3)** / radio ON but
registration 0 / no `rmnet` bearer and `0x0706` active scan returning
`error_raw=2`, this turn reversed the stock TD1A `libsitril` RF/network
bring-up **order** and found the deltas are all at **stage 0/1**, not on the
post-radio SIT path (which is at parity). The factory resolves the carrier/
region (`europen`→target 400→SGC `0x0101`) *before* radio callbacks and, on
`OnRadioAvailable` (wire `0x0803`→`0x0802` raw 0), sends `SetDebugTrace
0x090b` / `SetModemsConfig 0x093f` / **`SendSGCValue 0x0404`** / `SvnInfo
0x4605` and may `TrySetRadioPower(10)` for early camp-on — whereas our stack
applies SGC only +60 s late (or never) and never `0x093f`/early camp-on.
**`0x0706` err2 root-cause hypothesis:** generic RF refusal because the CP
carrier/regulatory profile (and/or early camp-on) is not established at the
factory stage; the signed SGC at the right stage is the first testable
precondition. Firmware/NV load order is **not** the gap (AP-side RilProperty→
SGC runtime step, signed image at parity). Implemented the contract's
`sgc-early-once` owner mode (radio-event observer + early dispatch of the one
factory `0x0404` at the trigger pair, no post sweep); host fixtures pass
native + ASan/UBSan and all four modes cross-build ARM64 `-Werror`. **No phone
run** (device NCM-only, no shell; run is operator-gated: fresh boot + EFS
preflight + review). **Bearer verified? no.** Detail:
[MODEM-07 §Early SGC implementation](../../sprints/MODEM-07-RFS-QUARANTINE.md#early-sgc-implementation--factory-order-consolidation-2026-10-02).

**Confirmed scan rejection (2026-10-01):** A second guarded boot, using the
same one-scan protocol but logging only the 16-bit result, returned
`0x0706 error_raw=2` immediately under SIM READY/PIN1 DISABLED, radio ON,
automatic selection and preferred raw 16. The prior error 2 was observed
under PIN lock; its recurrence in the PIN-free control rules out PIN lock as
the sole cause. Both boots had signal technology-presence mask low seven
bits 0 and no registration/bearer. Stop repeating scans. Next isolate the
factory AP/RFS startup prerequisite and CP RF state; error 2 alone is a
generic refusal, not proof which prerequisite is missing.

**One-shot scan control (2026-10-01):** A separately named, guarded
single-owner build sent one factory-shaped `0x0706` available-network scan
after fresh same-boot READY/PIN-disabled, radio-ON, automatic-selection and
broad-RAT checks. CP promptly returned a matching 12-byte **nonzero-error**
response; there was no timeout, cancellation or retry in this first boot.
Its error number was not logged; the later controlled boot above captured it.
The preceding signal GET's technology-presence mask had low seven
bits 0. CP stayed ONLINE, but registration remained 0 and `rmnet0` down.
The physical SIM and data worked in another handset. No eSIM profile was
deleted; an unfinished eSIM attempt does not prove that stack's RF-idle
state. Next isolate the CP rejection/missing factory AP or RFS prerequisite,
not another blind active scan. See [live details](MODEM-RUNTIME-2026-09-24.md).

**Extended control (2026-10-01):** With SIM READY/PIN1 DISABLED and radio
ON, the single-owner +60-second network GETs returned automatic selection
(0), preferred type raw 16, and successful operator/signal responses.
Registration remained 0 and `rmnet0` had no bearer. Operator and signal
payloads were suppressed. The Samsung factory table identifies SIT raw 16
as NR/LTE/GSM/WCDMA and maps it to Android mode 26; a restrictive preferred
mode is not supported as the blocker. At the time of this passive control,
the active scan had not yet been sent; the later one-shot result is above.
See [the runtime control](MODEM-RUNTIME-2026-09-24.md).

**Latest SIM-in control (2026-10-01):** The operator confirmed the card's PIN
prompt was actually enabled, then disabled it in another phone, reinserted
the card and reported replenishing the account. With one continuous IPC0/RFS0
owner, the physical-SIM-in boot reached app READY(5)/PIN1 DISABLED(3) without
a SaaiOS PIN command; the +60-second read-only sweep showed radio ON(10),
voice/data registration 0, no `rmnet0` IPv4 and RX/TX 0. Two-slot metadata
showed slot 0 port 0 logical 1 and slot 1 port 0 logical 0; the eSIM identity
is plausible but not proven. File-3 RFS cmd7/cmd6 again arrived without
replies. MODEM-06 is now clearly blocked at camp/registration under a
PIN-free READY state. The account top-up is operator-reported, not a network
attach measurement. See [the runtime control](MODEM-RUNTIME-2026-09-24.md).

**New channel evidence (2026-10-01):** the live kernel repeatedly reported
`umts_ipc0 is not opened` (190 matching entries in the inspected ring) and
one `umts_rfs0 is not opened` packet drop. A bounded, header-only IPC reader
then received 23 unsolicited `type=2, id=0x0906, len=206` frames in 30 s,
roughly one every 1.28 s. No frame payload or subscriber identity was logged.
The stock TD1A `libsitril.so` route table maps `0x0906` to
`MiscService::OnUnsolSignalStrength`, which forwards RIL unsolicited 1009
(`RIL_UNSOL_SIGNAL_STRENGTH`) without a command back to CP. The factory
library SHA-256 is
`efcca0d5fa5a3eb3a09d8c9f68fc35f8b194bb511379987fd4a353f12ed2d5b1`;
[AOSP defines RIL 1009 as signal-strength telemetry](https://android.googlesource.com/platform/hardware/ril/+/android-4.2.2_r1/include/telephony/ril.h).
Therefore losing these **particular** frames does not explain failure to camp;
other lost IPC events remain possible. The s5300 kernel discards RX when an
endpoint has no opener, and the
last close purges its shared RX queue. A long-lived, single-reader IPC/RFS
owner and safe boot handoff now precede more radio-setting experiments. A
bounded observer cannot substitute for that service.

**Earlier control (2026-10-01):** SIM reached **READY(5)** twice after a
signed `0x0201` VerifyPin request with AID and **without CardPower**.
Neither test consumed a PIN attempt (`remain=3`). The owner says the SIM
PIN is disabled; do not automatically send VerifyPin or guess digits.
The post-READY `0x0704` request succeeded, but data registration remained
0 and there is still no `rmnet` IPv4 bearer. MODEM-06 remains incomplete
at **network camp/registration**, not at PIN→READY.

The RFS diagnostic separately completed a bounded 7→3→6 exchange with
factory-defined status frames and no NV/EFS access. Its passive SIM watch
overlapped the other VerifyPin test: six silent query failures are
consistent with contention for the shared SIT lock. **Do not attribute
READY to RFS or CP self-init from this boot.** No stock AP encoder for the
internal CP `SIM_INIT_REQ` catalog id `0x2f50` has been found; an external
OEM frame is not established or required to explain the observed READY.
Sections below preserve earlier investigations, including hypotheses now
superseded by the signed VerifyPin→READY result.

**Live post-READY check (2026-10-01):** A verified static ARM64
`ready-network-once snapshot` read eight SIT responses under the common lock:
SIM READY(5), pin1=2, radio ON(10), voice registration=0, data
registration=0/reject=0/tech=0, automatic selection=0, preferred=11
(LTE_ONLY), and successful operator/signal responses (private payloads
suppressed). `rmnet0–5` RX remained 0. A guarded `run` sent **only**
AllowData(1) because selection was already auto; its ACK succeeded, but four
registration polls over roughly 10 seconds stayed 0 with no RX. Thus an
accepted AllowData request alone does not start camp on this boot. No PIN,
CardPower, RadioPower OFF, APN or NV/EFS write was performed in this check.

**Next isolated result (same boot):** guarded RadioPower **ON-only** `0x0800`
was accepted under freshly checked READY/ON. Four voice and data polls still
returned registration=0/reject=0 and zero RX. A subsequent eight-GET snapshot
confirmed READY(5), radio ON(10), auto selection and preferred LTE_ONLY(11)
were unchanged. Do not repeat ON-only as an established camp trigger.

**Reversible RAT check (same boot):** after fresh READY/ON/preferred=11 gates,
factory-encoded `0x070a` temporarily set LTE_WCDMA(12); its ACK and readback
both confirmed 12. Ten voice/data registration polls over about 30 s all
returned 0/reject=0/tech=0, with `rmnet0–5` RX remaining 0. A restore SET and
GET confirmed the original LTE_ONLY(11). Thus forced LTE_ONLY alone is not the
observed blocker. This test did not provide continuous RFS ownership, so it
does not exclude a boot/runtime RFS prerequisite.

**Continuous diagnostic owner (same boot, attached after ONLINE):** the
opt-in `modem-channel-owner --attach-online` acquired the stable common lock
and verified exclusive IPC0/RFS0 open counts. Its single reader returned
SIM READY(5), radio ON(10), voice/data registration=0/reject=0/tech=0 from
four read-only GETs. In two successive one-minute windows it received 46
and 47 `0x0906` signal indications, and **zero RFS requests**; `rmnet0–5` RX
remained 0. The last kernel `umts_ipc0 is not opened` message precedes this
owner's open. The owner keeps running; do not launch competing one-shot SIT
readers. Because it attached after ONLINE, early boot RFS/IPC traffic is
unknown. It intentionally does not service RFS or prove modem registration.

**RFS observation caveat:** opening and closing `/dev/umts_rfs0` is not a
harmless one-shot probe: the kernel discards incoming packets while no RFS
reader is open and purges its receive queue when the last reader closes.
The 7→3→6 broker exited before these network tests, so later RFS traffic
remains unmeasured. The old `rfs-poll` tool now refuses to run by default.

## Achieved remotely

| Milestone | Evidence |
| --- | --- |
| Stock CP ONLINE + handover | live |
| **SIM READY(5)** | signed VerifyPin A+AID, without CardPower; repeated twice in a parallel live session |
| **Pin1Verified** | `0x0201` candidate A **with AID** from `0x0200`, RFS-aware → **error 0**, remain unchanged; pin1 1→2 |
| AllowData / LTE_ONLY / voice reg | AllowData err0; preferred=11; voice reg=3 under PIN |
| Physical HotSwap ABSENT→PRESENT | live (tray pull/reinsert) — VerifyPin window only |

## Historical soft-lock after HotSwap (superseded by READY)

| Field | Live (post-HotSwap) | Gate |
| --- | --- | --- |
| `app_state` | **PIN (2)** | START_NETWORK allows only `{1,4,5}` — PIN denied |
| `pin1` | **2 = ENABLED_VERIFIED** | Pin1Verified set; does **not** open camp |
| Present `+0xBF6` | **Not measured on `0x0200` wire**; former `present_infer` claim withdrawn | Do not infer live Present from app state |
| SET#6 | needs LTE camp / SADR PAUSE | camp needs START_NETWORK → needs app∈{1,4,5} |

**Historical hypothesis (falsified for this boot):** GET_APP stayed PIN in
these earlier tests, but the later VerifyPin A+AID path reached READY.

### HotSwap live falsifier (2026-09-30) — conclusive

**2026-10-01 correction:** This rules out tray reseat *alone* in that test
window, not the later VerifyPin A+AID→READY path. The asserted live Present
value below was inferred, not measured; do not use it as a current gate.

Physical tray pull/reinsert while `tray-bearer-chase` armed:

1. `card=ABSENT` → reinsert `ABSENT→PRESENT`; `saw_absent=1`; **never**
   `saw_detected` / `saw_ready`.
2. `TRANSITION app=2(PIN) pin1=1 present_infer=notin_1_2_3` → VerifyPin A+AID
   **err0**, pin1 **1→2**.
3. **App stayed PIN**; `chase_gate START_NETWORK_ALLOWED=no`;
   `registration_raw=0`; `rmnet*` **rx=0**, **no IPv4**.

**Verdict:** HotSwap alone does **not** yield READY/bearer on this EU No-CDMA
image. Present=`+0xBF6==2` gate still blocks. Do **not** treat further reseats
as a path to bearer.

### Historical `present_infer` interpretation — **withdrawn**

| Claim | Result |
| --- | --- |
| `0x0200` layout | Factory HAL: card@12, apps@14, type@15, **app_state@16**, perso_substate@17, pin1@72, remain@74. Former app@17 was wrong. |
| Wire vs MAIN | Byte17 is personalization substate; internal `Present +0xBF6` is **not on this wire response**. |
| Inference | Published app state cannot be reversed into a current private Present value; former `present_infer` was a hypothesis. |
| Mis-read Present=2 while PIN? | Undetermined from SIT status; former categorical “No” is withdrawn. |
| Caveat | MAIN predicates remain useful static evidence but not a live measurement of Present. |

Script: `diagnostics/tmp-validate-present-infer.py` (MAIN B).

### Historical implication — no remaining signed AP path (falsified by READY)

Remote soft-lock **cannot** force READY/Present. Exhausted: EngMode, CardPower,
ds_detect, SET_APP invent, FN_A/RatMap NV, DRAM/ATU Present poke, **physical
HotSwap**, stock empty GETs, **SIT Misc NvRead/NvWrite (stubbed)**. STATUS→SET#5
needs LTE **SADR_MEASURE_RSP** (STATUS_WRAP; 2 callers only) **and** Present==2;
Present=2 latch remains **FN_A/CDMA-only** (EU RatMap blocks). No signed SIT for
SADR inject. HotSwap INSERT RO: DBT-only; SET#1 does not STRB Present=2. **Do
not** re-run EngMode / poke / CardPower spam / CDMA preferred / invent SADR /
unsigned MAIN / getobj`#0x10` poke / further “just reseat” hopes / invent NV
writes against stub builders.

### 2026-09-30 — SIT NV / alt signed CP (in-policy hunt)

| Lever | Evidence | Live? |
| --- | --- | --- |
| `ProtocolMiscBuilder::BuildNvReadItem` / `BuildNvWriteItem` | **size-8 stubs** (`MOV X0,XZR; RET`) — only MiscBuilder stubs; `ProtocolMiscNvReadItemAdapter::GetValue` also stub; sit-base has **no** Nv symbols | **not tried** (no payload; would NACK/null) |
| Ban nuance | Ban is **mount original EFS RW**, not every NV path — but this vendor sit-stream **does not ship a working SIT NV write** | n/a |
| Adjacent (not RatMap) | `BuildCdmaSubscription` `0x90f`; `BuildSetOpenCarierInfo` `0x90d`; `SendSGCValue` `0x404`; `BuildSetCpCarrierConfig` large `0x214` — **no** evidence they set `TCS_CDMA_SUPPORT` / SupportedRatMap | **no try** |
| Alt signed CP/MAIN | Host `diagnostics/fw/`: **A** `57465ab9…` / A-14784800, **B** `449eeab3…` / B-15346003 (live), CP2A factory modem `491993b0…` (same B). All embed `No CDMA in SupportedRatMap`. TD1A radio is **g5300g** (not UDL-compatible). **No CDMA-RatMap SKU** | **not booted** |
| Unsigned MAIN patch | Prior UDL MAIN DONE reject — still dead | no |
| Local factory `modem.img` (CP2A.260705.006) | SHA-256 `491993b0…`; ver `B-15346003`; **still** `No CDMA in SupportedRatMap` — same family as live B, **not** US/CDMA | **not booted** (would not enable FN_A) |
| Google panther factory | single-SKU per build; no separate US/CDMA radio package found | n/a |
| External hunt (A/B turn) | CP2A re-extracted + TD1A.221105.001 downloaded; cheetah/lynx CP2A HEAD-ok but stopped after identical EU CP2A. **No soft UDL** | scanned; stop |

**Still impossible under bans:** EFS RW TCS flip; unsigned MAIN; rild/cbd; invent SADR; AP Present poke.

**Newly actionable:** none proven. Need an **external signed** CP/MAIN/NV-defaults image with CDMA in RatMap (loadable via existing CPIF UDL), **or** policy change.

### 2026-09-30 — A) Soft-lock host gate vs CP gate (LIVE)

**2026-10-01 correction:** `0x0704` is SetNetworkSelectionAuto. Its response
does not establish that CP issued `NS_START_NETWORK_REQ` or camped; labels
calling it a START_NETWORK equivalent below are historical shorthand only.

Host chase refuses START_NETWORK unless `app∈{1,4,5}`. Forced the signed
START_NETWORK trigger (`BuildSetNetworkSelectionAuto` `0x0704`) **once**
while still PIN after Pin1Verified.

| Step | Result |
| --- | --- |
| Pre | ONLINE; app=PIN(2) pin1=1 remain=3; voice reg=3; data reg=0; rmnet rx=0 |
| VerifyPin A+AID | err0; pin1 **1→2**; **app stayed PIN** |
| Preferred LTE `0x070a` | err0 |
| **`0x0704` auto (START_NETWORK equiv)** | **length=12 error_raw=2** |
| AllowData `0x0710` / GetPs `0x0711` | err0 / err0 byte12=1 |
| Post +8s | app=**PIN**; pin1=2; voice/data **reg=0**; rmnet0–5 **rx=0 tx=0**; **no IPv4** |

**Verdict (refined):** later `fix-selection` → `0x0703`
`selection_mode_raw=0` (**already auto**) → set skipped. Prior forced
`0x0704` `error_raw=2` is **`RCM_E_GENERIC_FAILURE` already-auto no-op**, not
a unique SIM-gate fingerprint. Camp still hard-blocked while PIN: START_NETWORK
`@0x18e831a` `GET_APP∈{1,4,5}` else ignore. Host soft-lock still matches that
gate. Do not spam `0x0704` while auto+PIN.

### 2026-09-30 — `0x0704` / START_NETWORK precondition RE (MAIN B)

SIT `0x0704` registered `@0x1150b14` → `SIT_REG` `0x20d1afa`. Camp path =
START_NETWORK `@0x18e8028`.

| ID | Check | Live | Blocks camp | Signed SIT w/o Present/FN_A/EFS/rild? |
| --- | --- | --- | --- | --- |
| P1 | radio ON | **met** | no | yes (done) |
| P2 | not already auto | **already auto** (`0x0704`→err2 no-op) | n/a | n/a |
| P3 | `@0x18e831a` `GET_APP∈{1,4,5}` | **BLOCKED** PIN(2) | **yes** | **no** (READY↔Present=2/FN_A) |
| P4 | early `GET_APP==4` `@0x18e8072` | N/A | no | no |
| P5 | PIN branch `@0x18e81e0` +`0xBF5` | active; ≠ open P3 | indirect | VerifyPin done |
| P6/P7 | `LDRB +0xc3` / `+0x554` | unknown | unknown | no mapped SIT |
| P8 | Present `+0xBF6==2` | notin | READY only | **no** (FN_A) |
| P9–P11 | pin1=2 / card / LTE_ONLY | **met** | no | yes (done) |

**Satiable unmet → live SIT?** **NONE** (`fix-selection` query-only).

### 2026-09-30 — B) External CDMA-RatMap image hunt (STOP — no soft UDL)

| Candidate | Ver / SHA | `No CDMA in SupportedRatMap` | `EnableCdmaRat` | UDL vs live CPIF? | Soft UDL? |
| --- | --- | --- | --- | --- | --- |
| Live MAIN B | `g5300q-…-B-15346003` / `449eeab3…` | 1 | 0 | live | n/a |
| Probe A | `g5300q-…-B-14784800` / `57465ab9…` | 1 | 0 | same family | no (prior) |
| Factory CP2A.260705.006 `modem.img` | `B-15346003` / `491993b0…` | 1 | 0 | **identical EU** | **no** |
| Factory CP2A radio-panther | same `B-15346003` / radio `e02ff0be…` | 1 | 0 | identical EU | **no** |
| Factory TD1A.221105.001 radio | **`g5300g-220908-…-B-9040061`** / `4939b739…` | **0** (no RatMap strings) | **0** | **different family** (g5300g / FBPK; live is g5300q TOC) | **no** |
| cheetah/lynx CP2A.260705.006 | HEAD OK (~3.9GB); DL aborted after panther CP2A identical | — | — | likely same-era g5300q | skipped |

Google panther factory remains single-SKU per build. Verizon OTA
`panther-ota-td1a.221105.003-32ef0dee.zip` **HEAD 200** earlier (~2.33GB);
full DL this turn **blocked** (`curl: connect timeout` to dl.google.com) —
partial `144289792` bytes left for resume at
`fw/cdma-hunt/panther-ota-td1a.221105.003-32ef0dee.zip`. cheetah CP2A
`factory-23d564ad` HEAD-ok (~3.9GB); same-era EU twin expected. **No
g5300q+CDMA staged yet** (scan pending completed Verizon DL). **No soft UDL.**

Artifacts under `os/targets/panther/diagnostics/fw/` (+ `cdma-hunt/`).

### 2026-10-01 — Verizon OTA resume + scan (STOP — no soft UDL)

| Artifact | SHA-256 | Chip / ver | `No CDMA…` | `EnableCdmaRat` | Soft UDL? |
| --- | --- | --- | --- | --- | --- |
| OTA `panther-ota-td1a.221105.003-32ef0dee.zip` | `32ef0dee…` **ok** (2.33GB resumed) | payload→xz carve **`g5300g-220908-221006-B-9144834`** | 0 (no RatMap strings in carve) | 0 | **no** — wrong chip vs live g5300q TOC |
| OTA `panther-ota-ap1a.240505.005.a1-83fca43d.zip` (latest panther **Verizon**) | `83fca43d…` **ok** (2.42GB) | PIXELMODEM ext4 → `modem.bin.gz` → TOC **`g5300q-231218-240405-B-11675365`** / inner `809d9695…` | **1** | 0 | **no** — still EU-style No-CDMA string; not a CDMA RatMap SKU |
| Factory CP2A / TD1A (prior) | unchanged | g5300q No-CDMA / g5300g | — | — | **no** |

**Panther Verizon OTA page:** last Verizon-tagged build is **AP1A.240505.005.A1**
(May 2024). No panther Verizon rows in 2025–2026 (BP*/CP*); those later
rows are global/other-carrier only. Cheetah/lynx Verizon likewise stop at
AP1A. Pixel-10 `BD1A…A3 Verizon` is **wrong device** — do not download as
panther candidate.

**Method:** `payload-dumper-go -p modem` → debugfs PIXELMODEM `/images/…/modem.bin.gz`
→ gunzip → TOC scan. Scripts/logs under `diagnostics/fw/cdma-hunt/` +
`tmp-scan-ota-generic.py` / `tmp-gunzip-scan-ap1a.py`.

**Verdict:** no **usable signed g5300q + CDMA RatMap** image found. Verizon
SKU ≠ RatMap CDMA on this Shannon family. **No soft UDL. Bearer not verified.**

**Next (external/policy only):** non-Google signed CDMA-RatMap g5300q, or
policy lift (EFS `TCS_CDMA_SUPPORT` / rild / unsigned MAIN). Do not blind-UDL
AP1A or TD1A; do not spam `0x0704` under PIN; no BAR `0x81400000`.

### 2026-10-01 — Corrected READY model (Verizon No-CDMA proof)

**Game-change:** Verizon panther AP1A `g5300q-…-B-11675365` embeds
`No CDMA in SupportedRatMap` / no `EnableCdmaRat` string — **same family as
live EU MAIN B**. Stock Verizon/EU clearly reach SIM READY without FN_A
CDMA L1. Therefore **FN_A/CDMA cannot be the stock USIM READY path**, and
hunting CDMA-RatMap UDL / FN_A elicit was an over-fit.

**RE both images** (`tmp-usim-nocdma-ready-re.py` + `tmp-usim-present-init-re.py`):

| Item | Live MAIN B | Verizon AP1A extract |
| --- | --- | --- |
| `No CDMA in SupportedRatMap` | yes | yes |
| Sole `STRB.W +0xBF4` | inside SET_APP (14 BL callers) | same shape (14 BL) |
| SET#1 DETECTED | `0x146aaba` — **no** Present gate | `0x160cd12` — **no** Present gate |
| SET#4 / SET#5 | STATUS only; `#5` behind `LDRB +0xBF6` **CMP #2** | identical gate |
| PresentObj[0]=2 (`#636c`+MOVS#2+STRB#0) | **sole** FN_A `0x14f6a16` | **sole** FN_A-class site |
| FN_A callers | CDMA MEAS + TIMING_LATCH only | same class |
| STATUS_WRAP callers | SADR + L1TUNNEL only | (same architecture) |
| sit-stream SIM_INIT / START_STACK | **no builders** | n/a |
| `SimRefresh` in sit-stream | STK adapter only — not AP READY SIT | n/a |

**USIM No-CDMA READY model (corrected):**

```
USIM INSERT ──► SET_APP#1 DETECTED     (no Present gate; START_NETWORK allows #1)
         │
STATUS (SADR/L1TUNNEL) while GET∈{1,4}:
   Present 0 → SET#2 PIN     ◄── our soft-lock lands here (live)
   Present 1 → SET#3 PUK
   Present 2 → SET#5 READY   ◄── sole #5; gate identical on Verizon No-CDMA
   Present 3 → SET#4 PERSO
         │
While GET==PIN: STATUS skips Present re-copy / cannot promote to #5
         │
START_NETWORK allows GET∈{1,4,5} only — PIN blocks camp → SET#6/#7

Present=2 latch found in MAIN: still only FN_A (CDMA L1) — but that latch
CANNOT be stock No-CDMA USIM (Verizon proof). Hole = unidentified USIM
Present=2 (or alternate READY) writer / boot-init — NOT "need CDMA RatMap".
```

| Path | Role | No-CDMA USIM? |
| --- | --- | --- |
| SET#1 DETECTED | USIM insert; camp-eligible | **yes** (we leave it too early) |
| SET#5 + Present==2 | sole published READY | machine exists on No-CDMA SKUs; **stock path ≠ FN_A elicit** |
| FN_A CDMA MEAS/LATCH | only *found* Present=2 STRB | **not** stock USIM READY (Verizon) |
| Pin1Verified / ATR / OpenChannel / EngMode | no SET#5 | live-negated |
| Stock sitril after VerifyPin | re-poll `0x0200` only | **no** second READY SIT |

**Missing signed AP step vs stock:** **none** found this turn (GetImsi is
post-READY and secret-bearing — not tried; SimRefresh is STK-only;
`0x0245`/`0x0930`/`0x4104` already live-negated). **No live SIT experiment.**

**Live brief (COM13):** ONLINE; app=PIN(2) pin1=2 remain=3;
`present_infer=notin_1_2_3`; radio=10; preferred=11; rmnet0–5 rx=0/tx=0;
no rmnet IPv4. `peek-present-surfaces`: PresentObj still AP-unmapped.

**Bearer verified?** **no**.

**Next:** stop CDMA-RatMap/FN_A-as-USIM-READY framing. Hunt **non-FN_A
Present=2 / USIM boot-init** writers (SIM_INIT internal, not sit-stream) or
policy (cbd/rild SIM_INIT / EFS). Do not blind-UDL Verizon AP1A; no ATU MAIN
poke; no BAR `0x81400000`; no `0x0704` spam under PIN.

### 2026-10-01 — SIM_INIT / Present=2 hunt (prior)

**Live (then):** ONLINE; app=PIN(2) pin1=2 `present_infer=notin`; rmnet rx/tx=0; no IPv4.

**Non-FN_A Present=2?** **No.** Re-scan: sole `#636c`+MOVS#2+STRB(.W)#0 remains
FN_A `0x14f6a14`; STRH `+0xBF6` peers are false-friends; SET#5 sole @`0x14fb5c6`.

**SIM_INIT path:** USIM waits `USIM_NOT_INITIALISED` / `SIM_INIT_REQ` (table id
`0x2f50`); START_STACK id `0x2f58`. Table handler VAs are **codecs** (no Present
stores). sit-stream has **no** SIM_INIT/START_STACK builder. Soft CPIF cannot
emit that IPC without cbd/rild — and INIT itself is **not** shown to set
Present=2.

**Live try:** none (no signed lever). **Bearer verified?** no.

### 2026-10-01 — who sends `0x2f50` (this turn)

**Live brief:** **unavailable** — Pixel 7 USB present as MTP/charging phantoms
(`VID_18D1` PID `4EE0/4EE1/4EE7`, `CM_PROB_PHANTOM`); **no ADB** interface.
No ONLINE/PIN/bearer re-sample.

**Who sends bare `0x2f50` into USIM?** **AP OEM IPC (cbd/rild host channel) — not
GMC / boot / timer / other CP task.**

| Evidence | Result |
| --- | --- |
| OEM IPC catalog `@0x6de740` | `flags=0x2 msgid=0x2f50 name=SIM_INIT_REQ meta=0x10104` — same table as `SIM_VERIFYPIN_REQ` / `SIM_INFO_REQ` |
| Catalog neighborhood | `[OEM][IPC]` / `[OEM][SIT]` strings (`0x6df9aa`…) |
| GMC `==> SIM__` set | READ_ALL / **START_STACK** / CSG / READ / UPDATE only — **no SIM_INIT** |
| GMC descriptor `0x2f50` | **0** (START_STACK `0x2f58` **is** in GMC table `@0x19f7dc`) |
| `USIM_SCHEDULE_SIM_INIT_TIMER` | USIM **wait-state name** only — not a send |
| Bare MOVW `#0x2f50` (no MOVT) | name-reg / false-friends; **0** header STRH send; **0** litpool LDR msgid |
| sit-stream / soft CPIF | **no** `SIM_INIT` builder; `sitInformSimInit()` log-only |

**Soft trigger without cbd/rild?** **No evidenced signed path.** Emitting `0x2f50`
means speaking the **OEM IPC** dialect the catalog serves (cbd territory), not a
mailbox/SIT we already have. Existing soft SIT (VerifyPin / status / HotSwap) does
not alias into that REQ. Even if elicited, INIT handlers still **≠ Present=2**.

**Non-STRB PresentObj[0]=2?** **No.** Re-scan `#636c` vicinity: sole Present=2 =
FN_A `MOVS#2+STRB.W#0` `@0x14f6a14`; word/STRH/STM/memcpy-adjacent patterns
**0** outside FN_A.

**Live try / bearer?** **none** (no ADB + no soft lever). Bearer **not** verified.

**Next:** (1) re-attach ADB → brief confirm sticky PIN/`notin`; (2) stock No-CDMA
Present=2 paradox remains open under bans — policy (cbd SIM bring-up) or lift;
(3) do **not** invent OEM-IPC `SIM_INIT` / blind UDL / ATU MAIN poke.

### 2026-10-01 — OEM IPC `0x2f50` framing (prior)

**Live brief (COM13 / USB NCM `172.31.7.1`):** ONLINE; radio=10; app=PIN(2)
pin1=2 remain=3; `present_infer=notin`; data reg=0; rmnet rx=0; **no** cbd/rild.

**Policy note:** ban is **starting** cbd/rild — emitting the same OEM IPC ourselves
is allowed **only** if wire framing is RE-evidenced (not invented).

| Check | Result |
| --- | --- |
| Catalog `@0x6de740` | stride **28**; `flags=2` msgid=`0x2f50` meta=`0x10104` rsp=`0`; name `SIM_INIT_REQ` |
| Soft VerifyPin transport | **SIT** on `umts_ipc0`: type0 id **`0x0201`** len38 — **not** OEM `0x2f52` |
| SIT↔OEM dual (`0x0200`↔`0x2f50`, `0x0201`↔`0x2f52`) | **0** MOVW pairs in MAIN |
| sit-stream `Build*SimInit` / u16 `0x2f50` | **absent** / count **0** |
| libsitril `MOVZ #0x2f50` | table-init only (`@0x11f300`); **no** packet builder |
| Host OEM wire header (magic/len/seq) | **not recovered** from catalog or encode strings |
| `oem_ipc0` (493:12) | mknod ok; open → **EACCES** (even chmod 666); prior RO listen EOF |

**OEM IPC `0x2f50` sendable?** **No** — payload size hint (`flags=2`) without outer
header / channel protocol is not enough; soft SIT VerifyPin does **not** reuse as
OEM transport. **Live try:** none (no invent bytes). **Bearer verified?** no.

### 2026-10-01 — OEM host encode RE + oem_ipc0 open (this turn)

**Live:** ONLINE; radio=10; app=PIN pin1=2 remain=3; data reg=0 tech=UMTS;
rmnet rx=0; **no** cbd/rild.

| Source | Result |
| --- | --- |
| `cbd` (`raw-cbd`) | Opens **boot/ramdump only** — **not** OEM IPC encode path |
| CPIF (`ipc_io_device` / SIT) | Write to oem chardev: kernel prepends **EXYNOS 12B** when `link_header`; userspace = **app payload only** |
| `ipc_message_server` (MAIN) | Encode/decode **strings** present; typed `kMessageId<>` assert; **app header layout unrecovered** |
| libsitril msgid table `@0x95660` | stride **24**: `id\|0x23\|0\|0x402\|idx` — registry only; **no** `/dev/oem_ipc`; **no** builder |
| sit-stream / sit-base | **no** `oem_ipc` / **no** `0x2f50` builder |
| Catalog `@0x6de740` | still `flags=2` body-size hint; meta `0x10104`; rsp=0 |

**Exact missing fields (no invent):**
1. OEM **app-layer header** field order/size (SIT-12B reuse, classic `sipc_fmt_hdr`, or other — all **unproven**)
2. **2-byte body** contents for `flags=2` (zeros **unproven**)
3. Token/seq/transaction rules
4. Which `oem_ipcN` (0..7) stock uses for SIM_INIT

**`oem_ipc0` usable?** **Yes for open** — `mknod` 493:12 + `chmod 666` → `exec 3<>/dev/oem_ipc0` **RDWR OK** as root (prior EACCES cleared; node was missing until mknod). Write framing still unknown → **no send**.

**Live SIM_INIT?** **none**. **Bearer verified?** no.

**Next:** recover OEM app header from stock capture / host OEM writer binary (not cbd boot path), or typed encode in `oem_ipc_message_utils` with proven body; then ONE soft INIT on openable `oem_ipc0`.

### 2026-10-01 — host OEM encoder hunt (no send)

**Live brief (COM13 / USB NCM):** modem_state=ONLINE; radio=10; app=PIN(2)
pin1=2 remain=3; `present_infer=notin`; data reg=0 tech=UMTS; preferred=LTE_ONLY;
oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild; vendor `/vendor/lib64/*ril*` absent on
SaaiOS image; oem_ipc iod has **no** of_node attrs exposed.

| Source | Result |
| --- | --- |
| sit-stream `BuildOemSimRequest` `@0x806d0` | Builds **SIT** ids `0x208/0x20c/0x20f/0x247` + **12B** SIT hdr — **not** OEM `0x2f50` |
| sit-stream `BuildSimGetStatus` | SIT `0x0200` len12 — umts_ipc path (already soft) |
| sit-base `SIT_OEM_*` | OEM-over-**SIT** names (PIN_ENC / CQI / SVN…) — **no** `SIM_INIT_REQ` / `0x2f50` |
| libsitril IoChannel | `/dev/umts_ipc0` (+ipc1) only — **no** `/dev/oem_ipc*` |
| libsitril `@0x95660` | registry `0x2f50\|0x23\|0\|0x402\|0x3fc` — table-init MOVZ only; **0** builder |
| libsitril MOVZ `#0x2f52` | **0** |
| libsec-ril `OemIpcRecord` / `IpcTxSimInitMessage` | Samsung classic SIPC/STK — **0** MOVZ `0x2f50`/`0x2f52`; not Pixel Shannon OEM IPC |
| raw-cbd | boot/ramdump only (reconfirmed) |
| MAIN `oem_ipc_message_utils` | encode/decode strings; app header still unrecovered |
| sit-stream u16 `0x2f52` “hits” | instruction-encoding false-friends (e.g. `80 52 2f …`), not msgids |

**Encoder found?** **No** host writer that opens `oem_ipc*` and encodes `0x2f50`.
Closest builders speak **SIT FMT on umts_ipc**, which does not alias to OEM catalog
`SIM_INIT_REQ`.

**Exact missing (unchanged; no invent):**
1. OEM **app-layer header** layout (SIT-12B reuse **unproven** for oem_ipc)
2. **2-byte body** for catalog `flags=2`
3. Token/seq/transaction rules
4. Which `oem_ipcN` stock uses
5. Binary offset hint: still need stock OEM writer (not in libsitril/sit-stream/cbd/libsec-ril) — candidates: stock radio HAL / oemhook dump from factory vendor image, or MAIN typed encode in `oem_ipc_message_utils.c` / dispatcher near `[OEM][IPC] Unable to encode` `@0x6dfcf3` / not-REQUEST `@0x6dfe0d` (lit `@0x6dff00`; nearby LDRB imm `#9/#18/#19` = object fields, **not** proven wire offsets)

**Live SIM_INIT?** **none** (no send). **Bearer verified?** no.

**Next:** (1) pull stock Pixel vendor radio/oemhook binary that opens `oem_ipc*` (factory
image / vendor partition dump — not start rild); (2) or finish MAIN encode RE to
recover app header+body sizes from typed `kMessageId` paths; then ONE soft write.

### 2026-10-01 — MAIN encode RE + factory vendor oem_ipc0 (no send)

**Live brief (COM13 / USB NCM `172.31.7.1`):** modem_state=**ONLINE**;
`sit-sim-status`: card=PRESENT apps=1 **app=PIN(2)** pin1=2 remain=3;
oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild; rmnet rx=0 all; **no bearer**.

#### MAIN `@0x6dfcf3` / `@0x6dfe0d`

Those file offs are **rodata strings** (`Unable to encode IPC message` /
`[OEM][IPC] Message is not a REQUEST`), not Thumb code. Band `0x6de000..0x6e1000`
is catalog + string + **pointer-table island** (0 PUSH). Full-image MOVW+MOVT /
LDR.W-pc to encode-fail / not-REQUEST / utils.c VAs: **0 hits**.

Catalog (stride **28**, walk start `@0x6de708`):

| Entry | body (`flags`) | msgid | meta | rsp | name |
| --- | --- | --- | --- | --- | --- |
| `@0x6de740` | **2** | `0x2f50` | `0x10104` | `0` | `SIM_INIT_REQ` |
| `@0x6de874` | **10** | `0x2f52` | `0x10104` | `0x2fa1` | `SIM_VERIFYPIN_REQ` |
| `@0x6de724` | **2** | `0x2f57` | `0x10104` | `0x2fa8` | `SIM_INFO_REQ` |

Catalog xrefs exist (e.g. MOVW/MOVT `0x406d7b30` `@0x32e4494`, start
`0x406d7af8` `@0x331a566`) but go through helper `BL 0xca893e` and mutate
**C++ message objects** (`STRB` +8/+9/+14…); `MOVS #12` on that path is an
**error-path return**, not proven wire-header length. **No wire STR of
msgid/body recovered.**

#### Factory vendor.img oem_ipc writer (no rild start)

From `panther-td1a.221105.001` factory → nested `image-*.zip` → **vendor.img**
(extracted under `diagnostics/fw/cdma-hunt/factory-td1a-vendor/`).

| Hit | Meaning |
| --- | --- |
| `/dev/oem_ipc0` in carved ELF `@vendor+0x25e7b000` | **`SitOemHandler` / `ModemData` / `ModemDataTransmitter`** |
| Wire | **protobuf** `sit_ipc_message::IpcMessage` (`SerializeToArray`, `protobufSerialDataLen`, `initialMessageHeader`) |
| Messages present | Ping / Config / Thermal / DeviceState / Metrics / Traffic / Txas / Coex / Debug / Scone — **no SIM_*** |
| `SIM_INIT_REQ` / ASCII `0x2f50` in vendor.img | **0** |
| `/dev/oem_ipc1` carved ELF | extended-log / modemstat helper — not SIM_INIT |
| ueventd / sepolicy | `oem_ipc[0-7]` → `radio_dev` |

**Encoder for catalog `0x2f50`?** Still **no**. Stock opener of `oem_ipc0` is the
SitOem **protobuf** channel and does **not** implement `SIM_INIT_REQ`.

**Exact bytes still missing for soft `0x2f50` (no invent):**
1. OEM **app-layer / protobuf** framing that CP demuxes as catalog msgid `0x2f50` — SitOem protobuf schema has **no** SIM payload; classic raw header still **unproven**
2. **2-byte body** for catalog `flags=2` (zeros unproven)
3. Token/seq/transaction rules for that frame
4. Which host binary (if any) emits catalog `0x2f50` on which `oem_ipcN` — **not** the carved SitOem `.so`

**Live SIM_INIT?** **none**. **Bearer verified?** no.

**Next:** find who maps to CP catalog `0x2f50` (other vendor ELF / MAIN decode of
SitOem protobuf → internal msgid, or non-SitOem opener); do **not** send
SitOem ping/config as fake SIM_INIT; no start rild/cbd.

### 2026-10-01 — SitOem protobuf ↛ catalog `0x2f50` (demux proof)

**Live brief (COM13 / USB NCM `172.31.7.1`):** modem_state=**ONLINE**;
card=PRESENT apps=1 **app=PIN(2)** pin1=2 remain=3; oem_ipc0 **OEM_RDWR_OK**;
**no** cbd/rild (absent on SaaiOS image); rmnet0–29 **rx=0**; **no rmnet IPv4**.

Scripts: `tmp-sitoem-demux-2f50.py`, `tmp-sitoem-demux-2f50b.py`,
`tmp-cbd-2f50-hunt.py`.

| Axis | Evidence | Demux to `0x2f50`? |
| --- | --- | --- |
| SitOem carved `.so` types | Ping/Config/Thermal/Metrics/DeviceState/Traffic/Txas/Scone/Coex/Debug/DataFlow/DataValidation/Mch — **no** SimInit/SIM_* | **no** |
| SitOem immediates | MOVZ `#0x2f50` **0**; u16 LE `0x2f50` **0** | **no** |
| MAIN `[OEM][IPC]` protobuf | encode family = PERCALLSTATSKPI_IND / audio/RAT/band/… telemetry; **0** `[OEM][IPC]` strings contain `SIM_` | **no** |
| Catalog `SIM_*` ∩ protobuf OEM names | **empty** (45 catalog SIM_* incl. `SIM_INIT_REQ` `@0x6de740`) | **no** |
| Kernel CPIF | chardev pipe only; no protobuf→msgid demux in host path | n/a |
| vendor.img `/dev/oem_ipc*` | **only** SitOem→`oem_ipc0`, log helper→`oem_ipc1` | — |
| vendor `SIM_INIT_REQ` / `IpcTxSimInit` / `SimInitMessage` | **0** | — |
| libsitril | `MOVZ #0x2f50` table-init only; IoChannel=`umts_ipc0`; **no** oem_ipc / **no** builder | not emitter |
| factory `cbd` / `rild_exynos` | referenced in init.rc; **not** on live SaaiOS; **banned to start**; binary carve from rc path failed | unproven / banned |

**SitOem→`0x2f50`?** **No** — two separate OEM dialects (protobuf SitOem vs binary
SIM catalog). Do **not** soft-send Ping/Config as SIM_INIT.

**Alternate catalog-`0x2f50` emitter?** **Not found** under bans in factory
vendor/SitOem/libsitril/MAIN protobuf path.

**Live SIM_INIT?** **none** (frame still incomplete; no invent). **Bearer?** no.

**Next:** recover **binary catalog** OEM app header+2B body from MAIN
`oem_ipc_message_*` typed encode / stock capture (not SitOem protobuf), **or**
offline-extract factory `cbd` ELF properly and RE without starting it; then ONE
soft INIT. Same bans.

### 2026-10-01 — MAIN catalog encode island + carved cbd (no send)

**Live brief (COM13 / USB NCM):** modem_state=**ONLINE**;
card=PRESENT apps=1 **app=PIN(2)** pin1=2 remain=3; oem_ipc0 **OEM_RDWR_OK**;
**no** cbd/rild; rmnet0–2 **rx=0**; GetPsService ok; **no bearer**.

Scripts: `tmp-oem-frame-2f50-re.py`, `tmp-oem-encode-island.py`,
`tmp-carve-cbd-2f50.py` (+ carved ELFs under
`diagnostics/fw/cdma-hunt/factory-td1a-vendor/carved-cbd/`).

#### MAIN `@0x6de740` / `oem_ipc_message_*`

| Item | Result |
| --- | --- |
| Catalog `SIM_INIT_REQ` | body=`2` msgid=`0x2f50` meta=`0x10104` rsp=`0` `+0x18=4` |
| Cross-check `SIM_VERIFYPIN_REQ` `@0x6de874` | body=`10` msgid=`0x2f52` rsp=`0x2fa1` |
| Cross-check `SIM_INFO_REQ` / `SIM_STOP_REQ` | body=`2`; STOP also `+0x18=4` rsp=`0` |
| Encode-island `@0x6dfc00..` | **log metadata** only (msg string + `oem_ipc_message_utils.c` / `dispatcher.c` / `oem_sit_main.c` path ptrs) |
| Thumb CODE slots in island | telemetry/handler objects — **no** proven wire STR of msgid/len/token/body |
| Catalog xref `@0x32e4494` → `BL 0x42ca1d2e` | C++ message-object mutate (`STRB +8/+9`); **not** app wire encode |
| Bare `MOVW #0x2f50` | name-reg / log-id banks via `BL 0x420caeea` — **not** senders |
| USIM table `@0x10fb078` | handler `0x43909d49`; size word `0x10000` — **internal**, ≠ catalog body=2 |
| Typed `kMessageInfo->size` assert | present in rodata; **0** litpool xrefs recovered to wire path |

**App-layer header layout?** **No** (still unrecovered).
**2-byte body for `flags=2`?** **No** (contents unproven; zeros not assumed).

#### Factory `cbd` carve (offline; **not started**)

`/vendor/bin/cbd` path strings sit in init.rc text (no adjacent ELF). Real binary
carved via `/dev/umts_boot0` → **`carved-cbd-a45e000.elf`** (ELF64 AArch64,
~163KB, many `cbd:` / `S5300` strings).

| Check on carved cbd | Result |
| --- | --- |
| `/dev/umts_boot0`, `/dev/umts_ramdump0` | **yes** (boot/ramdump) |
| `/dev/oem_ipc*`, `SIM_INIT_REQ`, `MOVZ #0x2f50` | **0** |
| Other carved boot0-near ELFs | helpers / prior oem_ipc1 log ELF; **no** catalog `0x2f50` encoder |

**cbd encodes `SIM_INIT_REQ`?** **No** — boot/ramdump only (reconfirmed by carve).

**Live SIM_INIT?** **none** (no invent; frame incomplete). **Bearer verified?** no.

**Next:** need stock capture of catalog OEM wire on `oem_ipc*` **or** another
host binary that actually encodes msgid `0x2f50` (not cbd, not SitOem protobuf);
until then no soft write. Same bans (no start cbd/rild; no ATU MAIN; no BAR;
no 0x0704 spam).

### 2026-10-01 — CP RX consumer RE for `0x2f50` / `0x2f52` (no send)

**Live brief (COM13):** modem_state=**ONLINE**; card=PRESENT apps=1
**app=PIN(2)** pin1=2 remain=3; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild;
rmnet0–15 **rx=0**; **no bearer**.

Scripts: `tmp-rx-2f50-consumer.py`, `tmp-rx-2f50-demux-deep.py`.

#### RX / dispatch evidence

| Layer | Finding |
| --- | --- |
| Catalog `@0x6de740` | body_hint=`2` msgid=`0x2f50` meta=`0x10104` rsp=`0` `SIM_INIT_REQ` |
| Catalog `@0x6de874` | body_hint=`10` msgid=`0x2f52` meta=`0x10104` rsp=`0x2fa1` `SIM_VERIFYPIN_REQ` |
| USIM `<== SIM_INIT_REQ` `@0x10fb078` | handler codec `0x43909d49`; size_word=`0x10000` — **internal**, ≠ catalog body 2 |
| USIM `<== SIM_VERIFYPIN_REQ` `@0x10fbb00` | msgid=`0x2f52` rsp=`0x2fa1` size_word=`0xb` (≠ catalog body 10) |
| SIM_INIT handler body | builds internal codec state; **no** proven wire-header LDR of msgid/len/token |
| Bare `MOVW #0x2f50/#0x2f52` + nearby LDRH/B | name-reg / MOVT-VA false-friends; **no** RX demux that CMPs wire msgid |
| `[OEM][SIT] Received packet…` litpool | log-string island only; **0** MOVW+MOVT code refs to recv string |
| SIT-like parse cluster (LDRB+0 / LDRH+2 / LDRH+4) | **0** in OEM island / typed-IPC / catalog-xref bands |
| Soft SIT VerifyPin | umts `0x0201` hdr12+body26 — **≠** OEM `0x2f52` body 10 |
| Link | catalog SIM_* = **OEM** dialect (not umts SIT `0x02xx`; SitOem protobuf demux already closed) |

#### Kernel oem_ipc write→CP (live DT + prior CPIF)

| Item | Evidence |
| --- | --- |
| DT `io_device_13` `oem_ipc` | `attrs=0x2000` `ch=0x81` `fmt=0` `ch_count=8` `io_type=1` `link_type=1` |
| DT `umts_ipc` | same `attrs/fmt`; `ch=0xf5` `ch_count=2` |
| `ATTR_NO_LINK_HEADER` (`0x100`) | **not** set → `link_header=true` |
| Userspace write | **app payload only**; kernel prepends **EXYNOS 12B** (`sync=0xABCD`, len incl. hdr, ch=`0x81` family) |
| Sysfs iod | only `dev`/`uevent` — **no** runtime `link_header` toggle |

#### RX-derived frame layout?

**No** — not fully evidenced. Kernel EXYNOS wrap + catalog body_hint=2 are known;
**OEM app-layer header + 2B body contents** still missing.

#### Exact missing (no invent; unchanged blockers)

1. OEM **app-layer header** field order/size (SIT-12B reuse on `oem_ipc` still **unproven**)
2. Exact **2-byte body** for catalog `flags=2` (zeros unproven)
3. Token/seq/transaction rules
4. Which `oem_ipcN` (0..7 → ch `0x81`..`0x88`) carries **catalog** `SIM_INIT` (SitOem protobuf already owns `oem_ipc0` for a different dialect)

**Live SIM_INIT?** **none** (no send). **Bearer verified?** no.

**Next:** stock wire capture of catalog OEM on `oem_ipc*`, or recover header
parse from OEM RX code that consumes `[OEM][IPC] Message ID not found` /
preprocess paths (not the log litpool alone). Same bans.

### 2026-09-30 — FN_A vs LTE/SADR (re-confirmed)

Whole-MAIN BL scan: **FN_A callers = 2 only** (CDMA MEAS IND + CDMA TIMING_LATCH).  
`SADR_MEASURE_RSP` / L1TUNNEL → **STATUS_WRAP only** (READY eval), **not** Present=2.  
LTE/NR measure strings do **not** alias into FN_A. **No live LTE elicit** of Present=2.

## RE: what sets app_state PIN→READY / Present(+0xBF6)=2

| Writer | Site (MAIN B) | Gate / note |
| --- | --- | --- |
| Sole `SET_APP` READY(#5) | `0x14fb5c6` | after STATUS log `0x106a`, `LDRB +0xBF6` **CMP #2** |
| Sole `+0xBF6` STRB | `0x14fb380` | STATUS copy of Present trio `[r5,#0]` (STRH `#bf6` hits elsewhere = other structs) |
| STATUS trigger | STATUS_WRAP `0x14c6626` | **only 2 callers**: `MMCIF_L1LC_DSL1C_SADR_MEASURE_RSP` @`0x14b7ca8`; L1TUNNEL LTE RF @`0x1a3e6cc` — **LTE, not CDMA** |
| Sole reachable Present=2 | FN_A `0x14f6a16` | arg0==3 only; callers = CDMA TIMING_LATCH + CDMA MEAS IND |
| PresentObj ctor `[+#0]` | `0x1a552a4` | **MOVS#0 STRB** — init default **0**, not 2 |
| Present=0 clears near `#636c` | 6 sites incl. ctor | not “undoing a 2 default” |
| FN_B Present=2 | unreachable | Phase-2/CBZ dead on FDD path |
| pin1=VERIFIED(2) / Pin1Verified | **no** | STATUS never CMP pin1→#5; VerifyPin only sets Pin1V/`+0xBF5` |
| FCP DISABLED / preferred / EngMode | **no** STRB Present=2 | live-negated |
| SET#1 side getobj`#0x10` STRB=2 | `0x146ac48` | **not** PresentObj`#636c` (L1LC msg-handler + helper `0x1991838`) |
| HotSwap INSERT | DBT-only / SET#1 | **live-falsified** — ABSENT→PRESENT ≠ Present=2 / READY |
| Clean soft-bringup first GET | PIN / `notin` | no CardPower/EngMode/HotSwap; Present never was 2 |

### Stock EU / No-CDMA model (corrected 2026-10-01; Verizon proof)

```
PresentObj ctor ──► Present[0]=0 (0x1a552a4) — NOT default 2
         ▼
USIM INSERT ──► SET_APP#1 DETECTED (no Present gate; START_NETWORK allows #1)
         ▼
STATUS (SADR/L1TUNNEL) while GET∈{1,4}:
   Present==0 ──► SET#2 PIN     ◄── our soft-lock (live; clean-boot same)
   Present==2 ──► SET#5 READY   ◄── sole #5; same gate on Verizon No-CDMA MAIN
         ▼
While GET==PIN: STATUS will not re-copy Present / cannot SET#5
         ▼
camp needs START_NETWORK ∈{1,4,5} ──► blocked under PIN

Present=2 latch *found* in MAIN: FN_A (CDMA MEAS/LATCH) only.
Verizon AP1A also No-CDMA + same SET#5/Present==2 machine ⇒ FN_A cannot be
stock USIM READY. Prior "need CDMA RatMap / FN_A" framing = over-fit.
```

- **Init default:** Present=**0** (explicit ctor STRB). Hypothesis “default 2,
  our path clears to 0” = **FALSE** (RE + clean-boot live).
- **READY *evaluation*:** LTE `SADR_MEASURE_RSP` / L1TUNNEL → STATUS_WRAP (not FN_A).
- **Present=2 latch found:** still **FN_A-only** in MAIN scans — but that is
  **not** the stock No-CDMA USIM path (Verizon proof). Hole = unidentified
  USIM Present=2 / boot-init writer (SIM_INIT internal; not sit-stream).
- **HotSwap INSERT** does **not** store Present=2 (live-falsified).
- **Clean-boot first GET:** app=PIN pin1=1 `present_infer=notin` — same Present=0
  decision as post-HotSwap (HotSwap only changes pin1).
- **Stock-EU/Verizon gap:** how retail No-CDMA reaches Present=2 (or READY)
  without FN_A remains unexplained — **not** solved by CDMA UDL.
- **SADR AP path:** none signed (needs camp; camp denied while PIN).
- **pin1=2:** does not short-circuit to READY.
- **Missing signed SIT vs stock:** none (VerifyPin → re-poll `0x0200` only).

### Gate re-verification (2026-09-30)

Whole-MAIN scan: sole `SET_APP#5` @`0x14fb5c6` behind `LDRB +0xBF6` **CMP #2 / BNE**;
Present==1/3 → PUK/PERSO only. **Still ==2 only.** Re-validated same day after HotSwap
(`tmp-validate-present-infer.py`).

### 2026-09-30 — SET#5 / Present bypass hunt (exhaustive; **none**)

Scripts: `diagnostics/tmp-set5-bypass-hunt.py`, `tmp-set5-alt-entries.py` on MAIN B
`449eeab3…`.

| Hypothesis | Result |
| --- | --- |
| Alternate STATUS entries skip Present==2 | 4 BL targets in STATUS VA range: only real entry is STATUS_WRAP→prolog `0x14fb322`; mid-hits are false friends / mid-insn; paths that reach SET#5 still `LDRB +0xBF6` **CMP #2** |
| Different CMP for SET#5 | Sole `#5` BL @`0x14fb5c6`; gate OK |
| Non-STRB / wide / STM Present(+0xBF6) | STRB.W +0xBF6 **count=1** (STATUS copy); STRH/STR.W +0xBF6 **0**; no STM trio alias |
| STR.W/STRH touching +0xBF4/+0xBF5 | Bulk zero ctor @`0x19a539c`; enum-table STRH imm85 @`0x1816bf2` — **not** app_state READY; +0xBF5 site is Pin1V path |
| Wrong-object STRB=2 | PresentObj `MOVW #0x636c` + MOVS#2+STRB#0: **only FN_A** @`0x14f6a16`; getobj`#0x10` false-friend pattern **0** this pass |
| SET_APP callers skip Present | 14 BLs; sole `#5`; +0xBF4 STRB.W only **inside** SET_APP |
| Signed elicitation of bypass | **none** — no live try |

**Live one-liner (COM13, same turn):** CP `ONLINE`; soft-lock
`app=PIN(2) pin1=2 present_infer=notin_1_2_3`; `rmnet*` rx=0/tx=0; no rmnet IPv4.

### Stock-EU READY paradox — **unresolved under current MAIN RE**

Retail EU Pixel reaches READY without CDMA, but this signed MAIN B only latches
Present=2 via FN_A (CDMA MEAS/LATCH) and only SETs READY behind Present==2.
No alternate Present=2 writer, no SET#5 bypass, and no AP-signed elicit under
bans was found. Paradox stands; do **not** reseat hoping to restore Present=2.

**Remaining external / policy options only:**

1. External **signed** CP/MAIN/NV-defaults image with CDMA in SupportedRatMap (UDL)
2. Policy lift: EFS RW `TCS_CDMA_SUPPORT` / RatMap
3. Policy lift: stock `rild`/framework (banned today)
4. Policy lift: unsigned MAIN patch (UDL DONE historically rejected)

## FN_A / SupportedRatMap — signed vs blocked

| Lever | Verdict |
| --- | --- |
| `BuildSetPreferredNetworkType` `0x070a` LTE_ONLY(11) / LTE_WCDMA(12) | **signed, live-done** — preferred only; does **not** add CDMA to RatMap; does **not** make FN_A fire |
| CDMA / NR+CDMA preferred enums (sit-stream) | **signed opcode, but not a RatMap writer** — **do not live-try** |
| LTE_ONLY as unique FN_A blocker | **falsified** — RatMap absence blocks FN_A under 11 **and** 12 |
| Handover / CDT RAP flag | **none** |
| `TCS_CDMA_SUPPORT` / `DS_TCS_GV_CDMA_SUPPORT` | **NV/reg only** — EFS RW banned; **SIT `BuildNvRead/WriteItem` stubs** (no usable AP NV opcode on this sit-stream) |
| Alt signed A/B MAIN | both EU **No CDMA** RatMap; A RO same Present=2 gate; **no US/CDMA SKU** in tree |
| Unsigned MAIN patch / Present heap poke | **banned** / UDL DONE rejected |
| Physical HotSwap reseat | **live-falsified** — EDGE only; no READY/bearer |

**Chain:** NV/TCS → SupportedRatMap on `QM_MM_INIT` → EU `No CDMA in
SupportedRatMap` → no CDMA LATCH/MEAS → FN_A never → Present≠2 → no SET_APP#5 →
GET_APP stays PIN → START_NETWORK `{1,4,5}` denied.

**AP-reachable levers that force READY/Present:** **none** under bans.
`SET_APP` is CP-internal only (`0x19916d2`); `0x0200` GET mirrors `+0xBF4` only.

### Stock libsitril radio-on→READY vs ours (2026-09-30)

RE of `libsitril.so` + `sit-stream.so` (research tree hashes match RUNTIME):

| Stock order | id | Our soft-lock path |
| --- | --- | --- |
| GetRadioState | `0x0801` | sent |
| RadioPower ON | `0x0800` | sent |
| GetSimStatus | `0x0200` | sent |
| Get/SetPreferred | `0x070b`/`0x070a` | sent (11/12) |
| SetNetworkSelectionAuto | `0x0704` | sent |
| Reg poll | `0x0700`/`0x0701` | sent |
| VerifyPin if pin1 enabled | `0x0201` | sent A+AID → pin1=2, **app stayed PIN** |
| Adjacent: facility/ATR/och/slot/card/uicc/AllowData/EngMode | various | all already live |

**Missing on radio-on→READY critical path:** **none.** `OnGetSimStatusDone` /
`CheckAndAutoVerifyPin` do not emit a second SIT that latches Present=2.
`SET_APP` is CP-internal only.

**Never-sent empty GETs (exist in builders; not READY gates):**
`BuildSim3GPbCapa` `0x0245`, `BuildGetPreferredCallCapability` `0x0930`.
**Live ONE replay:** both **err0**; app stayed PIN; `present_infer=notin`;
rmnet rx=0; **no IPv4**. Tool: `diagnostics/stock-missing-gets`.

**Stock-only remainder under bans:** rild framework / NV-EFS RatMap
(`TCS_CDMA_SUPPORT`) / unsigned MAIN — **do not start rild; no EFS RW.**

## Honest remaining options toward bearer

SET#5-without-Present==2 hunt **closed** (none). CDMA-RatMap / FN_A-as-USIM-READY
framing **retracted** (Verizon No-CDMA proof). Soft-lock post-edge plumbing
ready; GOAL still incomplete until verified bearer.

| Option | Status |
| --- | --- |
| EFS RW / flip `TCS_CDMA_SUPPORT` → RatMap CDMA → FN_A | **banned**; also **not** stock No-CDMA USIM READY |
| SIT `BuildNvWriteItem` → TCS/RatMap | **stubs** — symbols only; no payload / no live |
| External signed CP with CDMA in RatMap (UDL) | **not found** — and **wrong goal** for No-CDMA USIM READY |
| Soft UDL CP2A / TD1A / Verizon AP1A radios | CP2A=EU No-CDMA; TD1A factory+OTA=g5300g; AP1A Verizon=g5300q+No-CDMA — **no UDL** |
| Unsigned MAIN patch (force SET#5 / Present=2) | **UDL DONE rejected** |
| Stock vendor `rild` / CBD (SIM_INIT / boot USIM) | **banned** — likely stock No-CDMA Present/READY prime |
| Further physical reseats | **falsified** — do not reseat for Present=2 |
| Invent SADR_MEASURE_RSP from AP | **no signed SIT**; needs camp anyway |
| AP PresentObj/`+0xBF6` poke | **dead** (heap; ATU/SHMEM open-bus) |
| Live RAM patch SET#5 `CMP #2` (MAIN DRAM) | **dead** — ATU open-bus; non-ATU maps ≠ MAIN; no write |
| Non-ATU CP DRAM via reserved-mem / BAR | **dead** — SHMEM≠MAIN; BAR ioremap panics |
| Replay more sit-stream empties (Pb/misc) | **0x0245+0x0930 live-negated** |
| Boot modem_a instead of B | ROADMAP slot-switch ban + **same** No-CDMA gate |
| SET#5 / aliased Present bypass on this MAIN | **falsified** |
| Force `0x0704` START_NETWORK while PIN | **CP err=2**; app stays PIN; no reg/rmnet — CP-enforced |
| Find non-FN_A Present=2 / USIM boot-init writer | **open** — `0x2f50` catalog dialect; SitOem **↛** demux; factory **cbd** carved = boot/ramdump only (no `0x2f50`); MAIN encode island = log metadata only; **app hdr + 2B body still missing**; INIT ≠ Present=2 |

Post-READY plumbing already on device: SetupDataCall `0x0600` len246,
GetDataCallList, APN file, chase pipeline — **unreachable** until app∈{1,4,5}.

**Watch:** leave one `tray-bearer-chase` PERSIST for observability only; do not
expect reseat to clear soft-lock.

```sh
# observability (HotSwap falsified as bearer path):
#   cat /data/saaios/var/tray-bearer.alive
#   tail /data/saaios/var/tray-bearer.log
cargo run -p saai-modemd -- soft-lock --app 2 --pin1 2 --present notin
cargo run -p saai-modemd -- post-edge --app 2 --pin1 2 --present notin
```

Success = IPv4 on `rmnet*` **or** rx>0 and tx>0. Soft-lock PIN+pin1=2 with rx=0
is **not** success.

## Dead ends (do not repeat)

| Path | Result |
| --- | --- |
| ATU OB2 / MAIN poke (SET#2 + SET#5 CMP) | open-bus / invisible (`ff`); no sticky write |
| Live DRAM patch READY gate `@VA 0x414f49ac` | ATU RO=`ff` — **not MAIN code**; patch skipped |
| Non-ATU DT/SHMEM/BAR phys (memremap/`phys_ro`) | IPC/PKTPROC real; **no** TOC/`02 20`; BAR ioremap **panics**; **no write** |
| SHMEM Present/GET_APP poke | FMT RX copies only |
| BTL SIM scan / PresentObj DRAM | SCAN NEGATIVE; **no poke** |
| PresentObj `getobj(#0x636c)` heap | AP-unreachable |
| `SET_APP` as AP SIT | falsified — CP-internal only |
| SetEngMode `0x0908` | live err0; no READY |
| Pin1Verified alone → READY | RO + live falsified |
| CardPower as tray reseat | no ABSENT→PRESENT; pin1 window only |
| **Physical HotSwap reseat** | **ABSENT→PRESENT + VerifyPin OK; app stayed PIN; no bearer** |
| Stock empties `0x0245`/`0x0930` | live err0; still PIN; no bearer |
| Force `0x0704` while PIN (Pin1Verified) | **CP err=2**; no camp/rmnet — CP-enforced |
| Soft UDL identical EU / g5300g TD1A | no CDMA-RatMap + incompatible — **do not UDL** |
| Soft UDL Verizon TD1A.221105.003 / AP1A.240505.005.A1 | TD1A=**g5300g**; AP1A=g5300q TOC but still **`No CDMA in SupportedRatMap`** — **do not UDL** |
| CDMA preferred `0x070a` | preferred ≠ RatMap; **no live** |
| Flip `TCS_CDMA_SUPPORT` | **EFS RW** — banned |
| SIT NvRead/NvWrite (`BuildNv*`) | **stubs** — do not invent ids/payloads |
| SADR / STATUS_WRAP inject | no signed AP path |
| getobj`#0x10` STRB=2 | L1LC timer object; not Present |
| modem_a slot switch | same EU No-CDMA Present=2; banned anyway |
| OemSim SIT `0x0208` SIM_IO STATUS | live SW9000; app stayed PIN; no bearer |
| OemSim `0x020c`/`0x020f`/`0x0247` as soft-init | same as prior APDU/OpenChannel — already live-negated |
| OemSim `0x0247`+`0x020f` SELECT+`0x0208` STATUS **with AID** (post-VerifyPin) | all SW9000; app stayed PIN pin1=2; no bearer |

### 2026-10-01 — BuildOemSimRequest RE + live SIM_IO 0x0208

**Live pre (COM13 / USB NCM):** ONLINE; radio=10; card=PRESENT apps=1
**app=PIN(2)** pin1=2 remain=3; data reg=0 tech=UMTS; rmnet rx=0; **no**
cbd/rild.

**RE (sit-stream `BuildOemSimRequest` `@0x806d0`, no invent):**

| RIL req (`w1`) | SIT id | Named builder | Purpose |
| --- | --- | --- | --- |
| **28** (`RIL_REQUEST_SIM_IO`) | **0x0208** | `BuildSimIO` | classic SIM_IO (STATUS/READ/GET_RESPONSE/…) |
| **114** (`SIM_TRANSMIT_APDU_BASIC`) | **0x020c** | `BuildSimTransmitApduBasic` | basic APDU |
| **115** (`SIM_OPEN_CHANNEL`) | **0x0247** | `BuildSimOpenChannelWithP2` | open channel **with P2** (not `0x020d`) |
| **117** (`SIM_TRANSMIT_APDU_CHANNEL`) | **0x020f** | `BuildSimTransmitApduChannel` | channel APDU |

`BuildOemSimRequest` is a thin remap: stamps SIT id + **12B** hdr, memcpy
caller payload (`len+12`). **Not** catalog OEM `0x2f50`.

`BuildSimIO` factory: id `0x0208`, length **0x23c**. Body: cmd@12,
fileid_lo@13, u16@14, path_len@16, path@17, p1@29, p2@30, p3@31,
data_len@32, data@34, pin2_len@546, pin2@547, aid_len@555, aid@556.
Command switch accepts `0xB0/B2/C0/D6/DC/F2`.

**Live try (ONE):** empty-default `SIM_IO` **STATUS** (`cmd=0xF2`, rest
zero). Response length=528 error=0 **SW=`9000`**. Post `0x0200`: still
app=2 pin1=2 remain=3. **No VerifyPin** (pin1=2, not EDGE window).
**No** bearer chase. **Bearer verified? no.**

### 2026-10-01 — post-VerifyPin AID-filled OpenChannel/SELECT/STATUS

**Hypothesis:** after VerifyPin, stock may `0x0247`+`0x020f`+`0x0208`
SELECT/STATUS before GET_APP leaves PIN; empty SIM_IO was insufficient.

**RE:** `OnVerifyPinDone` does **not** emit those SITs. `BuildSimIO` has
no `0xA4`; ADF SELECT is OpenChannel / channel APDU. Layouts match prior
factory decode (session@12; channel words @12..32; SIM_IO aid@555).

**Live ONE (COM13):** `0x0200` → `0x0247`(AID,P2=0) → `0x020f`(SELECT) →
`0x0208`(STATUS+AID) → `0x0200` → `0x020e`. All err0 SW=`9000`.
Post: app=**PIN(2)** pin1=**2** remain=3; data reg=0; rmnet rx=0.
**Bearer verified? no.** Tool: `diagnostics/tmp-post-verify-och-once`.

**Next:** OemSim umts_ipc soft-lock exit closed (empty **and** AID-filled).
Resume OEM catalog `0x2f50` wire recovery only — no invent; same bans.

### 2026-10-01 — OEM IPC preprocess RX / `Message ID not found` (no send)

**Live brief (COM13):** ONLINE; PRESENT apps=1 **app=PIN** pin1=2 remain=3;
oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild; rmnet rx=0; **no bearer**.

Scripts: `tmp-oem-preprocess-rx-re.py`, `tmp-oem-preprocess-rx-re2.py`.

| Finding | Evidence |
| --- | --- |
| msgid_nf / preprocess / not-REQUEST logs | DBT (`0xfecdba98`) → **`oem_ipc_message_dispatcher.c`** lines `0x51`/`0x57`/`0x62` |
| encode/decode/invalid logs | DBT → **`oem_ipc_message_utils.c`** |
| SIT receive log | DBT → **`oem_sit_main.c`** |
| MOVW/MOVT/LDR to those string or DBT-record VAs | **0** — DBT-indexed logging; litpool-alone RE insufficient |
| ASCII `preprocess_cb` | **gmetrics only** — false friend |
| Catalog `SIM_INIT` `@0x6de740` | body=2 msgid=`0x2f50` meta=`0x10104` rsp=0 **`+0x18=4`** (INIT-family class tag; not wire token) |
| App header + 2B body | **still unrecovered** |
| Kernel | unchanged — EXYNOS 12B wrap; userspace app payload |

**RX-derived full frame?** **no**. **SIM_INIT sent?** **no**. **Bearer?** **no**.

**Exact missing:** (1) app header layout (2) 2B body for flags=2 (3) token/seq
(4) which `oem_ipcN` for catalog INIT.

**Next:** stock catalog OEM capture **or** non-string dispatcher preprocess
xref (nanopb / catalog walk). Then ONE soft INIT. Same bans.

### 2026-10-01 — non-string dispatcher / catalog / nanopb (no send)

**Live brief (COM13 / NCM):** ONLINE; PRESENT apps=1 **app=PIN** pin1=2
remain=3; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild; rmnet rx=0; **no bearer**.

Scripts: `tmp-oem-dispatcher-nonstr-re.py`, `tmp-oem-dispatcher-nonstr-re2.py`.

| Finding | Evidence |
| --- | --- |
| `body_hint==2` cohort | `SIM_INFO` (`0x2f57` +18=0), `SIM_INIT` (`0x2f50` +18=4), `SIM_STOP` (`0x2f51` +18=4) |
| Catalog `+0x18` | **class/family tag** — INIT-family across SIM/CC/SMS/SS/SMREG/NS = **4**; PB bank = **5**; **not** wire token length |
| Catalog MOVW consumers | object helpers (`BL 0x2ca893e`); INIT STRB `#5→obj+8`; STOP STRB `#1→obj+8`; **no** TX wire stores |
| msgid→fn table in catalog island | **0** |
| `[OEM][PB]` nanopb | 3 decode/varint strings; **0** SIM/`0x2f50` overlap |
| DBT-record MOVW/lit to msgid_nf/preprocess | **0** (still DBT-indexed) |
| App header + 2B body | **still unrecovered** |

**Frame recovered?** **no**. **SIM_INIT sent?** **no**. **Bearer?** **no**.

**Next:** stock `oem_ipc*` catalog capture or non-cbd/non-SitOem host encoder.
Then ONE soft INIT. Same bans.

### 2026-10-01 — GET_APP parse re-validation + SitOem Ping (no `0x2f50`)

**Live brief (COM13 / NCM):** ONLINE; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild;
rmnet0–3 **rx=0**; **no bearer**.

#### GET_APP / `0x0200` parse vs libsitril APPSTATE

| Check | Evidence | Verdict |
| --- | --- | --- |
| Offsets | sit-stream `GetPinState`: PIN1→`#0x58`, PIN2→`#0x59`; remain PIN1→`#0x5a`; `Init` apps@`#14` app-base`#15`; libsitril `BuildRilCardStatusApplications` **ADD #63** stride | type@15 state@17 pin1@72 remain@74 **confirmed** |
| Enum | `covertAppStateToString` CMP `#1..#5`; `RIL_APPSTATE_PIN=2` `READY=5` | naming OK |
| Live bytes | `len=143` `apps=1` **type=2 (USIM)** **state=2 (PIN)** `pin1=2` remain1=3 | **not** type/state swap |
| Multi-app | `apps=1` only; no app1 slot | index OK |

**Parse bug?** **no** — confirmed soft-lock **PIN(2)** (type also 2 by coincidence).
`sit-sim-status.c` now prints type/state/pin1/remain so USIM≠PIN confusion is
visible. **No bearer chase** (app never left PIN).

#### SitOem Ping on `oem_ipc0` (protobuf dialect; ≠ catalog `0x2f50`)

Evidenced from carved SitOem ELF: `IpcMessageType` REQUEST=`1`; PayloadCase
ping=`5`; PingMessage request oneof=`1`; PingRequest string field=`1`;
userspace = raw protobuf (kernel EXYNOS 12B).

| Step | Result |
| --- | --- |
| ONE write len=11 | `08 01 10 01 2a 05 0a 03 0a 01 78` (tags/lengths only) |
| write | **ok** |
| RX len=9 | `08 02 15 00 00 00 00 20 05` — protobuf **type=RESPONSE(2)** |

**Channel alive?** **yes** (SitOem protobuf transport). **Does not recover
catalog `0x2f50` frame.** Shared EXYNOS link-header only (prior); app header
for catalog SIM_* still missing.

**Bearer verified?** **no**.

**Next:** still need catalog OEM wire on `oem_ipc*` (stock capture / non-SitOem
encoder). Same bans. Soft-lock unchanged.

### 2026-10-01 — SitOem schema exhaust + external `0x2f50` hunt (no send)

**Live brief (COM13 / NCM `172.31.7.1`):** ONLINE; PRESENT apps=1 **app=PIN**
pin1=2 remain=3; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild; rmnet rx=0;
wlan/usb IPv4 only; **no bearer**. ADB absent (MTP phantom); SSH pubkey denied.

Script: `diagnostics/tmp-sitoem-schema-exhaust.py` (+ `.out`) on carved
`carved-oemipc-25e7b000.so` (SitOem) / `carved-oemipc-cf64000.so` (log helper).

#### SitOem protobuf surface (complete type list)

`sit_ipc_message::*` Arena/`CreateMaybeMessage` + wrappers (no SIM family):

| Family | Types |
| --- | --- |
| Core | `IpcMessage`, `IpcMessageType`, PayloadCase via `initialMessageHeader` |
| Ping | `PingMessage`, `PingRequest` |
| Config | `ConfigMessage`, `ConfigRead/Write/Apply/VersionRequest`, `ConfigRead/VersionResponse` |
| Thermal | `ThermalMessage`, `ThermalRequest` |
| Metrics | `MetricsDataMessage`, `MetricsDataRequest`, `StatsAtomGetAtoms/Data Request/Response` |
| Device | `DeviceStateMessage`, `DeviceStateUpdateRequest` |
| Traffic / Txas | `TrafficStatsMessageModemData`, `TxasMessage`, `TxasRequest`, `TxasRead`, `TxasState` |
| Scone | `SconeActivityInputMessage`, `SconeRequest`, `Scone*IsValid` |
| Coex / Debug / Flow | `CoexMessage`, `BluetoothCoexInd`, `DebugMessage`, `DataFlowMessage`, `DataValidation*` |
| Ind / KPI | `NwCongestion*`, `MarginalCoverage*`, `RadioLinkConditionInd`, `PerCallStatsKpi*`, `MchMessage` |

**SIM/init/card/uicc protobuf msgs?** **NONE** (after excluding Ping⊃pin /
sitInitModem IPC bring-up / StatsAtom⊃sat false friends). Evidenced encodes
are Ping/Config/Thermal/Metrics/DeviceState/Traffic/Txas/Scone only —
**no** SIM encode path → **no live SitOem SIM try**.

#### External hunt (github / XDA / paste / project notes / pixel-mainline)

| Source | Result |
| --- | --- |
| github / XDA / paste queries for `SIM_INIT_REQ` `0x2f50` / oem_ipc wire | **no** complete frame (msgid `0x2f50`, body size 2) |
| `sit_ipc_message` / PayloadCase public dumps | **none** (unrelated IPMI/WebSocket hits) |
| pixel-mainline/modem (`cbd-lite`, `sit-smoke`) | SIT boot/smoke only — **no** oem_ipc / `0x2f50` |
| libsamsung-ipc / ShannonBaseband / FirmWire notes | classic SIPC / RE tooling — **not** Pixel catalog OEM INIT frame |
| In-tree prior RE outs | catalog meta only; app hdr + 2B body still missing |

**External complete frame?** **no** → **no** soft `oem_ipc0` catalog write.

**SIM_INIT sent?** **no**. **Bearer?** **no**. Soft-lock unchanged.

**Next:** still need stock catalog OEM capture / non-SitOem host encoder for
app header + 2B body. Same bans.

### 2026-10-01 — HARD WALL: catalog `0x2f50` encoder not in factory vendor (policy)

**Live brief (COM13 / USB NCM `172.31.7.1`):** modem_state=**ONLINE**;
`sit-sim-status query-sim-status` → card=PRESENT apps=1 **app=PIN(2)**
pin1=2 remain=3; radio_state=10; data_reg=0; oem_ipc0 **OEM_RDWR_OK**;
oem_ipc1–3 missing; **no** rild/cbd processes; rmnet* **rx=0**; usb0/
wlan IPv4 only — **no bearer**. ADB absent; SSH pubkey denied.

Scripts/artifacts: `diagnostics/tmp-vendor-2f50-host-hunt.py` (+`.out`),
`tmp-vendor-2f50-host-hunt2.py` (+`.out`), carves under
`diagnostics/fw/cdma-hunt/factory-td1a-vendor/` (SitOem, cbd, sitril).

#### Factory-td1a vendor offline RE (no daemon start)

| Binary / island | Opens | Catalog `0x2f50` / `SIM_INIT_REQ` | Verdict |
| --- | --- | --- | --- |
| `carved-oemipc-25e7b000.so` SitOem | `/dev/oem_ipc0` | MOVZ `#0x2f50` **0**; no SIM_* protobuf | SitOem protobuf dialect only (closed) |
| `carved-oemipc-cf64000.so` | `/dev/oem_ipc1` | **0** | log/modemstat helper |
| `carved-sitril-builder-0x25f54000.so` (~2.9MB) | `/dev/umts_ipc0` | MOVZ `#0x2f50` **0**; has `BuildOemSimRequest` | **SIT** umts dialect ≠ OEM catalog |
| research `libsitril.so` | `umts_ipc0` | MOVZ `#0x2f50` **only** `@0x11f300` table-init | registry, **no** packet builder |
| carved `cbd` (`a45e000` + peers) | `umts_boot`/`ramdump` | **0** oem_ipc / **0** `0x2f50` | boot only |
| `carved-cbd-1827b000.elf` (mislabel; sitril-like) | `umts_ipc0` | `SIM_INIT` string = FILE_UPDATE err text only; MOVZ **0** | not catalog encoder |
| vendor.img whole | — | `SIM_INIT_REQ` **0**; `IpcTxSimInit` **0**; `SimInitMessage` **0**; ASCII `0x2f50` **0** | no name-level encoder |

**Encoder SO/symbol for catalog OEM `SIM_INIT_REQ` / msgid `0x2f50`?**
**Not found.** No offline encode path that both opens `oem_ipc*` and builds
the catalog frame. SitOem≠SIM; OemSim/`BuildOemSimRequest`≠OEM `0x2f50`;
cbd≠SIM_INIT. **No invent bytes → no one-shot sender → no live write.**

**Live SIM_INIT?** **none**. **App after?** still **PIN(2)**. **Bearer?** **no**.

#### Policy exception (needed to progress under current bans)

Under bans (no start rild/cbd; no invent frame), catalog soft-INIT is
**blocked**. Propose one explicit exception (pick one):

1. **Capture-only stock rild** — brief stock Android (or pull
   `/vendor/bin/hw/rild_exynos` + deps onto a disposable rootfs), allow
   **start rild once** solely to capture the first catalog OEM write on
   `oem_ipc*` (tcpdump/strace/iod dump), then stop; feed capture into a
   minimal one-shot (still no long-running daemon on SaaiOS).
2. **External wire dump** — accept a third-party/stock log of the exact
   userspace bytes for msgid `0x2f50` (app header + 2B body) from a matching
   Shannon OEM IPC dialect.
3. **Keep ban** — accept soft-lock as terminal for ONLINE+bearer on this
   EU No-CDMA image without CDMA-RatMap / EFS / unsigned MAIN.

Until (1) or (2), do **not** soft-write SitOem Ping/Config as fake SIM_INIT,
do **not** invent 2B body/zeros, do **not** spam `0x0704` / ATU / BAR.

### 2026-10-01 — alternate chicken-egg breakers (L1/scan + 0x020a) — both DEAD

**Live brief (COM13 / USB NCM `172.31.7.1`):** modem_state=**ONLINE**;
`sit-sim-status` → PRESENT apps=1 **app=PIN(2)** type=USIM pin1=2 remain=3;
radio_state=10; data_reg=0 tech=UMTS; GetPsService err0 byte12=1;
rmnet0–5 **rx=0 tx=0**; usb0/wlan IPv4 only; **no** rild/cbd. Soft-lock
unchanged. Scripts: `diagnostics/tmp-l1-scan-sit-re.py` (+`.out`),
`tmp-020a-facility-re.py` (+`.out`).

#### 2) Signed NET/RADIO/L1 SIT → measure/camp/SADR_MEASURE_RSP w/o START_NETWORK?

| Lever | Evidence | Live? |
| --- | --- | --- |
| sit-stream `SADR` / `Measure` builders | **absent** (string hunt 0) | n/a |
| STATUS_WRAP callers | still **2 only** (SADR_MEASURE_RSP + L1TUNNEL) — L1-internal | no AP inject |
| `BuildQueryAvailableNetwork` `0x0706` | signed empty GET; **prior live err=2** under soft-lock | **do not re-spam** |
| `BuildStartNetworkScan` / Stop / Cancel | exist; Start needs RAS payload (not empty-safe); **no** evidence they post SADR_MEASURE_RSP or bypass `GET_APP∈{1,4,5}` | **no try** |
| `BuildSetNetworkSelectionManual` `0x0705` | signed; same family as auto/`START_NETWORK` | **no try** (PIN gate) |
| `BuildTriggerEmergencyNetworkScan` / MicroCell | incomplete / no SADR link evidenced | **no try** |
| GapMeasure / “Not camped on any frequency” | producer needs camp → START_NETWORK → app∈{1,4,5} | chicken-egg holds |

**L1/scan live sequence?** **none** — no evidenced signed SIT that triggers
`SADR_MEASURE_RSP` / camp **without** START_NETWORK and **without**
`GET_APP∈{1,4,5}`.

#### 3) `0x020a` facility SET (pin1 DISABLED path)

| Item | Evidence |
| --- | --- |
| Builder | `BuildSimSetFacilityLock` id **`0x020a`** len **72**; SC@12, **lock_mode@13**, pwd_len@14, digits@15+, class@54=7, AID@55+ |
| Prior live enable (mode=1) + cand A | **err=2**; pin1 stayed **2**; remain 3 — already closed |
| GET `0x0209` empty-pwd | facility **unlocked** (status byte13=0) historically |
| Disable (mode=0) | stock `setIccLockEnabled(false)` needs **PIN password** — no empty-pwd accept documented; wrong pwd may burn remain |
| pin1 DISABLED(3) → READY? | **no** — STATUS SET#5 still Present==2 only (ROADMAP/RUNTIME) |

**Safe documented payload to move pin/app without secrets logging?** **No.**
**0x020a live try this turn?** **none.**

#### App / reg / bearer after

Unchanged: app=**PIN(2)** pin1=2; data_reg=0; rmnet rx/tx=0; **bearer
verified? no.**

#### Policy ask (strengthened — soft-lock terminal under bans)

Both alternate breakers **dead**. Catalog `0x2f50` encoder still unrecovered;
OemSim closed; GET_APP confirmed PIN; Present=2/FN_A paradox stands.
**Pick one explicit exception to progress:**

1. **Capture-only stock rild** — start rild **once** only to capture the first
   catalog OEM write on `oem_ipc*` (app header + 2B body for msgid `0x2f50`),
   then stop; feed into a minimal one-shot (no long-running daemon).
2. **External `0x2f50` wire dump** — matching Shannon OEM IPC userspace bytes
   (header + body) from stock/third-party capture.
3. **Keep ban** — accept ONLINE+bearer as **unreachable** on this EU No-CDMA
   image without EFS/unsigned MAIN/CDMA-RatMap.

Until (1) or (2): no invent `0x2f50`; no start rild/cbd; no ATU poke; no BAR
`0x81400000`; no `0x0704` spam under PIN; no further L1/scan / `0x020a` hopes.

### 2026-10-01 — armed for external `0x2f50` frame (no send yet)

**Live try this turn:** none (no frame; ban on starting rild/cbd holds).

**Tooling (sources only):** diagnostics now ship a one-shot path so bearer
chase is immediate once an evidenced catalog OEM frame exists:

| Piece | Path | Role |
| --- | --- | --- |
| Inject | `os/targets/panther/diagnostics/oem-ipc-inject.c` | Write operator frame once to `oem_ipcN`; refuse empty/all-zero; **no** default `0x2f50` |
| Chase | `os/targets/panther/diagnostics/post-init-chase.sh` | Optional inject → poll GET_APP `{1,4,5}` → `CHASE_ONCE` on `tray-bearer-chase.sh` |
| Capture recipe | `os/targets/panther/diagnostics/OEM-IPC-CAPTURE.md` | Policy-gated capture-only rild outline; **do not run now** |
| Chase hook | `tray-bearer-chase.sh` `CHASE_ONCE=1` | Skip tray-watch; unlock + bearer pipeline once |

**Armed?** **Yes** — inject+chase ready. **Sendable `0x2f50`?** **Still no**
without external/capture frame. **Bearer verified?** **no** (expected).

**Next:** (1) policy grant for capture-only stock rild **or** (2) drop in an
external catalog OEM wire dump → `post-init-chase.sh --frame …`.

### 2026-10-01 — stock vs soft CP bring-up + broader vendor hunt (terminal)

**Live brief (COM13 / USB NCM `172.31.7.1`):** modem_state=**ONLINE**;
`sit-sim-status` → card=PRESENT apps=1 **app=PIN(2)** pin1=2 remain=3;
`/dev/umts_ipc0` + `/dev/oem_ipc0` present; **no** rild/cbd processes;
rmnet0 rx=0 tx=0; **no bearer**. ADB absent; SSH pubkey denied. Inject
tools now on device: `/data/saaios/bin/oem-ipc-inject`,
`post-init-chase.sh` (alongside existing `sit-sim-status` /
`tray-bearer-chase.sh`).

Scripts: `diagnostics/tmp-vendor-broad-2f50-hunt.py` (+`.out`),
`tmp-vendor-movz-2f50-owners.py` (+`.out`).

#### Stock CP bring-up vs soft CPIF/handover (mailbox / stages / SIM_INIT)

| Axis | Stock (Android + cbd/rild) | Soft (probe-handover) | Soft skip elicitible via signed SIT/CPIF **without** rild/cbd? |
| --- | --- | --- | --- |
| POWER_ON → START → HANDOVER → preamble → UDL → FIN → COMPLETE | cbd + CPIF | same ioctl path; ONLINE proven | **parity** — already done |
| `sim/ds_detect` mailbox | dual-SIM module param (live=2) | same | **no** — prior pulse inert for Present/`+0xBF6` |
| Boot-stage mailbox / united_status | IRQ/ctrl; `ds_det` at bring-up only | same | **no** — not SIM_INIT / Present |
| Post-ONLINE SIT radio-on→READY | rild: `0x0800`/`0x0200`/preferred/`0x0704`/reg; VerifyPin if needed | same SITs already live (incl. missing empties `0x0245`/`0x0930`) | **no remaining signed SIT gap** |
| `USIM_WAIT_FOR_INIT_REQ` / `SIM_INIT_REQ` | AP OEM catalog msgid **`0x2f50`** on `oem_ipc*` (not SIT) | soft never emits; sitril only re-polls SIT `0x0200` after VerifyPin | **no** — no SIT/mailbox alias; OEM frame unrecovered |
| CardPower / HotSwap / EngMode | stock may cycle UICC | already live-falsified | closed |

**Missing soft boot/init step (signed, no rild/cbd)?** **None evidenced.**
The only soft-path gap that still blocks READY under bans is the unrecovered
catalog OEM `0x2f50` write (or Present=2 FN_A/CDMA which EU RatMap blocks).

#### Broader vendor carve hunt (not just cbd/sitril/SitOem)

Full `factory-td1a-vendor/vendor.img` (665MB) needle + MOVZ map:

| Needle | Count |
| --- | --- |
| `SIM_INIT_REQ` / `IpcTxSimInit` / `SimInitMessage` / ASCII `0x2f50` | **0** |
| `/dev/oem_ipc0` / `oem_ipc1` ELF islands | SitOem protobuf (`0x25e7b000`) + log helper (`0xcf64000`) only |
| Global AArch64 `MOVZ #0x2f50` | **29** sites across ~6 ELF islands — **all** `oem_ipc=False`, `SIM_INIT_REQ=False` |
| `oem_ipc` ∧ `MOVZ #0x2f50` encoder candidate | **NONE** |
| research `libsitril.so` (2.3MB) | `MOVZ #0x2f50` **1** @`0x11f300` table-init; `umts_ipc` only; **no** oem_ipc |

**Other vendor encoder bin?** **No.**

#### Live try / bearer

**None** — no new signed soft-boot lever to try. Soft-lock unchanged.
**Bearer verified?** **no**.

**Next:** policy (1) capture-only stock rild once **or** (2) external
catalog OEM `0x2f50` wire dump → `oem-ipc-inject` + `post-init-chase.sh`.
Injector armed on device; still refuses invent/empty/all-zero.

### 2026-10-01 — post-kernel EXYNOS capture via SitOem Ping (no `0x2f50`)

**Live brief (COM13 / USB NCM `172.31.7.1`):** modem_state=**ONLINE**;
oem_ipc0 **OEM_RDWR_OK**; **no** rild/cbd; app=**PIN(2)** pin1=2 remain=3;
rmnet0–5 **rx=0**; **no bearer**. ADB absent; SSH pubkey denied.

**Method:** kprobe `exynos_build_header` (+0x50 dump of `buff`) around ONE
SitOem Ping on `/dev/oem_ipc0` (protobuf only; no catalog invent). Also
confirmed `vfs_write` sees userspace 11B unchanged.

#### Captured post-kernel outer layout (EXYNOS 12B + app)

Two Pings (frame_seq **4** then **5**); same shape:

| Off | Hex (Ping #2) | Field |
| --- | --- | --- |
| 0–1 | `CD AB` | sync `0xABCD` LE |
| 2–3 | `05 00` | frame_seq (increments) |
| 4–5 | `00 C0` | frag_cfg=`0xC000` (=49152; matches probe `cfg=`) |
| 6–7 | `17 00` | total len=`23` = 12 + userspace `count=11` |
| 8 | `81` | channel `0x81` (oem_ipc0; matches DT) |
| 9–11 | `00 00 00` | ch_seq / pad (live 0 on both Pings) |
| 12+ | `08 01 10 01 2a 05 0a 03 0a 01 78` | **passthrough** SitOem protobuf |

Userspace write = app payload only; kernel prepends the 12B. msgid/token for
Ping live **inside protobuf** (`type` tag1=`08 01`, `token` tag2=`10 01`) —
**not** in the EXYNOS header.

#### Catalog `0x2f50` share this outer header with msgid at fixed offset?

| Claim | Evidenced? |
| --- | --- |
| Same EXYNOS 12B wrap for any `oem_ipc*` `link_header` write | **yes** (kernel `exynos_build_header`; ch from iod) |
| Msgid at a fixed offset in that outer header | **no** — EXYNOS has sync/seq/cfg/len/ch only |
| Catalog REQUEST app header + `flags=2` 2-byte body | **still missing** (zeros unproven) |
| SitOem Ping outer/protobuf ⇒ catalog SIM_INIT | **no** (dialects already closed) |

**Enough for ONE soft `0x2f50`?** **No** — gaps remain. **Not sent.**
**SIM_INIT / bearer?** none / **no**.

**What Ping taught:** post-kernel wire = EXYNOS12 + opaque userspace bytes;
recovering catalog still needs the **app-layer** frame (header+2B body), not
another Ping/Config. Tools: `tmp-sitoem-ping-once`, host helpers
`tmp-run-exynos-hdr-probe.ps1` / lean vfs probe.

**Next:** unchanged — capture-only stock rild **or** external catalog OEM dump
→ `oem-ipc-inject` + `post-init-chase`. Same bans.

### 2026-10-01 — public FMT / shannon-ipc mapping (no send)

**Live brief (COM13):** ONLINE; app=PIN pin1=2 remain=3; oem_ipc0 RDWR OK;
no cbd/rild; rmnet rx=0; **no bearer**.

Public research (osmocom/Replicant `ipc_fmt_header` 7B; morphis `sec.h`;
Comsecuris/Hardwear SHM notes; AOSP `oem_ipc[0-7]` sepolicy; our EXYNOS Ping
capture + catalog stride-28):

| Closest public match | Binding to catalog `0x2f50` / body=2 |
| --- | --- |
| Classic FMT `group:index` | Hypothetical `0x2f:0x50` — **unproven**; public SEC is `0x05xx`, no `SIM_INIT_REQ` |
| Soft SIT 12B | **Falsified** as OEM dialect (msgid/body twin mismatch) |
| EXYNOS 12B | Evidenced wrap only; **not** app header |
| SitOem protobuf | **≠** catalog |

**SENDABLE?** **No** — app header bytes + 2B body CONTENTS still missing. **Not
sent.** Script: `diagnostics/tmp-public-fmt-2f50-map.py`.

### 2026-10-01 — deep const-build scan still no encoder

Live: ONLINE; soft-lock **PIN**/pin1=2; oem_ipc0 RDWR; no cbd/rild; **no bearer**.

Broader than prior bare-`MOVZ #0x2f50` hunts across all factory-td1a **carved**
ELFs/SOs + full `vendor.img` + research `libsitril.so`:

| Pattern | Hit that yields catalog encode? |
| --- | --- |
| MOVZ+MOVK / MOVN / ORR imm of `0x2f50` | **no** |
| rodata LE `50 2f` near oem/SIM/IPC | **no** encoder |
| oem_ipc open/write islands ∩ msgid `0x2f50` | **no** (SitOem/log only) |
| table-driven msgid → write(`/dev/oem_ipc*`) | **no** |

**Encoder found?** **No.** **Soft send?** **No** (no invent). **Bearer?** **No.**

**Policy need:** stock `oem_ipc*` catalog capture or external evidenced frame
(`OEM-IPC-CAPTURE.md`) before any inject. Injector remains armed.

Scripts: `diagnostics/tmp-vendor-deep-2f50-constbuild.py`,
`tmp-vendor-deep-2f50-constbuild-vendor.py`.

### 2026-10-01 — non-vendor factory partitions: still no catalog encoder

**Live brief (COM13):** modem_state=**ONLINE**; card=PRESENT apps=1
**app=PIN(2)** pin1=2 remain=3; oem_ipc0 **OEM_RDWR_OK**; **no** cbd/rild;
rmnet* **rx=0**; usb0=`172.31.7.1` / wlan IPv4 only — **no bearer**.
ADB absent. Injector remains armed (no frame → **no send**).

**Source:** factory `panther-td1a.221105.001` → inner
`image-panther-td1a.221105.001.zip` (extracted under
`diagnostics/fw/cdma-hunt/factory-td1a-images/`). Vendor already closed
prior turn; this pass is **beyond vendor**.

#### Partitions scanned

| Image | Size | `SIM_INIT_REQ` / `IpcTxSimInit` / `SimInitMessage` | `/dev/oem_ipc` / `oem_ipc0` | ASCII `0x2f50` | Notes |
| --- | --- | --- | --- | --- | --- |
| `system.img` | 872MB | **0** | **0** | **0** | AOSP `android.hardware.radio*-V1-ndk.so` name bank only; **0** `com.android.hardware.radio` APEX package string; **0** radio `.apex` path |
| `product.img` | 2.9GB | **0** | **0** | **0** | chunk + full string scan; no encoder |
| `system_ext.img` | 366MB | **0** | **0** | **0** | `rild_exynos` string **2×** in rilExternal symbol bank (not `/dev/oem_ipc` writer); Samsung `IOemSlsiRadioExternal` HIDL names — **≠** catalog OEM |
| `system_other.img` | 25MB | **0** | **0** | **0** | empty for needles |
| standalone radio APEX in image zip | — | — | — | — | **none** (no `*.apex` members); APEX content if any is inside FS images |

Scripts: `diagnostics/tmp-extract-factory-nonvendor.py` (+`.out`),
`tmp-nonvendor-apex-radio.py` (+`.out`).

**`oem_ipc` ∧ catalog `0x2f50` / `SIM_INIT_REQ` encoder?** **No** in
system / product / system_ext / system_other. Bare `MOVZ #0x2f50` hits in
large images are false-friends (no co-located oem_ipc / SIM_INIT).

**Live SIM_INIT?** **none** (no invent). **Bearer verified?** **no**.

#### Policy (unchanged; strengthened)

Non-vendor factory partitions do **not** supply the missing catalog OEM
app header + 2B body. Soft ONLINE+bearer still needs an explicit exception:

1. **Capture-only stock rild** once → `oem_ipc*` userspace bytes → stop →
   `oem-ipc-inject` + `post-init-chase` (see `OEM-IPC-CAPTURE.md`)
2. **External evidenced `0x2f50` wire dump** (matching Shannon OEM dialect)
3. **Keep ban** — accept soft-lock terminal on this EU image under current
   bans (no EFS / unsigned MAIN / CDMA-RatMap / invent frame)

Do **not** start cbd/rild under ban; do **not** invent `0x2f50` body/zeros;
do **not** treat AOSP `android.hardware.radio*` or `IOemSlsiRadioExternal`
as catalog `SIM_INIT_REQ`.

### 2026-10-01 — CP USIM self-init (passive) **falsified**

Hypothesis: without AP `0x2f50`, CP might self-init USIM to READY if AP
avoids EngMode/CardPower/OemSim/`0x0704`/VerifyPin storms.

| Step | Result |
| --- | --- |
| Soft sysrq → `probe-handover-clean` only | ONLINE; log clean (no early `0x0200`) |
| Passive ~5 min (`0x0200`+reg+rmnet @~20s) | **16/16** `app=PIN(2) pin1=1 present_infer=0` |
| READY/{1,4,5}? | **never** |
| Bearer? | **no** (rmnet rx=0, reg=0) |

Script: `diagnostics/passive-selfinit-wait.sh`. **Verdict:** self-init
**not** observed. Soft catalog `0x2f50` policy need **unchanged**.

### 2026-10-01 — overnight live recheck (COM13 / NCM)

**Live one-liner:** ONLINE; PRESENT apps=1 **app=PIN(2)** pin1=**1** remain=3;
radio=10; data_reg=0; oem_ipc0 RDWR OK; **no** cbd/rild; rmnet* rx=0;
usb0=`172.31.7.1` / wlan IPv4 only — **no bearer**.

Overnight reset vs prior pin1=2 chicken-egg: pin1 back to NOT_VERIFIED(1)
(VerifyPin window again). Inject tools still on device
(`/data/saaios/bin/oem-ipc-inject`, `post-init-chase.sh`,
`tray-bearer-chase.sh`). **No frames sent.** Soft-lock polish:
`saai-modemd soft-lock` / `post-edge` + chase `SOFT_LOCK_STATUS` now report
`cpif_caps_exercised=yes` + `blocker=waiting_external_catalog_oem_0x2f50…`.

**Bearer verified?** **no**. Still blocked on external catalog `0x2f50` /
capture-only policy.


### 2026-10-01 — VerifyPin A+AID (pin1=1) opens READY (live)

**Breakthrough (reproduced x2):** with overnight pin1=**1**, VerifyPin A+AID
**without CardPower** → app **READY(5)**, pin1 **2**, remain unchanged.
Late unsolicited `0x0201` err0 + `0x0200` app=5; also saw `0x0210`,
`0x4604`, `0x4602`, `0x0303`, `0x000d` (metadata only).

**Post-edge:** `0x0704` auto selection **err=0** under READY (falsifies
prior "0x0704 always NACK while PIN" as the permanent camp gate). AllowData /
LTE preferred err0. **Still** `registration_raw=0`, rmnet rx=0, no IPv4.

**SetupDataCall:** deferred — on-device APN `internet` fails dotted
`apn_usable` check; need operator APN (do not invent).

**Chicken-egg update:** GET_APP PIN soft-lock is **not** immutable — pin1=1
VerifyPin A+AID reaches READY. Prior "Present=2 / FN_A only path to READY"
framing is **too strong** for this live path. Goal still incomplete:
**no verified bearer**.

**Bearer verified?** **no**. Still blocked on **camp/reg** (and usable APN
after camp), not on PIN→READY.

### 2026-10-02 — stock stage-1 RE + `sgc-seq-once`: `0x093f` is a no-op

RE'd the stock `MiscService::OnRadioAvailable` (TD1A `libsitril` `efcca0d5`):
socket-0 stage-1 order is `0x090b`→**`0x093f`**→`0x0404`→`0x4605`; we had been
sending only `0x0404`. Decoded `0x0706` err **2** = **`RCM_E_GENERIC_FAILURE`**
(generic refusal, not the reg-ordering code 9). Added a 4th mutually-exclusive
owner mode `SAAIOS_SGC_SEQ_ONCE` (shared early machinery under new
`SAAIOS_SGC_EDGE`) that writes `0x093f` (13 B, payload 0 single-SIM) then
`0x0404` on the `0x0803`→`0x0802`-raw-0 edge; all 5 modes build `-Werror` + pass
self-test (ARM64 owner `12c07414`, probe `651515c8`). Live over COM13 (fresh
boot, guarded RO-persist handoff, CP→ONLINE): SGC accepted (`error_raw=0`) but
the CP returned **no ACK to `0x093f`**, and the +60 s snapshot was identical to
the early-SGC run (radio ON, voice/data `registration_raw=0`, signal
`mask_low7=0`). All `rmnet0–29` `rx=0 tx=0`, no IPv4.

**Bearer verified?** **no**. `0x093f` ruled out. Next stage-1 delta =
early camp-on **`0x0800`** (`NetworkService` `TrySetRadioPower(10)` →
`OnRequest(RADIO_POWER=23)`); needs the exact `DoRadioPower`/`BuildRadioPower`
body recovered (no guessed body) before building/running a `0x0800` candidate.

### 2026-10-02 — `sgc-camp-once`: `0x0800` accepted, START_NETWORK, zero signal

Recovered the exact `0x0800` body from `TrySetRadioPower(10)`→`DoRadioPower`→
`BuildRadioPower(1,0,0)`: 18 B, power word `+12 = 2` (ON), flags `+16/+17 = 0`
(derived, not guessed). Added mode `SAAIOS_SGC_CAMP_ONCE` sending
`0x093f`→`0x0404`→`0x0800` on the early edge (6 modes build `-Werror` + pass
self-test; ARM64 owner `0597553d`, probe `7a2914d4`). **Live:** the `0x0800`
was **accepted** (`error_raw=0`) and drove `radio_state_raw=2 = START_NETWORK` —
the modem began an active scan (`0x0906` ×47), which earlier runs never reached.
But `signal mask_low7=0` (no cell), `registration_raw=0`, all `rmnet rx/tx=0`,
no IPv4. **Bearer verified? no.**

**RF pivot.** Active search + zero signal ⇒ RF/NV precondition, not ordering.
Across every camp boot the modem drives `umts_rfs0` with the protected-NV
sequence **cmd 7 unprotect(state 3) → cmd 3 op-status → cmd 6 io_write @
`0x02e406`** (pinned in `rfs-error-probe.c`), which the camp owner never grants
(`rfs_responses=none`). The RIL gates RF on NV readiness
(`RCM_E_NO_RF_CALIBRATION_INFO`, `SIT_PWR_RADIO_SIM_STATE_NV_NOT_READY/READY`,
`GetRfCalDate`). **Most probable precondition:** the protected-NV RFS write must
be granted (to the existing quarantine, never original EFS) for the modem to
mark NV/RF-cal ready and produce signal. **Next action:** one owner holding
`ipc0`+`rfs0` that serves the `cmd 7/3/6` sequence into the quarantine
(`modem-rfs-one-grant-owner.c` / `modem-rfs-full-quarantine-owner.c`; fresh RO
EFS provenance check first) **and** issues `0x093f`→`0x0404`→`0x0800` on the
edge, then watch `mask_low7`/`0x0700`. Prereq: factory `cmd-6`-after-state-3
reply shape from stock `rfsd` (`58d7f885…`); do not invent it or write EFS.

## 2026-10-02 — combined owner: NV handshake COMPLETES+ACKed; still no registration

Recovered the factory reply shapes from `rfsd-cp2a` (`58d7f885…`) by
disassembly (responder `0x0f230`): 16-byte reply `{u16 cmd=3, u16 seq_echo,
u32 len=8, u32 status(0=ok), u32 state=3}` — confirms `status_7`/`final_status`
with no invented bytes. Built the combined `SAAIOS_RFS_CAMP` owner (into
`modem-rfs-full-quarantine-owner.c`): serves the `cmd 7/3/6` protected-NV
sequence to the quarantine copy (RO EFS provenance gate, never EFS RW) **and**
dispatches `0x093f`→`0x0404`→`0x0800` on the radio edge. Reproducible ARM64
owner `7a88e30b…`, probe `e32538e8…`; host + on-device `self-test` PASS.

**Live (fresh boot, guarded handoff, CP ONLINE):** the owner served the full
189446-byte protected-NV transfer to quarantine and sent the gated final
success ACK (`final_ack_sent=1`, +7.7 s) — the RFS handshake that was never
completed before. Camp `0x0404` and `0x0800` (radio power on) accepted
(`error_raw=0`); `0x093f` silent no-op. **But** no `0x0700`/`0x0701`
registration, `rmnet0` rx/tx = `0/0`, no IPv4 at +180 s. The passive observer
read `card_raw=0 apps=0` and its settled SIM GET timed out (self-poison), so
signal strength is unread. **Next:** extend the combined owner with a
signal-strength GET + sustained non-self-poisoning radio/registration trace and
a SIM-readiness drive, to test whether `mask_low7` goes non-zero now that the NV
write completes. Bearer verified: **no**.

## 2026-10-02 — SIM READY + signal + NV landed, yet CS REG_DENIED (CP-config)

Combined owner extended with an active, non-self-poisoning prober (drives SIM via
`0x0210 → 0x0200`, adds `0x0900` signal + voice/data registration polling across
the settle window). Owner `221cf0d2…`, probe `e32538e8…`, non-camp owner
unregressed `7d4b6d6d…`; host + on-device self-test PASS, `-Werror`.

Live (COM13, fresh `sysrq` boot → guarded RO-persist handoff): NV handshake
completes + ACKed; camp `0x0404`/`0x0800` accepted; prober rode through a
+10…+49 s GET-timeout storm (old owner self-poisoned here) and captured the
settled state:

| field | settled | meaning |
| --- | --- | --- |
| SIM | `card_raw=1 apps=1 app_state_raw=5` (50×) | **READY** |
| signal | `mask_low7=2` | **non-zero** (was 0) |
| voice `0x0700` | `registration_raw=3 reject_raw=0` (49×) | **REG_DENIED**, local/internal (no NAS cause) |
| data `0x0701` | `registration_raw=0 tech_raw=3` (49×) | **NOT_SEARCHING** (UMTS) |
| rmnet0 | rx=0 tx=0, no IPv4 | no bearer |

This **refutes** the 2026-09-30 "MM denies because `app≠READY`" hypothesis: app
is READY(5), NV landed, signal present — and CS is still internally denied, PS
still not searching. **Verdict: CP-config / missing real-EFS RF-cal·provisioning,
not environmental** (signal presence rules out dead antenna/no cell; local deny
rules out network rejection; quarantine never writes real EFS, so the CP gains no
config). Next unmet precondition: CS/PS registration. Next action: RE the CP
registration gate (`SIT_REG 0x20d1afa`; PresentObj `#636c +0xBF6`) — no EFS
write, no `0x0704` spam. Bearer verified: **no**.

## 2026-10-02 — registration triggers landed; blocked by modem PCIe link-drop

Acting on the CP-config verdict, the unified owner now runs a one-shot
registration-trigger sequence once the SIM reads READY: confirm radio ON
(`0x0801`), read selection mode (`0x0703`) and force **automatic** (`0x0704`)
unless already auto, read preferred RAT (`0x070b`) and **broaden `LTE_ONLY(11)`
→ `LTE_WCDMA(12)`** (`0x070a`), then `AllowData(1)` (`0x0710`). All builders are
the recovered/self-tested factory shapes from `ready-network-once.c` (no invented
bytes); SETs are sent at most once, GETs retry a bounded number of times then
advance so a slow/absent reply cannot stall the chain. Reproducible ARM64 owner
`8ba92be2…`; host + on-device `self-test` PASS (adds `test_camp_reg`), `-Werror`.

**Motivating hypothesis (RAT mismatch):** preferred RAT is `LTE_ONLY(11)` while
the only present signal is UMTS (`mask_low7=2`, data `tech_raw=3`), so CS is
denied on LTE and PS never searches. Broadening to `LTE_WCDMA(12)` lets the modem
use the present UMTS cell — a fixable config mismatch, constraint-safe (no EFS
write).

**Live partial evidence (one clean boot):** the sequence engaged correctly —
radio confirmed ON (`radio_raw=10`), then it attempted the selection GET. On that
boot `0x0703` never replied and the *old* build blocked on it; the bounded-retry
fix above now advances past it to the preferred-RAT broaden + AllowData. The
broaden experiment itself was **not yet observed live** (see blocker).

**Blocker (new, transport-level):** after that one boot, **7 consecutive**
`sysrq`-reboot → handoff cycles came up with the modem **PCIe endpoint dropping
right after RadioPower-ON** — `cpif: pcie_send_ap2cp_irq: Reserve doorbell
interrupt: PCI not powered on`. The link works for the first ~9 s (pre-dispatch
SIM/signal/voice/data GETs reply) and then dies after `0x0800`; the SIM never
initializes (`card_raw=0`), every IPC GET times out, so the registration
sequence (correctly gated on SIM READY) never runs. The CP itself boots cleanly
(`complete_normal_boot`, `CP2AP_WAKEUP=0x1`, no `cp_crash`; battery 100 %, 33 °C),
so this is a modem-side PCIe runtime-PM / L1.2 sleep that cpif cannot wake, not a
CP crash and not the new code (which only runs post-SIM-READY).

**Recovery attempts that did NOT help:** long power-framework settle (90 s); pin
RC `power/control=on`; disable the EP link L1.2 ASPM
(`.../0000:01:00.0/link/l1_2_aspm`+`l1_2_pcipm` → `0`, confirmed applied);
repeated plain reboots. The drop is cpif-managed CP runtime-PM (deeper than ASPM
L1.2) and warm `sysrq b` does not reset the modem power rail — a **cold hardware
power cycle** is the likely requirement. The read-only USIM EF diagnosis
(`EFfplmn`/`EFad`/`EFimsi` via `0x0208`) is blocked by the same dead transport.

**Next unmet precondition:** a stable PCIe link that keeps the CP reachable
through RadioPower-ON so the SIM initializes and the registration triggers run.
We have **not** reached the real-EFS constraint boundary — the RAT-broaden
experiment is constraint-safe and ready; it simply needs one boot where the modem
endpoint stays powered. Bearer verified: **no**.

## Constraints (unchanged)

No `IOCTL_POWER_OFF`, `do_cp_crash`, EFS RW, cbd/rild.
No ICCID/IMSI/AID/PIN digits in logs. No concurrent tray watches.

## Pointers

- Runtime: `docs/os/targets/panther/MODEM-RUNTIME-2026-09-24.md`
- Path B: `docs/os/targets/panther/MODEM-06-KERNEL-PATH-B.md`
- Roadmap: `docs/os/sprints/MODEM-ROADMAP.md`
- `services/saai-modemd/` soft-lock / post-edge CLI
