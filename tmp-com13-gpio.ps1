param([int]$WaitSeconds = 40)
$com13 = 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1'
& $com13 -WaitSeconds $WaitSeconds -Cmd @'
echo GPIOINFO
which gpioinfo 2>/dev/null; gpioinfo 2>/dev/null | grep -iE 'sim|tray|detect|usim|uicc' | head -40 || echo NO_GPIOINFO_MATCH
echo DEBUGGPIO
ls /sys/kernel/debug/gpio 2>/dev/null || echo NO_DEBUG_GPIO
cat /sys/kernel/debug/gpio 2>/dev/null | grep -iE 'sim|tray|detect|usim|uicc' | head -40 || true
echo CHIPLIST
ls /dev/gpiochip*
echo PINCTRL
ls /sys/kernel/debug/pinctrl 2>/dev/null | head -20 || echo NO_PINCTRL_DBG
find /sys/firmware/devicetree/base -iname '*gpio*' 2>/dev/null | grep -iE 'sim|tray|detect|usim' | head -20 || echo NO_DT_GPIO_SIM
echo LABELS
for c in /dev/gpiochip*; do echo CHIP:$c; done
# RO peek only: busybox may lack gpioinfo; try sysfs gpiochip labels
ls /sys/bus/gpio/devices 2>/dev/null || true
ls /sys/class/gpiochip* 2>/dev/null || true
for d in /sys/bus/gpio/devices/*; do echo DIR:$d; cat $d/label 2>/dev/null; ls $d 2>/dev/null | head -10; done
echo APP
rm -f /run/saaios-sit-status.lock
/data/saaios/bin/sit-sim-status query-sim-status | tr ' ' '\n' | grep -E '^(app|card|pin1|remain|present)=' | head -20
echo CHASE_LOG
tail -n 8 /data/saaios/var/tray-bearer.log
echo DONE
'@
