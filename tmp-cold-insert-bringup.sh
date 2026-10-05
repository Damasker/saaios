#!/bin/sh
# Post-sysrq: CPIF -> OFFLINE -> dense cold-INSERT probe hold (200ms 0x0200).
set -eu
LOG=/data/saaios/var/cold-insert-bringup.log
: > "$LOG"
log() { printf '%s\n' "$*" | tee -a "$LOG"; }
log "UP=$(cat /proc/uptime)"
insmod /lib/modules/shm_ipc.ko
insmod /lib/modules/cpif_page.ko
insmod /lib/modules/cpif.ko
insmod /lib/modules/cp_thermal_zone.ko
STATE=$(cat /sys/devices/platform/cpif/modem_state)
log "STATE=$STATE"
[ "$STATE" = OFFLINE ] || { log "need OFFLINE"; exit 1; }
HASH=$(sha256sum /lib/modules/cpif.ko | cut -c1-16)
log "cpif=$HASH"
ln -sf /data/saaios/bin/saaios-probe-b-modem.bin /tmp/saaios-probe-b-modem.bin
cp -a /data/saaios/bin/probe-handover /tmp/probe-handover
cp -a /data/saaios/bin/saaios-verify-nv-copies.sh /tmp/saaios-verify-nv-copies.sh
chmod +x /tmp/probe-handover /tmp/saaios-verify-nv-copies.sh
mkdir -p /mnt/vendor/persist /dev/block
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
mount -t ext4 -o ro,noload,nosuid,nodev,noexec /dev/block/sda1 /mnt/vendor/persist
ls -la /mnt/vendor/persist/modem/cpsha | tee -a "$LOG"
/tmp/probe-handover boot-b-with-verified-nv-handover /mnt/vendor/persist/modem/cpsha > /data/saaios/var/probe-cold.log 2>&1
PRC=$?
umount /mnt/vendor/persist || true
log "probe_rc=$PRC STATE=$(cat /sys/devices/platform/cpif/modem_state)"
grep -E 'app_state transition|HOLD RESULT|RadioPower|AllowData|rmnet_|COMPLETE|PROBE END' /data/saaios/var/probe-cold.log | tee -a "$LOG"
log "bringup done"
