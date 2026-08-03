#!/usr/bin/env bash
# Build a portable appliance tarball (runtime + console + deploy scripts).
# Usage:
#   ./deploy/package.sh                         # from host target/release
#   ./deploy/package.sh target/aarch64-unknown-linux-gnu/release
#   ./deploy/package.sh --out /tmp/saaios.tgz
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_DIR="$ROOT_DIR/target/release"
OUT_TGZ=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --out)
      OUT_TGZ="${2:?}"
      shift 2
      ;;
    -*)
      echo "unknown flag: $1" >&2
      exit 2
      ;;
    *)
      BIN_DIR="$1"
      shift
      ;;
  esac
done

RUNTIME="$BIN_DIR/saaios-runtime"
CONSOLE="$BIN_DIR/saaios-console"
STAGE_BIN="$BIN_DIR/saaios-stage"
if [[ ! -x "$RUNTIME" ]]; then
  echo "missing $RUNTIME — build first (cargo build -p saaios-runtime --release)" >&2
  exit 1
fi

VERSION="$(grep -m1 '^version' "$ROOT_DIR/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
STAGE="$(mktemp -d)"
NAME="saaios-${VERSION}-${STAMP}"
DEST="$STAGE/$NAME"
mkdir -p "$DEST/bin" "$DEST/deploy/ab" "$DEST/deploy/systemd" "$DEST/docs"

install -m 0755 "$RUNTIME" "$DEST/bin/saaios-runtime"
if [[ -x "$CONSOLE" ]]; then
  install -m 0755 "$CONSOLE" "$DEST/bin/saaios-console"
else
  echo "warning: saaios-console not found next to runtime; packaging runtime only" >&2
fi
if [[ -x "$STAGE_BIN" ]]; then
  install -m 0755 "$STAGE_BIN" "$DEST/bin/saaios-stage"
else
  echo "warning: saaios-stage not found next to runtime (optional)" >&2
fi

install -m 0755 "$ROOT_DIR/deploy/install.sh" "$DEST/deploy/install.sh"
install -m 0644 "$ROOT_DIR/deploy/saaios.toml" "$DEST/deploy/saaios.toml"
install -m 0755 "$ROOT_DIR/deploy/ab/"*.sh "$DEST/deploy/ab/"
install -m 0644 "$ROOT_DIR/deploy/systemd/"*.service "$DEST/deploy/systemd/"
install -m 0644 "$ROOT_DIR/docs/appliance.md" "$DEST/docs/appliance.md"
install -m 0644 "$ROOT_DIR/README.md" "$DEST/README.md"

# Rewrite install.sh paths so the tarball is self-contained (ROOT_DIR = package root).
cat >"$DEST/install.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$ROOT_DIR/deploy/install.sh" "$ROOT_DIR/bin/saaios-runtime"
EOF
chmod 0755 "$DEST/install.sh"

if [[ -z "$OUT_TGZ" ]]; then
  OUT_TGZ="$ROOT_DIR/dist/${NAME}.tar.gz"
fi
mkdir -p "$(dirname "$OUT_TGZ")"
tar -C "$STAGE" -czf "$OUT_TGZ" "$NAME"
rm -rf "$STAGE"

echo "wrote $OUT_TGZ"
ls -lh "$OUT_TGZ"
