echo BRIEF
date
# modem state
for d in /sys/devices/platform/*/modem_state /sys/devices/platform/*/*/modem_state /sys/class/cpif/*/modem_state; do
  [ -f "$d" ] && echo "STATE $d=$(cat "$d")"
done
ls -la /dev/oem_ipc0 /dev/umts_ipc0 2>&1
( exec 3<>/dev/oem_ipc0 && echo OEM_RDWR_OK || echo OEM_OPEN_FAIL:$? )
if [ -x /data/saaios/bin/sit-sim-status ]; then
  /data/saaios/bin/sit-sim-status query-sim-status 2>&1 | head -50
else
  echo NO_sit-sim-status
fi
ps | grep -E '[c]bd|[r]ild|[r]il-' || echo NO_cbd_rild
for n in /sys/class/net/rmnet*/statistics/rx_packets; do [ -f "$n" ] && echo "RX $n=$(cat $n)"; done
ip -4 addr show 2>/dev/null | grep -E 'rmnet|wlan|usb' | head -20
