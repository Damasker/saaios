#!/bin/sh
set -eu
pkg=/tmp/gtk4-probe
sys=/tmp/saaios-b2/os/targets/panther/artifacts/toolchain/alpine-gtk4-sysroot
# musl loader + any missing DT_NEEDED from nested dirs
cp -L "$sys/lib/ld-musl-aarch64.so.1" "$pkg/lib/" 2>/dev/null || cp -L "$sys/usr/lib/ld-musl-aarch64.so.1" "$pkg/lib/" || true
# girepository / gtk modules
find "$sys/usr/lib" -name 'libgtk-4.so*' -o -name 'libgdk-4.so*' | head
ls "$pkg/lib" | wc -l
# qemu ldd-ish
qemu-aarch64-static -L "$sys" "$pkg/bin/gtk4-demo" --version 2>&1 | head || true
readelf -d "$pkg/bin/gtk4-demo" | awk '/NEEDED/{print}' | head -40
# copy gtk-4.0 modules if any
if [ -d "$sys/usr/lib/gtk-4.0" ]; then
  cp -a "$sys/usr/lib/gtk-4.0" "$pkg/lib/"
fi
du -sh "$pkg"
