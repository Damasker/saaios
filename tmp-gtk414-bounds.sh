#!/bin/sh
set -eu
export PATH=/home/mike/.cargo/bin:/usr/bin:/bin
export CARGO_TARGET_DIR=/tmp/saaios-vui04-target
SYS=/tmp/saaios-b2/os/targets/panther/artifacts/toolchain/alpine-gtk4-sysroot
PROBE=/tmp/gtk4-probe
RT=/tmp/gtk414-rt
rm -rf "$RT"
mkdir -p "$RT"
chmod 700 "$RT"

# Need the musl loader next to the packed libs for qemu -L probe.
if [ ! -e "$PROBE/lib/ld-musl-aarch64.so.1" ]; then
  cp -L "$SYS/lib/ld-musl-aarch64.so.1" "$PROBE/lib/ld-musl-aarch64.so.1"
fi
if [ ! -d "$PROBE/share/X11/xkb" ]; then
  mkdir -p "$PROBE/share/X11"
  cp -a "$SYS/usr/share/X11/xkb" "$PROBE/share/X11/xkb"
fi

cd /tmp/saaios-b2
DISPLAYD=/tmp/saaios-vui04-target/debug/saai-displayd
if [ ! -x "$DISPLAYD" ]; then
  cargo build -p saai-displayd
fi

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
echo "socket=$SOCK pid=$DPID"
test -n "$SOCK"

export WAYLAND_DISPLAY="$SOCK"
export GDK_BACKEND=wayland
export GSK_RENDERER=cairo
export GTK_A11Y=none
export NO_AT_BRIDGE=1
export XKB_CONFIG_ROOT="$PROBE/share/X11/xkb"
export GIO_USE_VFS=local
unset DISPLAY

# qemu -L wants the loader at /lib inside the prefix.
if [ ! -e "$PROBE/lib/ld-musl-aarch64.so.1" ]; then
  echo missing loader
  exit 1
fi
mkdir -p "$PROBE/lib64"
# Some loaders look at /lib
ln -sfn lib "$PROBE/lib" 2>/dev/null || true

set +e
timeout 12 qemu-aarch64-static -L "$PROBE" "$PROBE/bin/gtk4-demo" --run=dialog \
  >"$RT/gtk.out" 2>"$RT/gtk.err"
RC=$?
set -e
echo GTK_RC=$RC
echo '=== gtk stderr (head) ==='
head -80 "$RT/gtk.err" || true
echo '=== displayd log ==='
cat "$RT/displayd.log"
echo '=== displayd err ==='
cat "$RT/displayd.err" || true
