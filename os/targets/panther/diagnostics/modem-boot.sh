#!/bin/sh
# One camp per boot. Archive the previous logs, then hand the boot to
# saai-modemd. If that binary is absent, run the handoff directly so PID 1
# still reaches the camp.
set -eu
export PATH=/saaios:/bin:/usr/bin
# cpif needs the PCIe root complex and modem control. Those are loaded
# later in native-init, so wait for them. One attempt, then exit.
ready=0
i=0
while [ "$i" -lt 90 ]; do
    if /saaios/busybox grep -q '^pcie_exynos_gs ' /proc/modules \
        && /saaios/busybox grep -q '^google_modemctl ' /proc/modules; then
        ready=1
        break
    fi
    i=$((i + 1))
    /saaios/busybox sleep 1
done
if [ "$ready" -ne 1 ]; then
    echo 'modem prerequisites missing' >&2
    exit 1
fi
base=$(/saaios/busybox date +%s)
ARCH=/data/saaios/var/boot-archive/$base
n=0
while [ -e "$ARCH" ]; do
    n=$((n + 1))
    ARCH=/data/saaios/var/boot-archive/${base}-$n
done
/saaios/busybox mkdir -p "$ARCH"
for name in owner-handoff-rfs-camp.log \
            modem-rfs-camp-combined-owner.log \
            oem-ipc-responder.log; do
    if [ -e "/data/saaios/var/$name" ]; then
        /saaios/busybox mv "/data/saaios/var/$name" "$ARCH/$name"
    fi
done
if [ -e /data/saaios/var/rfs-quarantine/replay ]; then
    /saaios/busybox mv /data/saaios/var/rfs-quarantine/replay "$ARCH/replay"
fi
if [ -x /data/saaios/bin/saai-modemd ]; then
    exec /data/saaios/bin/saai-modemd supervise
fi
exec /data/saaios/bin/owner-handoff-rfs-oemipc.sh rfs-camp-combined
