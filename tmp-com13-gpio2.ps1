param([int]$WaitSeconds = 60)
$com13 = 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1'
& $com13 -WaitSeconds $WaitSeconds -Cmd @'
echo GPIO_MATCH
gpioinfo 2>/dev/null | grep -iE 'sim|tray|det|usim|uicc|card|slot|cdm|uim|eid' | head -60 || echo NONE
echo GPIO_COUNT
gpioinfo 2>/dev/null | wc -l
echo DEBUG_MATCH
grep -iE 'sim|tray|det|usim|uicc|card|slot|uim' /sys/kernel/debug/gpio 2>/dev/null | head -40 || echo NONE
echo DONE
'@
