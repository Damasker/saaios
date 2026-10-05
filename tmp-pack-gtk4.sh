#!/bin/sh
set -eu
# Minimal Alpine v3.20 aarch64 musl gtk4-demo pack for APP-02 panther.
script_dir=/tmp/saaios-b2/os/targets/panther
toolchain_dir="$script_dir/artifacts/toolchain"
apk_tools_version="2.14.4-r1"
apk_tools_apk="$toolchain_dir/apk-tools-static-$apk_tools_version.apk"
apk_static_dir="$toolchain_dir/apk-static-extract"
alpine_sysroot="$toolchain_dir/alpine-gtk4-sysroot"
package_dir=/tmp/gtk4-probe
apk_static="$apk_static_dir/sbin/apk.static"
test -x "$apk_static"

rm -rf "$alpine_sysroot" "$package_dir"
mkdir -p "$alpine_sysroot" "$package_dir/bin" "$package_dir/lib" "$package_dir/share"
"$apk_static" \
    -X https://dl-cdn.alpinelinux.org/alpine/v3.20/main \
    -X https://dl-cdn.alpinelinux.org/alpine/v3.20/community \
    --root "$alpine_sysroot" --arch aarch64 --allow-untrusted --initdb \
    add gtk4.0 gtk4.0-demo || true

ls -l "$alpine_sysroot/usr/bin/gtk4-demo" "$alpine_sysroot/usr/bin/gtk4-demo-application" 2>/dev/null || true
find "$alpine_sysroot/usr" -name 'gtk4-demo' | head
if [ ! -e "$alpine_sysroot/usr/bin/gtk4-demo" ]; then
    find "$alpine_sysroot" -name '*gtk4*' | head -40
    exit 1
fi
cp -L "$alpine_sysroot/usr/bin/gtk4-demo" "$package_dir/bin/gtk4-demo"
find "$alpine_sysroot/lib" "$alpine_sysroot/usr/lib" -maxdepth 1 -name '*.so*' \
    -exec cp -L {} "$package_dir/lib/" \;
# gdk pixbuf loaders and glib schemas if present
if [ -d "$alpine_sysroot/usr/lib/gdk-pixbuf-2.0" ]; then
    cp -a "$alpine_sysroot/usr/lib/gdk-pixbuf-2.0" "$package_dir/lib/"
fi
if [ -d "$alpine_sysroot/usr/share/glib-2.0/schemas" ]; then
    mkdir -p "$package_dir/share/glib-2.0"
    cp -a "$alpine_sysroot/usr/share/glib-2.0/schemas" "$package_dir/share/glib-2.0/"
fi
ls -lh "$package_dir/bin/gtk4-demo"
du -sh "$package_dir"
file "$package_dir/bin/gtk4-demo"
