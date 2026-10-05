#!/bin/sh
set -eu
SYS=/tmp/saaios-b2/os/targets/panther/artifacts/toolchain/alpine-gtk4-sysroot
ZIG=/home/mike/.local/zig-linux-x86_64-0.13.0/zig
SRC=/tmp/gtk414-entry.c
OUT=/tmp/gtk4-probe/bin/gtk414-entry
export PKG_CONFIG_SYSROOT_DIR="$SYS"
export PKG_CONFIG_LIBDIR="$SYS/usr/lib/pkgconfig:$SYS/usr/share/pkgconfig"
cflags=$(pkg-config --cflags gtk4)
# pkg-config --libs pulls host paths; link from the sysroot by hand.
$ZIG cc -target aarch64-linux-musl \
  --sysroot "$SYS" \
  $cflags \
  -o "$OUT" "$SRC" \
  -L"$SYS/usr/lib" -lgtk-4 -lgdk-4 -lpangocairo-1.0 -lpango-1.0 \
  -lharfbuzz -lgdk_pixbuf-2.0 -lcairo-gobject -lcairo -lgraphene-1.0 \
  -lgio-2.0 -lgobject-2.0 -lglib-2.0 \
  -Wl,-rpath,/lib
file "$OUT"
ls -l "$OUT"
