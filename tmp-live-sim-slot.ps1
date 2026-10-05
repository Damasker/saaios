$cmd = @'
echo SLOT
cat /proc/cmdline 2>/dev/null | tr " " "\n" | grep -E "slot|androidboot" || true
echo MODEM_PARTS
for d in /sys/block/sda/sda*; do
  pn=$(grep PARTNAME "$d/uevent" 2>/dev/null | cut -d= -f2)
  case "$pn" in *modem*|modem*) echo "$d $pn $(cat $d/dev)";; esac
done
echo LIVE_SIM
/data/saaios/bin/sit-sim-status query-sim-status
echo REG
/data/saaios/bin/sit-sim-status query-data-registration 2>&1 | head -5
echo RADIO
/data/saaios/bin/sit-sim-status query-radio-state 2>&1 | head -3
'@
# Write cmd to a temp approach via com13 -Cmd single line
$one = ($cmd -replace "`r","" -replace "`n","; ")
& "C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1" -Port COM13 -WaitSeconds 40 -Cmd $one
