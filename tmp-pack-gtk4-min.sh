#!/bin/sh
set -eu
sys=/tmp/saaios-b2/os/targets/panther/artifacts/toolchain/alpine-gtk4-sysroot
out=/tmp/gtk4-probe
rm -rf "$out"
mkdir -p "$out/bin" "$out/lib"
cp -L "$sys/usr/bin/gtk4-demo" "$out/bin/gtk4-demo"

copy_needed() {
    bin=$1
    # BFS of DT_NEEDED
    printf '%s\n' "$bin" > /tmp/gtk4-need.list
    : > /tmp/gtk4-seen.list
    while [ -s /tmp/gtk4-need.list ]; do
        path=$(head -n1 /tmp/gtk4-need.list)
        tail -n +2 /tmp/gtk4-need.list > /tmp/gtk4-need.rest
        mv /tmp/gtk4-need.rest /tmp/gtk4-need.list
        case "$path" in
            */ld-musl*) continue ;;
        esac
        base=$(basename "$path")
        grep -qxF "$base" /tmp/gtk4-seen.list && continue
        printf '%s\n' "$base" >> /tmp/gtk4-seen.list
        if [ "$path" != "$out/bin/gtk4-demo" ]; then
            cp -L "$path" "$out/lib/$base"
        fi
        readelf -d "$path" 2>/dev/null | awk '/NEEDED/{gsub(/[\[\]]/,"",$NF); print $NF}' | while read -r n; do
            [ "$n" = "ld-linux-aarch64.so.1" ] && continue
            [ "$n" = "ld-musl-aarch64.so.1" ] && continue
            [ "$n" = "libc.musl-aarch64.so.1" ] && continue
            found=$(find "$sys/lib" "$sys/usr/lib" -maxdepth 2 -name "$n" | head -n1)
            if [ -n "$found" ]; then
                printf '%s\n' "$found" >> /tmp/gtk4-need.list
            fi
        done
    done
}

copy_needed "$out/bin/gtk4-demo"
# cairo/gdk often dlopen pixbuf loaders
if [ -d "$sys/usr/lib/gdk-pixbuf-2.0" ]; then
    mkdir -p "$out/lib/gdk-pixbuf-2.0"
    cp -a "$sys/usr/lib/gdk-pixbuf-2.0/." "$out/lib/gdk-pixbuf-2.0/"
fi
ls -lh "$out/bin/gtk4-demo"
du -sh "$out"
qemu-aarch64-static -L "$sys" "$out/bin/gtk4-demo" --version
tar -C /tmp -cf /tmp/gtk4-probe.tar gtk4-probe
ls -lh /tmp/gtk4-probe.tar
