#!/bin/sh
# Live brief + SitOem Ping with post-kernel frame capture via kprobe/ftrace.
# No rild/cbd. No catalog 0x2f50 invent.
set -eu

OUT=/data/saaios/var/ping-postkernel-$(date +%Y%m%d-%H%M%S)
mkdir -p /data/saaios/var
exec >"$OUT.log" 2>&1
echo "OUT=$OUT"

echo "===STATE==="
uname -a
for p in /sys/class/cpif/modem_state /sys/devices/platform/cpif/modem_state; do
  if [ -r "$p" ]; then echo "$p=$(cat "$p")"; fi
done
ls -la /dev/oem_ipc0 /dev/umts_ipc0 2>&1 || true
if ( exec 3<>/dev/oem_ipc0 ); then echo OEM_RDWR_OK; else echo OEM_OPEN_FAIL:$?; fi
ps | grep -E '[r]ild|[c]bd' || echo NO_RILD_CBD

echo "===SIM==="
if [ -x /data/saaios/bin/sit-sim-status ]; then
  /data/saaios/bin/sit-sim-status query-sim-status 2>&1 | head -60
else
  echo NO_sit-sim-status
fi

echo "===RMNET==="
i=0
while [ "$i" -le 5 ]; do
  if [ -d "/sys/class/net/rmnet$i" ]; then
    rx=$(cat "/sys/class/net/rmnet$i/statistics/rx_bytes")
    tx=$(cat "/sys/class/net/rmnet$i/statistics/tx_bytes")
    ip=$(ip -4 -o addr show "rmnet$i" 2>/dev/null | awk '{print $4}')
    echo "rmnet$i rx=$rx tx=$tx ip=${ip:-none}"
  fi
  i=$((i + 1))
done

echo "===KALLSYMS_IPC==="
grep -E 'exynos_sipc|sipc5_|create_link_header|build_link|iod_ipc|ipc_write|memcpy_to_link|exynos_.*hdr' /proc/kallsyms 2>/dev/null | head -60 || true
grep -E 'cpif|modem_io|io_device' /proc/kallsyms 2>/dev/null | grep -iE 'write|send|tx|header|sipc' | head -40 || true

TR=/sys/kernel/debug/tracing
echo "===TRACE_CAPS==="
if [ -d "$TR" ]; then
  echo TRACE_OK
  ls "$TR" | head -20
  echo 0 >"$TR/tracing_on" 2>/dev/null || true
  echo >"$TR/trace" 2>/dev/null || true
  echo nop >"$TR/current_tracer" 2>/dev/null || true
  # clear old kprobes if any
  if [ -d "$TR/events/kprobes" ]; then
    echo >"$TR/kprobe_events" 2>/dev/null || true
  fi
else
  echo NO_TRACEFS
fi

# Prefer known CPIF symbols; fall back to vfs_write filtered by length later.
SYM=""
for cand in \
  exynos_sipc5_create_link_header \
  sipc5_build_header \
  create_exynos_sipc_header \
  exynos_ipc_create_header \
  mif_ipc_write \
  ipc_write \
  cpif_write \
  iod_write \
  modem_io_write
do
  if grep -q " $cand\$" /proc/kallsyms 2>/dev/null || grep -q " $cand " /proc/kallsyms 2>/dev/null; then
    SYM=$cand
    echo "FOUND_SYM=$SYM"
    break
  fi
done

# Broader symbol hunt for write path near sipc / link
if [ -z "$SYM" ]; then
  echo "===BROADER_SYMS==="
  grep -E 'sipc5|link_header|exynos_hdr|ipc_pkt|pktproc.*tx|shmem.*write' /proc/kallsyms 2>/dev/null | head -80 || true
fi

setup_kprobe() {
  # Dump first 32 bytes of buffer arg (x1/x2 vary by ABI; try common patterns).
  # AArch64: arg0=x0, arg1=x1, arg2=x2
  # Probe: print len + first 32B from buffer pointer.
  # Format: p:name symbol +0 args
  local sym=$1
  # Try buffer in x1, len in x2 (common write(fd,buf,len) style after iod lookup)
  echo "p:saaios_ipc_tx ${sym} len=%x2 buf=+0(%x1):x64 buf8=+8(%x1):x64 buf16=+16(%x1):x64 buf24=+24(%x1):x64" >"$TR/kprobe_events" 2>"$OUT.kprobe_err" || return 1
  echo 1 >"$TR/events/kprobes/saaios_ipc_tx/enable" 2>>"$OUT.kprobe_err" || return 1
  return 0
}

setup_kprobe_vfs() {
  # vfs_write: x0=file*, x1=buf, x2=count — filter later by count==11 (ping) or >=12
  echo "p:saaios_vfs_w vfs_write buf=+0(%x1):x64 buf8=+8(%x1):x64 buf16=+16(%x1):x64 count=%x2" >"$TR/kprobe_events" 2>"$OUT.kprobe_err" || return 1
  echo 1 >"$TR/events/kprobes/saaios_vfs_w/enable" 2>>"$OUT.kprobe_err" || return 1
  return 0
}

PROBE_MODE=none
if [ -d "$TR" ]; then
  if [ -n "$SYM" ] && setup_kprobe "$SYM"; then
    PROBE_MODE="kprobe:$SYM"
  elif setup_kprobe_vfs; then
    PROBE_MODE="kprobe:vfs_write"
  else
    echo "KPROBE_FAIL"
    cat "$OUT.kprobe_err" 2>/dev/null || true
    # try raw function tracer on candidate
    PROBE_MODE=none
  fi
fi
echo "PROBE_MODE=$PROBE_MODE"

if [ "$PROBE_MODE" != none ]; then
  echo >"$TR/trace"
  echo 1 >"$TR/tracing_on"
fi

echo "===PING==="
PING=/data/saaios/bin/tmp-sitoem-ping-once
if [ ! -x "$PING" ]; then
  PING=/tmp/tmp-sitoem-ping-once
fi
if [ -x "$PING" ]; then
  "$PING" 2>&1 || echo PING_EXIT:$?
else
  echo NO_PING_BIN
  # inline minimal write if busybox/hex available — use printf bytes
  if [ -c /dev/oem_ipc0 ]; then
    # 08 01 10 01 2a 05 0a 03 0a 01 78
    printf '\x08\x01\x10\x01\x2a\x05\x0a\x03\x0a\x01\x78' >/dev/oem_ipc0 && echo INLINE_PING_WRITE_OK || echo INLINE_PING_FAIL:$?
  fi
fi

# allow TX to settle
sleep 1

if [ "$PROBE_MODE" != none ]; then
  echo 0 >"$TR/tracing_on"
  echo "===TRACE==="
  cat "$TR/trace" | head -200
  # disable / clear
  echo 0 >"$TR/events/kprobes/enable" 2>/dev/null || true
  echo >"$TR/kprobe_events" 2>/dev/null || true
fi

# CPIF debug dumps if present
echo "===CPIF_DEBUG==="
find /sys/kernel/debug -maxdepth 3 -iname '*cpif*' -o -iname '*sipc*' -o -iname '*modem*' 2>/dev/null | head -40 || true
for d in /sys/kernel/debug/cpif /sys/kernel/debug/modem_if /sys/kernel/debug/sipc; do
  if [ -d "$d" ]; then
    echo "DIR $d"
    ls -la "$d" | head -40
  fi
done

echo "LOG=$OUT.log"
echo "===DONE==="
# also print log path to console via symlink
ln -sf "$OUT.log" /data/saaios/var/ping-postkernel-latest.log
cat "$OUT.log"
