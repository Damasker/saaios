#!/bin/sh
set -e
ZIG=/home/mike/.local/zig-linux-x86_64-0.13.0/zig
PKG=/home/mike/worktrees/saaios-som/dist/panther/packages/org.saaios.demo.pcmanfm
SRC=/tmp/saaios-b2/services/saai-displayd/tests/qt5_lineedit.cpp
DEV=/tmp/qt5-aarch64-dev
OUT=/tmp/qt5-lineedit-aarch64
test -f "$SRC"
test -f "$DEV/usr/include/qt5/QtWidgets/QLineEdit"
test -f "$PKG/lib/libQt5Widgets.so.5.15.10"
cd "$PKG/lib"
ln -sfn libQt5Widgets.so.5.15.10 libQt5Widgets.so
ln -sfn libQt5Gui.so.5.15.10 libQt5Gui.so
ln -sfn libQt5Core.so.5.15.10 libQt5Core.so
"$ZIG" c++ -target aarch64-linux-musl -O1 -fPIC -std=c++17 \
  -I"$DEV/usr/include/qt5" \
  -I"$DEV/usr/include/qt5/QtWidgets" \
  -I"$DEV/usr/include/qt5/QtGui" \
  -I"$DEV/usr/include/qt5/QtCore" \
  -DQT_WIDGETS_LIB -DQT_GUI_LIB -DQT_CORE_LIB \
  "$SRC" -o "$OUT" \
  -L"$PKG/lib" -lQt5Widgets -lQt5Gui -lQt5Core
ls -l "$OUT"
strings "$OUT" | grep -E 'QT_LINEEDIT_COMPETE|local field' | head
