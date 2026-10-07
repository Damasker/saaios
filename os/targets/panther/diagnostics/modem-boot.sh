#!/bin/sh
# One camp per boot. Move the previous diagnostic logs aside so the
# guarded handoff can run, then replace this process with it.
set -eu
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
ARCH=/data/saaios/var/boot-archive/$(/saaios/busybox date +%s)
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
exec /data/saaios/bin/owner-handoff-rfs-oemipc.sh rfs-camp-combined
