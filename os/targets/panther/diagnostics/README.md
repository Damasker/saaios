# Pixel 7 modem diagnostic — not a boot service

These files preserve the September 24 diagnostic implementation separately
from the user's dirty legacy cp-boot.c. The support include is a snapshot
with the ACK reader and CRC policy corrections, bounded MAIN options, and
the historical loader entry point explicitly compiled out. It is not a clean
production implementation. Do not add it to init or run concurrent loaders.

Build on the host with an ARM64 cross-compiler:

```sh
aarch64-linux-gnu-gcc -O2 -static -ffunction-sections -fdata-sections \
  -Wl,--gc-sections -DPROBE_PREAMBLE -DPROBE_FULL_MAIN \
  -DPROBE_FIRMWARE_ONLY -DPROBE_COMPLETE \
  cp-boot-probe.c -o cp-boot-probe
```

No-argument invocation refuses execution. Complete mode requires explicit
`boot-b-with-verified-nv`, the repository verifier installed at
`/tmp/saaios-verify-nv-copies.sh`, and the reviewed B modem.bin in tmpfs at
`/tmp/saaios-probe-b-modem.bin`. SHA-256 and NV checks precede POWER_ON.
Only existing userdata NV copies are read. No EFS mounting or copying is
performed here. Never substitute invented/default/zero NV.

The operator must verify original EFS remains unmounted, current CP is
fresh OFFLINE, module provenance matches the running kernel, and device
nodes match current sysfs major/minor. BOOTING/ONLINE/CRASH_EXIT is not a
fresh start: preserve logs and reboot AP before another experiment.

Hardware validation used the same transfer code in the work-in-progress
wrapper: preamble, MAIN CRC/DONE, VSS/APM/NV without CRC, FIN, then COMPLETE.
The committed packaging adds argument/hash/verification guards and disables
the historical main; do not confuse a compile check with another device run.

Opening ipc0/rfs0 allows kernel INIT_END; this probe does not implement an
RFS server or telephony service. It observes for ten seconds and exits.
ONLINE is kernel/CP boot acceptance, not SIM registration, calls or data.
No runtime NV writes are serviced. This is not suitable for unattended use.

## One-shot SIM query

Build `sit-sim-status.c` with the same static ARM64 compiler. Run `self-test`
on the host build first. `query-sim-status` sends only GET_SIM_STATUS, with
bounded receive time, no retransmission and no identifier/payload logging.
It consumes unrelated queued events; use only on the isolated diagnostic
stack, never alongside a production reader. It does not service RFS.

Optional PROBE_QUERY_SIM on the full boot wrapper invokes
`/tmp/sit-sim-status query-sim-status` immediately after COMPLETE, while
IPC/RFS remain open. The SIM result is separate from the boot exit status.
This packaging was subsequently live-tested: boot ONLINE succeeds again,
but the query remains unconsumed in FMT TX. See MODEM-RUNTIME-2026-09-24.md;
neither a working SIM query nor cellular service is claimed.

## Passive runtime snapshot

`sh runtime-snapshot.sh snapshot` reads an allowlist of ring/control/counter
attributes, CPU policy names and selected kernel diagnostics. No modem
endpoint is opened and no commands are sent. Missing attributes are labelled
unavailable. IRQ counters include boot traffic; `rx_int_enable` is not a
PCI-MSI enable verdict. See the runtime investigation for interpretation.

`sh handover-preflight.sh check-sources` inventories candidate factory
handover inputs using only availability/length checks. It prints no source
contents and never sends a handover ioctl. Exit 1 denotes missing sources
(including potentially optional ones), not a modem failure. Presence does
not validate contents, field mapping or ABI. Do not construct a zero-filled
or guessed hardware identity block to bypass missing inputs.

The pure `../src/sit-handover.h` helper is not integrated into the loader.
Run its synthetic host tests before any source-adapter work:

```sh
gcc -std=c11 -Wall -Wextra -Werror -fsanitize=address,undefined \
  ../src/test-sit-handover.c -o /tmp/test-sit-handover
/tmp/test-sit-handover
```

It validates encoding and forbids reset/factory control fields, but does not
establish field provenance or authenticate signatures. Never supply guessed
zeros for unresolved board fields simply to obtain a serialized block.

`handover-source-check.c` is an independent read-only ARM64 diagnostic:
compile with C11 -Wall -Wextra -Werror -static, invoke `check-sources`.
It checks the bootconfig CDT syntax and both identity representations without
printing their values. No signature access, block creation or ioctl exists
in its execution path. Exit 0 is representation acceptance only, not modem
readiness. It was live-tested successfully on the diagnostic phone.

Optional `candidate-no-json <signature-path>` constructs and discards a
candidate in memory, with no device ioctl or output artifact. Use only with
the documented fresh no-JSON/normal-user assumptions and a verified read-only
signature source. Project 4 and non-neutral control words are rejected.
Core dumps/dumpability are disabled and execution has a 15-second deadline.
The candidate path was live-tested; no handover was sent to CP.

## Opt-in handover comparison (subsequent successful live test)

Add `-DPROBE_HANDOVER -DPROBE_QUERY_SIM` to the complete guarded build.
It now also includes handover-source-check.c and ../src/sit-handover.h.
Requires the distinct argument `boot-b-with-verified-nv-handover` followed
by the verified read-only signature pathname. It builds the block in RAM,
sends ioctl 0x6f57 after START before the preamble, then clears its buffer.
This is intentionally not the default diagnostic mode or a boot service.

The reviewed device wrapper `run-handover-comparison.sh run-once` expects
fresh module/B firmware preparation and binaries at /tmp/probe-handover and
/tmp/sit-sim-status, plus the existing NV verifier. It creates runtime nodes
only if absent and never replaces existing ones. No concurrent loaders or
RIL consumers are allowed. The no-JSON normal profile remains device-specific.

One live comparison succeeded: SIM response length 80, error_raw 0, TX ring
consumed 24/24. This supersedes the prior statement that all runtime queries
stall; it does not establish cellular service. See the runtime report.

`sit-sim-status query-radio-state` sends one factory-verified GET_RADIO_STATE
(0x0801) with token 2 and prints only the raw 32-bit state. Live response:
error 0, state 10 (factory ON enum). A later SIM query reports one application;
early post-boot zero-application snapshots must not be treated as permanent
absence. Neither query proves network registration.

`sit-sim-status tray-watch` dense-polls `0x0200` for up to 180 s (exits early
on READY/DETECTED/ABSENT→PRESENT/pin1 change). Logs Present inference from
app_state only (`+0xBF6` is not on the wire). Use while physically reseating
the SIM tray; then chase Radio/LTE/reg/rmnet if state leaves PIN+DISABLED.

## Soft-lock unblock: one command

Stock CP often boots ONLINE with `app_state=PIN` and Present ∉ {1,2,3}
(MODEM-06). Live shapes: `pin1=DISABLED(3)` or `pin1=ENABLED_VERIFIED(2)`
after remote VerifyPin — both still block START_NETWORK. ATU/SHMEM Present
pokes are dead — see `docs/os/targets/panther/MODEM-BLOCKER.md`.

**Post-sysrq soft ONLINE (one command):**

```sh
# after echo b > /proc/sysrq-trigger and shell back:
sh /data/saaios/bin/reboot-soft-bringup.sh
# prefers probe-handover-clean + probe-no0200 (no early 0x0200 NET path)
```

Repo copy: `reboot-soft-bringup.sh` in this directory.

**CardPower one-shot (tray analog — not a reseat substitute):**

```sh
# ONLINE only; proven 0x024c DOWN(4)→UP(1); no VerifyPin in helper
sh /data/saaios/bin/cardpower-reseat.sh
# or: /tmp/cardpower-reseat
```

Live: pin1 may drop 2→1 (VerifyPin window); card stays PRESENT. Does **not**
produce ABSENT→PRESENT / READY. Do not spam. Arm `tray-bearer-chase` after
(or before) so PIN+pin1=1 is treated as EDGE → VerifyPin A+AID.

After ONLINE, deploy `sit-sim-status` and run:

```sh
# ~36 min per batch (12×180s); PERSIST=1 re-arms forever until bearer
PERSIST=1 WATCH_ROUNDS=12 OUT=/data/saaios/var/tray-bearer.log \
  sh os/targets/panther/diagnostics/tray-bearer-chase.sh
# on phone after deploy:
#   nohup env PERSIST=1 WATCH_ROUNDS=12 \
#     sh /data/saaios/bin/tray-bearer-chase.sh \
#     >/data/saaios/var/tray-bearer.nohup 2>&1 &
# confirm looping: cat /data/saaios/var/tray-bearer.alive
# optional APN for SetupDataCall 0x0600 (hostname only; never log):
#   echo internet >/data/saaios/etc/apn   # Life (life:) public default
#   tree example: diagnostics/apn.example-life
```

The script loops `tray-watch` (PERSIST re-arms batches). On DETECTED/READY/ABSENT→PRESENT (or
pin1 left soft-lock): if pin1≠2 and remain>1, runs `VERIFY_TOOL`
(`/tmp/card-then-verify-a`: CardPower then VerifyPin A **with AID**);
then Radio / LTE_ONLY / AllowData / GetPsService / data-reg /
GetDataCallList / SetupDataCall(if APN) / `rmnet`
IPv4 (or bidirectional rx/tx). Logs CPIF capability offsets at start.
Stops if remain≤1. Never logs PIN digits or AID. No POWER_OFF / crash.
Refuses a second concurrent chase process.

`sim_left_pin`: app_raw=2 + pin1∈{2,3} is **not** EDGE (soft-lock).
app_raw=2 + pin1∈{0,1} **is** EDGE (VerifyPin window after CardPower/tray).
BusyBox BRE `\|` previously false-chased; use `grep -E` / numeric checks.

`cp-dram-mmap-probe probe-mmap-only` opens CPIF iod nodes and tries
`mmap` RW/RO after ONLINE. Live result: **ENODEV** (no userspace CP DRAM
map). Do not use this as a poke tool — it only proves the mapping gap.

`query-data-registration` sends only the factory domain-2 status GET (0x0701,
token 3), printing registration/reject/technology bytes without location or
subscriber data. Live result was error 0, registration 0 (not registered,
not searching). No operator selection or attachment command is implemented.

## Catalog OEM `0x2f50` — armed inject + chase (no invent)

Soft-lock still needs an **external** catalog OEM `SIM_INIT_REQ` (`0x2f50`)
frame. Do **not** invent header/body bytes. Do **not** start `rild`/`cbd`
under the current ban. Capture procedure (policy-gated): `OEM-IPC-CAPTURE.md`.

```sh
# host parser self-test
gcc -std=c11 -Wall -Wextra -Werror oem-ipc-inject.c -o /tmp/oem-ipc-inject
/tmp/oem-ipc-inject self-test

# device (once operator has a capture file — raw or hex)
aarch64-linux-gnu-gcc -O2 -static -Wall -Wextra -Werror \
  oem-ipc-inject.c -o /data/saaios/bin/oem-ipc-inject
sh post-init-chase.sh --frame /data/saaios/var/oem-2f50.frame
# CHASE_ONCE=1 is also accepted by tray-bearer-chase.sh after READY
```

`oem-ipc-inject inject <file> [N]` writes once to `/dev/oem_ipcN`, refuses
empty/all-zero frames, logs length/errno only. `post-init-chase` polls
GET_APP for `app∈{1,4,5}` then runs the existing post-edge bearer pipeline.
