param(
  [int]$WaitSeconds = 45,
  [string]$Port = "COM13"
)
# Probe post-kernel EXYNOS header builders around SitOem Ping.
# Legacy: exynos_build_header(iod, ld, buff, cfg, ctl, count)
# Newer: sipc5_build_header(...) — dump regs + buff bytes on entry and near-end.
$Cmd = @'
set +e
TR=/sys/kernel/debug/tracing
mount -t debugfs none /sys/kernel/debug 2>/dev/null
echo 0 > $TR/tracing_on
echo > $TR/trace
echo > $TR/kprobe_events
# entry: x0=iod x1=ld x2=buff x3=cfg x4=ctl x5=count (legacy layout)
echo 'p:eh0 exynos_build_header cfg=%x3:u16 ctl=%x4:u8 count=%x5:u64' >> $TR/kprobe_events
# near-end of ~0x60 func: dump first 24B of buff (x2 may still hold buff)
echo 'p:eh1 exynos_build_header+0x50 b0=+0(%x2):u8 b1=+1(%x2):u8 b2=+2(%x2):u8 b3=+3(%x2):u8 b4=+4(%x2):u8 b5=+5(%x2):u8 b6=+6(%x2):u8 b7=+7(%x2):u8 b8=+8(%x2):u8 b9=+9(%x2):u8 b10=+10(%x2):u8 b11=+11(%x2):u8 b12=+12(%x2):u8 b13=+13(%x2):u8 b14=+14(%x2):u8 b15=+15(%x2):u8 b16=+16(%x2):u8 b17=+17(%x2):u8 b18=+18(%x2):u8 b19=+19(%x2):u8 b20=+20(%x2):u8 b21=+21(%x2):u8 b22=+22(%x2):u8 b23=+23(%x2):u8' >> $TR/kprobe_events
# sipc5_build_header entry regs
echo 'p:sh0 sipc5_build_header x0=%x0 x1=%x1 x2=%x2 x3=%x3 x4=%x4 x5=%x5' >> $TR/kprobe_events
echo 'p:sh1 sipc5_build_header+0x40 b0=+0(%x0):u8 b1=+1(%x0):u8 b2=+2(%x0):u8 b3=+3(%x0):u8 b4=+4(%x0):u8 b5=+5(%x0):u8 b6=+6(%x0):u8 b7=+7(%x0):u8 b8=+8(%x0):u8 b9=+9(%x0):u8 b10=+10(%x0):u8 b11=+11(%x0):u8 b12=+12(%x0):u8 b13=+13(%x0):u8 b14=+14(%x0):u8 b15=+15(%x0):u8' >> $TR/kprobe_events
echo 'p:sh2 sipc5_build_header+0x40 b0=+0(%x1):u8 b1=+1(%x1):u8 b2=+2(%x1):u8 b3=+3(%x1):u8 b4=+4(%x1):u8 b5=+5(%x1):u8 b6=+6(%x1):u8 b7=+7(%x1):u8 b8=+8(%x1):u8 b9=+9(%x1):u8 b10=+10(%x1):u8 b11=+11(%x1):u8 b12=+12(%x1):u8 b13=+13(%x1):u8 b14=+14(%x1):u8 b15=+15(%x1):u8' >> $TR/kprobe_events
echo 'p:sh3 sipc5_build_header+0x40 b0=+0(%x2):u8 b1=+1(%x2):u8 b2=+2(%x2):u8 b3=+3(%x2):u8 b4=+4(%x2):u8 b5=+5(%x2):u8 b6=+6(%x2):u8 b7=+7(%x2):u8 b8=+8(%x2):u8 b9=+9(%x2):u8 b10=+10(%x2):u8 b11=+11(%x2):u8 b12=+12(%x2):u8 b13=+13(%x2):u8 b14=+14(%x2):u8 b15=+15(%x2):u8' >> $TR/kprobe_events
echo KP_SETUP:$?
cat $TR/kprobe_events
echo 1 > $TR/events/kprobes/enable
echo 1 > $TR/tracing_on
/data/saaios/bin/tmp-sitoem-ping-once; echo PING:$?
sleep 1
echo 0 > $TR/tracing_on
echo 0 > $TR/events/kprobes/enable
echo ===TRACE===
cat $TR/trace
echo > $TR/kprobe_events
echo ===DONE===
'@
& 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1' -Port $Port -WaitSeconds $WaitSeconds -Cmd $Cmd
