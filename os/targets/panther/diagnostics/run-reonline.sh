#!/bin/sh
set -eu
cp -f /data/saaios/bin/saaios-probe-b-modem.bin.STOCK /tmp/saaios-probe-b-modem.bin
cp -a /data/saaios/bin/probe-handover /tmp/probe-handover
cp -a /data/saaios/bin/saaios-verify-nv-copies.sh /tmp/saaios-verify-nv-copies.sh
chmod +x /tmp/probe-handover /tmp/saaios-verify-nv-copies.sh
for n in umts_boot0 umts_ipc0 umts_rfs0; do
  if [ ! -e /dev/$n ]; then
    set -- $(cat /sys/class/cpif/$n/dev | tr : ' ')
    mknod /dev/$n c "$1" "$2"
  fi
done
mkdir -p /mnt/vendor/persist /dev/block
if [ ! -e /dev/block/sda1 ]; then
  set -- $(cat /sys/block/sda/sda1/dev | tr : ' ')
  mknod /dev/block/sda1 b "$1" "$2"
fi
umount /mnt/vendor/persist 2>/dev/null || true
mount -t ext4 -o ro,noload,nosuid,nodev,noexec /dev/block/sda1 /mnt/vendor/persist
/tmp/probe-handover boot-b-with-verified-nv-handover /mnt/vendor/persist/modem/cpsha
RC=$?
umount /mnt/vendor/persist || true
echo DONE:$RC
exit $RC
