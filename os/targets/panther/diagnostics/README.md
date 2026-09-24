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
