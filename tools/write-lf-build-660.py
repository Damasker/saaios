from pathlib import Path

Path("/home/mike/saaios-pixel7-flash/build-saai-shell-660.sh").write_text(
    """#!/bin/sh
set -eu
export PATH=\"$HOME/.cargo/bin:$PATH\"
export ZIG=/home/mike/.local/zig-linux-x86_64-0.13.0/zig
export SYSROOT=/home/mike/saaios-build/src/os/targets/panther/artifacts/toolchain/sysroot-aarch64-musl
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=/home/mike/saaios-pixel7-flash/target
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=/home/mike/saaios-pixel7-flash/os/targets/panther/tools/zig-aarch64-musl.sh
export CC_aarch64_unknown_linux_musl=/home/mike/saaios-pixel7-flash/os/targets/panther/tools/zig-aarch64-musl.sh
export RUSTFLAGS=\"-C link-self-contained=no\"
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_SYSROOT_DIR=\"$SYSROOT\"
export PKG_CONFIG_PATH=\"$SYSROOT/usr/local/lib/pkgconfig\"
cd /mnt/c/Users/Admin/Projects/saaios-pixel7-main
echo starting cargo
cargo +nightly-2025-06-01 build -Z build-std=std,panic_abort \\
    --target aarch64-unknown-linux-musl --profile pixel7 \\
    -p saai-shell
BIN=\"$CARGO_TARGET_DIR/aarch64-unknown-linux-musl/pixel7/saai-shell\"
ls -l \"$BIN\"
sha256sum \"$BIN\"
""",
    encoding="utf-8",
    newline="\n",
)
