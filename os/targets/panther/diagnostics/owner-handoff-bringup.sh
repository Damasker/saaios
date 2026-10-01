#!/bin/sh
# Explicit, one-shot Pixel 7 modem diagnostic after an AP reboot.
# Never install in native-init or run alongside another CP loader/IPC reader.
set -eu
umask 077

PROBE=/data/saaios/bin/probe-handover-owner
OWNER=/data/saaios/bin/modem-channel-owner
VERIFIER=/data/saaios/bin/saaios-verify-nv-copies.sh
FIRMWARE=/data/saaios/bin/saaios-probe-b-modem.bin
LOG=/data/saaios/var/owner-handoff-bringup.log
OWNER_LOG=/data/saaios/var/modem-channel-owner.log
PERSIST=/mnt/vendor/persist
STATE=/sys/devices/platform/cpif/modem_state

fail() { printf 'ABORT %s\n' "$*" >&2; exit 1; }
[ -x "$PROBE" ] || fail 'owner handoff probe missing'
[ -x "$OWNER" ] || fail 'owner binary missing'
[ -x "$VERIFIER" ] || fail 'NV-copy verifier missing'
[ -f "$FIRMWARE" ] || fail 'reviewed B firmware missing'
if [ -e "$STATE" ]; then
    [ "$(cat "$STATE" 2>/dev/null)" = OFFLINE ] ||
        fail 'CP must be OFFLINE before diagnostic setup'
fi
[ ! -L "$LOG" ] && [ ! -L "$OWNER_LOG" ] || fail 'log path is a symlink'
[ ! -e "$LOG" ] && [ ! -e "$OWNER_LOG" ] || fail 'diagnostic logs already exist'
if grep -q " $PERSIST " /proc/mounts; then
    fail 'persist already mounted'
fi

mkdir -p /data/saaios/var "$PERSIST" /dev/block
: > "$LOG"
: > "$OWNER_LOG"
chmod 600 "$LOG" "$OWNER_LOG"
printf 'BEGIN owner handoff; no APN/PIN/CardPower/NV/EFS writes\n' >> "$LOG"

insmod /lib/modules/shm_ipc.ko 2>/dev/null || true
insmod /lib/modules/cpif_page.ko 2>/dev/null || true
insmod /lib/modules/cpif.ko 2>/dev/null || true
insmod /lib/modules/cp_thermal_zone.ko 2>/dev/null || true
[ "$(cat "$STATE" 2>/dev/null)" = OFFLINE ] ||
    fail 'CP must be OFFLINE after module setup'

if [ ! -e /dev/block/sda1 ]; then
    set -- $(cat /sys/block/sda/sda1/dev | tr : ' ')
    mknod /dev/block/sda1 b "$1" "$2"
fi
set -- $(cat /sys/block/sda/sda1/dev | tr : ' ')
[ "$(stat -c '%t:%T' /dev/block/sda1)" = "$(printf '%x:%x' "$1" "$2")" ] ||
    fail 'persist block-node identity mismatch'
for name in umts_boot0 umts_ipc0 umts_rfs0; do
    if [ ! -e "/dev/$name" ]; then
        set -- $(cat "/sys/class/cpif/$name/dev" | tr : ' ')
        mknod "/dev/$name" c "$1" "$2"
    fi
    set -- $(cat "/sys/class/cpif/$name/dev" | tr : ' ')
    [ "$(stat -c '%t:%T' "/dev/$name")" = "$(printf '%x:%x' "$1" "$2")" ] ||
        fail "$name character-node identity mismatch"
done

if [ -e /tmp/saaios-probe-b-modem.bin ] &&
   [ ! -L /tmp/saaios-probe-b-modem.bin ]; then
    fail 'firmware tmp path is occupied by a non-symlink'
fi
ln -sfn "$FIRMWARE" /tmp/saaios-probe-b-modem.bin
cp "$VERIFIER" /tmp/saaios-verify-nv-copies.sh
chmod 700 /tmp/saaios-verify-nv-copies.sh

mount -t ext4 -o ro,noload,nosuid,nodev,noexec /dev/block/sda1 "$PERSIST"
probe_rc=0
"$PROBE" boot-b-with-verified-nv-handover "$PERSIST/modem/cpsha" \
    >> "$LOG" 2>&1 || probe_rc=$?
umount "$PERSIST" || fail 'could not unmount read-only persist'
printf 'probe_rc=%s cp_state=%s\n' "$probe_rc" "$(cat "$STATE" 2>/dev/null)" >> "$LOG"
tail -n 8 "$LOG"
[ "$probe_rc" -eq 0 ] || exit "$probe_rc"
[ "$(cat "$STATE" 2>/dev/null)" = ONLINE ] || fail 'CP did not reach ONLINE'
printf 'probe returned ONLINE; verify owner continuity in %s\n' "$OWNER_LOG"
