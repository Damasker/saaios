param([int]$WaitSeconds = 45)
$com13 = 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1'
& $com13 -WaitSeconds $WaitSeconds -Cmd @'
echo ===LIVE===
echo STATE:$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null)
echo UP:$(cut -d. -f1 /proc/uptime)
echo ALIVE
cat /data/saaios/var/tray-bearer.alive 2>/dev/null || echo NO_ALIVE
echo PS
ps 2>/dev/null | grep -E '[t]ray-bearer-chase|[t]ray-watch' || echo NO_PS
echo LOCK
ls -l /run/saaios-tray-chase.lock /run/saaios-sit-status.lock 2>/dev/null || echo NO_LOCK
echo DS_DETECT
ls -la /sys/devices/platform/cpif/sim 2>/dev/null
ls -la /sys/devices/platform/cpif/sim/ds_detect 2>/dev/null
echo -n 'ds_detect_val='; cat /sys/devices/platform/cpif/sim/ds_detect 2>/dev/null; echo
# module param siblings if any
ls -la /sys/module/*/parameters/ds_detect 2>/dev/null || true
for p in /sys/module/*/parameters/ds_detect; do
  [ -e "$p" ] || continue
  echo -n "modparam $p="; cat "$p"; echo
  ls -la "$p"
done
echo SIT
SIT=/data/saaios/bin/sit-sim-status; [ -x "$SIT" ] || SIT=/tmp/sit-sim-status
$SIT query-sim-status 2>/dev/null | tr '\n' ' '; echo
echo RADIO
$SIT query-radio-state 2>/dev/null | tr '\n' ' '; echo
echo RMNET
for n in 0 1 2 3; do
  [ -d /sys/class/net/rmnet$n ] || continue
  rx=$(cat /sys/class/net/rmnet$n/statistics/rx_bytes)
  tx=$(cat /sys/class/net/rmnet$n/statistics/tx_bytes)
  ipv4=$(ip -4 addr show rmnet$n 2>/dev/null | sed -n 's/.*inet \([^ ]*\).*/\1/p' | head -1)
  echo "rmnet$n rx=$rx tx=$tx ipv4=${ipv4:-none}"
done
echo LOGTAIL
tail -n 12 /data/saaios/var/tray-bearer.log 2>/dev/null || echo NO_LOG
echo APN
[ -f /data/saaios/etc/apn ] && echo apn_bytes=$(wc -c </data/saaios/etc/apn) || echo NO_APN
echo TOOLS
ls -la /data/saaios/bin/setup-data-call /data/saaios/bin/get-data-call-list /data/saaios/bin/tray-bearer-chase.sh 2>/dev/null | awk '{print $1,$5,$9}'
echo ===DONE===
'@
