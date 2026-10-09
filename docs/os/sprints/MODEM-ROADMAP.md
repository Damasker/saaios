# SaaiOS Pixel Modem — delivery roadmap

**Priority correction (2026-10-01):** the CP sends unsolicited
`type=2/id=0x0906/len=206` frames about every 1.28 s when IPC0 is held open
(23 headers in a 30 s redacted observation). Without an opener, the kernel
drops them; the last close purges the shared queue. The currently short-lived
diagnostics therefore cannot establish reliable modem runtime ownership.
Stock TD1A `libsitril.so` identifies `0x0906` as the signal-strength
indication forwarded to RIL 1009; its handler sends no modem command. Loss of
that telemetry is **not** a proven cause of the camp failure. Implement one
continuous IPC reader/dispatcher plus an RFS owner, with a no-gap boot
handoff, to make all runtime traffic observable and safely serviceable.

An opt-in single-reader diagnostic owner first attached to the already-ONLINE
CP. Its read-only snapshot had READY/ON but registration=0; several one-minute
windows saw about 47 signal indications each and no RFS requests. A later
fresh-AP-boot test of the guarded no-gap pre-FIN handoff succeeded: CP reached
ONLINE, the owner remained alive, and the first minute captured RFS command 7
at +7.282 s followed by command 6 (sequence 1, payload 16) at +17.282 s.
No RFS reply was sent. The initial read-only SIT snapshot had card=0/apps=0,
radio=1, registration=0; `rmnet0` RX stayed 0. This establishes early RFS
traffic, not its causal role in SIM/network failure. The handoff is still a
one-shot diagnostic, not production MODEM-08.

Status: **MODEM-00/01 complete. MODEM-02 diagnostic complete, not a
service. MODEM-03 diagnostic complete with first runtime responses. MODEM-04
started: maintained host boot model, reviewed boot plan, executor-facing
action contract, failure state machine + `inspect-image`. No autostart modem
daemon.**

**MODEM-06 update (2026-10-01):** The previous PIN→READY hard-blocker is
superseded: signed VerifyPin A+AID without CardPower reached READY(5) twice.
`0x0704` auto, `0x070a` preferred LTE and `0x0710` AllowData returned success,
but data registration remains 0 and no cellular IPv4 bearer exists. Do not
repeat PIN attempts automatically: the owner says the SIM PIN is disabled.
The simultaneous RFS 7→3→6 diagnostic cannot be credited with the READY
transition because a separate VerifyPin test ran during its passive watch.
`0x2f50` remains an internal CP catalog message, not a proven external OEM
command. Next milestone: capture voice/data registration, operator and signal
under stable READY; then isolate the missing camp prerequisite. The exact
post-READY one-shot is `diagnostics/ready-network-once.c` and requires
exclusive SIT ownership. Do not call MODEM-06 done until registration and
`rmnet` IPv4 are demonstrated.

**Live 2026-10-01 post-READY:** `ready-network-once snapshot` returned
READY(5), radio ON(10), voice/data registration=0, automatic selection=0,
preferred=11, and successful operator/signal response headers; all `rmnet`
RX counters remained zero. Guarded `run` sent AllowData(1) once (accepted)
and four subsequent data-registration polls stayed 0. This rules out a
missing AllowData request alone; the next isolated experiment must target
the start of network search, not SetupDataCall/APN.

Guarded RadioPower ON-only `0x0800` was also accepted under READY/ON; four
voice/data registration polls remained 0 and the subsequent snapshot kept
READY/ON/auto/LTE_ONLY unchanged. A reversible LTE_WCDMA(12) check was then
ACKed/read back and observed for ten voice/data polls, all registration=0;
the original LTE_ONLY(11) was restored and verified. Neither ON-only nor the
LTE-only restriction alone explains the lack of camp. A short-lived RFS poll
is unsafe because closing the final channel descriptor purges pending packets.

**No-gap control update (2026-10-01):** The passive single-owner boots
repeated RFS cmd7 at about +7.27 s and cmd6 at about +17.27 s without a
reply. One boot's post-indication SIM GET reported card PRESENT/app PIN;
two later boots' +60-second GETs reported card ABSENT/apps 0, radio raw 3
(`SIM_LOCK_OR_ABSENT`), registration 0 and no `rmnet` RX. The operator later
confirmed that the physical SIM and tray were **already out before the last
reboot**. Its IPC0 absent result is consistent with that fact, not evidence
of a new failure.
The same boot's factory slot-status indication at +8.008 s reported slot
count 2 and slot-0 card PRESENT; that record may be the eUICC/another
slot-port representation, but its identity and active-profile state are
unproven. The third boot's SIM-removal timing is unknown, so it is not a
controlled comparison. [The runtime log](../targets/panther/MODEM-RUNTIME-2026-09-24.md)
has the exact captures. After the physical SIM is returned and its PIN state
checked separately, take a documented SIM-in read-only comparison. A
two-slot card/port-scalar logger was committed for that next comparison. Do not
infer a mapping bug, RFS causality, or a need for `0x0250`/PIN/radio SETs
from the absent-tray boot. No additional phone reboot was made in this
control series.

**SIM-in control update (2026-10-01):** The operator found that the physical
card had indeed required PIN, disabled its PIN prompt in another phone,
returned it to the Pixel and reported replenishing the account. The tester
later confirmed that the same SIM registered and mobile internet worked in
another handset **after** the top-up. This is a useful control against an
account/SIM-wide block, not proof that Pixel RF hardware or SaaiOS camp is
healthy. Hot insertion under the existing owner gave card PRESENT/app
READY(5)/PIN1 DISABLED(3)
without a SaaiOS VerifyPin. One new guarded AP boot then deployed the
redacted two-slot logger and reached CP ONLINE with one IPC0/RFS0 owner.
Slot 0 reported card 1/port 0 logical 1; slot 1 card 1/port 0 logical 0.
This is consistent with an embedded-SIM record plus the removable SIM, but
does not prove an active eSIM profile. The TD1A `sit-stream.so` (SHA-256
`cef8756461c74102f9a78f91177d1baff80fb9af11c14994497fb8854e0f530a`)
parses 105-byte slot records at `0x626ac–0x626ec` and `0x62940–0x62970`:
card state at +0, EID length at +35, port count at +52, port-0 ICCID
length at +53, logical ID at +64 and port state at +65. State 1 describes
an active port-to-modem mapping, not an enabled subscription: [AOSP explicitly
allows an active eSIM port without an enabled profile](https://android.googlesource.com/platform/hardware/interfaces/+/2e2c6e7b316f9f23844c2becfa5109e28bc6afd1/radio/aidl/android/hardware/radio/config/SimSlotStatus.aidl),
and [its ICCID may come from a default boot profile](https://android.googlesource.com/platform/hardware/interfaces/+/2e2c6e7b316f9f23844c2becfa5109e28bc6afd1/radio/aidl/android/hardware/radio/config/SimPortInfo.aidl).
With the tray out, IPC0 had card 0/apps 0 while slot 0 still reported card 1;
with the tray in, IPC0 had card 1/apps 1 READY and logical 0 mapped to slot 1.
Together these strongly identify IPC0 with the removable slot 1, although
`0x0200` carries no explicit physical-slot ID. No EID/ICCID bytes were logged.
At +60 s IPC0 remained READY/DISABLED,
radio was ON(10), yet voice/data registration were 0, `rmnet0` had no IPv4
address and RX/TX stayed 0. RFS cmd7/cmd6 repeated without replies. The
current blocker is **network camp/registration**, not physical SIM absence,
PIN entry or account balance alone. Next isolate the missing camp trigger
from any RFS prerequisite with a bounded, single-owner experiment; neither
cause is proven. Details: [runtime log](../targets/panther/MODEM-RUNTIME-2026-09-24.md).

**Latest scan control (2026-10-01):** A separate guarded one-shot owner ran
on two PIN-free READY boots. Both `0x0706` requests were promptly rejected;
the second recorded `error_raw=2`, with radio ON, broad preferred RAT,
automatic selection, signal technology-presence mask 0 and no network
registration. PIN lock is not the sole cause. Stop scan repeats; MODEM-06
now needs an isolated factory AP/RFS/CP-radio prerequisite check before
any registration or bearer work.

**Extended network control (2026-10-01):** A separately reviewed owner
build kept the same single-owner handoff and added only four bounded,
redacted status GETs after the +60-second SIM/radio/registration sweep.
Under SIM READY/PIN disabled/radio ON, selection mode was automatic (0),
preferred type SIT raw 16, and operator and signal GETs succeeded. Voice/data
registration stayed 0 and `rmnet0` had no bearer. Payloads containing
operator names/PLMN or signal data were not logged. The Samsung factory
table names SIT raw 16 NR/LTE/GSM/WCDMA and maps it to Android mode 26,
**not** Android mode 16. A restrictive preferred mode is not supported as
the blocker; do not change it. One `0x0706` available-network query is the
next possible search-vs-registration discriminator, but it is an **active RF
scan**, not another passive GET. Exact TD1A factory RIL gates it on its own
and the opposite stack's in-process PLMN-scan transactions and call state;
those are not a CP/SIT read-only RF-idle measurement. With no Android RIL and
one SaaiOS IPC owner, its own queue can establish the analogous *AP-request*
idle condition, but cannot prove that either CP/eSIM stack is autonomously
RF-idle. Require a separately reviewed opt-in single-owner scan state machine,
verified no-competing-client boundary, bounded timeout and `0x0707` cancel
path before any live scan. At the time of this passive control, the owner
lacked that state machine; the later guarded scan result is above. See
[the factory guard audit](MODEM-07-RFS-QUARANTINE.md#separate-active-rf-scan-gate).
No further PIN or blind radio SET is justified by this data.

A [host-only scan state model](../targets/panther/SIT-NETWORK-SCAN-HOST-MODEL.md)
now tests one bounded query, strict count-only parsing, late replies and a
single cancel path. It is deliberately not linked into the owner or installed
on the phone; passing synthetic tests is not live-scan approval.

**Passive follow-up (2026-10-01):** after 22 minutes of the same single-owner
boot, the owner process was still alive and `rmnet0` remained down with zero
RX/TX. The owner makes no registration GET after its +60-second settled pass,
so this later observation is **not** a fresh registration reading. The same
SIM had working registration and mobile data in another handset after the
top-up. Keep the current boot passive; the redacted signal-mask extension is
committed but not yet installed on the phone.

**Completed quarantined RFS control (2026-10-01):** a guarded, manual,
single-owner boot stored all 95 file-3 chunks (189446 bytes) in a private
candidate and sent one final success ACK only after candidate/sidecar
durability and integrity checks. No candidate was promoted or used as a boot
source, and original EFS was not written; it passed a read-only four-file
comparison again after reboot. At owner start +60 s, SIM was READY/PIN
disabled and radio ON, but voice/data registration stayed 0 and `rmnet0` was
down with RX/TX 0, matching the passive no-reply control. Completing this RFS
exchange is therefore insufficient to establish camp or a bearer. The passive owner
is restored. The stock radio-available audit found conditional startup SETs
but no proven missing camp prerequisite or global ordering against RFS.

**Matched event trace (2026-10-01):** two fresh, single-owner boots compared
the passive no-reply path with a second complete, quarantined RFS exchange.
Both owners received only `0x0803` and `0x0802` radio-range indications near
+9.8 s in their first minute; the full owner's local final ACK write returned
at +7.753 s. Both still had READY/ON, registration 0 and `rmnet0` down with
RX/TX 0 at +60 s. Thus a complete RFS exchange was not required for those
two observed headers and did not establish camp. These are owner receipt
times, not CP emission order; missing `0x07xx` headers do not prove RF idle.
Original EFS passed read-only postflight and was unmounted; original owner
binaries were restored and the passive owner returned CP to ONLINE.

**Factory `0x0802` decode (2026-10-01):** exact TD1A `libsitril.so` and
`sit-stream.so` independently read the unsolicited radio-state scalar as
little-endian `u32` at frame `+8`. Factory labels map raw 0/1 to RIL OFF,
raw 2 (`START_NETWORK`) to ON, and raw 3/4 (`POWER_OFF`/`RESET`) to
UNAVAILABLE. `0x0803` radio-ready is a distinct path and initially sets
UNAVAILABLE, not ON; the solicited `0x0801` GET state resides at `+12`.
Because the matched traces logged headers only, the early `0x0802` scalar
and ON transition remained unknown in those earlier traces. The subsequent
scalar-only passive boot observed raw 0 (`INITIALIZED`) at +9815 ms, with
zero first-window overflow; its +60-second GETs still reported READY/ON and
registration 0. The full-RFS owner remains header-only; do not claim a new
matched A/B until it has equivalent reviewed instrumentation. This passive
scalar run added no reader, GET or SET. Neither an early OFF/UNAVAILABLE indication nor
ON alone proves RF reset, camp or eSIM/carrier causality. The past `0x0706`
scan and full RFS exchange were separate boots, not a shared `dmesg`
timeline. Do not guess a radio, carrier, SIM or network SET. Details and
binary provenance are in
[MODEM-07-RFS-QUARANTINE.md](MODEM-07-RFS-QUARANTINE.md).

**Native service direction (2026-10-01):** reproduce the factory radio
service's state handling in SaaiOS; a full Android framework port is not the
next dependency. Exact TD1A code proves a separate logical-modem status GET
`0x0810` (12-byte request, mode byte at response +12), distinct from radio
power. READY/radio ON does not measure that state. After the scalar
observation, commit `2d5c28a` adds one bounded GET in the same owner's settled
pass; malformed, error or timeout results remain unknown. Its fresh phone
boot returned a 13-byte success response with **enabled=yes**, while the
same boot retained READY/ON and registration 0. A disabled logical stack
does not explain that measured state; do not send the corresponding enable
SET `0x080f`. See [factory evidence, live results and concrete rollback](MODEM-07-RFS-QUARANTINE.md#native-radio-service-boundary-and-logical-stack-check-2026-10-01).

**Late factory-SGC comparison (2026-10-01):** commit `c44ead4` adds a
separate, reviewed one-shot owner/probe; the default passive files stay
unchanged. Factory config resolution proves TD1A `europen` target 400 maps
to SGC `0x0101`, with two zero auxiliary words from the exact caller. In a
fresh single-owner phone boot, the settled baseline was READY/ON/stack
enabled and registration 0. One `0x0404` sent at +60785 ms received error 0
at +60830 ms. The independent five-GET sweep at +70.9 s still reported
READY/ON/stack enabled and voice/data registration 0; `rmnet0` remained down
with RX/TX 0. The late request did not establish registration in that
window. Original/userdata NV equality passed read-only checks before and
after the run. See [exact artifacts and observations](MODEM-07-RFS-QUARANTINE.md#late-sgc-phone-result-2026-10-01).

Next MODEM-06 task: separately specify and review an **early** one-shot SGC
comparison at the proven factory radio-available transition, using the same
payload and no additional SET or RFS reply. ACK0 is not proof of application
before network startup. Preserve the existing late variant and its guards;
the early experiment needs its own eligibility state machine and control.
Do not port Android or replay the entire factory callback list to bypass
this evidence gap. MODEM-06 remains open until actual registration and a
cellular bearer are observed.

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
| MODEM-04 | Maintained boot model library, no hardware actions | **Done** (host plan and executor actions; phone boot stays the diagnostic owner) | no |
| MODEM-05 | Controlled runtime query mode in `saai-modemd` | **Started** (a held endpoint refuses the GET even without the camp argv) | **yes** |
| MODEM-06 | Factory post-SIM init / registration prerequisites | **In progress** (home data registration and a live bearer observed) | **yes** |
| MODEM-07 | RFS design and refusal policy | **Started** (quarantine decision; carrier-config stays on its open id; not a boot service) | **yes** |
| MODEM-08 | Long-running `saai-modemd` lifecycle | **Started** (this boot attends the live camp and refuses to open the endpoint) | **yes** |
| MODEM-09 | World/Observation + Shell cellular facts | **Done** (this boot’s row and cache name only live facts) | **yes** |
| MODEM-10 | Real camp+bearer via valid dual-handle NV (VERDICT 27 reframe) | **Started** (live bearer on device) | **yes** |

## MODEM-00

**Goal:** preserve the evidence trail before productizing it.

**Current state:** the September 24 investigation documents firmware
provenance, A/B comparison, factory CBD disassembly, guarded live probes,
native boot success, handover, and runtime query results. The reports are
committed as `docs/os/targets/panther/MODEM-*.md`.

**Breakthrough (2026-10-05) — reframes MODEM-06:** стоковый контроль на
том же телефоне/SIM (слот `_b`, root) даёт CS/PS `REG_HOME`, RAT LTE,
PLMN 25503, `SETUP_DATA_CALL cause=NONE`, `rmnet1` UP за ~1 с. Провал
регистрации на SaaiOS — **артефакт пустого `NV_NORM` (crc 0) + RFS
только на handle-3**, а не «PIN soft-lock / Present==2 / отсутствие CDMA
RatMap» и не «CP-internal RF-cal стена» (VERDICT 24-26). Весь MODEM-06
PIN/Present/CDMA/`0x2f50` разбор — исторические гипотезы, снятые этим
фактом. Путь к связи — подать валидный dual-handle NV (оба handle из
верифицированных копий реального EFS, read-only) и пройти стоковую
именованную RIL-последовательность до bearer. См.
[MODEM-10](MODEM-10-REAL-NV-DUAL-HANDLE.md),
[modem-stock-reproduction.md](../targets/panther/modem-stock-reproduction.md),
[hardware-risks.md](../targets/panther/hardware-risks.md). Запрет vendor
`cbd`/`rfsd`/`rild` как сервисов сохраняется — фикс host-only.

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

**Status на этой загрузке (`d5d9b02`, бинарник `1eda45df…`).**
`saai-modemd status` печатает `modem_state=ONLINE`,
`endpoint=owner` и `hardware_actions=none`. Список имён каналов
включает `rmnet1`. Устройства не открывались. Дежурный
`saai-modemd` остался pid 155, `saai-displayd` остался pid 408,
owner один.
Тот же статус называет каталог архива (`a103187`, бинарник
`179f1d10…`): `epoch=1791379663`, рядом `endpoint=owner` и
`hardware_actions=none`. Это имя числового каталога в
`boot-archive`. Устройства не открывались. Дежурный
`saai-modemd` остался pid 155, `saai-displayd` остался pid 408,
owner один.
Тот же статус называет живой канал (`66e4c6c`, бинарник
`c6afd110…`): `bearer=rmnet1`. Полный список имён `rmnet` остаётся
отдельной строкой. Адрес в вывод не попадает. Рядом
`epoch=1791379663`, `endpoint=owner`, `hardware_actions=none`.
Дежурный `saai-modemd` остался pid 155, `saai-displayd` остался
pid 408, owner один.
Тот же статус называет процесс camp (`1854f79`, бинарник
`12b97366…`): `owner=running`. Рядом `bearer=rmnet1`,
`epoch=1791379663`, `endpoint=owner`, `hardware_actions=none`.
Устройства не открывались. Дежурный `saai-modemd` остался pid 155,
`saai-displayd` остался pid 408, owner один.
Тот же статус называет слово дежурства (`a10a956`, бинарник
`bdfc23c5…`): `supervisor=hold`. Это последнее `supervise=` из
`/run/modem-boot.log`. Рядом `owner=running`, `bearer=rmnet1`,
`epoch=1791379663`, `endpoint=owner`, `hardware_actions=none`.
Устройства не открывались. Дежурный `saai-modemd` остался pid 155,
`saai-displayd` остался pid 408, owner один.
Тот же статус называет регистрацию данных (`b0a22b2`, бинарник
`fe58fff2…`): `registration_raw=1`. Строка есть, пока owner
запущен и CP `ONLINE`. Отказ и идентификатор соты в вывод не
попадают. Рядом `supervisor=hold`, `owner=running`,
`bearer=rmnet1`, `hardware_actions=none`. Дежурный
`saai-modemd` остался pid 155, `saai-displayd` остался pid 408,
owner один.
Тот же статус называет радио (`47b1546`, бинарник `18f3c95b…`):
`radio=on`. Это stock-значение `radio_raw=10`, и строка есть,
пока owner запущен и CP `ONLINE`. Состояние PIN в вывод не
попадает. Рядом `registration_raw=1`, `hardware_actions=none`.
Дежурный `saai-modemd` остался pid 155, `saai-displayd` остался
pid 408, owner один.
Тот же статус называет присутствие SIM (`555b5ef`, бинарник
`93eb3005…`): `sim=present`. Это последнее решающее слово из лога
camp, пока owner запущен и CP `ONLINE`. Состояние PIN в вывод не
попадает. Рядом `radio=on`, `registration_raw=1`,
`hardware_actions=none`. Дежурный `saai-modemd` остался pid 155,
`saai-displayd` остался pid 408, owner один.

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

**Допуск (2026-10-07, `cc310e1`, бинарник `5b1b8e92…`).**
`saai-modemd query sim-status` на этой загрузке напечатал
`query=refuse name=sim-status reason=owner` и
`hardware_actions=none`. Owner остался один, второго handoff не
было, CP остался `ONLINE`. Команда не открывает endpoint. Пока
owner или `sit-sim-status` живы, отправка разобранных GET
`0x0200` / `0x0801` / `0x0701` не начинается. Путь ответа
(`3703b95`, бинарник `155573a3…`) пишет один такой GET только
после повторной проверки, что owner и `sit-sim-status` уже нет:
сверка major/minor `umts_ipc0`, неблокирующий lock, одна запись,
чтение до 10 с. Строка ответа оставляет `sim=present` /
`sim=absent`, `radio=on` и `registration_raw` 0…5. Состояние PIN
и чужие кадры в неё не входят. На этой загрузке снова
`query=refuse name=sim-status reason=owner` и
`hardware_actions=none`: запись в endpoint не делалась, owner
один, CP `ONLINE`.
Отказ называет токен запроса (`7fb4027`, бинарник `c25976e6…`).
На этой загрузке: `sim-status` — `token=1`, `radio-state` —
`token=2`, `data-registration` — `token=3`. У всех трёх
`reason=owner` и `hardware_actions=none`. Дежурный `saai-modemd`
остался pid 155, owner один, CP `ONLINE`. Срок 10 с в этой
загрузке не начинался: запись в endpoint не делалась. Строка
ответа, когда она будет, начинается с того же токена.
Отказ этой загрузки ещё называет каталог архива (`98c350b`,
бинарник `0b896e36…`): `query=refuse name=sim-status reason=owner
token=1 epoch=1791379663`, `hardware_actions=none`. Это имя
числового каталога в `boot-archive`, не новый camp. Дежурный
`saai-modemd` остался pid 155, owner один, CP `ONLINE`.
Отказ называет держателя по symlink из `/proc` (`489043b`,
бинарник `56708059…`): `query=refuse name=sim-status reason=owner
token=1 epoch=1791379663 endpoint=owner`, `hardware_actions=none`.
Устройства не открывались. Дежурный `saai-modemd` остался pid 155,
`saai-displayd` остался pid 408, owner один.
Занятый дескриптор сам по себе отказывает GET (`c51f657`,
бинарник `c10d9712…`), даже если argv camp не совпал. На этой
загрузке процесс camp запущен, поэтому причина остаётся
`reason=owner`: `query=refuse name=sim-status reason=owner token=1
epoch=1791379663 endpoint=owner`, `hardware_actions=none`.
Запись в endpoint не делалась. Дежурный `saai-modemd` остался
pid 155, `saai-displayd` остался pid 408, owner один.

## MODEM-06

**Goal:** understand factory post-SIM initialization before trying to attach.

**Erratum (2026-10-01):** The 2026-09-30 conclusions below that PIN→READY
is unreachable, that `0x2f50` must be sent externally, or that a CDMA/Present
change is the only solution are superseded by two live VerifyPin A+AID→READY
observations. They are historical hypotheses, **not executable next steps**.
`0x0200` publishes app state at byte **16**, not 17; byte 17 is the
personalization substate. The private CP Present byte was not measured.
`0x0704` accepts automatic selection but its ACK does not prove network camp.
The current blocker is registration=0 under READY; use the guarded read-only
network snapshot before any further SET.

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

**Non-string dispatcher RE (2026-10-01):** Live ONLINE/PIN/oem_ipc0 OK; no
bearer. Catalog `+0x18` = **class tag** (INIT-family=4 across SIM/CC/SMS/SS),
not token len. Catalog MOVW sites = object helpers only; `[OEM][PB]` nanopb
has **0** SIM overlap; app hdr + 2B body still missing; **no INIT send**.
Next: stock `oem_ipc*` catalog capture. BLOCKER + RUNTIME §§ nonstr.

**GET_APP parse + SitOem Ping (2026-10-01):** Live ONLINE; type@15=USIM(2)
**and** state@17=PIN(2) (libsitril offsets confirmed; not a misread); pin1=2;
apps=1. SitOem protobuf Ping on oem_ipc0: write_ok + RX type=RESPONSE —
transport alive, **≠** catalog `0x2f50`. Bearer **no**. Next: catalog OEM
wire still missing. BLOCKER + RUNTIME §§ parse/ping.

**SitOem schema exhaust + external `0x2f50` hunt (2026-10-01):** Live ONLINE /
PIN / pin1=2 / oem_ipc0 OK; no bearer. Full SitOem protobuf type list from
factory carve — **zero** SIM/init/card/uicc messages with encode path (Ping /
sitInitModem are not catalog INIT). External github/XDA/paste/pixel-mainline
hunt: **no** complete `0x2f50` body=2 frame. **No live try**. Next: stock
catalog OEM capture only. BLOCKER § schema-exhaust.

**HARD WALL — factory vendor no catalog encoder (2026-10-01):** Live ONLINE /
PRESENT / **app=PIN(2)** pin1=2 / oem_ipc0 RDWR OK / no rild·cbd / rmnet rx=0
/ **no bearer**. Offline RE of factory-td1a vendor carves (SitOem, cbd,
sitril/`BuildOemSimRequest`, oem_ipc1 helper): **no** SO opens `oem_ipc*`
**and** encodes msgid `0x2f50`; vendor has **0** `SIM_INIT_REQ` /
`IpcTxSimInit`. Encode unrecovered without invent bytes → **no one-shot /
no live write**. **Policy need:** capture-only stock rild once, or external
wire dump of catalog OEM frame; else accept soft-lock terminal under bans.
BLOCKER § hard-wall-policy.

**Alternate breakers dead (2026-10-01):** Live ONLINE / PIN(2) / pin1=2 /
data_reg=0 / rmnet rx=0 / **no bearer**. (1) Signed NET/L1 scan SITs
(`0x0706` QueryAvailable, StartNetworkScan, Manual/Auto select, emergency
scan) — **no** evidence any posts `SADR_MEASURE_RSP` or camps without
START_NETWORK/`GET_APP∈{1,4,5}`; sit-stream has **no** SADR builder;
`0x0706` already live err2 — **no re-try**. (2) `0x020a` facility SET —
enable already err2; disable needs PIN secrets; pin1 DISABLED ≠ READY —
**no safe payload / no try**. Soft-lock **terminal** under bans until
policy (capture-only rild **or** external `0x2f50` dump). BLOCKER §
alternate-breakers.

**Armed for frame (2026-10-01):** Diagnostics add `oem-ipc-inject.c`,
`post-init-chase.sh`, `OEM-IPC-CAPTURE.md`, and `tray-bearer-chase.sh`
`CHASE_ONCE=1`. Inject refuses empty/all-zero; **no** invented `0x2f50`
default. **No live send** until external/capture frame. Soft-lock /
bearer still blocked. BLOCKER § armed-for-frame.

**Stock vs soft bring-up + broad vendor hunt (2026-10-01):** Live ONLINE /
PIN(2) / pin1=2 / rmnet rx=0 / **no bearer**. Soft CPIF handover matches
stock POWER_ON→COMPLETE; mailbox `ds_detect` inert; no signed SIT/mailbox
alias for `USIM_WAIT_FOR_INIT` / catalog `0x2f50`. Full vendor.img hunt:
**0** `SIM_INIT_REQ`/`IpcTxSimInit`; **0** `oem_ipc`∧`MOVZ #0x2f50`
encoder islands (29 bare MOVZ sites are non-oem). Inject tools **deployed**
on device (`/data/saaios/bin/oem-ipc-inject`, `post-init-chase.sh`).
**No live try** (no new lever). Soft-lock **terminal** under bans until
capture-only rild or external frame. BLOCKER § stock-vs-soft-broad-hunt.

**Non-vendor factory partitions (2026-10-01):** Live ONLINE / PIN(2) /
pin1=2 / oem_ipc0 RDWR / rmnet rx=0 / **no bearer**. Extracted factory
td1a `system.img` + `product.img` + `system_ext.img` + `system_other.img`
(no standalone radio APEX in image zip). Needles `SIM_INIT_REQ` /
`IpcTxSimInit` / `/dev/oem_ipc` / ASCII `0x2f50` all **0**. system_ext has
`rild_exynos` symbol-bank strings only (no oem_ipc). **Encoder found? No.**
**No live send.** Policy still: capture-only rild or external `0x2f50`
dump. BLOCKER § non-vendor-partitions.

**Overnight recheck + soft-lock polish (2026-10-01):** Live ONLINE /
PIN(2) / pin1=**1** (reset from prior 2) / reg=0 / rmnet rx=0 / **no
bearer**. Inject tools still on device; **no frames sent**. `saai-modemd
soft-lock` / `post-edge` + chase `SOFT_LOCK_STATUS` report
`cpif_caps_exercised=yes` + waiting `0x2f50` blocker. Soft-lock
**terminal** until capture-only / external frame.

## MODEM-07

**Goal:** design RFS before any long-running modem service.

**Current state (2026-10-01):** the
[quarantined transaction design](MODEM-07-RFS-QUARANTINE.md) and a
[host-only model](../../../os/targets/panther/diagnostics/RFS-QUARANTINE-HOST.md)
and [host-only C core](../../../os/targets/panther/diagnostics/RFS-QUARANTINE-C-HOST.md)
cover the observed file-3 write request, bounded 95-chunk exchange and
factory final status. The [Linux private-storage host fixture](../../../os/targets/panther/diagnostics/RFS-QUARANTINE-STORAGE-HOST-LINUX.md)
also tests copy isolation, fsync/reread and withholding the final ACK on
faults. A separate [host-only transport fixture](../../../os/targets/panther/diagnostics/RFS-QUARANTINE-TRANSPORT-HOST.md)
tests split/coalesced packets, one outstanding grant, ambiguous writes and
deadlines. A synthetic end-to-end fixture now integrates protocol, transport
and storage; a separate host-only verified-FD fixture tests source integrity.
Linux ASan/UBSan passes. A read-only, no-recovery phone check found both NV
files and their sidecars byte-identical to the userdata copies. After an
initial one-grant run and three fail-closed full-transfer diagnostics, the
separate guarded full owner completed a manual 95-chunk/189446-byte file-3
exchange in a private quarantine candidate. It sent the final ACK only after
candidate and sidecar durability/integrity checks. The candidate was not
promoted to a boot copy; original EFS was not written and passed a read-only
post-reboot comparison. The passive owner was restored. Its no-reply control
and the completed exchange both had READY/ON but registration 0 and no
`rmnet0` bearer at +60 s. This is a completed diagnostic transaction, not a
production RFS service or MODEM-06 camp success. MODEM-07 remains in progress
for general refusal policy and maintained service integration. The factory
radio-available path has been audited offline without identifying a proven
camp prerequisite. The matched passive/full-RFS event-trace boots above had
the same early radio-range IDs and lengths, without decoding their payload,
and the same +60-second registration.
Keep MODEM-06 open for a separate camp prerequisite; do not send an unreviewed
radio, carrier, SIM or network SET to imitate factory startup.

**Change:** specify what requests may be served from verified copies, what is
read-only, what is denied, and how writes are rejected or quarantined. Original
EFS is never exposed to a new RFS server.

**Test:** host fake RFS client fixtures and refusal cases. Read-only original
EFS access is limited to provenance checks; synthetic tests never use it.

**Acceptance:** design review and fixtures before code is wired into any boot.

**Rollback:** no RFS server.

**Threat:** NV mutation or identity leakage.

**Решение (2026-10-07, `1a6ca5c`, бинарник `00fe7877…`).**
`saai-modemd rfs-policy` ничего не открывает. Файл 1 и файл 3:
команда 7 — `open-copy`, команды 2 и 6 — `quarantine-write`,
команда 3 — `quarantine-status`. Любой другой файл или команда —
`deny`. Исходный EFS в этом решении не источник и не назначение.
На этой загрузке: файл 3 команда 2 дала `quarantine-write
file=protected`, файл 1 команда 7 дала `open-copy file=normal`,
файл 9 команда 6 дала `deny`. Все три с `hardware_actions=none`.
Owner остался один, CP `ONLINE`. В загрузку это не вшито.
Carrier-config на том же канале — отдельное решение (`b19b303`,
бинарник `32f9a229…`). Команда 4 и команда 6 с операцией 1 —
`read-copy file=carrier-config`. Команда 6 с операцией 2 и
команда 4 на NV-файле 3 — `deny`. На этой загрузке все четыре
строки с `hardware_actions=none`. Дежурный `saai-modemd` остался
pid 155, owner один, CP `ONLINE`. В owner и в загрузку это не
вшито: живой camp по-прежнему отвечает на carrier-config сам.
Один и тот же номер команды разбирается по открытому id
(`890523f`, бинарник `dabede8f…`). `rfs-dispatch` прогоняет
восемь фиксированных заголовков и ничего не открывает: открытый
carrier-config даёт `read-copy`, запись того же id — `deny`,
команда 6 для NV-файла 1 — `quarantine-write file=normal`,
команда 3 для файла 3 — `quarantine-status file=protected`,
команда 7 для файла 3 — `open-copy file=protected`, закрытие —
`close-copy`, после закрытия тот же id — `deny`. В конце
`hardware_actions=none`. Дежурный `saai-modemd` остался pid 155,
owner один, CP `ONLINE`. В owner и в загрузку это не вшито.

## MODEM-08

**Goal:** long-running `saai-modemd` owns modem lifecycle.

**Prerequisites:** MODEM-04, MODEM-05, MODEM-06, MODEM-07. No shortcut.

**Change:** one owner process for boot, handover, runtime receive loop, bounded
failure recovery, CP crash handling, and shutdown policy. No automatic retry
storm. No shell-facing registration claim until observed.

Native implementation tasks (reuse Android's separation of responsibilities,
not its application framework or vendor-daemon runtime):

1. Preserve the verified boot-to-runtime descriptor handoff and exclusive
   dispatcher. Associate every response with its request token, boot epoch
   and deadline; expire observations after reset or loss of ownership.
2. Keep SIM readiness, logical-stack enablement, radio indication/query,
   voice/data registration and bearer state separate. Early indications and
   later GET replies retain their observation times instead of fabricating
   one simultaneous snapshot.
3. Encode only reviewed factory startup transitions. Each active command
   needs known input provenance, eligibility, response handling and bounded
   failure behavior; do not replay the factory callback list wholesale.
4. Integrate the separately reviewed RFS/storage policy and event dispatcher
   without exposing original EFS as writable storage or promoting quarantine.
5. After live registration is established, bring up the authorized data
   context and host IP/routes/DNS, then publish timestamped facts to SaaiOS.
   Calls, SMS and IMS remain separate milestones.

**Test:** host state-machine tests; phone run with fresh boot, one runtime
query, controlled exit, and no residual mounted sensitive partitions.

**Rollback:** do not start service from PID 1; return to diagnostics.

**Threat:** hard-to-debug boot regressions. PID 1 must remain bootable without
`saai-modemd`.

**Один запуск на загрузку (2026-10-07, `083c29c`).** `modem-boot.sh`
(`d7e0eb70…`) ждёт PCIe, убирает прошлые логи и делает exec
`saai-modemd supervise` (`889e0dc1…`). Если бинарника нет, скрипт
запускает тот же handoff. CP отсутствует или `OFFLINE`, и owner не
запущен: один handoff. Иначе процесс только остаётся. После выхода
handoff второй camp не стартует. На этой загрузке лог:
`supervise=launch-once cp=missing`, `handoff-exit code=0`,
`supervise=hold`; один owner; `camp_setup` на rmnet1 `up=1 add=1
route=1`; wget завершился 0, тело 577 байт, счётчики rmnet1
`0/192` → `1445/760`. PID 1 по-прежнему стартует только
скрипт. Уход owner или CP (`e54a9ad`, бинарник `1ad87e9f…`) даёт
одну строку `supervise=owner-gone` или `supervise=cp-left` и больше
ничего: возврат того же факта молчит, второй camp не стартует, CP
не выключается. На этой загрузке ухода не было: `launch-once`,
`handoff-exit code=0`, `hold`, один owner, один `saai-modemd`,
CP `ONLINE`, `camp_setup` на rmnet1 `up=1 add=1 route=1`, wget
завершился 0, тело 577 байт, счётчики `0/192` → `1446/820`. Кэш
по-прежнему `cellular.supervisor=hold`. Края «процесс ушёл» и
«CP ушёл» проверены на хосте.
Факты из лога owner живут, пока процесс запущен (`39e783a`,
`beb1944`). Регистрация, радио и SIM берутся из последних
решающих строк всего лога, а не из хвоста в 256 КиБ.
Кэш всегда пишет `cellular.owner=running` или `gone`. На этой
загрузке owner не останавливался: runtime `d4a277bd…`, оболочка
`683210bb…`, `saai-displayd` 408, один owner, CP `ONLINE`,
`cellular.owner=running`, `cellular.registration_raw=1`,
`cellular.radio=on`, `cellular.sim_app=present`,
`cellular.bearer=rmnet1`, `cellular.supervisor=hold`. Путь
`gone` проверен на хосте: лог с регистрацией без живого процесса
даёт только `cellular.owner=gone`.
Те же факты требуют ещё и CP `ONLINE` (`c820b7b`, runtime
`64e9e2e0…`, оболочка `6ffb511a…`). На этой загрузке CP
`ONLINE`, поэтому кэш по-прежнему `cellular.owner=running`,
`cellular.registration_raw=1`, `cellular.radio=on`,
`cellular.sim_app=present`, `cellular.bearer=rmnet1`,
`cellular.supervisor=hold`. `saai-displayd` остался 408, owner
один. Путь CP `OFFLINE` проверен на хосте: регистрация, радио и
SIM из лога пропадают, живой `rmnet1`, слово CP, дежурство и
`cellular.owner=running` остаются.
Команда `lifecycle` называет решение этой загрузки (`c51f657`,
бинарник `c10d9712…`): `action=attend`, `open=refuse reason=owner`,
`endpoint=owner`, `owner=running`, `cp=ONLINE`,
`hardware_actions=none`. Второй camp не стартует, устройства не
открываются. Путь `launch-once` при отсутствующем CP и путь
`open=refuse reason=endpoint`, когда дескриптор держит `saai-modemd`
без argv camp, проверены на хосте. Дежурный `saai-modemd` остался
pid 155, `saai-displayd` остался pid 408, owner один.
Одно правило `camp_action` (`b19f783`) пишет то же решение в кэш и
в status. На этой загрузке кэш даёт `cellular.action=attend`,
одноразовый status — `action=attend`. Рядом остаются
`modem_state=ONLINE`, `bearer=rmnet1`, `registration_raw=1`,
`radio=on`, `sim=present`, `epoch=1791379663`, `endpoint=owner`,
`supervisor=hold`, `owner=running`, `hardware_actions=none`.
Runtime `3d8eae15…` (pid 30644), status-бинарник `e1ef0919…`.
Дежурный `saai-modemd` остался pid 155, `saai-displayd` остался
pid 408, owner один. Путь `launch-once` при отсутствующем CP
проверен на хосте. Второй camp не стартует, устройства не
открываются.
То же правило допуска (`37f290f`) пишет в кэш `cellular.open=owner`
и в status `open=refuse reason=owner`. На этой загрузке рядом
остаются `action=attend`, `modem_state=ONLINE`, `bearer=rmnet1`,
`registration_raw=1`, `radio=on`, `sim=present`,
`epoch=1791379663`, `endpoint=owner`, `supervisor=hold`,
`owner=running`, `hardware_actions=none`. Runtime `8fd05843…`
(pid 32113), status-бинарник `5077cc83…`. Дежурный `saai-modemd`
остался pid 155, `saai-displayd` остался pid 408, owner один.
Путь `open=refuse reason=lock`, когда занят `sit-sim-status`, и
путь `open=ready` проверены на хосте. Устройства не открываются.
Голосовая регистрация пишется отдельно от data (`ce974bc`).
На этой загрузке обе равны `1`. В кэш это
`cellular.voice_registration_raw=1`, в status —
`voice_registration_raw=1`. Reject, LAC и CID в строку не попадают.
Сырое значение вне `0..=5` на хосте отбрасывается.
Режим выбора сети (`5bb0d60`) читается из той же строки camp:
`0` — automatic, `1` — manual. На этой загрузке status даёт
`selection=automatic`, кэш — `cellular.selection=automatic`.
Неизвестное сырое значение и `unknown_short` в кэш не попадают.
Оператор не выбирается и не пишется.
Логический стек (`948ae93`) читается отдельно от радио. Stock
считает `mode_raw=1` включённым и `0` выключенным. На этой
загрузке status даёт `stack=enabled`, кэш —
`cellular.stack=enabled`. Другое сырое значение и
`unknown_short` в кэш не попадают. Команда включения стека не
отправлялась.
Режим службы (`e2f4961`) читается отдельно от стека. Stock
называет `mode_raw=1` голосовым центром и `2` пакетным. На этой
загрузке status даёт `device_service=voice-centric`, кэш —
`cellular.device_service=voice-centric`. Другое сырое значение
в кэш не попадает. Команда смены службы не отправлялась.
Голосовая операция (`db77101`) читается отдельно от регистрации
голоса. Stock называет `mode_raw=3` включённой. На этой загрузке
status даёт `voice_operation=enabled`, кэш —
`cellular.voice_operation=enabled`. Другое сырое значение в кэш
не попадает. Команда смены операции не отправлялась.
Разрешение данных (`02aea16`) читается из ответа `set=allow_data`.
Stock называет `error_raw=0` принятым. На этой загрузке status
даёт `allow_data=accepted`, кэш — `cellular.allow_data=accepted`.
Другой код ошибки в кэш не попадает. Повторная команда не
отправлялась.
Начальное присоединение (`fe368a4`) читается из ответа
`set=initial_attach_apn`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `initial_attach=accepted`, кэш —
`cellular.initial_attach=accepted`. Имя точки доступа в строку не
попадает. Другой код ошибки в кэш не попадает. Повторная команда
не отправлялась.
Резолвер IPv4 (`63a53b9`) читается из строки `camp_setup dns=`.
Владелец пишет `yes` или `no`. На этой загрузке status даёт
`dns=yes`, кэш — `cellular.dns=yes`. Адреса серверов и счётчик в
строку не попадают. Строка `dns6` этим фактом не считается.
Резолвер IPv6 (`365e9cd`) читается из строки `camp_setup dns6=`.
Владелец пишет `yes` или `no`. На этой загрузке status даёт
`dns6=yes`, кэш — `cellular.dns6=yes`. Адреса серверов и счётчик в
строку не попадают. Строка `dns` этим фактом не считается.
Конфиг модема (`8364cbb`) читается из ответа
`camp_ack cmd=0x093f`. Stock называет `error_raw=0` принятым. На
этой загрузке status даёт `config=accepted`, кэш —
`cellular.config=accepted`. Другой код ошибки в кэш не попадает.
Ответы `cmd=0x0404` и `cmd=0x0800` этим фактом не считаются.
Повторная команда не отправлялась.
SGC (`16cfdf8`) читается из ответа `camp_ack cmd=0x0404`. Stock
`SendSGCValue` называет `error_raw=0` принятым. На этой загрузке
status даёт `sgc=accepted`, кэш — `cellular.sgc=accepted`. Само
значение в строку не попадает. Другой код ошибки в кэш не попадает.
Ответы `cmd=0x093f` и `cmd=0x0800` этим фактом не считаются.
Повторная команда не отправлялась.
Питание радио (`40d0bd3`) читается из ответа `camp_ack cmd=0x0800`.
Stock `BuildRadioPower` называет `error_raw=0` принятым. На этой
загрузке status даёт `power=accepted`, кэш —
`cellular.power=accepted`. Слово питания в строку не попадает.
Строка `radio=on` этим фактом не считается. Ответы `cmd=0x093f` и
`cmd=0x0404` этим фактом не считаются. Повторная команда не
отправлялась.
Задание голосовой операции (`70ec626`) читается из ответа
`camp_opx set=set_voice_operation`. Stock называет `error_raw=0`
принятым. На этой загрузке status даёт `voice_set=accepted`, кэш —
`cellular.voice_set=accepted`. Другой код ошибки в кэш не попадает.
Строка `get=voice_operation` этим фактом не считается. Другие
`set=` этим фактом не считаются. Повторная команда не отправлялась.
Применение IPv4 (`3cceaf3`) читается из строки
`camp_setup ipv4=yes prefix=32`. `yes` только когда связь, адрес и
маршрут все прошли. На этой загрузке status даёт `ipv4=yes`, кэш —
`cellular.ipv4=yes`. Адрес и три флага в строку не попадают. Строка
`ipv6` этим фактом не считается.
Применение IPv6 (`59d20e9`) читается из строки
`camp_setup ipv6=yes prefix=64`. `yes` только когда связь, адрес и
маршрут все прошли. На этой загрузке status даёт `ipv6=yes`, кэш —
`cellular.ipv6=yes`. Адрес и три флага в строку не попадают. Строка
`ipv4` этим фактом не считается.
Принятие сессии данных (`1175fb6`) читается из строки
`camp_setup response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `setup=accepted`, кэш —
`cellular.setup=accepted`. Адрес и длина ответа в строку не
попадают. Строки `dns`, `ipv4` и `ipv6` этим фактом не считаются.
Ответы `camp_profile` и `camp_ims` этим фактом не считаются.
Повторная команда не отправлялась.
Принятие интернет-профиля (`9e05c22`) читается из строки
`camp_profile response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `profile=accepted`, кэш —
`cellular.profile=accepted`. Тело профиля в строку не попадает.
Ответы `camp_setup`, `camp_ims` и `camp_sos` этим фактом не
считаются. Повторная команда не отправлялась.
Чтение активности модема (`68463ae`) читается из строки
`camp_activity response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `activity=accepted`, кэш —
`cellular.activity=accepted`. Длина ответа в строку не попадает.
Ответы `camp_profile` и `camp_fastdorm` этим фактом не считаются.
Повторная команда не отправлялась.
Fast dormancy (`5327029`) читается из строки
`camp_fastdorm response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `fastdorm=accepted`, кэш —
`cellular.fastdorm=accepted`. Длина ответа в строку не попадает.
Ответ `camp_activity` этим фактом не считается. Повторная команда
не отправлялась.
Чтение ENDC (`1e66129`) читается из строки `camp_endc response=yes`.
Stock называет `error_raw=0` принятым. На этой загрузке status даёт
`endc=accepted`, кэш — `cellular.endc=accepted`. Режим и длина ответа
в строку не попадают. Ответы `camp_vonrcapa`, `camp_rcnet`, `camp_ims`
и `camp_sos` этим фактом не считаются. Повторная команда не
отправлялась.
Троттлинг данных (`350a3eb`) читается из строки
`camp_throttle response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `throttle=accepted`, кэш —
`cellular.throttle=accepted`. Длительность в строку не попадает.
Ответы `camp_screen`, `camp_unsol` и `camp_endc` этим фактом не
считаются. Повторная команда не отправлялась.
Широкий фильтр индикаций (`f6dd834`) читается из строки
`camp_unsolff response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `unsolff=accepted`, кэш —
`cellular.unsolff=accepted`.
Слово фильтра в строку не попадает. Ответы `camp_unsol`,
`camp_screen` и `camp_throttle` этим фактом не считаются.
Повторная команда не отправлялась.
Установившийся фильтр индикаций (`b289ee3`) читается из строки
`camp_unsol response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `unsol=accepted`, кэш —
`cellular.unsol=accepted`. Слово фильтра в строку не попадает.
Ответы `camp_unsolff`, `camp_screen` и `camp_throttle` этим фактом
не считаются. Повторная команда не отправлялась.
Состояние экрана (`a259cfc`) читается из строки
`camp_screen response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `screen=accepted`, кэш —
`cellular.screen=accepted`. Состояние в строку не попадает.
Ответы `camp_unsol`, `camp_unsolff` и `camp_throttle` этим фактом
не считаются. Повторная команда не отправлялась.
Список сот (`91c5d32`) читается из строки `camp_cellinfo response=yes`.
Stock называет `error_raw=0` принятым. На этой загрузке status даёт
`cellinfo=accepted`, кэш — `cellular.cellinfo=accepted`. Длина и состав
списка в строку не попадают. Ответы `camp_smsc`, `camp_screen` и
`camp_vonrget` этим фактом не считаются. Повторная команда не
отправлялась.
Чтение SMSC (`ddad740`) читается из строки `camp_smsc response=yes`.
Stock называет `error_raw=0` принятым. На этой загрузке status даёт
`smsc=accepted`, кэш — `cellular.smsc=accepted`. Адрес в строку не
попадает. Ответы `camp_cellinfo`, `camp_vonrget` и `camp_screen` этим
фактом не считаются. Повторная команда не отправлялась.
Чтение VoNR (`06c955b`) читается из строки `camp_vonrget response=yes`.
Stock называет `error_raw=0` принятым. На этой загрузке status даёт
`vonrget=accepted`, кэш — `cellular.vonrget=accepted`. Тело ответа в
строку не попадает. Ответы `camp_vonrcapa`, `camp_smsc` и
`camp_cellinfo` этим фактом не считаются. Повторная команда не
отправлялась.
Чтение времени AP (`e4c8072`) читается из строки
`camp_aptime response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `aptime=accepted`, кэш —
`cellular.aptime=accepted`. Часы и длительность в строку не попадают.
Ответы `camp_vonrget` и `camp_dbgtrace` этим фактом не считаются.
Повторная команда не отправлялась.
Чтение трассы (`90f2c91`) читается из строки
`camp_dbgtrace response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `dbgtrace=accepted`, кэш —
`cellular.dbgtrace=accepted`. Тело ответа в строку не попадает.
Ответы `camp_aptime` и `camp_tty` этим фактом не считаются.
Повторная команда не отправлялась.
Чтение TTY (`d15a128`) читается из строки `camp_tty response=yes`.
Stock называет `error_raw=0` принятым. На этой загрузке status даёт
`tty=accepted`, кэш — `cellular.tty=accepted`. Тело ответа в строку не
попадает. Ответы `camp_dbgtrace` и `camp_pssvc` этим фактом не
считаются. Повторная команда не отправлялась.
Чтение пакетной службы (`00e146c`) читается из строки
`camp_pssvc response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `pssvc=accepted`, кэш —
`cellular.pssvc=accepted`. Тело ответа в строку не попадает.
Ответы `camp_tty` и `camp_prefmodem` этим фактом не считаются.
Повторная команда не отправлялась.
Чтение предпочтения модема (`b5f5257`) читается из строки
`camp_prefmodem response=yes`. Stock называет `error_raw=0` принятым.
На этой загрузке status даёт `prefmodem=accepted`, кэш —
`cellular.prefmodem=accepted`. Тело ответа в строку не попадает.
Ответы `camp_pssvc` и `camp_slot` этим фактом не считаются.
Повторная команда не отправлялась.

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

**Shell row (2026-10-07, `1725fa0`, бинарник `bc7df64c…` на `/data`).**
«Сотовая сеть» читает слово CP из `modem_state`, последнюю
`field=data registration_raw` из лога camp и имя iface только если на
нём есть IPv4 или оба счётчика ненулевые. На этой загрузке CP
`ONLINE`, data `registration_raw=1`, живой `rmnet1`: строка
`ONLINE · домашняя · rmnet1`. Оператор, RAT и адрес не пишутся.
`saai-shell` перезапущен один раз, `saai-displayd` остался.
Телеметрия runtime (`92e94ae`, бинарник `da06a622…`) пишет в кэш
`cellular.cp_state=ONLINE`, `cellular.registration_raw=1`,
`cellular.bearer=rmnet1`. Оболочка показывает их в «Наблюдениях»
как «Модем», «Регистрация / домашняя» и «Канал / rmnet1».
Последнее слово дежурства (`7475ff9`, runtime `e8a89c7b…`,
оболочка `02f91cb0…`) читается из `/run/modem-boot.log`. На этой
загрузке кэш даёт `cellular.supervisor=hold`. В «Наблюдениях» это
«Дежурство / удержание». `handoff-exit` и прочие слова в кэш не
попадают. Owner остался один, CP `ONLINE`, счётчики `rmnet1`
ненулевые. Перезагрузки для этого шага не было.
Радио и присутствие SIM (`29092e3`, runtime `7904ac65…`, оболочка
`9ff8d8fb…`) читаются из того же лога camp. В кэш попадают только
`radio_raw=10` как `cellular.radio=on` и последнее состояние
приложения: `ready`, `present` или `absent`. На этой загрузке кэш
даёт `cellular.radio=on` и `cellular.sim_app=present` (последняя
строка `apps=1`; более ранняя `camp_sim=ready` остаётся в логе).
В «Наблюдениях» это «Радио / включено» и «SIM / есть». Состояние
PIN в кэш не пишется. Owner один, CP `ONLINE`, `saai-displayd`
остался.
Строка «Сотовая сеть» (`9ba90f7`, оболочка `489b7877…`) ставит те же
слова между регистрацией и каналом. Для этой загрузки это
`ONLINE · домашняя · включено · есть · rmnet1`. Оболочка
перезапущена один раз, `saai-displayd` остался, owner остался один.
Пока owner запущен, строка «Сотовая сеть» и «Наблюдения» читают
те же слова. В «Наблюдениях» процесс camp — «Процесс / есть»
(`cellular.owner=running`). Когда процесса нет, регистрация,
радио и SIM из старого лога не публикуются. На этой загрузке
процесс не останавливался.
Строка «Сотовая сеть» (`5025648`, оболочка `09bdbfa3…`) ставит
слово процесса перед каналом. Для этой загрузки это
`ONLINE · домашняя · включено · есть · процесс · rmnet1`.
«есть» остаётся за SIM. Если процесс ушёл, а слово CP уже есть,
строка добавляет «без процесса»; пустой набор по-прежнему
«Нет модема». Оболочка перезапущена один раз, `saai-displayd`
остался pid 408, owner один, CP `ONLINE`.
Кэш пишет номер той же загрузки (`58cd57b`, runtime `ba71ec00…`,
оболочка `712ee577…`): `cellular.boot_epoch=1791379663`. В
«Наблюдениях» это «Загрузка / 1791379663». Нечисловое имя каталога
в кэш не попадает. `saai-displayd` остался pid 408, owner один,
CP `ONLINE`.
Кэш называет держателя `umts_ipc0` / `umts_rfs0` по symlink из
`/proc` (`60cb9da`, runtime `f5283197…`, оболочка `79424bc4…`).
Устройства не открываются. На этой загрузке `cellular.endpoint=owner`.
В «Наблюдениях» это «Дескриптор / camp». Рядом остаются
`cellular.cp_state=ONLINE`, `cellular.registration_raw=1`,
`cellular.radio=on`, `cellular.sim_app=present`,
`cellular.bearer=rmnet1`, `cellular.supervisor=hold`,
`cellular.owner=running`, `cellular.boot_epoch=1791379663`.
`saai-displayd` остался pid 408, `saai-modemd` supervise остался
pid 155, owner один, CP `ONLINE`. Перезагрузки не было.
Строка «Сотовая сеть» (`9852057`, оболочка `4eed7d65…`) ставит
то же слово держателя перед каналом. Для этой загрузки, по
проверенной функции и живым словам кэша, это
`ONLINE · домашняя · включено · есть · процесс · camp · rmnet1`.
Чужое слово в строку не попадает. Кадр экрана не снимался.
Оболочка перезапущена один раз, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, runtime не перезапускался,
owner один, CP `ONLINE`. Перезагрузки не было.
Строка «Сотовая сеть» (`fa736ae`, оболочка `09c7d7eb…`) ставит
слово дежурства перед каналом. Для этой загрузки, по проверенной
функции и живому `supervisor=hold`, это
`ONLINE · домашняя · включено · есть · процесс · camp · удержание · rmnet1`.
Чужое слово лога в строку не попадает. Кадр экрана не снимался.
Оболочка перезапущена один раз, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, runtime не перезапускался,
owner один, CP `ONLINE`. Перезагрузки не было.
Строка «Сотовая сеть» и «Наблюдения» (`b19f783`, runtime
`3d8eae15…`, оболочка `bc131609…`) называют то же решение. В
«Наблюдениях» это «Решение / уже запущен»
(`cellular.action=attend`). Для этой загрузки, по проверенной
функции и живым словам кэша, строка
`ONLINE · домашняя · включено · есть · процесс · camp · удержание · уже запущен · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 30653), `saai-displayd` остался pid 408, `saai-modemd`
supervise остался pid 155, owner один, CP `ONLINE`. Перезагрузки
не было.
Строка «Сотовая сеть» и «Наблюдения» (`37f290f`, runtime
`8fd05843…`, оболочка `72b8a6b7…`) называют тот же отказ.
В «Наблюдениях» это «Открытие / отказ процесса»
(`cellular.open=owner`). Для этой загрузки, по проверенной
функции и живым словам кэша, строка
`ONLINE · домашняя · включено · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 32122), `saai-displayd` остался pid 408, `saai-modemd`
supervise остался pid 155, owner один, CP `ONLINE`. Перезагрузки
не было.
Голосовая регистрация (`ce974bc`, runtime `b7db7698…`, оболочка
`5dd3c0de…`) стоит рядом с data и называется отдельно. На этой
загрузке кэш даёт `cellular.registration_raw=1` и
`cellular.voice_registration_raw=1`. В «Наблюдениях» это
«Регистрация / домашняя» и «Голос / домашняя». Для этой загрузки,
по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 2129), runtime pid 2120, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`bfc9360c…`, owner один, CP `ONLINE`. Перезагрузки не было.
Режим выбора сети (`5bb0d60`, runtime `de0a102e…`, оболочка
`2f05f5cc…`) стоит после радио. На этой загрузке кэш даёт
`cellular.selection=automatic`. В «Наблюдениях» это «Выбор / авто».
Для этой загрузки, по проверенной функции и живым словам кэша,
строка
`ONLINE · домашняя · голос домашняя · включено · авто · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 3290), runtime pid 3281, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`e61fe46f…`, owner один, CP `ONLINE`. Перезагрузки не было.
Логический стек (`948ae93`, runtime `1e66b464…`, оболочка
`dc899e95…`) стоит после режима выбора. На этой загрузке кэш даёт
`cellular.stack=enabled`. В «Наблюдениях» это «Стек / стек».
Для этой загрузки, по проверенной функции и живым словам кэша,
строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 5431), runtime pid 5422, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`9dca5a42…`, owner один, CP `ONLINE`. Перезагрузки не было.
Режим службы (`e2f4961`, runtime `c2218957…`, оболочка
`559a8773…`) стоит после стека. На этой загрузке кэш даёт
`cellular.device_service=voice-centric`. В «Наблюдениях» это
«Служба / голосовая». Для этой загрузки, по проверенной функции
и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 6636), runtime pid 6627, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`36cea880…`, owner один, CP `ONLINE`. Перезагрузки не было.
Голосовая операция (`db77101`, runtime `4114b991…`, оболочка
`8b315ed8…`) стоит после службы. На этой загрузке кэш даёт
`cellular.voice_operation=enabled`. В «Наблюдениях» это
«Операция / включена». Для этой загрузки, по проверенной
функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 7435), runtime pid 7426, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`0164e591…`, owner один, CP `ONLINE`. Перезагрузки не было.
Разрешение данных (`02aea16`, runtime `1a19c8f7…`, оболочка
`adb918d0…`) стоит после голосовой операции. На этой загрузке
кэш даёт `cellular.allow_data=accepted`. В «Наблюдениях» это
«Разрешение / дано». Для этой загрузки, по проверенной функции
и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 30476), runtime pid 30467, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`d76cb3ed…`, owner один, CP `ONLINE`. Перезагрузки не было.
Начальное присоединение (`fe368a4`, runtime `f9d8a4b8…`, оболочка
`d36f3f0c…`) стоит после разрешения данных. На этой загрузке кэш
даёт `cellular.initial_attach=accepted`. В «Наблюдениях» это
«Присоединение / дано». Для этой загрузки, по проверенной функции
и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 31437), runtime pid 31428, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`24dd5a71…`, owner один, CP `ONLINE`. Перезагрузки не было.
Резолвер IPv4 (`63a53b9`, runtime `a182ade6…`, оболочка
`fbd87128…`) стоит после начального присоединения. На этой
загрузке кэш даёт `cellular.dns=yes`. В «Наблюдениях» это
«DNS / есть». Для этой загрузки, по проверенной функции и живым
словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 4504), runtime pid 4495, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`47d707e0…`, owner один, CP `ONLINE`. Перезагрузки не было.
Резолвер IPv6 (`365e9cd`, runtime `db5790b3…`, оболочка
`af1a3974…`) стоит после резолвера IPv4. На этой загрузке кэш
даёт `cellular.dns6=yes`. В «Наблюдениях» это «DNS6 / есть». Для
этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 7843), runtime pid 7834, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`b913583f…`, owner один, CP `ONLINE`. Перезагрузки не было.
Конфиг модема (`8364cbb`, runtime `d28431d5…`, оболочка
`c358cf61…`) стоит после резолвера IPv6. На этой загрузке кэш
даёт `cellular.config=accepted`. В «Наблюдениях» это
«Конфиг / дано». Для этой загрузки, по проверенной функции и
живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 9975), runtime pid 9966, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`4da56c59…`, owner один, CP `ONLINE`. Перезагрузки не было.
SGC (`16cfdf8`, runtime `3fee548e…`, оболочка `52ddbba8…`) стоит
после конфига модема. На этой загрузке кэш даёт
`cellular.sgc=accepted`. В «Наблюдениях» это «SGC / дано». Для этой
загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 11620), runtime pid 11611, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`23338952…`, owner один, CP `ONLINE`. Перезагрузки не было.
Питание радио (`40d0bd3`, runtime `e17f6965…`, оболочка
`788f900c…`) стоит после SGC. На этой загрузке кэш даёт
`cellular.power=accepted`. В «Наблюдениях» это «Питание / дано».
Для этой загрузки, по проверенной функции и живым словам кэша,
строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 13184), runtime pid 13175, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`b582e13f…`, owner один, CP `ONLINE`. Перезагрузки не было.
Задание голосовой операции (`70ec626`, runtime `ae4729fe…`,
оболочка `792bea9c…`) стоит после питания радио. На этой загрузке
кэш даёт `cellular.voice_set=accepted`. В «Наблюдениях» это
«Задание / дано». Для этой загрузки, по проверенной функции и
живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 14113), runtime pid 14104, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`7e1748c5…`, owner один, CP `ONLINE`. Перезагрузки не было.
Применение IPv4 (`3cceaf3`, runtime `372f6f4c…`, оболочка
`52345ac0…`) стоит после задания голосовой операции. На этой
загрузке кэш даёт `cellular.ipv4=yes`. В «Наблюдениях» это
«IPv4 / есть». Для этой загрузки, по проверенной функции и живым
словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 15239), runtime pid 15230, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`e1c9b983…`, owner один, CP `ONLINE`. Перезагрузки не было.
Применение IPv6 (`59d20e9`, runtime `2235084d…`, оболочка
`6d89117b…`) стоит после применения IPv4. На этой загрузке кэш
даёт `cellular.ipv6=yes`. В «Наблюдениях» это «IPv6 / есть». Для
этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 16221), runtime pid 16212, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`88a75b82…`, owner один, CP `ONLINE`. Перезагрузки не было.
Принятие сессии данных (`1175fb6`, runtime `c73a534c…`, оболочка
`57779e9d…`) стоит после применения IPv6. На этой загрузке кэш
даёт `cellular.setup=accepted`. В «Наблюдениях» это
«Сессия / дано». Для этой загрузки, по проверенной функции и
живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 24077), runtime pid 24068, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`5443389b…`, owner один, CP `ONLINE`. Перезагрузки не было.
Принятие интернет-профиля (`9e05c22`, runtime `3eff1a41…`, оболочка
`c7f88e21…`) стоит после сессии данных. На этой загрузке кэш даёт
`cellular.profile=accepted`. В «Наблюдениях» это «Профиль / дано».
Для этой загрузки, по проверенной функции и живым словам кэша,
строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 25383), runtime pid 25374, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`6d6ca9ec…`, owner один, CP `ONLINE`. Перезагрузки не было.
Чтение активности модема (`68463ae`, runtime `3bea135b…`, оболочка
`2ceeda70…`) стоит после интернет-профиля. На этой загрузке кэш
даёт `cellular.activity=accepted`. В «Наблюдениях» это
«Активность / дано». Для этой загрузки, по проверенной функции и
живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 25972), runtime pid 25963, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`af674382…`, owner один, CP `ONLINE`. Перезагрузки не было.
Fast dormancy (`5327029`, runtime `e26e92a4…`, оболочка
`bad32682…`) стоит после чтения активности. На этой загрузке кэш
даёт `cellular.fastdorm=accepted`. В «Наблюдениях» это
«Дремота / дано». Для этой загрузки, по проверенной функции и
живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 26739), runtime pid 26730, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`024381f8…`, owner один, CP `ONLINE`. Перезагрузки не было.
Чтение ENDC (`1e66129`, runtime `5dd01eaf…`, оболочка `8b040a1f…`)
стоит после fast dormancy. На этой загрузке кэш даёт
`cellular.endc=accepted`. В «Наблюдениях» это «ENDC / дано». Для
этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 28010), runtime pid 28001, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`98a7ec17…`, owner один, CP `ONLINE`. Перезагрузки не было.
Троттлинг данных (`350a3eb`, runtime `ac361adf…`, оболочка
`511c2b9c…`) стоит после чтения ENDC. На этой загрузке кэш даёт
`cellular.throttle=accepted`. В «Наблюдениях» это «Троттлинг / дано».
Для этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 29794), runtime pid 29779, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`bf6ed923…`, owner один, CP `ONLINE`. Перезагрузки не было.
Широкий фильтр индикаций (`f6dd834`, runtime `525f6c9f…`, оболочка
`18c1f703…`) стоит после троттлинга данных. На этой загрузке кэш даёт
`cellular.unsolff=accepted`. В «Наблюдениях» это «Фильтр / дано». Для
этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 30581), runtime pid 30566, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`911726aa…`, owner один, CP `ONLINE`. Перезагрузки не было.
Установившийся фильтр индикаций (`b289ee3`, runtime `3137e227…`,
оболочка `1f28a42f…`) стоит после широкого фильтра. На этой загрузке
кэш даёт `cellular.unsol=accepted`. В «Наблюдениях» это «Отбор / дано».
Для этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · отбор дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 31650), runtime pid 31634, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`74160a18…`, owner один, CP `ONLINE`. Перезагрузки не было.
Состояние экрана (`a259cfc`, runtime `7051a24d…`, оболочка
`3fc029b8…`) стоит после установившегося фильтра. На этой загрузке
кэш даёт `cellular.screen=accepted`. В «Наблюдениях» это «Экран / дано».
Для этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · отбор дано · экран дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 32388), runtime pid 32374, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`f3826bbc…`, owner один, CP `ONLINE`. Перезагрузки не было.
Список сот (`91c5d32`, runtime `56f02ac2…`, оболочка `a1705b67…`)
стоит после состояния экрана. На этой загрузке кэш даёт
`cellular.cellinfo=accepted`. В «Наблюдениях» это «Соты / дано». Для
этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · отбор дано · экран дано · соты дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 821), runtime pid 806, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`b93b7774…`, owner один, CP `ONLINE`. Перезагрузки не было.
Чтение SMSC (`ddad740`, runtime `6a3eb77f…`, оболочка `6b01b97b…`)
стоит после списка сот. На этой загрузке кэш даёт
`cellular.smsc=accepted`. В «Наблюдениях» это «SMSC / дано». Для этой
загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · отбор дано · экран дано · соты дано · smsc дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 1468), runtime pid 1453, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`23685fb2…`, owner один, CP `ONLINE`. Перезагрузки не было.
Чтение VoNR (`06c955b`, runtime `7bfaee7c…`, оболочка `b47eb159…`)
стоит после чтения SMSC. На этой загрузке кэш даёт
`cellular.vonrget=accepted`. В «Наблюдениях» это «VoNR / дано». Для
этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · отбор дано · экран дано · соты дано · smsc дано · vonr дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 2066), runtime pid 2057, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`3687c935…`, owner один, CP `ONLINE`. Перезагрузки не было.
Чтение времени AP (`e4c8072`, runtime `c4622bf5…`, оболочка
`6cfbabdd…`) стоит после чтения VoNR. На этой загрузке кэш даёт
`cellular.aptime=accepted`. В «Наблюдениях» это «Время / дано». Для
этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · отбор дано · экран дано · соты дано · smsc дано · vonr дано · время дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 2832), runtime pid 2823, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`93eb390c…`, owner один, CP `ONLINE`. Перезагрузки не было.
Чтение трассы (`90f2c91`, runtime `d05f4bb0…`, оболочка `39dddea3…`)
стоит после чтения времени AP. На этой загрузке кэш даёт
`cellular.dbgtrace=accepted`. В «Наблюдениях» это «Трасса / дано». Для
этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · отбор дано · экран дано · соты дано · smsc дано · vonr дано · время дано · трасса дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 3419), runtime pid 3410, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`36d2078d…`, owner один, CP `ONLINE`. Перезагрузки не было.
Чтение TTY (`d15a128`, runtime `6fccd611…`, оболочка `9dca66b3…`)
стоит после чтения трассы. На этой загрузке кэш даёт
`cellular.tty=accepted`. В «Наблюдениях» это «TTY / дано». Для этой
загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · отбор дано · экран дано · соты дано · smsc дано · vonr дано · время дано · трасса дано · tty дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 4003), runtime pid 3994, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`d06f275d…`, owner один, CP `ONLINE`. Перезагрузки не было.
Чтение пакетной службы (`00e146c`, runtime `5b8718fa…`, оболочка
`29ff8d04…`) стоит после чтения TTY. На этой загрузке кэш даёт
`cellular.pssvc=accepted`. В «Наблюдениях» это «PS / дано». Для этой
загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · отбор дано · экран дано · соты дано · smsc дано · vonr дано · время дано · трасса дано · tty дано · ps дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 4586), runtime pid 4577, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`3b00489c…`, owner один, CP `ONLINE`. Перезагрузки не было.
Чтение предпочтения модема (`b5f5257`, runtime `a0330c5a…`, оболочка
`2240b47e…`) стоит после чтения пакетной службы. На этой загрузке кэш
даёт `cellular.prefmodem=accepted`. В «Наблюдениях» это «Pref / дано».
Для этой загрузки, по проверенной функции и живым словам кэша, строка
`ONLINE · домашняя · голос домашняя · включено · авто · стек · служба голосовая · операция включена · разрешение дано · присоединение дано · dns есть · dns6 есть · конфиг дано · sgc дано · питание дано · задание дано · ipv4 есть · ipv6 есть · сессия дано · профиль дано · активность дано · дремота дано · endc дано · троттлинг дано · фильтр дано · отбор дано · экран дано · соты дано · smsc дано · vonr дано · время дано · трасса дано · tty дано · ps дано · pref дано · есть · процесс · camp · удержание · уже запущен · отказ процесса · rmnet1`.
Кадр экрана не снимался. Оболочка перезапущена один раз
(pid 5170), runtime pid 5161, `saai-displayd` остался pid 408,
`saai-modemd` supervise остался pid 155, status-бинарник
`c0d56dae…`, owner один, CP `ONLINE`. Перезагрузки не было.
Пункт закрыт (`5a9aca3`). Тест дорожной карты выполнен: без модема
строка — «Нет модема», `ONLINE` без регистрации не называет оператора
и данные, имя канала берётся только у живого `rmnet`. На этой загрузке
кэш даёт `cellular.bearer=rmnet1` и `cellular.ipv4=yes`. Оператор,
уровень сигнала и идентификаторы абонента в строку не входят.

## Non-goals For Now

- voice/IMS/VoLTE
- SMS
- emergency calling
- APN editor
- operator selection
- radio firmware updates
- Android RIL, vendor cbd, or vendor rfsd execution
- generalized modem support beyond Pixel 7 `panther`
