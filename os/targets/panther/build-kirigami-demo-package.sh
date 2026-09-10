#!/bin/sh
set -eu

# Builds the org.saaios.demo.kirigami package (S08 Change 7) -- a real
# Kirigami2/QtQuick application, installable through saai-appd's ordinary
# lifecycle (S05) like any other package, not a throwaway spike.
#
# Unlike build-demo-package.sh, this app's runtime is not built from this
# repo's own source: apps/kirigami-demo/{manifest.toml,app.qml,launch.c}
# are the only things this repo owns. Everything else -- qmlscene-qt5, the
# whole Qt5/Kirigami2 shared-library closure, its QML modules, its Wayland
# platform/shell-integration plugins, xkeyboard-config's data -- comes from
# Alpine Linux's prebuilt aarch64 `kirigami2` package (ADR-021's decision:
# Alpine musl packages over from-source rebuild). There is no "build Qt"
# step here, only "fetch the pinned Alpine package set and extract what
# this app needs" -- physically verified end to end on a real Pixel 7
# (ADR-026, ADR-027).
#
# Trust model: apk-tools-static and the Alpine package repository are
# fetched over pinned HTTPS URLs (a specific apk-tools-static version, a
# specific Alpine release branch -- v3.20, not `edge`) and not further
# signature-verified (--allow-untrusted). That is the same trust boundary
# this repo already accepts elsewhere for build-time dependencies (e.g.
# build-cross-sysroot.sh's `git clone --branch v3.2.14` of eudev over
# plain HTTPS, ADR-008) -- a build-time toolchain input, not something
# shipped to end users unverified. The resulting package still goes
# through saai-appd's own install()/sandbox at runtime like any other.
#
# Safe to re-run: apk-tools-static is cached under artifacts/toolchain/
# once fetched; the Alpine sysroot itself is rebuilt from scratch every
# run (apk's own state, never committed).

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
zig=${ZIG:?set ZIG to the zig binary path}
package_dir=${KIRIGAMI_PACKAGE_DIR:-"$repo_root/dist/panther/packages/org.saaios.demo.kirigami"}

case "$package_dir" in
    "$repo_root"/dist/panther/packages/*) ;;
    *)
        printf '%s\n' "refusing package output outside dist/panther/packages: $package_dir" >&2
        exit 1
        ;;
esac

toolchain_dir="$script_dir/artifacts/toolchain"
apk_tools_version="2.14.4-r1"
apk_tools_url="https://dl-cdn.alpinelinux.org/alpine/v3.20/main/x86_64/apk-tools-static-$apk_tools_version.apk"
apk_tools_apk="$toolchain_dir/apk-tools-static-$apk_tools_version.apk"
apk_static_dir="$toolchain_dir/apk-static-extract"
alpine_sysroot="$toolchain_dir/alpine-kirigami-sysroot"

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
# `|| true`: apk exits non-zero here purely because this host can't chroot
# as non-root -- "ERROR: N errors updating directory permissions" and each
# post-install trigger failing with "chroot: Operation not permitted".
# Confirmed non-fatal in ADR-021's original spike: every file still lands
# on disk correctly, only cosmetic chown/icon-cache/mime-db regeneration
# steps (irrelevant to this package -- we extract a handful of files
# below, not the whole sysroot) don't run. A real failure to fetch or
# extract a package prints its own distinct error and still leaves
# needed files missing, which the extraction step below would then fail
# loudly on.
"$apk_static" \
    -X https://dl-cdn.alpinelinux.org/alpine/v3.20/main \
    -X https://dl-cdn.alpinelinux.org/alpine/v3.20/community \
    --root "$alpine_sysroot" --arch aarch64 --allow-untrusted --initdb \
    add kirigami2 || true

if [ ! -e "$alpine_sysroot/usr/bin/qmlscene-qt5" ]; then
    printf '%s\n' "apk add kirigami2 did not produce qmlscene-qt5 -- see output above" >&2
    exit 1
fi

# ADR-020's store.rs rejects symlinks in installed packages as a security
# boundary -- Alpine's shared libraries are symlink chains (soname -> real
# file), so every copy below dereferences them (cp -L / cp -a on content
# that happens to already be symlink-free, checked explicitly at the end).
rm -rf "$package_dir"
mkdir -p "$package_dir/bin" "$package_dir/lib" "$package_dir/qml" \
    "$package_dir/plugins" "$package_dir/share/X11"

cp -L "$alpine_sysroot/usr/bin/qmlscene-qt5" "$package_dir/bin/qmlscene-qt5"
find "$alpine_sysroot/lib" "$alpine_sysroot/usr/lib" -maxdepth 1 -name '*.so*' \
    -exec cp -L {} "$package_dir/lib/" \;
cp -a "$alpine_sysroot/usr/lib/qt5/qml/." "$package_dir/qml/"
cp -a "$alpine_sysroot/usr/lib/qt5/plugins/." "$package_dir/plugins/"
cp -a "$alpine_sysroot/usr/share/X11/xkb" "$package_dir/share/X11/xkb"

# Any symlink that survived the cp -a calls above (qml/ and plugins/,
# unlike the flat lib/ copy, aren't guaranteed dereferenced) breaks
# store.rs's install() the same way -- fail loudly here, not there.
if find "$package_dir/qml" "$package_dir/plugins" "$package_dir/share" -type l | grep -q .; then
    printf '%s\n' "package contains symlinks under qml/plugins/share -- dereference them" >&2
    find "$package_dir/qml" "$package_dir/plugins" "$package_dir/share" -type l >&2
    exit 1
fi

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$script_dir/tools/zig-aarch64-musl.sh"
export CC_aarch64_unknown_linux_musl="$script_dir/tools/zig-aarch64-musl.sh"
export ZIG="$zig"
"$zig" cc -target aarch64-linux-musl -static -O2 \
    -o "$package_dir/bin/launch" "$repo_root/apps/kirigami-demo/launch.c"

cp "$repo_root/apps/kirigami-demo/manifest.toml" "$package_dir/manifest.toml"
cp "$repo_root/apps/kirigami-demo/app.qml" "$package_dir/app.qml"
chmod 0755 "$package_dir/bin/launch" "$package_dir/bin/qmlscene-qt5"

printf '%s\n' "$package_dir"
du -sh "$package_dir"
