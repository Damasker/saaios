param([int]$WaitSeconds = 40)
$com13 = 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1'
& $com13 -WaitSeconds $WaitSeconds -Cmd @'
echo ===WATCH_POLL===
echo STATE:$(cat /sys/devices/platform/cpif/modem_state)
echo UP:$(cut -d. -f1 /proc/uptime)
echo ALIVE
cat /data/saaios/var/tray-bearer.alive 2>/dev/null || echo NO_ALIVE
echo PS
ps 2>/dev/null | grep -E '[t]ray-bearer-chase|[t]ray-watch' || echo NO_PS
echo -n 'ds_detect='; cat /sys/devices/platform/cpif/sim/ds_detect; echo
echo LOG
grep -E 'EDGE|READY|ABSENT|PRESENT|DETECTED|batch=|PERSIST|SIM snapshot|TRANSITION|rmnet|SetupData|bearer' /data/saaios/var/tray-bearer.log 2>/dev/null | tail -n 20
echo RMNET
for n in 0 1 2 3; do
  [ -d /sys/class/net/rmnet$n ] || continue
  rx=$(cat /sys/class/net/rmnet$n/statistics/rx_bytes)
  tx=$(cat /sys/class/net/rmnet$n/statistics/tx_bytes)
  ipv4=$(ip -4 addr show rmnet$n 2>/dev/null | sed -n 's/.*inet \([^ ]*\).*/\1/p' | head -1)
  echo "rmnet$n rx=$rx tx=$tx ipv4=${ipv4:-none}"
done
echo ===POLL_DONE===
'@
