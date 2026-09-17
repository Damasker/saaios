#!/bin/sh
set -eu

# Builds the private-per-app session D-Bus daemon staged for saai-appd
# (ADR-097): a prebuilt Alpine aarch64 `dbus-daemon` + its two runtime
# libs + a stock `session.conf`, the same package source and trust model
# as build-kirigami-demo-package.sh/build-pcmanfm-qt-package.sh
# (ADR-021/095) -- fetched over pinned HTTPS, --allow-untrusted at build
# time only, not re-verified at runtime beyond what saai-appd's own
# supervisor does by only ever running it with a fixed argv it
# constructs itself (services/saai-appd/src/supervisor.rs's
# ensure_dbus_daemon).
#
# This is NOT an installable app package (no manifest.toml, does not go
# through saai-appd's store/sandbox) -- it is a system component
# deployed once to `/data/saaios/system/dbus` (SandboxPaths::dbus_socket
# and AppSupervisor::dbus_binary_dir's production default), spawned
# directly by the trusted root daemon itself, one fresh instance per
# app_id, never inside any app's own sandbox.
#
# See build-kirigami-demo-package.sh for the shared apk-tools-static
# trust-model and "|| true" chroot-trigger-failure notes -- identical
# here, not repeated.

if ! command -v patchelf >/dev/null 2>&1; then
    printf '%s\n' "patchelf is required (rewrites dbus-daemon's ELF interpreter -- see below), install it first" >&2
    exit 1
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
output_dir=${SESSION_DBUS_OUTPUT_DIR:-"$repo_root/dist/panther/system/dbus"}

case "$output_dir" in
    "$repo_root"/dist/panther/system/*) ;;
    *)
        printf '%s\n' "refusing output outside dist/panther/system: $output_dir" >&2
        exit 1
        ;;
esac

toolchain_dir="$script_dir/artifacts/toolchain"
apk_tools_version="2.14.4-r1"
apk_tools_url="https://dl-cdn.alpinelinux.org/alpine/v3.20/main/x86_64/apk-tools-static-$apk_tools_version.apk"
apk_tools_apk="$toolchain_dir/apk-tools-static-$apk_tools_version.apk"
apk_static_dir="$toolchain_dir/apk-static-extract"
alpine_sysroot="$toolchain_dir/alpine-session-dbus-sysroot"

mkdir -p "$toolchain_dir"

if [ ! -x "$apk_static_dir/sbin/apk.static" ]; then
    if [ ! -f "$apk_tools_apk" ]; then
        curl -fL -o "$apk_tools_apk" "$apk_tools_url"
    fi
    rm -rf "$apk_static_dir"
    mkdir -p "$apk_static_dir"
    tar -xzf "$apk_tools_apk" -C "$apk_static_dir"
fi
apk_static="$apk_static_dir/sbin/apk.static"

rm -rf "$alpine_sysroot"
mkdir -p "$alpine_sysroot"
"$apk_static" \
    -X https://dl-cdn.alpinelinux.org/alpine/v3.20/main \
    -X https://dl-cdn.alpinelinux.org/alpine/v3.20/community \
    --root "$alpine_sysroot" --arch aarch64 --allow-untrusted --initdb \
    add dbus || true

if [ ! -e "$alpine_sysroot/usr/bin/dbus-daemon" ]; then
    printf '%s\n' "apk add dbus did not produce usr/bin/dbus-daemon -- see output above" >&2
    exit 1
fi

rm -rf "$output_dir"
mkdir -p "$output_dir/bin" "$output_dir/lib" "$output_dir/share/dbus-1"

cp -L "$alpine_sysroot/usr/bin/dbus-daemon" "$output_dir/bin/dbus-daemon"
cp -L "$alpine_sysroot/usr/lib/libdbus-1.so.3" "$output_dir/lib/"
cp -L "$alpine_sysroot/usr/lib/libexpat.so.1" "$output_dir/lib/"
# Alpine's dbus-daemon is dynamically linked against musl's own loader at
# a hardcoded absolute path (`/lib/ld-musl-aarch64.so.1`, `readelf -l`'s
# PT_INTERP) -- unlike an app package (sandbox.rs bind-mounts its own
# `lib/` at `/lib` inside that one app's mount namespace), this daemon
# runs directly on the real host root, which this device's rootfs never
# gives a loader at all (every first-party binary here is static
# specifically because of that). `patchelf` rewrites PT_INTERP (and
# DT_RUNPATH) to this component's own, always-present, real path instead
# -- confirmed necessary and sufficient by hand before this was folded
# into the script (ADR-097's follow-up integration): without it, every
# launch failed with "No such file or directory" at exec(), not a dbus
# protocol error.
cp -L "$alpine_sysroot/lib/ld-musl-aarch64.so.1" "$output_dir/lib/"
cp "$alpine_sysroot/usr/share/dbus-1/session.conf" "$output_dir/share/dbus-1/session.conf"
mkdir -p "$output_dir/share/dbus-1/session.d"

if find "$output_dir" -type l | grep -q .; then
    printf '%s\n' "output contains symlinks -- dereference them" >&2
    find "$output_dir" -type l >&2
    exit 1
fi

chmod 0755 "$output_dir/bin/dbus-daemon"
patchelf --set-interpreter /data/saaios/system/dbus/lib/ld-musl-aarch64.so.1 \
    --set-rpath /data/saaios/system/dbus/lib \
    "$output_dir/bin/dbus-daemon"

printf '%s\n' "$output_dir"
du -sh "$output_dir"
