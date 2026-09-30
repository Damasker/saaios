#!/bin/sh
# Clean soft ONLINE then FIRST 0x0200 present_infer — no CardPower/EngMode/HotSwap/tray.
set -eu
LOG=/data/saaios/var/clean-present-snap.log
: > "$LOG"
log() { printf '%s\n' "$*" | tee -a "$LOG"; }
log "BEGIN UP=$(cut -d' ' -f1 /proc/uptime)"
pkill -f tray-bearer-chase 2>/dev/null || true
pkill -f 'sit-sim-status' 2>/dev/null || true
rm -f /run/saaios-sit-status.lock
insmod /lib/modules/shm_ipc.ko 2>/dev/null || true
insmod /lib/modules/cpif_page.ko 2>/dev/null || true
insmod /lib/modules/cpif.ko 2>/dev/null || true
insmod /lib/modules/cp_thermal_zone.ko 2>/dev/null || true
STATE=$(cat /sys/devices/platform/cpif/modem_state)
log "STATE=$STATE"
[ "$STATE" = OFFLINE ] || { log "need OFFLINE got=$STATE"; exit 1; }
ln -sf /data/saaios/bin/saaios-probe-b-modem.bin /tmp/saaios-probe-b-modem.bin
cp -a /data/saaios/bin/probe-handover-clean /tmp/probe-handover
cp -a /data/saaios/bin/saaios-verify-nv-copies.sh /tmp/saaios-verify-nv-copies.sh
chmod +x /tmp/probe-handover /tmp/saaios-verify-nv-copies.sh
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
if grep -E 'HOLD RESULT|SIM length=|query-sim|0x0200' "$LOG" >/dev/null 2>&1; then
  log "CONTAMINATED: handover sent SIM"
fi
log "ONLINE clean — FIRST 0x0200 (no CardPower/EngMode/HotSwap)"
/data/saaios/bin/sit-sim-status query-sim-status 2>&1 | tee -a "$LOG"
log "rmnet:"
for i in 0 1 2 3; do
  rx=$(cat /sys/class/net/rmnet$i/statistics/rx_bytes 2>/dev/null || echo -)
  tx=$(cat /sys/class/net/rmnet$i/statistics/tx_bytes 2>/dev/null || echo -)
  log " rmnet$i rx=$rx tx=$tx"
done
ip -4 -o addr show 2>/dev/null | grep rmnet | tee -a "$LOG" || log " no rmnet ipv4"
log "DONE"
