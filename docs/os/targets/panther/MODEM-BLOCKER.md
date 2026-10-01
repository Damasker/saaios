# Panther modem blocker (MODEM-06) — one pager

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

## Constraints (unchanged)

No `IOCTL_POWER_OFF`, `do_cp_crash`, EFS RW, cbd/rild.
No ICCID/IMSI/AID/PIN digits in logs. No concurrent tray watches.

## Pointers

- Runtime: `docs/os/targets/panther/MODEM-RUNTIME-2026-09-24.md`
- Path B: `docs/os/targets/panther/MODEM-06-KERNEL-PATH-B.md`
- Roadmap: `docs/os/sprints/MODEM-ROADMAP.md`
- `services/saai-modemd/` soft-lock / post-edge CLI
