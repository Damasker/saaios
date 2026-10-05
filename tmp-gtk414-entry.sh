#!/bin/sh
set -eu
export PATH=/home/mike/.cargo/bin:/usr/bin:/bin
export CARGO_TARGET_DIR=/tmp/saaios-vui04-target
PROBE=/tmp/gtk4-probe
RT=/tmp/gtk414-entry-rt
rm -rf "$RT"
mkdir -p "$RT"
chmod 700 "$RT"
DISPLAYD=/tmp/saaios-vui04-target/debug/saai-displayd
export XDG_RUNTIME_DIR="$RT"
"$DISPLAYD" >"$RT/displayd.log" 2>"$RT/displayd.err" &
DPID=$!
trap 'kill $DPID 2>/dev/null || true; wait $DPID 2>/dev/null || true' EXIT
i=0
while [ $i -lt 50 ]; do
  if grep -q 'listening on WAYLAND_DISPLAY=' "$RT/displayd.log" 2>/dev/null; then
    break
  fi
  i=$((i+1))
  sleep 0.1
done
SOCK=$(sed -n 's/^saai-displayd: listening on WAYLAND_DISPLAY=//p' "$RT/displayd.log" | head -1)
echo "socket=$SOCK"
export WAYLAND_DISPLAY="$SOCK"
export GDK_BACKEND=wayland
export GSK_RENDERER=cairo
export GTK_A11Y=none
export NO_AT_BRIDGE=1
export XKB_CONFIG_ROOT="$PROBE/share/X11/xkb"
export GIO_USE_VFS=local
unset DISPLAY
DEMO="${1:-entry}"
echo "demo=$DEMO"
set +e
timeout 8 qemu-aarch64-static -L "$PROBE" "$PROBE/bin/gtk4-demo" --run="$DEMO" \
  >"$RT/gtk.out" 2>"$RT/gtk.err"
echo GTK_RC=$?
set -e
echo '=== enable lines ==='
grep -E 'text-input-v[23] enable|new xdg_toplevel|frame sha256' "$RT/displayd.log" || true
echo '=== gtk err head ==='
head -40 "$RT/gtk.err" || true
