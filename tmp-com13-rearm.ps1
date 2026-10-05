param([int]$WaitSeconds = 25)
$com13 = 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1'
& $com13 -WaitSeconds $WaitSeconds -Cmd @'
echo REARM
# Restart chase so TOOLS resolve setup-data-call + APN (started before both existed).
kill $(ps | grep tray-bearer-chase | grep -v grep | awk '{print $1}') 2>/dev/null || true
sleep 1
rm -f /run/saaios-tray-chase.lock
nohup env PERSIST=1 WATCH_ROUNDS=12 OUT=/data/saaios/var/tray-bearer.log \
  sh /data/saaios/bin/tray-bearer-chase.sh \
  >/data/saaios/var/tray-bearer.nohup 2>&1 &
echo STARTED:$!
sleep 2
echo ALIVE
cat /data/saaios/var/tray-bearer.alive
echo PS
ps | grep tray-bearer | grep -v grep || echo NO_PS
echo LOG
tail -n 20 /data/saaios/var/tray-bearer.log
echo DONE
'@
