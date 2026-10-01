param(
  [int]$WaitSeconds = 40,
  [string]$Port = "COM13"
)
$Cmd = @'
set +e
TR=/sys/kernel/debug/tracing
echo 0 > $TR/tracing_on
echo > $TR/trace
echo > $TR/kprobe_events
echo 'p:eh0 exynos_build_header cfg=%x3:u16 ctl=%x4:u8 count=%x5:u64' > $TR/kprobe_events
echo 'p:eh1 exynos_build_header+0x50 b0=+0(%x2):u8 b1=+1(%x2):u8 b2=+2(%x2):u8 b3=+3(%x2):u8 b4=+4(%x2):u8 b5=+5(%x2):u8 b6=+6(%x2):u8 b7=+7(%x2):u8 b8=+8(%x2):u8 b9=+9(%x2):u8 b10=+10(%x2):u8 b11=+11(%x2):u8 b12=+12(%x2):u8 b13=+13(%x2):u8 b14=+14(%x2):u8 b15=+15(%x2):u8 b16=+16(%x2):u8 b17=+17(%x2):u8 b18=+18(%x2):u8 b19=+19(%x2):u8 b20=+20(%x2):u8 b21=+21(%x2):u8 b22=+22(%x2):u8 b23=+23(%x2):u8' >> $TR/kprobe_events
echo 1 > $TR/events/kprobes/enable
echo 1 > $TR/tracing_on
/data/saaios/bin/tmp-sitoem-ping-once >/tmp/ping2.out 2>&1
sleep 1
echo 0 > $TR/tracing_on
echo 0 > $TR/events/kprobes/enable
echo ===PING===
cat /tmp/ping2.out
echo ===TRACE===
cat $TR/trace
echo > $TR/kprobe_events
/data/saaios/bin/sit-sim-status query-sim-status 2>&1 | head -20
echo ===DONE===
'@
& 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1' -Port $Port -WaitSeconds $WaitSeconds -Cmd $Cmd
