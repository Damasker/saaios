#!/bin/sh
echo ===LIVE===
for p in /sys/devices/platform/cpif/modem/state /sys/class/cpif/*/state /proc/cpif/modem/state; do
  [ -f "$p" ] && echo "state_path=$p" && cat "$p" && break
done
uptime
for i in 0 1 2 3; do
  f=/sys/class/net/rmnet$i
  if [ -d "$f" ]; then
    echo "rmnet$i rx=$(cat $f/statistics/rx_packets) tx=$(cat $f/statistics/tx_packets)"
    ip -4 addr show rmnet$i 2>/dev/null | awk '/inet /{print " ipv4="$2}'
  fi
done
SIT=/data/saaios/bin/sit-sim-status
if [ -x "$SIT" ]; then
  "$SIT" query-sim-status 2>&1 | tr ' ' '\n' | grep -E '^(app_raw|pin1_state_raw|pin1_remain_raw|card_state_raw|apps|error)='
  "$SIT" query-radio-state 2>&1 | tr ' ' '\n' | grep -E '^(radio_state_raw|state|error)='
  "$SIT" query-data-registration 2>&1 | tr ' ' '\n' | grep -E '^(registration_raw|tech_raw|error)=' | head
fi
[ -f /data/saaios/var/tray-bearer.alive ] && echo "tray=$(cat /data/saaios/var/tray-bearer.alive)"
echo ===ENDLIVE===
