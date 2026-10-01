# saai-modemd

Maintained boundary for Pixel 7 native modem work.

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
# MODEM-06 soft-lock → prints tray-bearer-chase one-command (no modem I/O)
cargo run -p saai-modemd -- soft-lock --app 2 --pin1 2 --present notin
cargo run -p saai-modemd -- soft-lock --app 2 --pin1 3 --present notin
cargo run -p saai-modemd -- soft-lock --from-text /data/saaios/var/tray-bearer.log
# Post-EDGE plan (host-safe): VerifyPin→START_NETWORK{1,4,5}→LTE→PS→rmnet
cargo run -p saai-modemd -- post-edge --app 2 --pin1 2 --present notin
cargo run -p saai-modemd -- post-edge --app 5 --pin1 2 --present 2
cargo run -p saai-modemd -- post-edge --app 5 --pin1 2 --apn internet.example.apn
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

Live Panther often stays `app_state=PIN` after stock ONLINE (MODEM-06). Two
shapes share the chicken-egg (START_NETWORK only allows app∈{1,4,5}):

| pin1 | meaning | remote status |
| --- | --- | --- |
| 3 DISABLED | classic soft-lock | no VerifyPin path |
| 2 ENABLED_VERIFIED | Pin1Verified OK | app still PIN; no camp |

Kernel ATU/SHMEM cannot poke Present/GET_APP. Detect and print the chase
command without opening modem endpoints:

```sh
cargo run -p saai-modemd -- soft-lock --app 2 --pin1 2 --present notin
```

Until a factory-clean SIM session path lands in this service, the durable
operator unblock is physical tray reseat + diagnostics chase:

- Blocker one-pager: `docs/os/targets/panther/MODEM-BLOCKER.md`
- Post-sysrq soft ONLINE (one command on device):
  `sh /data/saaios/bin/reboot-soft-bringup.sh`
  (repo: `os/targets/panther/diagnostics/reboot-soft-bringup.sh`; prefers
  `probe-handover-clean` then `probe-no0200`)
- Then arm chase: `os/targets/panther/diagnostics/tray-bearer-chase.sh`
  (`WATCH_ROUNDS=12`, log `/data/saaios/var/tray-bearer.log`)

Signed CPIF capabilities (AP part0=3, CP part0=7) negotiate at `INIT_START`
and are reported by `soft-lock` / chase (`cpif_caps_exercised=yes`); they are
**not** a READY lever. Soft-lock `blocker=` is
`waiting_external_catalog_oem_0x2f50_SIM_INIT_REQ_frame` (capture-only rild
or external dump → `oem-ipc-inject` + `post-init-chase`; no invent / no rild
under ban). HotSwap reseat and CP USIM self-init were live-falsified.

### RE conclusion (PIN→READY)

READY(#5) sole writer needs Present/`+0xBF6==2`. Present=2 only via FN_A
(CDMA L1); panther EU `No CDMA in SupportedRatMap` (NV/TCS on QM_MM_INIT)
⇒ remote cannot READY. Preferred `0x070a` (11/12 live) does not mutate
RatMap — do not try CDMA preferred. `post-edge` plans the automatic path
**after** tray EDGE opens app∈{1,4,5}:
VerifyPin (if pin1 0|1) → Radio → LTE_ONLY → selection auto → AllowData
`0x0710` → GetPsService `0x0711` → data-reg → GetDataCallList `0x0602` →
`SetupDataCall` `0x0600` len246 **if** `/data/saaios/etc/apn` has a real
carrier hostname (never invent) → `rmnet` IPv4 or rx+tx.
Soft-lock PIN never arms SetupDataCall (`blocked_soft_lock`).
`tray-bearer-chase.sh` executes that plan on device (`PERSIST=1` default;
tools from `/tmp` or `/data/saaios/bin`). Confirm watch:
`cat /data/saaios/var/tray-bearer.alive`.

`saai-modemd` must not power the modem or issue SIT from init. Goal complete
only with live `rmnet`/IPv4 observed on device.
