#!/bin/sh
set -eu

# Cross-compile S12 Change 3's staged-download tool for the persistent
# SaaiOS system directory on /data, same as saai-taskd/saai-appd -- it
# stays outside the fixed-size init_boot ramdisk and isn't started by
# native-init.c, run manually until physically proven (ADR-030's
# "prove it before wiring boot-critical init").

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
zig=${ZIG:?set ZIG to the zig binary path}

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$script_dir/tools/zig-aarch64-musl.sh"
export CC_aarch64_unknown_linux_musl="$script_dir/tools/zig-aarch64-musl.sh"
export AR_aarch64_unknown_linux_musl="$script_dir/tools/zig-ar.sh"
export CRATE_CC_NO_DEFAULTS=1
export RUSTFLAGS="-C link-self-contained=no"
export ZIG="$zig"

cd "$repo_root"
cargo build --profile pixel7 --locked --target aarch64-unknown-linux-musl \
    -p saai-ota-stage

binary="$repo_root/target/aarch64-unknown-linux-musl/pixel7/saai-ota-stage"
printf '%s\n' "$binary"
sha256sum "$binary"
