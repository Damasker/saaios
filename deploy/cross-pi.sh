#!/usr/bin/env bash
# Cross-compile SaaiOS binaries for Raspberry Pi (aarch64).
# Usage:
#   ./deploy/cross-pi.sh              # release aarch64
#   ./deploy/cross-pi.sh --debug
#
# Requires: rustup target aarch64-unknown-linux-gnu + a linker:
#   - apt: gcc-aarch64-linux-gnu
#   - or: cargo install cargo-zigbuild && zig  (set SAAIOS_CROSS=zig)
#   - or: cargo install cross               (set SAAIOS_CROSS=cross)
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PROFILE="release"
CARGO_ARGS=(build -p saaios-runtime -p console-tui --target aarch64-unknown-linux-gnu)
for arg in "$@"; do
  case "$arg" in
    --debug) PROFILE="dev"; ;;
    --release) PROFILE="release"; ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

if [[ "$PROFILE" == "release" ]]; then
  CARGO_ARGS+=(--release)
fi

rustup target add aarch64-unknown-linux-gnu >/dev/null

CROSS_MODE="${SAAIOS_CROSS:-auto}"

pick_linker() {
  if command -v aarch64-linux-gnu-gcc >/dev/null 2>&1; then
    echo gnu
    return
  fi
  if command -v zig >/dev/null 2>&1 && command -v cargo-zigbuild >/dev/null 2>&1; then
    echo zig
    return
  fi
  if command -v cross >/dev/null 2>&1; then
    echo cross
    return
  fi
  echo none
}

if [[ "$CROSS_MODE" == "auto" ]]; then
  CROSS_MODE="$(pick_linker)"
fi

echo "cross mode: $CROSS_MODE  profile: $PROFILE"

case "$CROSS_MODE" in
  gnu)
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
    cargo "${CARGO_ARGS[@]}"
    ;;
  zig)
    # cargo-zigbuild accepts the same package flags.
    cargo zigbuild "${CARGO_ARGS[@]}"
    ;;
  cross)
    cross "${CARGO_ARGS[@]}"
    ;;
  none)
    cat >&2 <<'EOF'
No aarch64 linker found.

Install one of:
  sudo apt-get install -y gcc-aarch64-linux-gnu
  # or
  cargo install cargo-zigbuild   # plus zig on PATH
  # or
  cargo install cross            # plus Docker

Then re-run: ./deploy/cross-pi.sh
EOF
    exit 1
    ;;
  *)
    echo "unknown SAAIOS_CROSS=$CROSS_MODE (use auto|gnu|zig|cross)" >&2
    exit 2
    ;;
esac

OUT="target/aarch64-unknown-linux-gnu"
if [[ "$PROFILE" == "release" ]]; then
  OUT="$OUT/release"
else
  OUT="$OUT/debug"
fi

echo
echo "Built:"
ls -lh "$OUT/saaios-runtime" "$OUT/saaios-console"
echo
echo "Package:  ./deploy/package.sh $OUT"
echo "Install:  sudo ./deploy/install.sh $OUT/saaios-runtime  (also installs console if adjacent)"
