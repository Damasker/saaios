#!/bin/sh
set -eu

# Cross-compiles the persistent system shell for the Pixel 7 init_boot
# ramdisk. Keep the same size-focused build-std policy as saai-displayd:
# both binaries share the fixed 8MB partition budget (ADR-013/014).

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
zig=${ZIG:?set ZIG to the zig binary path}
nightly=${SAAI_SHELL_NIGHTLY:-nightly-2025-06-01}

rustup toolchain install "$nightly" --profile minimal
rustup component add rust-src --toolchain "$nightly"

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$script_dir/tools/zig-aarch64-musl.sh"
export CC_aarch64_unknown_linux_musl="$script_dir/tools/zig-aarch64-musl.sh"
export RUSTFLAGS="-C link-self-contained=no"
export ZIG="$zig"

cd "$repo_root"
cargo "+$nightly" build -Z build-std=std,panic_abort \
    -Z build-std-features=panic_immediate_abort \
    --target aarch64-unknown-linux-musl --profile pixel7 \
    -p saai-shell

printf '%s\n' "$repo_root/target/aarch64-unknown-linux-musl/pixel7/saai-shell"
