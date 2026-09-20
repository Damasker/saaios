#!/bin/sh
set -eu

# Builds org.saaios.demo.falkon (APP-06) -- Falkon on Qt6 WebEngine,
# same Alpine v3.20 aarch64 musl recipe as PCManFM-Qt (ADR-021/269).
# Does not install on panther. Does not flash displayd.

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
zig=${ZIG:?set ZIG to the zig binary path}
package_dir=${FALKON_PACKAGE_DIR:-"$repo_root/dist/panther/packages/org.saaios.demo.falkon"}

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
alpine_sysroot="$toolchain_dir/alpine-falkon-sysroot"

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
# `|| true`: apk exits non-zero here because this host can't chroot
# as non-root. Same as build-pcmanfm-qt-package.sh / ADR-021.
"$apk_static" \
    -X https://dl-cdn.alpinelinux.org/alpine/v3.20/main \
    -X https://dl-cdn.alpinelinux.org/alpine/v3.20/community \
    --root "$alpine_sysroot" --arch aarch64 --allow-untrusted --initdb \
    add falkon qt6-qtwayland qt6-qtwebengine || true

if [ ! -e "$alpine_sysroot/usr/bin/falkon" ]; then
    printf '%s\n' "apk add falkon did not produce usr/bin/falkon -- see output above" >&2
    exit 1
fi

webengine_process="$alpine_sysroot/usr/lib/qt6/libexec/QtWebEngineProcess"
if [ ! -e "$webengine_process" ]; then
    printf '%s\n' "qt6-qtwebengine did not produce QtWebEngineProcess" >&2
    exit 1
fi

rm -rf "$package_dir"
mkdir -p "$package_dir/bin" "$package_dir/lib" "$package_dir/libexec" \
    "$package_dir/plugins" "$package_dir/share/X11" \
    "$package_dir/share/qt6/resources" "$package_dir/share/qt6/translations"

cp -L "$alpine_sysroot/usr/bin/falkon" "$package_dir/bin/falkon"
cp -L "$webengine_process" "$package_dir/libexec/QtWebEngineProcess"
find "$alpine_sysroot/lib" "$alpine_sysroot/usr/lib" -maxdepth 1 -name '*.so*' \
    -exec cp -L {} "$package_dir/lib/" \;

if [ -d "$alpine_sysroot/usr/lib/qt6/plugins" ]; then
    cp -a "$alpine_sysroot/usr/lib/qt6/plugins/." "$package_dir/plugins/"
fi
if [ -d "$alpine_sysroot/usr/share/qt6/resources" ]; then
    cp -a "$alpine_sysroot/usr/share/qt6/resources/." "$package_dir/share/qt6/resources/"
fi
if [ -d "$alpine_sysroot/usr/share/qt6/translations" ]; then
    cp -a "$alpine_sysroot/usr/share/qt6/translations/." "$package_dir/share/qt6/translations/"
fi
if [ -d "$alpine_sysroot/usr/share/X11/xkb" ]; then
    cp -a "$alpine_sysroot/usr/share/X11/xkb" "$package_dir/share/X11/xkb"
fi

rm -f "$package_dir/plugins/platformthemes/libqgtk3.so" \
    "$package_dir/plugins/platformthemes/libqxdgdesktopportal.so"

if find "$package_dir/plugins" "$package_dir/share" -type l | grep -q .; then
    printf '%s\n' "package contains symlinks under plugins/share -- dereference them" >&2
    find "$package_dir/plugins" "$package_dir/share" -type l >&2
    exit 1
fi

# Alpine v3.20 qt6-qtwebengine uses system ICU (libicuuc.so.74), not
# bundled icudtl.dat. The packed Chromium bits are the .pak files and
# v8_context_snapshot.bin.
if [ ! -f "$package_dir/share/qt6/resources/qtwebengine_resources.pak" ]; then
    printf '%s\n' "missing qtwebengine_resources.pak -- WebEngine resources were not packed" >&2
    exit 1
fi
if [ ! -f "$package_dir/share/qt6/resources/v8_context_snapshot.bin" ]; then
    printf '%s\n' "missing v8_context_snapshot.bin -- WebEngine snapshot was not packed" >&2
    exit 1
fi
if [ ! -e "$package_dir/lib/libicuuc.so.74" ] && [ ! -e "$package_dir/lib/libicuuc.so.74.2" ]; then
    printf '%s\n' "missing libicuuc -- Alpine WebEngine needs system ICU" >&2
    exit 1
fi

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$script_dir/tools/zig-aarch64-musl.sh"
export CC_aarch64_unknown_linux_musl="$script_dir/tools/zig-aarch64-musl.sh"
export ZIG="$zig"
"$zig" cc -target aarch64-linux-musl -static -O2 \
    -o "$package_dir/bin/launch" "$repo_root/apps/falkon-demo/launch.c"

cp "$repo_root/apps/falkon-demo/manifest.toml" "$package_dir/manifest.toml"
chmod 0755 "$package_dir/bin/launch" "$package_dir/bin/falkon" \
    "$package_dir/libexec/QtWebEngineProcess"

printf '%s\n' "$package_dir"
du -sh "$package_dir"
ls -l "$package_dir/bin/falkon" "$package_dir/libexec/QtWebEngineProcess" \
    "$package_dir/share/qt6/resources/qtwebengine_resources.pak" \
    "$package_dir/share/qt6/resources/v8_context_snapshot.bin"
