#!/bin/sh
# Clean handover (no 0x0200) then ASAP no-0x0200 NET probe.
set -eu
LOG=/data/saaios/var/no0200-live.log
: > "$LOG"
log() { printf '%s\n' "$*" | tee -a "$LOG"; }
log "UP=$(cut -d' ' -f1 /proc/uptime)"
insmod /lib/modules/shm_ipc.ko 2>/dev/null || true
insmod /lib/modules/cpif_page.ko 2>/dev/null || true
insmod /lib/modules/cpif.ko 2>/dev/null || true
insmod /lib/modules/cp_thermal_zone.ko 2>/dev/null || true
STATE=$(cat /sys/devices/platform/cpif/modem_state)
log "STATE=$STATE"
[ "$STATE" = OFFLINE ] || { log "need OFFLINE got=$STATE"; exit 1; }
ln -sf /data/saaios/bin/saaios-probe-b-modem.bin /tmp/saaios-probe-b-modem.bin
cp -a /tmp/probe-handover-clean /tmp/probe-handover
cp -a /data/saaios/bin/saaios-verify-nv-copies.sh /tmp/saaios-verify-nv-copies.sh
chmod +x /tmp/probe-handover /tmp/saaios-verify-nv-copies.sh /tmp/probe-no0200
mkdir -p /mnt/vendor/persist /dev/block /data/saaios/var
if [ ! -e /dev/block/sda1 ]; then
  set -- $(cat /sys/block/sda/sda1/dev | tr : ' ')
  mknod /dev/block/sda1 b "$1" "$2"
fi
for n in umts_boot0 umts_ipc0 umts_rfs0; do
  if [ ! -e /dev/$n ]; then
    set -- $(cat /sys/class/cpif/$n/dev | tr : ' ')
    mknod /dev/$n c "$1" "$2"
  fi
done
umount /mnt/vendor/persist 2>/dev/null || true
mount -t ext4 -o ro,noload,nosuid,nodev,noexec /dev/block/sda1 /mnt/vendor/persist
/tmp/probe-handover boot-b-with-verified-nv-handover /mnt/vendor/persist/modem/cpsha >> "$LOG" 2>&1
PRC=$?
umount /mnt/vendor/persist || true
log "probe_rc=$PRC STATE=$(cat /sys/devices/platform/cpif/modem_state)"
[ "$PRC" = 0 ] || exit "$PRC"
[ "$(cat /sys/devices/platform/cpif/modem_state)" = ONLINE ] || { log "not ONLINE"; exit 2; }
# Prove handover log has no SIM/HOLD pollution
if grep -E 'HOLD RESULT|SIM length=|query-sim|0x0200' "$LOG" >/dev/null 2>&1; then
  log "CONTAMINATED: handover sent SIM; abort"
  exit 3
fi
log "handover clean (no SIM in log)"
/tmp/probe-no0200 2>&1 | tee -a "$LOG"
log "DONE"
