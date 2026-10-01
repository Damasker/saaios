# One-shot COM13 live brief for modem soft-lock status.
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot '..\..\..\tmp-com13.ps1'
if (-not (Test-Path $script)) {
  $script = 'C:\Users\Admin\Projects\saaios-som\tmp-com13.ps1'
}
$cmd = @'
echo BRIEF; date; echo STATE:$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null); ls -la /dev/oem_ipc0 /dev/umts_ipc0 2>&1; (exec 3<>/dev/oem_ipc0 && echo OEM_RDWR_OK || echo OEM_OPEN_FAIL); if [ -x /data/saaios/bin/sit-sim-status ]; then /data/saaios/bin/sit-sim-status query-sim-status 2>&1 | head -40; else echo NO_sit; fi; ps | grep -E "[c]bd|[r]ild|[r]il-" || echo NO_cbd_rild; for n in /sys/class/net/rmnet*/statistics/rx_packets; do [ -f "$n" ] && echo RX $n=$(cat "$n"); done; ip -4 addr show 2>/dev/null | grep -E "rmnet|wlan|usb" | head -20
'@
& $script -Cmd $cmd -WaitSeconds 55 -Port COM13
