param(
  [int]$WaitSeconds = 50,
  [string]$Port = "COM13"
)
$Cmd = @'
set +e
OUT=/data/saaios/var/ping-pk-lean.log
: > $OUT
{
echo ===LEAN===
cat /sys/devices/platform/cpif/modem_state
( exec 3<>/dev/oem_ipc0 && echo OEM_RDWR_OK )
TR=/sys/kernel/debug/tracing
mount -t debugfs none /sys/kernel/debug 2>/dev/null
ls $TR/available_tracers | head -1
echo 0 > $TR/tracing_on
echo > $TR/trace
echo > $TR/kprobe_events
# userspace write path: vfs_write(file,buf,count) -> x1=buf x2=count
echo 'p:saaios_vfs_w vfs_write count=%x2:u64 b0=+0(%x1):u8 b1=+1(%x1):u8 b2=+2(%x1):u8 b3=+3(%x1):u8 b4=+4(%x1):u8 b5=+5(%x1):u8 b6=+6(%x1):u8 b7=+7(%x1):u8 b8=+8(%x1):u8 b9=+9(%x1):u8 b10=+10(%x1):u8 b11=+11(%x1):u8 b12=+12(%x1):u8 b13=+13(%x1):u8 b14=+14(%x1):u8 b15=+15(%x1):u8 b16=+16(%x1):u8 b17=+17(%x1):u8 b18=+18(%x1):u8 b19=+19(%x1):u8 b20=+20(%x1):u8 b21=+21(%x1):u8 b22=+22(%x1):u8 b23=+23(%x1):u8' > $TR/kprobe_events
echo KPROBE_RC:$?
cat $TR/kprobe_events
echo 1 > $TR/events/kprobes/saaios_vfs_w/enable
echo 1 > $TR/tracing_on
echo ===PING===
/data/saaios/bin/tmp-sitoem-ping-once; echo PING_EXIT:$?
sleep 1
echo 0 > $TR/tracing_on
echo 0 > $TR/events/kprobes/saaios_vfs_w/enable
echo ===TRACE===
cat $TR/trace
echo > $TR/kprobe_events
echo ===SYMS===
grep -E ' sipc5_|exynos_sipc|create_link_header|build_header|iod_ipc_write|ipc_tx|mif_irq' /proc/kallsyms | head -30
echo ===CPIFDBG===
ls /sys/kernel/debug/cpif 2>/dev/null | head
ls /sys/kernel/debug/modem* 2>/dev/null | head
echo ===DONE===
} 2>&1 | tee $OUT
'@

& 'C:\Users\Admin\Projects\saaios\os\targets\panther\tools\com13.ps1' -Port $Port -WaitSeconds $WaitSeconds -Cmd $Cmd
