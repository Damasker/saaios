#!/bin/sh
set -eu
SYS=/tmp/saaios-b2/os/targets/panther/artifacts/toolchain/alpine-gtk4-sysroot
ZIG=/home/mike/.local/zig-linux-x86_64-0.13.0/zig
SRC=/tmp/saaios-b2/services/saai-displayd/tests/gtk414_entry.c
OUT=/tmp/gtk4-probe/bin/gtk414-entry
test -f "$SRC"
test -d "$SYS"
export PKG_CONFIG_SYSROOT_DIR="$SYS"
export PKG_CONFIG_LIBDIR="$SYS/usr/lib/pkgconfig:$SYS/usr/share/pkgconfig"
cflags=$(pkg-config --cflags gtk4)
mkdir -p /tmp/gtk4-probe/bin
# GTK4 has no separate libgdk-4.
$ZIG cc -target aarch64-linux-musl \
  --sysroot "$SYS" \
  $cflags \
  -o "$OUT" "$SRC" \
  -L"$SYS/usr/lib" -lgtk-4 -lpangocairo-1.0 -lpango-1.0 \
  -lharfbuzz -lgdk_pixbuf-2.0 -lcairo-gobject -lcairo -lgraphene-1.0 \
  -lgio-2.0 -lgobject-2.0 -lglib-2.0 \
  -Wl,-rpath,/lib
file "$OUT"
ls -l "$OUT"
strings "$OUT" | grep -E 'GTK4_COMPETE|GTK_ENTRY_TEXT' | head
