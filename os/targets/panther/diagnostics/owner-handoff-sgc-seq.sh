#!/bin/sh
# Explicit, one-shot Pixel 7 modem diagnostic after an AP reboot.
# Never install in native-init or run alongside another CP loader/IPC reader.
#
# sgc-seq variant: identical guarded bring-up to the sgc-early wrapper, but the
# owner dispatches the stock stage-1 pair in order on the early radio edge
# (0x0803 then 0x0802 raw 0): SetModemsConfig (0x093f) then the factory SGC
# (0x0404), matching MiscService::OnRadioAvailable. Original EFS is never
# mounted read-write; persist is mounted read-only only to read the cpsha for
# the verified NV handover, exactly as the sgc-once path does.
set -eu
umask 077

# Keep the early EFS mount guard independent of the invoking shell's applet
# lookup; the native image has BusyBox awk but no /bin/awk symlink.
awk() { /saaios/busybox awk "$@"; }

case "$#:$*" in
    '1:sgc-seq-once')
        # Stock-ordered SetModemsConfig then carrier SET on the radio-available
        # edge, then a separate status sweep. Separate paths preserve the
        # passive binary and its recovery route.
        PROBE=/data/saaios/bin/probe-handover-sgc-seq-once
        OWNER=/data/saaios/bin/modem-channel-owner-sgc-seq-once
        OWNER_MODE=sgc-seq-once
        LOG=/data/saaios/var/owner-handoff-sgc-seq.log
        OWNER_LOG=/data/saaios/var/modem-channel-owner-sgc-seq-once.log
        ;;
    *) printf 'usage: %s sgc-seq-once\n' "$0" >&2; exit 64 ;;
esac

VERIFIER=/data/saaios/bin/saaios-verify-nv-copies.sh
FIRMWARE=/data/saaios/bin/saaios-probe-b-modem.bin
PERSIST=/mnt/vendor/persist
STATE=/sys/devices/platform/cpif/modem_state

fail() { printf 'ABORT %s\n' "$*" >&2; exit 1; }
[ -x "$PROBE" ] || fail 'owner handoff probe missing'
[ -x "$OWNER" ] || fail 'owner binary missing'
[ "$("$OWNER" --mode 2>/dev/null)" = "$OWNER_MODE" ] ||
    fail 'diagnostic owner binary mode mismatch'
[ "$("$PROBE" --owner-exec 2>/dev/null)" = "$OWNER" ] ||
    fail 'diagnostic probe owner path mismatch'
[ "$("$PROBE" --owner-log 2>/dev/null)" = "$OWNER_LOG" ] ||
    fail 'diagnostic probe log path mismatch'
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
printf 'BEGIN active sgc-seq-once; stock-ordered 0x093f then TD1A europen carrier SET on radio-available edge; no APN/PIN/CardPower commands or host NV/EFS writes\n' >> "$LOG"

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
