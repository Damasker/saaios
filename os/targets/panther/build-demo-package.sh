#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
zig=${ZIG:?set ZIG to the zig binary path}
package_dir=${DEMO_PACKAGE_DIR:-"$repo_root/dist/panther/packages/org.saaios.demo-surface"}

case "$package_dir" in
    "$repo_root"/dist/panther/packages/*) ;;
    *)
        printf '%s\n' "refusing package output outside dist/panther/packages: $package_dir" >&2
        exit 1
        ;;
esac

export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER="$script_dir/tools/zig-aarch64-musl.sh"
export CC_aarch64_unknown_linux_musl="$script_dir/tools/zig-aarch64-musl.sh"
export AR_aarch64_unknown_linux_musl="$script_dir/tools/zig-ar.sh"
export CRATE_CC_NO_DEFAULTS=1
export RUSTFLAGS="-C link-self-contained=no"
export ZIG="$zig"

cd "$repo_root"
cargo build --profile pixel7 --locked --target aarch64-unknown-linux-musl \
    -p saai-demo-surface

rm -rf "$package_dir"
mkdir -p "$package_dir/bin"
cp apps/demo-surface/manifest.toml "$package_dir/manifest.toml"
cp target/aarch64-unknown-linux-musl/pixel7/saai-demo-surface \
    "$package_dir/bin/saai-demo-surface"
chmod 0755 "$package_dir/bin/saai-demo-surface"

printf '%s\n' "$package_dir"
sha256sum "$package_dir/manifest.toml" "$package_dir/bin/saai-demo-surface"
