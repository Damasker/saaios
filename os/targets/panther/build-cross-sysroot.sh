#!/bin/sh
set -eu

# Builds a minimal aarch64-unknown-linux-musl sysroot containing the C
# libraries Smithay's backend_drm/backend_session_libseat/backend_udev/
# backend_libinput features need to cross-compile: libseat, libxkbcommon,
# libudev (via eudev, not systemd's udev -- see below), and libinput plus
# its own dependencies (mtdev, libevdev).
#
# Everything is built static: this target links fully statically
# (aarch64-unknown-linux-musl's default), so there is no point producing
# .so files, and two of these projects (seatd, libinput) hardcode
# shared_library() in their upstream meson.build regardless of
# --default-library -- patches/seatd-static-library.patch and
# patches/libinput-static-library.patch switch them to static_library().
#
# libudev comes from eudev (https://github.com/eudev-project/eudev), a
# systemd-independent fork -- not Debian's libudev1, which is part of
# systemd itself and not worth cross-compiling just for the small libudev
# API surface Smithay's backend_udev actually uses.
#
# Host prerequisites (not cross-compiled, used to run this script):
# meson, ninja, autoconf, automake, libtool, gperf, pkg-config, git, and
# Debian's deb-src entries enabled (apt-get source needs them).
#
# Safe to re-run: each library's source/build dir is wiped and refetched.

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
zig=${ZIG:?set ZIG to the zig binary path}
work_dir=${BUILD_DIR:-"$script_dir/artifacts/toolchain"}
sysroot=${SYSROOT:-"$work_dir/sysroot-aarch64-musl"}
cross_file="$work_dir/zig-aarch64-musl-meson.cross"
cc_wrapper="$script_dir/tools/zig-aarch64-musl-cc.sh"
ar_wrapper="$script_dir/tools/zig-ar.sh"

mkdir -p "$work_dir" "$sysroot/usr/local/include" "$sysroot/usr/local/lib/pkgconfig"
cd "$work_dir"

cat > "$cross_file" <<EOF
[binaries]
c = ['$zig', 'cc', '-target', 'aarch64-linux-musl']
ar = ['$zig', 'ar']
pkg-config = 'pkg-config'

[built-in options]
# One function/datum per ELF section, so the final Rust link's
# --gc-sections (already on by default) can discard individual unused
# functions instead of being forced to keep or drop whole .o files.
# Without this, libxkbcommon's keymap-compiler alone -- entirely unreachable
# once ADR-012 dropped keyboard capability from the panther-hardware
# build -- stayed linked in wholesale (~300KB+, confirmed via nm) because
# its many static helper functions share translation units with the
# handful of symbols smithay's seat/keyboard plumbing does reference
# unconditionally. See ADR-013.
c_args = ['-ffunction-sections', '-fdata-sections']

[host_machine]
system = 'linux'
cpu_family = 'aarch64'
cpu = 'aarch64'
endian = 'little'

[properties]
needs_exe_wrapper = true
EOF

meson_build() {
	# meson_build <src-dir-glob> [extra meson setup args...]
	src_glob=$1
	shift
	src=$(ls -d $src_glob)
	name=$(basename -- "$src")
	rm -rf "$work_dir/build-$name"
	meson setup "$work_dir/build-$name" "$src" \
		--cross-file "$cross_file" --default-library static -Dwerror=false "$@"
	DESTDIR="$sysroot" meson install -C "$work_dir/build-$name"
}

# --- libseat ---
rm -rf seatd-*
apt-get source libseat1
patch -p1 -d "$(ls -d seatd-*)" < "$script_dir/patches/seatd-static-library.patch"
meson_build 'seatd-*' \
	-Dlibseat-logind=disabled -Dlibseat-seatd=enabled -Dlibseat-builtin=disabled \
	-Dserver=disabled -Dexamples=disabled -Dman-pages=disabled

# --- libxkbcommon ---
rm -rf libxkbcommon-*
apt-get source libxkbcommon0
meson_build 'libxkbcommon-*' \
	-Denable-tools=false -Denable-x11=false -Denable-docs=false \
	-Denable-wayland=false -Denable-xkbregistry=false -Denable-bash-completion=false

# --- libudev (eudev) ---
rm -rf eudev
git clone --depth 1 --branch v3.2.14 https://github.com/eudev-project/eudev.git
(
	cd eudev
	./autogen.sh || true # man page generation fails without xsltproc; configure is already written by then
	env ZIG="$zig" CC="$cc_wrapper" AR="$ar_wrapper" RANLIB=true \
		./configure --host=aarch64-linux-musl --build=x86_64-linux-gnu --prefix=/usr/local \
		--disable-shared --enable-static --disable-programs --disable-blkid \
		--disable-selinux --disable-manpages --disable-kmod --disable-mtd_probe
	env ZIG="$zig" make -C src/shared
	env ZIG="$zig" make -C src/libudev
)
cp eudev/src/libudev/.libs/libudev.a "$sysroot/usr/local/lib/"
cp eudev/src/libudev/libudev.h "$sysroot/usr/local/include/"
cp eudev/src/libudev/libudev.pc "$sysroot/usr/local/lib/pkgconfig/"

# --- mtdev (libinput's touch protocol-A/B translation dependency) ---
rm -rf mtdev-*
apt-get source libmtdev-dev
(
	cd "$(ls -d mtdev-*)"
	env ZIG="$zig" CC="$cc_wrapper" AR="$ar_wrapper" RANLIB=true \
		./configure --host=aarch64-linux-musl --build=x86_64-linux-gnu --prefix=/usr/local \
		--disable-shared --enable-static
	env ZIG="$zig" make
)
mtdev_dir=$(ls -d mtdev-*)
cp "$mtdev_dir/src/.libs/libmtdev.a" "$sysroot/usr/local/lib/"
cp "$mtdev_dir"/include/*.h "$sysroot/usr/local/include/"
cp "$mtdev_dir/mtdev.pc" "$sysroot/usr/local/lib/pkgconfig/"

# --- libevdev ---
rm -rf libevdev-*
apt-get source libevdev-dev
meson_build 'libevdev-*' -Dtests=disabled -Dtools=disabled -Ddocumentation=disabled

# --- libinput ---
rm -rf libinput-*
apt-get source libinput10
patch -p1 -d "$(ls -d libinput-*)" < "$script_dir/patches/libinput-static-library.patch"
PKG_CONFIG_SYSROOT_DIR="$sysroot" \
PKG_CONFIG_PATH="$sysroot/usr/local/lib/pkgconfig" \
PKG_CONFIG_LIBDIR="$sysroot/usr/local/lib/pkgconfig" \
	meson_build 'libinput-*' \
	-Dlibwacom=false -Ddebug-gui=false -Dtests=false -Ddocumentation=false

echo "sysroot ready: $sysroot"
