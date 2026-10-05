param([int]$WaitSeconds = 20)
$com13 = 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1'
& $com13 -WaitSeconds $WaitSeconds -Cmd @'
echo TOOLS
ls -la /tmp/setup-data-call /tmp/get-data-call-list /data/saaios/bin/setup-data-call /data/saaios/bin/get-data-call-list 2>/dev/null || true
echo APN
test -r /data/saaios/etc/apn && echo APN_OK bytes=$(wc -c </data/saaios/etc/apn)
echo ALIVE
cat /data/saaios/var/tray-bearer.alive
echo DONE
'@
