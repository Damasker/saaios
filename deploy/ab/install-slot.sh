#!/usr/bin/env bash
# Install a saaios-runtime binary into slot A or B (inactive preferred).
# Usage: ./deploy/ab/install-slot.sh A|B [/path/to/saaios-runtime]
set -euo pipefail

AB_ROOT="${SAAIOS_AB_ROOT:-/var/lib/saaios/ab}"
SLOT="${1:-}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BIN_SRC="${2:-$ROOT_DIR/target/release/saaios-runtime}"

if [[ "$SLOT" != "A" && "$SLOT" != "B" ]]; then
  echo "usage: $0 A|B [binary]" >&2
  exit 1
fi

if [[ ! -d "$AB_ROOT/$SLOT" ]]; then
  echo "layout missing; run deploy/ab/layout.sh first" >&2
  exit 1
fi

if [[ ! -f "$BIN_SRC" ]]; then
  echo "binary not found: $BIN_SRC" >&2
  echo "Build first: cargo build -p saaios-runtime --release" >&2
  exit 1
fi

install -d -m 0755 "$AB_ROOT/$SLOT/bin"
install -m 0755 "$BIN_SRC" "$AB_ROOT/$SLOT/bin/saaios-runtime"
# New install clears previous BOOT_OK for that slot.
rm -f "$AB_ROOT/$SLOT/BOOT_OK"

echo "Installed $BIN_SRC -> $AB_ROOT/$SLOT/bin/saaios-runtime"
echo "BOOT_OK cleared for slot $SLOT (will be set after healthy boot)."
