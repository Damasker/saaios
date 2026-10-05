#!/bin/sh
set -eu
export PATH=/home/mike/.cargo/bin:/usr/bin:/bin
PKG=/home/mike/worktrees/saaios-som/dist/panther/packages/org.saaios.demo.pcmanfm
RT=/tmp/pcmanfm-v2-rt
rm -rf "$RT"
mkdir -p "$RT/home/.config/pcmanfm-qt/default"
chmod 700 "$RT"
cat >"$RT/home/.config/pcmanfm-qt/default/settings.conf" <<'EOF'
[FolderView]
ShowFilter=true
[Window]
PathBarButtons=false
ShowMenuBar=true
SidePaneVisible=true
AlwaysShowTabs=true
EOF
DISPLAYD=/tmp/saaios-vui04-target/debug/saai-displayd
export XDG_RUNTIME_DIR="$RT"
"$DISPLAYD" >"$RT/displayd.log" 2>"$RT/displayd.err" &
DPID=$!
trap 'kill $DPID 2>/dev/null || true; wait $DPID 2>/dev/null || true' EXIT
i=0
while [ $i -lt 50 ]; do
  grep -q 'listening on WAYLAND_DISPLAY=' "$RT/displayd.log" 2>/dev/null && break
  i=$((i+1)); sleep 0.1
done
SOCK=$(sed -n 's/^saai-displayd: listening on WAYLAND_DISPLAY=//p' "$RT/displayd.log" | head -1)
echo "socket=$SOCK"
unset DISPLAY QT_IM_MODULE QT_QPA_PLATFORMTHEME
set +e
dbus-run-session -- env \
  XDG_RUNTIME_DIR="$RT" \
  WAYLAND_DISPLAY="$SOCK" \
  HOME="$RT/home" \
  QT_PLUGIN_PATH="$PKG/plugins" \
  QT_QPA_PLATFORM=wayland \
  XKB_CONFIG_ROOT="$PKG/share/X11/xkb" \
  WAYLAND_DEBUG=1 \
  timeout 6 qemu-aarch64-static -L "$PKG" "$PKG/bin/pcmanfm-qt" \
  >"$RT/qt.out" 2>"$RT/qt.err"
echo QT_RC=$?
set -e
echo '=== displayd ime ==='
grep -E 'text-input|new xdg_toplevel|frame sha256' "$RT/displayd.log" || true
echo '=== client text_input ==='
grep -E 'text_input|zwp_text' "$RT/qt.err" | head -40 || true
echo '=== bind ==='
grep -E 'bind.*text|zwp_text_input' "$RT/qt.err" | head -20 || true
