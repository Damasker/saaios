#!/bin/sh
set -eu

# Cross-compiles saai-displayd (real DRM/KMS + touch input, the
# `panther-hardware` cargo feature) for the Pixel 7's init_boot ramdisk.
#
# Uses nightly Rust's -Z build-std with panic=immediate-abort (ADR-013):
# init_boot is a fixed 8MB partition, and a plain stable-toolchain build
# -- with std's normal panic-unwinding and panic-message-formatting
# machinery included -- doesn't fit once saai-displayd is added alongside
# everything else already bundled there. Rebuilding std itself without
# that machinery, for this one binary only, saves the ~400KB needed.
#
# This does not affect anything else in the workspace: only this one
# build invocation uses nightly. Headless tests, CI's `test` job, and
# saaios-runtime/saaios-console all keep using the pinned stable 1.97.1
# toolchain exactly as before -- this script never touches the default
# toolchain or the shared [profile.pixel7] in the root Cargo.toml.
#
# Requires the same cross sysroot as the DRM/touch backend
# (build-cross-sysroot.sh, ADR-008) for libxkbcommon, which links via a
# bare #[link(...)] with no build.rs of its own (see the saai-displayd
# build.rs and its comment for why -- unrelated to this script, just the
# same sysroot dependency).
#
# Safe to re-run: rustup install/component add are no-ops once done.

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
zig=${ZIG:?set ZIG to the zig binary path}
sysroot=${SYSROOT:-"$script_dir/artifacts/toolchain/sysroot-aarch64-musl"}
nightly=${SAAI_DISPLAYD_NIGHTLY:-nightly-2025-06-01}

rustup toolchain install "$nightly" --profile minimal
rustup component add rust-src --toolchain "$nightly"

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$script_dir/tools/zig-aarch64-musl.sh"
export CC_aarch64_unknown_linux_musl="$script_dir/tools/zig-aarch64-musl.sh"
export RUSTFLAGS="-C link-self-contained=no"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR="$sysroot"
export PKG_CONFIG_PATH="$sysroot/usr/local/lib/pkgconfig"
export ZIG="$zig"

cd "$repo_root"
cargo "+$nightly" build -Z build-std=std,panic_abort \
    -Z build-std-features=panic_immediate_abort \
    --target aarch64-unknown-linux-musl --profile pixel7 \
    -p saai-displayd --features saai-displayd/panther-hardware

printf '%s\n' "$repo_root/target/aarch64-unknown-linux-musl/pixel7/saai-displayd"
