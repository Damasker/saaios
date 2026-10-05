echo ===SOFT_LOCK===
for f in /sys/devices/platform/*/modem_state /sys/devices/platform/*/*/modem_state /sys/class/*/modem_state; do
  [ -f "$f" ] && echo "$f=$(cat "$f")"
done
echo ===SIT===
/data/saaios/bin/sit-sim-status query-sim-status 2>&1 | head -40
echo ===RADIO===
/data/saaios/bin/sit-sim-status get-radio-state 2>&1 | head -10
/data/saaios/bin/get-ps-service 2>&1 | head -10
echo ===PS===
ps | grep -iE 'rild|cbd|ril-daemon' | grep -v grep | head -20
echo ===DEV===
ls -la /dev/umts* /dev/oem_ipc* 2>&1 | head -30
echo ===RMNET===
for i in /sys/class/net/rmnet*; do
  [ -d "$i" ] && echo "$i rx=$(cat "$i/statistics/rx_bytes") tx=$(cat "$i/statistics/tx_bytes")"
done
echo ===INJECT===
ls -la /data/saaios/bin/oem-ipc-inject /data/saaios/bin/post-init-chase.sh /data/saaios/bin/sit-sim-status /data/saaios/bin/tray-bearer-chase.sh 2>&1
echo ===DONE===
