#!/bin/sh
# One-shot live probe: APN seed + bearer + tray-detect sysfs scan.
# Never prints PIN/IMSI/ICCID/AID/cpsha/NV.
set -eu
mkdir -p /data/saaios/etc /data/saaios/var
echo LIVE
cat /sys/devices/platform/cpif/modem_state 2>/dev/null || echo DEAD
echo ALIVE
if [ -r /data/saaios/var/tray-bearer.alive ]; then
  cat /data/saaios/var/tray-bearer.alive
else
  echo NO_ALIVE
fi
date -Iseconds 2>/dev/null || date
echo PS
ps 2>/dev/null | grep -E 'tray-bearer-chase' | grep -v grep || echo NO_CHASE_PS
echo APN
if [ -r /data/saaios/etc/apn ]; then
  echo APN_PRESENT=1
  wc -c </data/saaios/etc/apn
else
  # Life (Belarus / life:) public default APN hostname only.
  printf '%s\n' internet >/data/saaios/etc/apn
  chmod 644 /data/saaios/etc/apn
  echo APN_SEEDED=1
  echo APN_PRESENT=1
  wc -c </data/saaios/etc/apn
fi
echo BEARER
for i in /sys/class/net/rmnet*; do
  [ -d "$i" ] || continue
  b=$(basename "$i")
  rx=$(cat "$i/statistics/rx_packets" 2>/dev/null || echo 0)
  tx=$(cat "$i/statistics/tx_packets" 2>/dev/null || echo 0)
  echo "$b rx=$rx tx=$tx"
done
ip -4 -o addr show 2>/dev/null | grep rmnet || echo NO_RMNET_IPV4
echo SIM_SYSFS
ls /sys/devices/platform/cpif/sim 2>/dev/null || echo NO_SIM_DIR
if [ -r /sys/devices/platform/cpif/sim/ds_detect ]; then
  echo -n 'ds_detect='
  cat /sys/devices/platform/cpif/sim/ds_detect
fi
echo GPIO_EXPORT
ls /sys/class/gpio 2>/dev/null | head -20 || true
ls /dev/gpiochip* 2>/dev/null || echo NO_GPIOCHIP
echo FIND_DET
find /sys/firmware/devicetree/base -iname '*sim*' 2>/dev/null | head -40 || true
find /sys -iname '*sim*det*' 2>/dev/null | head -20 || true
find /sys -iname '*tray*' 2>/dev/null | head -20 || true
find /sys/class -iname '*sim*' 2>/dev/null | head -20 || true
echo EDGE
rm -f /run/saaios-sit-status.lock
SIT=/data/saaios/bin/sit-sim-status
[ -x /tmp/sit-sim-status ] && SIT=/tmp/sit-sim-status
if [ -x "$SIT" ]; then
  "$SIT" query-sim-status 2>/dev/null | tr ' ' '\n' | grep -E '^(app_raw|pin1_state_raw|pin1_remain_raw|card_state_raw|apps)=' || true
else
  echo NO_SIT
fi
echo DONE
