#!/bin/sh
set -eu

# Builds the org.saaios.demo.pcmanfm package (APP-05) -- PCManFM-Qt, a
# real QtWidgets (not QtQuick/declarative) file manager, installable
# through saai-appd's ordinary lifecycle like any other package. Same
# recipe as build-kirigami-demo-package.sh: fetch Alpine's prebuilt
# aarch64 packages (ADR-021/ADR-095: musl, no from-source build, no
# glibc island), extract what this app needs.
#
# ADR-095's package survey named this candidate "2.4.0-r0" from a newer
# Alpine branch; the v3.20 branch this script pins (same branch the
# working Kirigami package already uses) actually carries pcmanfm-qt
# 1.4.1-r0, which is Qt5-based (libQt5Core/Gui/Widgets/X11Extras/DBus),
# not Qt6. This is a *new* toolkit shape versus APP-01's proof
# (QtWidgets + X11Extras + DBus, no QML/declarative engine at all) --
# worth verifying in its own right, not just a rebuild of APP-01.
#
# `qt5-qtbase-x11`'s own platformthemes plugin set ships
# `libqgtk3.so`, which Alpine's packaging makes a hard apk dependency on
# gtk+3.0 (and its own pango/cairo/atk/gdk-pixbuf/X11 closure) even
# though nothing in this app calls into GTK directly (confirmed: neither
# pcmanfm-qt nor libfm-qt's own `so:` dependency list names any GTK/
# cairo/pango library -- `apk info -R`). Qt only loads a platform theme
# plugin when QT_QPA_PLATFORMTHEME names it or desktop-session
# autodetection fires, neither of which happens here (saai-appd's
# spawn() never sets QT_QPA_PLATFORMTHEME, ADR-025's GTK4 crash context
# doesn't apply to passively-present GTK3 libraries that are never
# dlopen'd) -- but the plugin *file* is deleted below anyway so that
# code path can't be reached even by accident, rather than trusting that
# nothing ever sets that env var in the future.
#
# See build-kirigami-demo-package.sh for the shared apk-tools-static
# trust-model and "|| true" chroot-trigger-failure notes -- identical
# here, not repeated.

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
zig=${ZIG:?set ZIG to the zig binary path}
package_dir=${PCMANFM_PACKAGE_DIR:-"$repo_root/dist/panther/packages/org.saaios.demo.pcmanfm"}

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
alpine_sysroot="$toolchain_dir/alpine-pcmanfm-sysroot"

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
    add pcmanfm-qt qt5-qtwayland || true

if [ ! -e "$alpine_sysroot/usr/bin/pcmanfm-qt" ]; then
    printf '%s\n' "apk add pcmanfm-qt did not produce usr/bin/pcmanfm-qt -- see output above" >&2
    exit 1
fi

rm -rf "$package_dir"
mkdir -p "$package_dir/bin" "$package_dir/lib" "$package_dir/plugins" "$package_dir/share/X11"

cp -L "$alpine_sysroot/usr/bin/pcmanfm-qt" "$package_dir/bin/pcmanfm-qt"
find "$alpine_sysroot/lib" "$alpine_sysroot/usr/lib" -maxdepth 1 -name '*.so*' \
    -exec cp -L {} "$package_dir/lib/" \;
cp -a "$alpine_sysroot/usr/lib/qt5/plugins/." "$package_dir/plugins/"
cp -a "$alpine_sysroot/usr/share/X11/xkb" "$package_dir/share/X11/xkb"

# Remove the one plugin file that would ever dlopen GTK3 -- see the
# header comment above. Everything else under plugins/ is Qt5's own
# platform/imageformat/sqldriver machinery, already exercised by
# ADR-026's Kirigami package for the platforms/ subset.
rm -f "$package_dir/plugins/platformthemes/libqgtk3.so"
rm -f "$package_dir/plugins/platforminputcontexts/libibusplatforminputcontextplugin.so"
rm -f "$package_dir/plugins/platforms/libqwayland-egl.so" \
    "$package_dir/plugins/wayland-graphics-integration-client/libqt-plugin-wayland-egl.so" \
    "$package_dir/plugins/wayland-graphics-integration-client/libdrm-egl-server.so"

if find "$package_dir/plugins" "$package_dir/share" -type l | grep -q .; then
    printf '%s\n' "package contains symlinks under plugins/share -- dereference them" >&2
    find "$package_dir/plugins" "$package_dir/share" -type l >&2
    exit 1
fi

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$script_dir/tools/zig-aarch64-musl.sh"
export CC_aarch64_unknown_linux_musl="$script_dir/tools/zig-aarch64-musl.sh"
export ZIG="$zig"
"$zig" cc -target aarch64-linux-musl -static -O2 \
    -o "$package_dir/bin/launch" "$repo_root/apps/pcmanfm-demo/launch.c"

mkdir -p "$package_dir/etc/fonts"
cp "$repo_root/apps/pcmanfm-demo/fonts.conf" "$package_dir/etc/fonts/fonts.conf"
cp "$repo_root/apps/pcmanfm-demo/manifest.toml" "$package_dir/manifest.toml"
chmod 0755 "$package_dir/bin/launch" "$package_dir/bin/pcmanfm-qt"

printf '%s\n' "$package_dir"
du -sh "$package_dir"
