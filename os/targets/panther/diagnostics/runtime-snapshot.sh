#!/bin/sh
# Read-only allowlist: never open modem character devices or dump NV/payloads.
set -eu
if [ "$#" -ne 1 ] || [ "$1" != "snapshot" ]; then
    echo "Usage: sh runtime-snapshot.sh snapshot" >&2
    exit 64
fi
base=/sys/devices/platform/cpif
for item in modem_state legacy/status legacy/region info_region \
    modem/pcie_event_stats modem/power_stats napi/rx_poll_count \
    napi/rx_int_count napi/rx_int_enable; do
    printf '\n[%s]\n' "$item"
    if [ -r "$base/$item" ]; then
        cat "$base/$item"
    else
        echo unavailable
    fi
done
printf '\n[CPU frequency policies]\n'
found=0
for policy in /sys/devices/system/cpu/cpufreq/policy*; do
    [ -d "$policy" ] || continue
    found=1
    printf '%s\n' "${policy##*/}"
    for item in scaling_driver related_cpus; do
        [ ! -r "$policy/$item" ] || cat "$policy/$item"
    done
done
[ "$found" -ne 0 ] || echo none
printf '\n[Selected interrupt counters]\n'
awk '/cp2ap|phone_active|s5300/ {print}' /proc/interrupts
printf '\n[Transport and QoS diagnostics; no packet dumps]\n'
dmesg | awk '/Reserve doorbell interrupt:|Can.t send Interrupt|cpufreq_cpu_get\(\) error|freq_qos_add_request for cpu|WARNING:.*freq_qos_update_request/ {print}'
