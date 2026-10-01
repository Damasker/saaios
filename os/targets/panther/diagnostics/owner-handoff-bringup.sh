#!/bin/sh
# Explicit, one-shot Pixel 7 modem diagnostic after an AP reboot.
# Never install in native-init or run alongside another CP loader/IPC reader.
set -eu
umask 077

# Keep the early EFS mount guard independent of the invoking shell's applet
# lookup; the native image has BusyBox awk but no /bin/awk symlink.
awk() { /saaios/busybox awk "$@"; }

case "$#:$*" in
    '0:')
        PROBE=/data/saaios/bin/probe-handover-owner
        OWNER=/data/saaios/bin/modem-channel-owner
        OWNER_MODE=passive
        LOG=/data/saaios/var/owner-handoff-bringup.log
        OWNER_LOG=/data/saaios/var/modem-channel-owner.log
        ;;
    '1:scan-once')
        # A distinct probe is compiled with PROBE_OWNER_EXEC/LOG pointing at
        # a distinct owner built with SAAIOS_SCAN_ONCE. Never swap the default.
        PROBE=/data/saaios/bin/probe-handover-scan-once
        OWNER=/data/saaios/bin/modem-channel-owner-scan-once
        OWNER_MODE=scan-once
        LOG=/data/saaios/var/owner-handoff-scan-once.log
        OWNER_LOG=/data/saaios/var/modem-channel-owner-scan-once.log
        ;;
    '1:rfs-one-grant')
        # Separately compiled probe/owner; never replace the passive default.
        PROBE=/data/saaios/bin/probe-handover-rfs-one-grant
        OWNER=/data/saaios/bin/modem-rfs-one-grant-owner
        OWNER_MODE=rfs-one-grant
        LOG=/data/saaios/var/owner-handoff-rfs-one-grant.log
        OWNER_LOG=/data/saaios/var/modem-rfs-one-grant-owner.log
        ;;
    *) printf 'usage: %s [scan-once|rfs-one-grant]\n' "$0" >&2; exit 64 ;;
esac

VERIFIER=/data/saaios/bin/saaios-verify-nv-copies.sh
FIRMWARE=/data/saaios/bin/saaios-probe-b-modem.bin
PERSIST=/mnt/vendor/persist
STATE=/sys/devices/platform/cpif/modem_state
ORIGINAL_VERIFIER=/data/saaios/bin/verify-original-efs-readonly.sh
RFS_PIN=/run/saaios-rfs-one-grant/expected.sha256
RFS_QUARANTINE=/data/saaios/var/rfs-quarantine

fail() { printf 'ABORT %s\n' "$*" >&2; exit 1; }
if [ "$OWNER_MODE" = rfs-one-grant ]; then
    efs_dev=$(cat /sys/block/sda/sda5/dev 2>/dev/null) ||
        fail 'original EFS device identity unavailable'
    efs_mounts=$(awk -v dev="$efs_dev" '$3 == dev { n++ } END { print n+0 }' \
        /proc/self/mountinfo) || fail 'cannot inspect original EFS mounts'
    [ "$efs_mounts" -eq 0 ] || fail 'original EFS is mounted'
fi
[ -x "$PROBE" ] || fail 'owner handoff probe missing'
[ -x "$OWNER" ] || fail 'owner binary missing'
if [ "$OWNER_MODE" != passive ]; then
    [ "$("$OWNER" --mode 2>/dev/null)" = "$OWNER_MODE" ] ||
        fail 'diagnostic owner binary mode mismatch'
    [ "$("$PROBE" --owner-exec 2>/dev/null)" = "$OWNER" ] ||
        fail 'diagnostic probe owner path mismatch'
    [ "$("$PROBE" --owner-log 2>/dev/null)" = "$OWNER_LOG" ] ||
        fail 'diagnostic probe log path mismatch'
fi
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
if [ "$OWNER_MODE" = rfs-one-grant ]; then
    printf 'BEGIN one-grant RFS handoff; original EFS read-only; quarantine-only writes\n' >> "$LOG"
else
    printf 'BEGIN owner handoff; scan_mode=%s; no APN/PIN/CardPower/NV/EFS writes\n' \
        "$OWNER_MODE" >> "$LOG"
fi

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

if [ "$OWNER_MODE" = rfs-one-grant ]; then
    [ -x "$ORIGINAL_VERIFIER" ] || fail 'read-only original-EFS verifier missing'
    [ ! -L "$RFS_QUARANTINE" ] || fail 'quarantine parent is linked'
    if [ ! -e "$RFS_QUARANTINE" ]; then
        mkdir -m 700 "$RFS_QUARANTINE" || fail 'quarantine parent creation failed'
    fi
    [ -d "$RFS_QUARANTINE" ] &&
        [ "$(stat -c '%u:%a' "$RFS_QUARANTINE")" = 0:700 ] ||
        fail 'quarantine parent identity mismatch'
    "$ORIGINAL_VERIFIER" pin-read-only >> "$LOG" 2>&1 ||
        fail 'fresh original-EFS provenance pin failed'
    [ -f "$RFS_PIN" ] && [ ! -L "$RFS_PIN" ] &&
        [ "$(stat -c '%u:%a:%h:%s' "$RFS_PIN")" = 0:600:1:64 ] ||
        fail 'fresh pin identity mismatch'
    [ "$(cat "$STATE" 2>/dev/null)" = OFFLINE ] ||
        fail 'CP left OFFLINE during read-only EFS check'
fi

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
