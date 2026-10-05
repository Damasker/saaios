#!/bin/sh
# Explicit, one-shot Pixel 7 modem diagnostic after an AP reboot.
# Never install in native-init or run alongside another CP loader/IPC reader.
#
# rfs-camp variant: the combined owner holds BOTH umts_ipc0 and umts_rfs0. It
# serves the modem's protected-NV cmd7/cmd3/cmd6 sequence INTO THE QUARANTINE
# COPY only (original EFS is never mounted read-write; persist is mounted
# read-only only to read the cpsha, exactly as the sgc/one-grant paths do) and
# dispatches the stock stage-1 0x093f -> 0x0404 -> 0x0800 on the early radio
# edge (0x0803 then 0x0802 raw 0).
set -eu
umask 077

# Keep the early EFS mount guard independent of the invoking shell's applet
# lookup; the native image has BusyBox awk but no /bin/awk symlink.
awk() { /saaios/busybox awk "$@"; }

case "$#:$*" in
    '1:rfs-camp-combined')
        PROBE=/data/saaios/bin/probe-handover-rfs-camp-replay
        OWNER=/data/saaios/bin/modem-rfs-camp-combined-owner
        OWNER_MODE=rfs-camp-combined
        LOG=/data/saaios/var/owner-handoff-rfs-camp.log
        OWNER_LOG=/data/saaios/var/modem-rfs-camp-combined-owner.log
        ;;
    *) printf 'usage: %s rfs-camp-combined\n' "$0" >&2; exit 64 ;;
esac

VERIFIER=/data/saaios/bin/saaios-verify-nv-copies.sh
FIRMWARE=/data/saaios/bin/saaios-probe-b-modem.bin
PCIE_STABILIZE=/data/saaios/bin/pcie-stabilize-cp.sh
PCIE_RC=/sys/devices/platform/11920000.pcie
PERSIST=/mnt/vendor/persist
STATE=/sys/devices/platform/cpif/modem_state
EFS_COPY=/data/saaios/var/efs-copy/nv_protected.bin
QDIR=/run/saaios-rfs-one-grant
PIN="$QDIR/expected.sha256"

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
REPLAY_COPY=/data/saaios/var/efs-copy/replay_region.bin
[ -f "$REPLAY_COPY" ] && [ ! -L "$REPLAY_COPY" ] || fail 'REPLAY copy missing'
[ "$(stat -c '%s:%u' "$REPLAY_COPY")" = 524288:0 ] || fail 'REPLAY copy size/owner'

# Immutable quarantine source (prior read-only EFS provenance copy). This path
# is an input only; it is never created, modified or renamed here.
[ -f "$EFS_COPY" ] || fail 'verified 524288-byte NV copy missing (run provenance first)'
[ ! -L "$EFS_COPY" ] || fail 'NV copy is a symlink'
[ "$(stat -c '%s' "$EFS_COPY" 2>/dev/null)" = 524288 ] ||
    fail 'NV copy size unexpected'
[ "$(stat -c '%u' "$EFS_COPY" 2>/dev/null)" = 0 ] || fail 'NV copy not root-owned'
# Re-run the existing provenance/hash/MD5 verifier before any candidate.
"$VERIFIER" >/dev/null 2>&1 || fail 'NV-copy provenance verifier failed'

if [ -e "$STATE" ]; then
    [ "$(cat "$STATE" 2>/dev/null)" = OFFLINE ] ||
        fail 'CP must be OFFLINE before diagnostic setup'
fi
[ ! -L "$LOG" ] && [ ! -L "$OWNER_LOG" ] || fail 'log path is a symlink'
[ ! -e "$LOG" ] && [ ! -e "$OWNER_LOG" ] || fail 'diagnostic logs already exist'
if grep -q " $PERSIST " /proc/mounts; then
    fail 'persist already mounted'
fi

# Private tmpfs quarantine control dir and the expected-source pin. The pin is
# the SHA-256 of the whole immutable NV copy; it gates the final durable ACK,
# never the CP's quarantined bytes.
grep -q ' /run tmpfs ' /proc/mounts || fail '/run is not tmpfs'
[ ! -e "$QDIR" ] || fail 'quarantine control dir already exists'
mkdir -m 700 "$QDIR" || fail 'could not create quarantine control dir'
[ "$(stat -c '%u:%a' "$QDIR")" = '0:700' ] || fail 'quarantine control dir perms'
sha256sum "$EFS_COPY" | awk '{printf "%s", $1}' > "$PIN" || fail 'could not write pin'
chmod 600 "$PIN"
[ "$(stat -c '%s' "$PIN")" = 64 ] || fail 'pin not 64 hex bytes'

mkdir -p /data/saaios/var "$PERSIST" /dev/block
: > "$LOG"
: > "$OWNER_LOG"
chmod 600 "$LOG" "$OWNER_LOG"
printf 'BEGIN rfs-camp-combined; serves protected-NV cmd7/3/6 to quarantine copy only; dispatches 0x093f/0x0404/0x0800 on radio edge; no APN/PIN/CardPower commands, no original-EFS write\n' >> "$LOG"

insmod /lib/modules/shm_ipc.ko 2>/dev/null || true
insmod /lib/modules/cpif_page.ko 2>/dev/null || true
insmod /lib/modules/cpif.ko 2>/dev/null || true
insmod /lib/modules/cp_thermal_zone.ko 2>/dev/null || true
[ "$(cat "$STATE" 2>/dev/null)" = OFFLINE ] ||
    fail 'CP must be OFFLINE after module setup'

# Transport stability: disable PCIe root-complex runtime suspend before the CP
# boots so the link never enters an unrecoverable low-power state. The endpoint
# L1.2 disable (which needs the EP enumerated) is handled by the bounded
# background stabilizer launched after ONLINE. PCIe ASPM/PM policy only.
if [ -w "$PCIE_RC/power/control" ]; then
    echo on > "$PCIE_RC/power/control" 2>/dev/null || true
    printf 'pcie_rc_runtime_pm=%s\n' "$(cat "$PCIE_RC/power/control" 2>/dev/null)" >> "$LOG"
fi

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

RESPONDER=/data/saaios/bin/oem-ipc-responder
RESPONDER_LOG=/data/saaios/var/oem-ipc-responder.log
UECAP=/data/saaios/var/uecap/WILDCARD.binarypb
[ -x "$RESPONDER" ] || fail 'oem-ipc responder missing'
[ "$(sha256sum "$UECAP" | awk '{print $1}')" = 7e556ae2b771c19a297a58ad941f29a7fcfdcc5e08061c178eb6038b3f8ad9f3 ] ||
    fail 'uecap copy differs from stock vendor file'
[ ! -e "$RESPONDER_LOG" ] || fail 'responder log already exists'
# CP REPLAY_PATH writes (dds.bin) land only here, never in modem_userdata.
REPLAY_Q=/data/saaios/var/rfs-quarantine/replay
mkdir -p -m 700 /data/saaios/var/rfs-quarantine
[ ! -e "$REPLAY_Q" ] || fail 'replay quarantine dir already exists'
mkdir -m 700 "$REPLAY_Q" || fail 'could not create replay quarantine dir'
[ -d "$REPLAY_Q" ] && [ ! -L "$REPLAY_Q" ] || fail 'replay quarantine dir is not a plain dir'
for name in oem_ipc1 oem_ipc3; do
    if [ ! -e "/dev/$name" ]; then
        set -- $(cat "/sys/class/cpif/$name/dev" | tr : ' ')
        mknod "/dev/$name" c "$1" "$2"
    fi
    set -- $(cat "/sys/class/cpif/$name/dev" | tr : ' ')
    [ "$(stat -c '%t:%T' "/dev/$name")" = "$(printf '%x:%x' "$1" "$2")" ] ||
        fail "$name character-node identity mismatch"
done
setsid "$RESPONDER" run 600 "$RESPONDER_LOG" </dev/null >/dev/null 2>&1 &
printf 'oem_ipc_responder pid=%s window=600s\n' "$!" >> "$LOG"
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
# Launch the bounded PCIe stabilizer in the background: it re-applies the RC
# runtime-PM and endpoint L1.2 disable every second across the owner's
# RadioPower-ON (0x0800) dispatch, keeping/recovering the link at L0 so cpif can
# keep ringing the ap2cp doorbell. PCIe ASPM/PM policy only; bounded; reversible.
if [ -x "$PCIE_STABILIZE" ]; then
    "$PCIE_STABILIZE" 120 >> "$LOG" 2>&1 &
    printf 'pcie_stabilizer_launched pid=%s window=120s\n' "$!" >> "$LOG"
fi
printf 'probe returned ONLINE; verify owner continuity in %s\n' "$OWNER_LOG"
