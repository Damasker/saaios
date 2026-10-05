#!/bin/sh
set -e
ZIG=/home/mike/.local/zig-linux-x86_64-0.13.0/zig
PKG=/tmp/saaios-b2/dist/panther/packages/org.saaios.demo.falkon
SRC=/tmp/saaios-b2/services/saai-displayd/tests/qt5_lineedit.cpp
DEV=/tmp/qt6-aarch64-dev
OUT=/tmp/qt6-lineedit-aarch64
test -f "$SRC"
test -f "$DEV/usr/include/qt6/QtWidgets/QLineEdit"
test -f "$PKG/lib/libQt6Widgets.so.6.6.3"
cd "$PKG/lib"
for n in Qt6Core Qt6Gui Qt6Widgets Qt6WaylandClient; do
  if [ -e "lib${n}.so.6.6.3" ] && [ ! -e "lib${n}.so" ]; then
    ln -sfn "lib${n}.so.6.6.3" "lib${n}.so"
  fi
done
"$ZIG" c++ -target aarch64-linux-musl -O1 -fPIC -std=c++17 \
  -I"$DEV/usr/include/qt6" \
  -I"$DEV/usr/include/qt6/QtWidgets" \
  -I"$DEV/usr/include/qt6/QtGui" \
  -I"$DEV/usr/include/qt6/QtCore" \
  -DQT_WIDGETS_LIB -DQT_GUI_LIB -DQT_CORE_LIB \
  "$SRC" -o "$OUT" \
  -L"$PKG/lib" -lQt6Widgets -lQt6Gui -lQt6Core
ls -l "$OUT"
strings "$OUT" | grep -E 'QT_LINEEDIT_COMPETE|local field' | head
