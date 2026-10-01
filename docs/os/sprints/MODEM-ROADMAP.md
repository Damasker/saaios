# SaaiOS Pixel Modem — delivery roadmap

Status: **MODEM-00/01 complete. MODEM-02 diagnostic complete, not a
service. MODEM-03 diagnostic complete with first runtime responses. MODEM-04
started: maintained host boot model, reviewed boot plan, executor-facing
action contract, failure state machine + `inspect-image`. No autostart modem
daemon.**

Evidence:
[MODEM-RESEARCH-2026-09-24.md](../targets/panther/MODEM-RESEARCH-2026-09-24.md),
[MODEM-EXPERIMENTS-2026-09-24.md](../targets/panther/MODEM-EXPERIMENTS-2026-09-24.md),
[MODEM-RUNTIME-2026-09-24.md](../targets/panther/MODEM-RUNTIME-2026-09-24.md).
Phone path: [PIXEL-PATH.md](PIXEL-PATH.md).

This is a hardware track for the Pixel 7 `panther` S5300/S5100SIT modem.
It is independent of APP-COMPAT and Visual work. It does not reopen the
kernel track, does not authorize radio partition writes, and does not make
Android vendor daemons part of SaaiOS.

## Outcome

Bring the native modem from proven diagnostic boot to a maintained SaaiOS
subsystem:

```text
diagnostic proof → maintained boot model → controlled runtime queries →
post-SIM init understanding → long-running modem service → shell-visible facts
```

Do not skip directly from `ONLINE` to "cellular works". Separate milestones:

1. CP boot reaches stable `ONLINE`.
2. Runtime SIT requests are consumed.
3. SIM/radio state is understood.
4. Registration/attach is observed.
5. Data bearer appears.
6. Calls/SMS/IMS are separate later milestones.

## Safety Rules

- No radio/modem partition writes, slot switches, radio flashing, or vendor
  daemon execution.
- No original EFS writes. Do not mount original EFS read-write. Do not expose
  original EFS to a newly written RFS service.
- No invented, copied, zero-filled, logged, committed, or substituted
  identity, NV, or signature values.
- No automatic reset loops, POWER_OFF loops, background polling, or unattended
  startup.
- A diagnostic run requires fresh `OFFLINE`, matching `cpif.ko`, verified
  B-slot firmware in RAM, verified NV copies, read-only persist signature
  source, current endpoint major/minor checks, and preserved before/after logs.
- Shell/UI may show only facts from live kernel/runtime state. No LTE bars,
  operator, dBm, IMEI, subscriber identifiers, or registration claims without
  a source.

## Delivery

| ID | Result | State | Phone? |
|---|---|---|---|
| MODEM-00 | ADR/research evidence + this roadmap | **Done** | no |
| MODEM-01 | Maintained safe boundary (`saai-modemd` status/preflight) | **Done** (host) | no |
| MODEM-02 | Factory S5100SIT boot sequence reaches CP `ONLINE` | **Done** (diagnostic) | **yes** |
| MODEM-03 | RAM-only handover unlocks first SIT runtime responses | **Done** (diagnostic) | **yes** |
| MODEM-04 | Maintained boot model library, no hardware actions | **Started** (host TOC/stage/plan/executor/failure model) | no |
| MODEM-05 | Controlled runtime query mode in `saai-modemd` | Backlog | gated |
| MODEM-06 | Factory post-SIM init / registration prerequisites | **Hard blocker** (HotSwap falsified Present=2) | gated |
| MODEM-07 | RFS design and refusal policy | Backlog | no |
| MODEM-08 | Long-running `saai-modemd` lifecycle | Backlog | **yes** |
| MODEM-09 | World/Observation + Shell cellular facts | Backlog | **yes** |

## MODEM-00

**Goal:** preserve the evidence trail before productizing it.

**Current state:** the September 24 investigation documents firmware
provenance, A/B comparison, factory CBD disassembly, guarded live probes,
native boot success, handover, and runtime query results. The reports are
committed as `docs/os/targets/panther/MODEM-*.md`.

**Acceptance:** this roadmap exists and names which results are diagnostic,
which are production candidates, and which actions remain forbidden.

**Rollback:** remove this roadmap; diagnostic reports remain historical
evidence.

**Threat:** none. Docs only.

## MODEM-01

**Goal:** create a safe maintained home for modem work without running the
modem.

**Change:** `services/saai-modemd` provides status/preflight only. It reads
`modem_state`, live bearer names, mount status, and optional SHA-256 inputs.
It does not open modem endpoints, power on CP, issue ioctls, mount partitions,
serve RFS, or run the diagnostic loader.

**Test:** host `cargo test -p saai-modemd`; `cargo check -p saai-modemd`.

**Acceptance:** missing modem hardware is safe; `preflight` fails closed when
state is not `OFFLINE`, original EFS appears mounted, or hashes mismatch.

**Rollback:** remove `services/saai-modemd`; diagnostics remain opt-in.

**Threat:** false confidence. The command must continue to print
`hardware_actions=none` for preflight success.

## MODEM-02

**Goal:** prove native CP boot, still as a diagnostic.

**Result:** factory S5100SIT preamble, reviewed firmware stages, verified NV
copies, FIN, and COMPLETE reached `ONLINE`. Kernel log confirmed
`PHONE_START <- s5300` and `INIT_END -> s5300`.

**Key findings:**

- The repeated `0x201b098` MAIN transfer wall was caused by missing factory
  pre-MAIN protocol, not by A/B firmware version alone.
- Required preamble is initial READY plus TOC START/BIN/DONE before MAIN.
- MAIN requires CRC; VSS/APM/NV stages do not.
- NV copies are accepted only with the factory `Samsung_SIT_RIL` checksum
  algorithm documented in the experiment report.

**Acceptance:** documented phone log and host copy in the report. No autostart.

**Rollback:** reboot AP; do not retry against a stale BOOTING/ONLINE state.

**Threat:** treating `ONLINE` as cellular service. It is not.

## MODEM-03

**Goal:** prove normal runtime can make progress after factory handover.

**Result:** RAM-only `IOCTL_HANDOVER_BLOCK_INFO` before preamble caused the
first matching SIT response. Earlier identical query stalled at FMT TX 24/0;
handover run consumed the request. Later one-shot queries observed radio raw
ON, SIM status with one application, and packet-domain registration raw zero.

**Acceptance:** documented phone log and host copy in the runtime report.
No identifiers or signature contents in reports.

**Rollback:** do not install the handover probe. Reboot AP before another
comparison.

**Threat:** equating one consumed GET with registration, data, SMS, calls, or
long-term service.

## MODEM-04

**Goal:** move the boot knowledge out of the diagnostic snapshot into
maintained, host-testable code.

**Current state:** `services/saai-modemd/src/boot_model.rs` now covers TOC
parsing, reviewed Panther layout validation, stage CRC policy, SIT command
bits, BIN packet bytes, ring-fit chunk sizing, an ACK4 reader, and the reviewed
boot plan order: load BOOT, start bootloader, one initial READY, TOC
START/BIN/DONE, MAIN/VSS/APM/NV stages, FIN, COMPLETE. The model explicitly
does not insert a second READY before FIN. It also lowers steps into
executor-facing actions, validates approved payload sources and CRC policy
before any future hardware executor can run them, and reports ACK mismatches
with structured expected/got words. `BootProgress` records success or one
failure, refuses to advance after failure, and classifies failures after
bootloader start as requiring a fresh AP boot before retry. Executor outcomes
now map success, timeout, ACK mismatch, configuration, I/O, unexpected-state,
and layout results into the same progress state machine. The `inspect-image`
command reads one local modem image, validates BOOT, MAIN, VSS, APM, NV_NORM
and NV_PROT, builds validated executor actions, and performs no hardware
actions.

**Change:** continue extracting pure model pieces for richer failure
classification and executor-facing invariants. No device opens. No ioctls. No
firmware/NV file reads in the core model. The remaining host-side MODEM-04
work is mostly packaging: keep the executor contract stable while MODEM-05
runtime query parsing is introduced separately.

**Test:** host fixtures for TOC bounds, stage names/indices, CRC policy,
executor action invariants, ACK split/coalesced reads, ACK mismatches, error
paths, and "failed stage cannot advance".

**Acceptance:** `cp-boot-probe-support.inc` is no longer the only source of
truth for the successful sequence; `cargo test -p saai-modemd` covers the
maintained model.

**Rollback:** keep diagnostics as historical proof.

**Threat:** accidentally copying hardware side effects into the model.

## MODEM-05

**Goal:** controlled runtime query mode in `saai-modemd`, not a service.

**Change:** explicit command for one-shot SIT GETs on an already `ONLINE`
diagnostic boot. It must verify endpoint major/minor, hold an exclusive lock,
send one request, bound receive time, drop unrelated events without logging
payloads, and exit.

**Allowed initial queries:** SIM status, radio state, packet-domain data
registration. No PIN/PUK/APDU, no network selection, no radio-power mutation,
no APN, no SMS/call.

**Test:** host frame parser fixtures; live only after MODEM-04 and a fresh
gated diagnostic boot.

**Rollback:** remove the command; keep `sit-sim-status.c` diagnostic.

**Threat:** concurrent IPC consumers. The command must fail if a lock is held.

## MODEM-06

**Goal:** understand factory post-SIM initialization before trying to attach.

**Live blocker (2026-09-30) — CP soft-lock:** `0x0200` byte17=PIN and
byte72=pin1 DISABLED are independent CP fields. STATUS ADF TLV C6:
card PIN disabled + life_cycle activated, matches cp pin1=3 — yet
app_state stays PIN. `0x0247` OpenChannelWithP2 P2=0 + channel STATUS
`0x020f` SW9000 (after 6C46 Le retry); still ready=0, `rmnet` 0/0.

**RO MAIN (same day):** Present=2 builders **not** on FCP path —
FN_A via `MMC_LTEL1_CDMA_MEAS_RESULT_IND` case (arg0==3) and
SIM-wrap `0x14f6d02`; FN_B via LTEL1 meas neighbour. FCP/`0x18e`/
`sitSetPin1` have no BL there. Live app=PIN ⇒ Present∉{1,2,3}. See
RUNTIME § «Present=2 callers».

**RO MAIN (same day+):** `SIM_START_IND` / `SIM_PIN_STATUS_IND` are
CP→AP TX (`0x105a` USIM builders). **Do not** write Present=2 / call
FN_A/FN_B / `#0xBF6` / Pin1Verified. Stock prelude: `SIM_INIT_REQ` →
`SIM_PRESENT_IND` → START/PIN_STATUS; then optional
`SIM_START_STACK_SERVICES_REQ`. No proven AP trigger → no live probe.
RUNTIME § «SIM START / PIN_STATUS IND».

**RO MAIN (same day++):** Present stores 0–3 listed (FN_A/FN_B only).
SIM-wrap `0x14c3986` = handler for **`MMC_LTEL1_CDMA_TIMING_LATCH_CNF`**;
`[msg+8]==3` → Present=2. Factory HAL has **no** SIT for
`SIM_INIT_REQ` / `STACK_SERVICES`. No live probe. RUNTIME § «Present
writers + SIM-wrap».

**RO MAIN (same day+++):** CNF `[+8]` = **`rat_mode`** (not Present);
producer = L1LC→EVDO latch REQ/CNF. FN_B only via
`MMC_LTEL1_UMTS_MEASURE_CNF` (0/1/2 internal). Live app=PIN ⇒ Present∉{1,2,3}.
No AP trigger for `rat_mode==3`. No live probe. RUNTIME § «LATCH +8 =
rat_mode».

**RO MAIN (same day++++):** FN_B remap — Present stores on
`UMTS_TDD_PARTIAL_SEARCH_CNF`; Present=2 STRB **unreachable** (Phase-2/CBZ).
FDD UMTS live path cannot READY via FN_B. Latch needs CDMA IRAT; LTE_ONLY
suppresses. preferred=12 already negative — **no live probe**. RUNTIME §
«FN_B Present=2 dead».

**RO MAIN (challenge):** `CDMA_TIMING_LATCH_REQ` = L1LC CDMA IRAT only;
`No CDMA in SupportedRatMap` on this image. READY re-audit: still sole
`SET_APP#5` / sole `+0xBF6` / sole reachable Present=2=FN_A(CDMA). EU
LTE-only PIN-disabled→READY **missing** in this MAIN — CP hole. No CDMA
preferred live. RUNTIME § «Challenge — LATCH never on EU».

**RO MAIN (ctor):** PresentObj=`getobj(r1=#0x636c)`; **no** Present=2 at
init/INSERT/PIN_DISABLED. Exhaustive: only FN_A (+ dead FN_B). **MODEM-06
hardened** — EU LTE PIN-disabled cannot READY on this MAIN. Next: RO
factory cold-boot SIT order vs ours (missing proven Build*). Not
UpdateGoal-complete (no `rmnet`/IPv4).

**RO MAIN (cold-boot + VerifyPin):** Factory order
`0x0801→0x0200→0x070b→0x070a→0x0704→0x0701` (+`0x0201` only if PIN
enabled). **No unsent proven Build*** for PIN-disabled READY → **no
live**. First PIN1 → Pin1Verified only; sole `SET_APP#5` still needs
`+0xBF6==2`; FN_A only LATCH/MEAS. **CDMA-only Present=2 writer model
falsified as stock-EU explanation**; READY **gate** Present==2 holds.
Hole = missing non-CDMA Present=2 writer. RUNTIME § «cold-boot SIT vs
VerifyPin→READY».

**RO MAIN (image + broader writer):** Host probe SHA `449eeab3…` =
modem_b `g5300q-260317-260505-B-15346003`; live phone re-hash **blocked**
(host unreachable). Broader search: no memcpy/reg-offset Present=2;
MEAS_FN sole caller = CDMA wrap; no non-CDMA FN_A arg==3 → **no live**.
RUNTIME § «image match + broader Present=2».

**Live (link restore):** USB NCM `172.31.7.1` + **COM13** work; Wi‑Fi
down; SSH pubkey denied. Live probe SHA **matches** `449eeab3…`.
`0x0200` still PIN+DISABLED; radio ON; reg 0; rmnet 0/0. Paradox not
HAL remap / not wrong image. RUNTIME § «link restore + live match».

**RO+live (gate + modem_a):** `+0xBF6` **= PresentObj[0] copy** (STATUS
`LDRB [r5,#0]`→STRB); sole READY CMP#2. Live Present not in sysfs/dmesg.
modem_a same CDMA-only Present=2; slot switch **banned** — skip. No live.
RUNTIME § «READY gate provenance + modem_a».

**RO (SIT-boundary):** `0x0200` app_state@17 = **SET_APP `+0xBF4` only**
(GET_APP `0x18ec8c0`; sole STRB `0x1991734`). Not Pin1Verified/pin1/
TX-composite. PIN-disabled / Pin1Verified **cannot** READY without
Present==2 (sole `#5` still `0x14fb5c6`). SupportedRatMap = init-only
(no Set/Update / no proven SIT). PIN_SKIP: stub eSIM check always fail
`0x11d6` — **no non-eSIM trigger**. No live. RUNTIME § «SIT-boundary
falsify».

**RO (FN_A exhaustive):** BL→FN_A **n=2 only** — WRAP_A←`CDMA_MEAS_RESULT_IND`
(`[msg+0]==3`) and WRAP_SIM←`CDMA_TIMING_LATCH_CNF` (`[msg+8]/rat_mode==3`).
LTE-only **cannot** supply arg==3. Sole PresentObj `#636c` Present=2 =
FN_A `0x14f6a14`. InitRapMap «No CDMA» = QM_MM_INIT input; handover has
**no** RAP flag. CDMA preferred **not** live (unproven). VerifyPin↛Present
— **no** PIN-reenable ask. Next: tray reseat **or** RO who fills RatMap
into QM_MM_INIT. RUNTIME § «Exhaustive FN_A + InitRapMap».

**RO (RatMap population):** SupportedRatMap arrives on **QM_MM_INIT_REQ**
(from GMC/RRM); TCS `CDMA_SUPPORT` GV from **reg/NV**. Only «No CDMA»
logs; no non-EFS flip. **READY unreachable** here (PIN soft-lock ∧ no
CDMA ⇒ no FN_A Present=2). Live lever: none. **Next:** physical tray
**or** blocked pending new evidence (other CP/SKU). RUNTIME §
«SupportedRatMap population».

**RO+live (SET_APP / alias / AllowData):** 14 BL→SET_APP; sole `#5` @
`0x14fb5c6` behind Present==2; sole STRB `+0xBF4`. CDMA MEAS/LATCH lo16
bind unique WRAP (not LTE/NR). Live: voice `0x0700` reg=3 + operator
PLMN ⇒ MM/NAS alive; ONE `BuildAllowData` `0x0710` allow=1 → err 0;
still PIN+DISABLED, data reg=0, rmnet 0/0. **Next remote ≠ tray:** RO
`umts_dm0`/`oem_ipc*` listen for CDMA IND vs Present/SET_APP#5 under
LTE preferred. RUNTIME § «SET_APP exhaustive + CDMA ID non-alias +
AllowData live».

**Live (DM listen + PDN GET):** LTE_ONLY+radio ON. `umts_dm0`/`oem_ipc*`
EOF 0 bytes — no CDMA IND/SET_APP strings. SetupDataCall not sent (APN
from Telephony DB, `isValidPdpApn`, not in factory libs). `0x0602`
GetDataCallList length 13 err 0 (empty list). After: still PIN+DISABLED,
voice reg=3, data 0, rmnet 0/0. **Next:** decode voice reg enum 3 vs
HOME/DENIED; or RO-map Present `+0xBF6`. RUNTIME § «DM/OEM listen +
voice/PIN paradox + no PDN».

**Live (voice enum):** `0x0700` byte12=3 → HAL convert **RIL 3 REG_DENIED**
(not HOME). Data `0x0701` raw=0 = not-reg/not-searching. CS denied vs
PS idle under PIN+DISABLED; no PDN. **Next:** MM deny-vs-READY xref, or
RO-map Present `+0xBF6`. RUNTIME § «voice raw=3 is DENIED».

**RO+live (attach gate):** HAL `OnSimStatusChanged` cmp **#5**; CP
`START_NETWORK Ignored: SIM is not ready` + `GET_APP` CMP#1/4/5
(`0x18e831a`). PIN not in set → no PS search. Voice DENIED reject=0 =
local (byte13). No extra SIT. **Next ≠ tray:** log-id xref that START_
NETWORK block, or RO-map Present. RUNTIME § «attach gated on SIM READY».

**RO ({1,4,5}):** 1=DETECTED (`SET_APP` sole `0x146aaba`, no Present
gate); 4=SUBSCRIPTION_PERSO (sole `0x14fb526`, Present **==3**); 5=READY
(Present==2). PIN-disabled is **not** 1/4. No AP path. **Next:** skip
PIN SET when pin1 DISABLED → #5, or RO-map Present. RUNTIME § «app_state
1/4 are not PIN-disabled READY».

**RO (STATUS `0x14fb3xx`):** pin1 DISABLED **does not** skip SET#2 and
**does not** READY if Present≠2. Order: Present→`+0xBF6`; Present==0
**CBZ `0x14fb3ac`→SET#2 `0x14fb404`**; ==1 SET#3 PUK; else PERSO/READY
(`CMP #3` `0x14fb520`, `CMP #2` `0x14fb5c0`). Pin byte `r4+3` **CMP #0
`0x14fb4a2` after SET#2** — **no CMP #3**. FCP `PIN_DISABLED` is
PinStatus only. **Missing CMP:** before `0x14fb404`, no `pin1==3`→
`0x14fb5c4` `#5`. No skipped BL / no live. RUNTIME § «STATUS 0x14fb3xx
no DISABLED skip».

**Live+RO (Present peek):** no AP export of `+0xBF6`/PresentObj
(`info_region`/legacy/sysfs). Live still PIN+DISABLED; rmnet 0/0.
Infer Present==0 (SET#2 CBZ). Once PIN, STATUS skips Present→READY.
DETECTED race / START_NETWORK@#1 **not reachable** from PIN via proven
card-power/RadioPower (already PIN endpoints). No live race. **Next:**
listen-sim on card-power 4→1 timing, or RO ABSENT→SET#1 re-entry.
RUNTIME § «live Present infer 0».

**Live (CardPower DETECTED race):** ONE `0x024c` 4→1 with fast
`0x0200` poll. **Never DETECTED(1).** Saw apps=0 then ABSENT(card=0);
end **UNKNOWN(0)**; Radio pulse; data reg=0; rmnet 0/0. SET#1 callers
`0x14538e4`/`0x1a2abf8`; ABSENT cluster also SET_APP**#0**. Race to
START_NETWORK via #1 **not workable**. RUNTIME § «CardPower race no
DETECTED».

**RO+live (SET#1 stable):** SIM restored PRESENT+**PIN**. SET1FN only
via CP event `0x14535a2`←`0x1432d0a` and init-table `0x1a2aac8`←
`0x1a421ae` — **no AP SIT**. DETECTED not keepable
(STATUS Present==0→PIN). No live #1 hold. MODEM-06 holds. RUNTIME §
«restore + SET#1 no AP DETECTED».

**Live (cold INSERT dense race):** sysrq→CPIF→handover ONLINE; 200 ms
`0x0200` from first ONLINE + RadioPower ON ASAP. Published path
**ABSENT → PIN+DISABLED** only; `saw_detected=0`,
`pin_disabled_identical=1`, rmnet 0/0. Race failed on cold INSERT too.
**MODEM-06 blocked** pending physical tray or new CP evidence. RUNTIME §
«cold INSERT dense race no DETECTED».

**RO (A+B MAIN re-scan):** host fw A-14784800 + live B-15346003 —
sole `SET_APP#5` at A`0x14fc9a2` / B`0x14fb5c6`; preceding CMPs are
**only #2 (Present)**. **No** `CMP pin1,#3`→`#5`. No in-repo CP DRAM
peek/poke to force Present/SET_APP (`/dev/mem` ENXIO). Live poll still
PIN+DISABLED, rmnet 0/0. Tool: `sit-sim-status tray-watch` (Present
infer from app_state for tray). RUNTIME § «A/B MAIN re-scan + no CP
DRAM poke».

**Live (tray-watch 180s):** no reseat — only PIN+DISABLED
(`present_infer=notin_1_2_3`); no ABSENT/DETECTED/READY; rmnet 0/0.
Usage in RUNTIME § «tray-watch 180s». MODEM-06 still blocked. Goal
incomplete.

**RO+live (Present poke/peek):** no CPIF/sysfs/ioctl/debugfs write to
Present or SET_APP (`ds_detect`≠Present). No userspace CP DRAM map for
`+0xBF6`. Tool: `peek-present-surfaces`. RUNTIME § «no CPIF Present
poke». Goal incomplete.

**Live (READY-patch MAIN COPY):** our probe SHA gate widened
(stock\|`193e4f48…`); patch `0x14fb404` PIN→READY. POWER_ON…CRC OK; **UDL
MAIN DONE fail** (OEM bind post-CRC). Stock restored. RUNTIME §
«READY-patch MAIN COPY — live load aborted».

**Live (post-abort):** stock `449eeab3…` → ONLINE; `0x0200` PIN+DISABLED;
rmnet no IPv4. **tray-watch ≥5 min** — no reseat / no edge. Unsigned MAIN
dead.

**Live (boot_base/UDL):** `boot_base`=AP `SHMEM_CP` staging; PCIE BOOT
uses SHMEM_IPC carveout + MSI PA. MAIN = SIT UDL after START (boot_base
already NULL) → CP DRAM. **No** post-DONE AP write window for SET#2.
KERNEL_SRC absent — no cpif rebuild. RUNTIME § «boot_base vs UDL MAIN».

**HARD-BLOCK (2026-09-30):** PCIe BAR0 = **doorbell 4B only**; no CP
DRAM ioremap; no SIT RAM-poke opcode; no BTL userspace map. MODEM-06
blocked until **physical tray** or **KERNEL_SRC+CP-DRAM export**. Stop
inventing paths.

**Unblock checklist:** RUNTIME § «KERNEL_SRC / BTL re-check + unblock
checklist» — (A) tray-watch + reseat; (B) pantah
`android-gs-pantah-6.1-android16` s5300 + gs `Module.symvers` (or R620
`panthor-backport`) then reviewed CP-DRAM export.

**Path B progress:** s5300 pantah + `kernel/common@bd23337e42e7` +
phone `config.gz` + `saaios_cp_poke.c` draft. **Build blocked:** no
sudo/`/usr/bin/m4` for `modules_prepare` (see
`MODEM-06-KERNEL-PATH-B.md`). Live PIN+DISABLED, no rmnet IPv4. Goal
incomplete.

**RO adversarial (VerifyPin→READY paradox):** FirstPIN → Pin1Verified
only; **0** BL Present/`+0xBF6`/STATUS/FN_A. Sole `#5` still
`+0xBF6==2`. After SET#2, STATUS READY only if GET_APP∈{**6,7**}.
**SET#6** ← L1 `SADR_GAP_MEASURE_PAUSE_REQ` (not VerifyPin);
**SET#7** ← FirstPIN→`0x18ec31c` case10 **iff** app==6. Stock PIN unlock
= early Present latch + L1 SET#6 + VerifyPin SET#7 + STATUS#5 — **not**
“VerifyPin writes Present=2”. DISABLED shares {6,7}→#5 if `+0xBF6==2`
∧ SET#6; no AP SADR → no live. Present=2 EU writer still open. RUNTIME §
«Adversarial VerifyPin→READY». Goal incomplete.

**RO (SADR_GAP producer + Present re-audit):** PAUSE_REQ posted
**SR_IF→LTE_L1LC** (gap-measure pause; LTE-only). Consumer
`0x14b7cbc`→SET#6. **No** SIT/preferred/band lever (LTE_ONLY+Radio ON
already live-neg). Present=2 / `+0xBF6` writers unchanged (FN_A only;
SET#6 does not latch Present). No live. RUNTIME § «SADR_GAP SET#6
producer». Goal incomplete.

**RO (GapMeasure vs READY chicken-egg):** SET#6/PAUSE poster **no**
GET_APP check, but RSM/SRL1RC meas needs camp; START_NETWORK only
app∈{1,4,5} → PIN blocks → no PAUSE_REQ. Init getobj `#636c` stores
Present**=0**; sole Present=2 still FN_A. No AP sequence → no live.
RUNTIME § «GapMeasurePause vs SIM READY». Goal incomplete.

**Live (no-0x0200):** clean handover (no SIM poll) → Radio/LTE/AllowData →
75s `0x0700`/`0x0701`/rmnet only → final `0x0200` still **PIN(2)**+DISABLED;
no reg/rmnet. Hypothesis falsified. RUNTIME § «avoid AP 0x0200». Goal incomplete.

**Live (ATU OB2 sig-scan):** SET#2 sig `02 20 96 f0…` / TOC; RO retarget
bands `0x0`/`0x4`/`0x8`/`0xa`/`0xc`×GB + BTL/hyp fine — all DUMP `ff`,
**0** SIG HIT. No poke. MODEM-06 ATU-discovery dead. MODEM-06 § «OB2
systematic RO sig-scan». Goal incomplete.

**Live (SHMEM_IPC RO):** IPC `0xea400000` via `cp_shmem_get_region` —
SIM-PAT hits **only** in FMT RX (stale `0x0200` copies PIN+DISABLED);
srinfo/united_status **not** Present/GET_APP mirrors. No poke.
`tray-bearer-chase.sh` for physical reseat. RUNTIME § «SHMEM_IPC RO».
Goal incomplete.

**Durable:** one-pager `docs/os/targets/panther/MODEM-BLOCKER.md`; one
command `diagnostics/tray-bearer-chase.sh` (also noted in saai-modemd
README). Background watch may already be running — do not duplicate.

**Live (Pin1Verified remote):** VerifyPin A+AID err0 → pin1=2; START_NETWORK
still {1,4,5}; camp/SET#6 chicken-egg. `tray-bearer-chase` EDGE now runs
CardPower→VerifyPin A+AID then Radio/LTE/rmnet (remain guard; no secrets).
MODEM-BLOCKER updated. Goal incomplete without rmnet IPv4.

**Live dead-end (EngMode / +0xBF4):** GET_APP `+0xBF4` = same PresentObj
`#636c` heap — AP-unreachable. ONE SetEngMode `0x0908`/13 mode=1 → err0,
app stayed PIN, pin1=2, reg/rmnet unchanged. **Remote soft-lock cannot force
READY/Present.** Do not re-run. **Waiting on physical tray reseat** with
armed `tray-bearer-chase` (VerifyPin-on-EDGE). MODEM-BLOCKER + RUNTIME.
Goal incomplete (no rmnet IPv4).

**Live (false EDGE fix):** soft-lock still ONLINE/PIN/pin1=2/rmnet rx=0 no
IPv4. `tray-bearer-chase` wrongly chased every round (BusyBox BRE `\|`).
Fixed + single watch re-armed. No new remote Present/READY lever; Path B
poke still dead (no PA). Operator reseat required. RUNTIME § «false EDGE».

**Live (pin1=2 soft-lock + caps + chase harden):** still ONLINE / PIN /
pin1=2 / rmnet rx=0 no IPv4. New remote Present/GET_APP PA **not** found
(heap `#636c` singleton ATU open-bus already closed). Signed CPIF caps
live AP part0=**3** CP part0=**7** (PKTPROC_UL|CH_EXT|+36BIT) — already
negotiated at INIT_START; pktproc/toe surfaces present; **not** a READY
lever. `saai-modemd soft-lock` now detects pin1=2 chicken-egg; chase logs
caps, refuses concurrent watches, longer bearer poll + GetPsService after
EDGE. Watch re-armed. Goal incomplete.

**Change:** static analysis of factory RIL/CBD/SIT sequence plus read-only
diagnostics. Identify which requests are neutral GETs, which initialize local
state, and which mutate radio/network state. Do not send a command merely
because it appears in factory code.

**Test:** host packet fixtures and a written command classification table.

**Acceptance:** one bounded live comparison only after a specific missing
neutral prerequisite is proven.

**Rollback:** stop at observation. No reset loop.

**Threat:** accidentally initiating network selection or exposing subscriber
data.


**RO+policy (FN_A / RatMap closed):** SupportedRatMap = NV/TCS on QM_MM_INIT
only; preferred `0x070a` 11/12 live-neg for FN_A; CDMA preferred **not**
tried (no RatMap mutation). LTE_ONLY not unique FN_A block. Remote lever
**none** under bans. Watch armed; MODEM-BLOCKER § FN_A/RatMap. Goal
incomplete (no rmnet IPv4). RUNTIME § «FN_A / RatMap remote lever closed».

**RO (SET_APP ≠ AP SIT):** Challenged READY(#5) `SET_APP` — **CP-internal
only** (`0x19916d2`); no HAL builder / `SIT_SET_APP` / FORCE_READY; 14 BL
sites lack nearby `0x02xx`; `0x0200` GET mirrors `+0xBF4`. Phone host down
this turn (no live/re-arm). No invent send. RUNTIME § «SET_APP is
CP-internal only». Goal incomplete.

**Live HotSwap falsifier + present_infer OK (2026-09-30):** Physical tray
ABSENT→PRESENT seen; VerifyPin A err0 pin1 1→2; **app stayed PIN**;
`present_infer=notin_1_2_3` **validated** (byte17=+0xBF4 not +0xBF6;
STATUS Present→SET table holds; sole SET#5 still CMP Present==2). HotSwap
**insufficient** for READY/bearer on EU No-CDMA. No signed bypass
(SADR inject / alternate STATUS / wrong-object). **No live try** this turn.
Hard blocker: only EFS TCS / unsigned MAIN / stock rild remain (all banned
or rejected). Post-READY SetupDataCall/APN already armed. MODEM-BLOCKER +
RUNTIME § «present_infer validation + HotSwap hard blocker».

**SIT NV + alt CP hunt (2026-09-30):** `BuildNvReadItem`/`BuildNvWriteItem`
are **stubs** (no usable SIT NV write despite ban nuance allowing non-EFS
NV). Host fw only A/B — both EU `No CDMA in SupportedRatMap`; no US/CDMA
signed image to UDL. Live still soft-lock PIN+pin1=2; **no bearer**.
Next: external signed CDMA-RatMap CP **or** policy change. See MODEM-BLOCKER
§ SIT NV / alt signed CP + RUNTIME § «SIT NV stubs + no alt CDMA-signed CP».

**FN_A LTE/SADR re-scan + factory radio (2026-09-30):** Whole-MAIN BL→FN_A
**still n=2** (CDMA MEAS + TIMING_LATCH only). SADR/L1TUNNEL → STATUS_WRAP
only — **not** Present=2. LTE/NR measure strings do not alias into FN_A.
**No live LTE elicit.** Local factory `modem.img` SHA `491993b0…` is same
`B-15346003` + `No CDMA in SupportedRatMap` — **not** US/CDMA; not booted.
Live soft-lock holds (PIN+pin1=2, rmnet rx=0). Stock-EU paradox open.
RUNTIME § «FN_A re-analysis + US image hunt». Goal incomplete.

**SET#5 / Present bypass hunt (2026-09-30):** Exhaustive MAIN B scan —
sole SET#5 @`0x14fb5c6` still Present==2; sole +0xBF6 STRB; sole
PresentObj Present=2=FN_A; alt STATUS mid-BL / wide BF4 stores are noise.
**Bypass=none → no live try.** Live COM13: ONLINE PIN+pin1=2
`present_infer=notin` rmnet 0/0. Stock-EU paradox **unresolved** under
current MAIN RE. Remaining: external signed CDMA-RatMap CP **or** policy
(EFS TCS / rild / unsigned MAIN). Do not reseat. MODEM-BLOCKER + RUNTIME.

**Live A+B (2026-09-30 night):** (A) Pin1Verified then force `0x0704` while
PIN → **CP error_raw=2**; app stayed PIN; reg/rmnet unchanged (0) — **CP
gate, not host-only**. (B) CP2A factory = identical EU No-CDMA; TD1A radio =
`g5300g` (no RatMap/CDMA strings; not UDL-compatible); cheetah/lynx CP2A
aborted. **No soft UDL. No bearer.** Next = policy or true CDMA-RatMap
`g5300q` signed image. MODEM-BLOCKER §§ A/B + RUNTIME § A/B.

**RE `0x0704` + live (2026-09-30 late):** `0x0703` = **already auto** → prior
`0x0704` err=2 = already-auto GENERIC_FAILURE (not unique SIM reject).
START_NETWORK `@0x18e831a` still requires `GET_APP∈{1,4,5}` (PIN blocked).
Precondition matrix: no satiable unmet under bans → **no live SIT seq**;
`fix-selection` skipped set. Verizon OTA `td1a.221105.003` HEAD-ok but
full DL blocked (Google timeout); resume+scan next. Bearer **no**.
MODEM-BLOCKER + RUNTIME.

**`0x2f50` sender = AP OEM IPC (2026-10-01):** Catalog `@0x6de740`
`SIM_INIT_REQ` beside VerifyPin in `[OEM][IPC]`/`[OEM][SIT]` bank. GMC
sends START_STACK (`0x2f58`) **not** INIT. Soft CPIF cannot elicit
`0x2f50` without cbd-like OEM IPC; INIT ≠ Present=2. Non-STRB Present=2
still **none** (FN_A only). Live ADB **down** this turn — no try / no
bearer. Next: restore ADB brief; stock No-CDMA Present paradox → policy
under bans. MODEM-BLOCKER + RUNTIME §§ who-sends.

**OEM `0x2f50` framing (2026-10-01 later):** Device back (COM13/NCM). Soft
VerifyPin = SIT `0x0201` on `umts_ipc0`, **not** OEM `0x2f52` — no dual.
Catalog gives `flags=2` body hint only; host OEM header + `oem_ipc0` write
path **unrecovered** (`oem_ipc0` open EACCES). **No live INIT send** (no
invent bytes). Sticky PIN/`notin`; bearer **no**. Next: RE `cbd`/
ipc_message_server encode or stock capture. MODEM-BLOCKER + RUNTIME §§
framing.

**OEM encode + oem_ipc0 open (2026-10-01 night):** ONLINE/PIN sticky; cbd =
boot-only (no OEM encode); CPIF SIT prepends EXYNOS on oem write; libsitril
msgid table only; **app header + 2B body still missing**. `oem_ipc0` now
**RDWR-openable** (mknod+chmod; no daemon). **No INIT send.** Bearer **no**.
Next: stock capture / host OEM writer (not cbd). MODEM-BLOCKER §§ encode.

**OemSim AID-filled OCH (2026-10-01):** Hypothesis post-VerifyPin
`0x0247`→`0x020f` SELECT→`0x0208` STATUS+AID. Live all SW9000; app stayed
PIN pin1=2; reg/rmnet unchanged; **no bearer**. `OnVerifyPinDone` does not
emit those SITs. OemSim soft-lock exit closed. Next: `0x2f50` wire only.

**OEM preprocess RX (2026-10-01):** Live ONLINE/PIN/pin1=2/oem_ipc0 OK; no
bearer. DBT pins msgid_nf/preprocess to `oem_ipc_message_dispatcher.c`, SIT
recv to `oem_sit_main.c`; no code xref to those strings (DBT-indexed).
`preprocess_cb` = gmetrics false friend. App hdr + 2B body still missing;
**no INIT send**. Next: stock capture or non-string dispatcher RE. BLOCKER +
RUNTIME §§ preprocess.

## MODEM-07

**Goal:** design RFS before any long-running modem service.

**Change:** specify what requests may be served from verified copies, what is
read-only, what is denied, and how writes are rejected or quarantined. Original
EFS is never exposed to a new RFS server.

**Test:** host fake RFS client fixtures; refusal cases; no original EFS path
in tests.

**Acceptance:** design review and fixtures before code is wired into any boot.

**Rollback:** no RFS server.

**Threat:** NV mutation or identity leakage.

## MODEM-08

**Goal:** long-running `saai-modemd` owns modem lifecycle.

**Prerequisites:** MODEM-04, MODEM-05, MODEM-06, MODEM-07. No shortcut.

**Change:** one owner process for boot, handover, runtime receive loop, bounded
failure recovery, CP crash handling, and shutdown policy. No automatic retry
storm. No shell-facing registration claim until observed.

**Test:** host state-machine tests; phone run with fresh boot, one runtime
query, controlled exit, and no residual mounted sensitive partitions.

**Rollback:** do not start service from PID 1; return to diagnostics.

**Threat:** hard-to-debug boot regressions. PID 1 must remain bootable without
`saai-modemd`.

## MODEM-09

**Goal:** expose modem facts to World/Observation and Shell honestly.

**Change:** publish typed observations such as CP state, radio state, SIM app
presence, packet registration raw status, and live bearer names. Shell remains
honest-empty until facts exist.

**Test:** no modem → «Нет модема» or diagnostic/offline state; ONLINE without
registration does not show carrier/data; bearer row appears only from live
`rmnet`/`wwan`/`qmimux`/`ccmni`.

**Rollback:** shell returns to ADR-255 bearer-only row.

**Threat:** invented UX confidence. Never show operator, bars, dBm, IMEI, or
subscriber identity without a live, reviewed source.

## Non-goals For Now

- voice/IMS/VoLTE
- SMS
- emergency calling
- APN editor
- operator selection
- radio firmware updates
- Android RIL, vendor cbd, or vendor rfsd execution
- generalized modem support beyond Pixel 7 `panther`
