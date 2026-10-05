param([int]$WaitSeconds = 20)
$com13 = 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1'
& $com13 -WaitSeconds $WaitSeconds -Cmd 'echo CHK; cat /sys/devices/platform/cpif/modem_state; echo ALIVE; cat /data/saaios/var/tray-bearer.alive 2>/dev/null || echo NO_ALIVE; echo PS; ps | grep tray-bearer | grep -v grep || echo NO_PS; echo LOCK; ls -la /run/saaios-tray-chase.lock 2>/dev/null || echo NO_LOCK; echo LOG; tail -n 12 /data/saaios/var/tray-bearer.log; echo DONE'
