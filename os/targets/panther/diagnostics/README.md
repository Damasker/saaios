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
