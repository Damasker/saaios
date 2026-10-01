# saai-modemd

Maintained boundary for Pixel 7 native modem work.

**Current hardware evidence (2026-10-01):** the separate diagnostic owner
reaches CP ONLINE, SIM READY/PIN disabled and radio ON. Its newly reviewed
logical-stack status GET `0x0810` also returned enabled, but voice/data
registration remain 0 and there is no cellular bearer. The older PIN/soft-lock
notes below are historical cases, not the current blocker. See the
[modem roadmap](../../docs/os/sprints/MODEM-ROADMAP.md) and
[factory evidence/live results](../../docs/os/sprints/MODEM-07-RFS-QUARANTINE.md#native-radio-service-boundary-and-logical-stack-check-2026-10-01).
The native service must keep SIM, logical-stack, radio, registration and
bearer observations separate; READY/ON/enabled do not imply registration.
An isolated late factory-carrier SGC request was also accepted, but its
separate post-request status sweep still showed registration 0. The next
diagnostic hypothesis is early factory initialization timing, not a missing
Android application framework or a proven need for another enable command.

This service is intentionally conservative. It does not power on the modem,
issue ioctls, mount EFS, serve RFS, start a RIL consumer, or run the September
diagnostic loader. The first landed surface is a safe status/preflight CLI so
the proven modem sequence has a product home without making the probe an init
service.

Current host-safe commands:

```sh
cargo run -p saai-modemd -- status
cargo run -p saai-modemd -- preflight --modem-state /path/to/state
cargo run -p saai-modemd -- inspect-image --image /path/to/modem.bin
# MODEM-06 published PIN state → prints observations (no modem I/O)
cargo run -p saai-modemd -- soft-lock --app 2 --pin1 2
cargo run -p saai-modemd -- soft-lock --app 2 --pin1 3
cargo run -p saai-modemd -- soft-lock --from-text /data/saaios/var/tray-bearer.log
# Post-EDGE plan (host-safe): guarded READY(5)→registration→rmnet
cargo run -p saai-modemd -- post-edge --app 2 --pin1 2
cargo run -p saai-modemd -- post-edge --app 5 --pin1 2
cargo run -p saai-modemd -- post-edge --app 5 --pin1 2 --apn internet.example.apn
cargo run -p saai-modemd -- post-edge --app 2 --pin1 1 --allow-pin-verify
cargo run -p saai-modemd -- post-edge --app 5 --pin1 2 --apn internet --allow-setup-data-call
cargo run -p saai-modemd -- post-edge --app 5 --pin1 2 --apn-file /data/saaios/etc/apn
cargo run -p saai-modemd -- post-edge --app 5 --pin1 2 --rmnet-rx 0 --rmnet-tx 336
```

`inspect-image` parses the TOC, validates the reviewed Panther BOOT/MAIN/VSS/
APM/NV stage layout, builds the reviewed boot plan, validates the executor
action contract, and prints stage command metadata. It opens no device and
performs no modem action.

The maintained boot model also includes a pure progress state machine. After a
recorded failure it refuses to advance further stages; after bootloader start,
failures are classified as requiring a fresh AP boot before another attempt.
Executor-facing helpers bind stage payload sources, enforce the reviewed CRC
policy, report ACK mismatches with explicit expected/got words, and map
executor outcomes into the same progress state machine.

Next production work belongs here:

- port the verified S5100SIT boot sequence out of diagnostics;
- preserve the factory handover block guards;
- add an isolated RFS design before enabling long-running runtime use;
- expose registration state only after factory post-SIM init is understood.

## Soft-lock / bearer (operator, not daemon)

Live Panther can report `app_state=PIN` after stock ONLINE (MODEM-06).
START_NETWORK only allows app∈{1,4,5}; observed PIN forms include:

| pin1 | meaning | remote status |
| --- | --- | --- |
| 3 DISABLED | PIN published despite disabled PIN | no VerifyPin path |
| 2 ENABLED_VERIFIED | Pin1Verified OK | app may still be PIN; no camp |
| 1 NOT_VERIFIED | observed after reboot | explicit VerifyPin A+AID path reached READY twice |

SIT status reports the application state at wire byte 16. Byte 17 is the
personalization substate; neither byte reports the CP's current Present field.
`soft-lock` uses the published application state and PIN status for its
verdict. It can print the watch command without opening modem endpoints:

```sh
cargo run -p saai-modemd -- soft-lock --app 2 --pin1 2
```

The diagnostic watch records subsequent SIM state changes. Physical tray
reseat was tested on this device and did not by itself reach READY.

The SIM's PIN request is disabled according to the owner. The watch never
attempts VerifyPin by default, even if the modem reports pin1=1. A separate,
intentional PIN test requires `ALLOW_PIN_VERIFY=1`, an explicit `VERIFY_TOOL`
path, and more than one remaining attempt. Tool output is withheld from the
watch log; the script records only its exit code and the subsequent SIM state.
Each watcher process permits at most one VerifyPin attempt.
The host-side `post-edge` command is also conservative by default: `--apn`
alone only records the candidate APN; it does not arm SetupDataCall. Its
`--allow-pin-verify` and `--allow-setup-data-call` flags change the printed
conditional plan only, never perform device I/O. On device, the watcher has
separate environment opt-ins and checks a fresh successful SIM query with
card present before any VerifyPin attempt.

Diagnostic references:

- Blocker one-pager: `docs/os/targets/panther/MODEM-BLOCKER.md`
- Post-sysrq soft ONLINE (one command on device):
  `sh /data/saaios/bin/reboot-soft-bringup.sh`
  (repo: `os/targets/panther/diagnostics/reboot-soft-bringup.sh`; prefers
  `probe-handover-clean` then `probe-no0200`)
- Then arm chase: `os/targets/panther/diagnostics/tray-bearer-chase.sh`
  (`WATCH_ROUNDS=12`, log `/data/saaios/var/tray-bearer.log`)

Signed CPIF capabilities (AP part0=3, CP part0=7) negotiate at `INIT_START`
and are reported by `soft-lock` / chase (`cpif_caps_exercised=yes`). The
observed blocker **while app=PIN** is `cp_app_state_pin_blocks_start_network`.
A concurrent diagnostic run twice reached READY/pin1=2 from PIN/pin1=1 by
sending VerifyPin A+AID without CardPower. Its SIT ownership overlapped the
factory RFS 7→3→6 test, explaining the temporary empty queries: the RFS test
cannot establish CP self-init or cause the later READY observation. From
READY, network selection auto returned err0, but registration remained 0 and
there was no rmnet IPv4. These observations do not establish the CP's
internal Present field. Catalog id
`0x2f50` is internal to the CP and is not an evidenced AP injection frame.

### Next diagnostic question (READY→network registration)

VerifyPin A+AID without CardPower has now opened the network gate twice from
PIN/pin1=1. The guarded `ready-network-once run` requires fresh READY(5)
and radio ON(10), checks selection mode, sends auto selection only when needed,
and sends AllowData once. With pin1=1, VerifyPin first requires the explicit
opt-in described above. The subsequent path is: guarded runner → data
registration 1/5 → `SetupDataCall` `0x0600` len246 **only after registration
1/5, explicit `ALLOW_SETUP_DATA_CALL=1`, and with** an
operator-provided APN in `/data/saaios/etc/apn` (single-label `internet` or
dotted; never invent a carrier value) → `rmnet` IPv4 or rx+tx.
Without PIN opt-in, soft-lock PIN never arms SetupDataCall
(`blocked_soft_lock`). An opted-in host plan remains conditional on a later
fresh READY state and registration 1/5.
`tray-bearer-chase.sh` calls `/data/saaios/bin/ready-network-once run` when a
fresh SIM status confirms READY(5); it no longer calls unsupported SET modes
through `sit-sim-status`, opens RFS through old GET helpers, or repeats
AllowData. It waits for data registration 1/5 before an explicitly opted-in
SetupDataCall (`PERSIST=1` default). Confirm watch:
`cat /data/saaios/var/tray-bearer.alive`.

`saai-modemd` must not power the modem or issue SIT from init. Goal complete
only with live `rmnet`/IPv4 observed on device.
