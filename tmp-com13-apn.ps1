param([int]$WaitSeconds = 30)
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot '..\saaios\os\targets\panther\tools\com13.ps1'
# Resolve absolute path to sibling saaios tree.
$com13 = 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1'
& $com13 -WaitSeconds $WaitSeconds -Cmd @'
mkdir -p /data/saaios/etc
if test -r /data/saaios/etc/apn; then echo APN_ALREADY=1; else printf internet >/data/saaios/etc/apn; chmod 644 /data/saaios/etc/apn; echo APN_SEEDED=1; fi
echo APN_BYTES; wc -c /data/saaios/etc/apn
echo BEARER
ls /sys/class/net | grep rmnet || true
cat /sys/class/net/rmnet0/statistics/rx_packets; echo RX0
cat /sys/class/net/rmnet0/statistics/tx_packets; echo TX0
ip -4 -o addr show | grep rmnet || echo NO_RMNET_IPV4
echo SIM
ls /sys/devices/platform/cpif/sim
cat /sys/devices/platform/cpif/sim/ds_detect; echo
echo GPIOCLASS
ls /sys/class/gpio || true
ls /dev/gpiochip* || echo NO_GPIOCHIP
echo DT_SIM
find /sys/firmware/devicetree/base -iname '*sim*' | head -40
echo DET
find /sys -iname '*sim*det*' 2>/dev/null | head -20
find /sys -iname '*tray*' 2>/dev/null | head -20
echo EDGE
rm -f /run/saaios-sit-status.lock
/data/saaios/bin/sit-sim-status query-sim-status | tr ' ' '\n' | grep -E '^(app_raw|pin1_state_raw|pin1_remain_raw|card_state_raw|apps)='
echo ALIVE2
cat /data/saaios/var/tray-bearer.alive
echo DONE
'@
