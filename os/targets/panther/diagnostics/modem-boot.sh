#!/saaios/busybox sh
# One camp per boot. Move the previous diagnostic logs aside so the
# guarded handoff can run, then replace this process with it.
set -eu
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
