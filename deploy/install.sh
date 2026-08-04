#!/usr/bin/env bash
set -euo pipefail

# Minimal appliance install for Linux hosts (incl. Raspberry Pi OS).
# Usage: sudo ./deploy/install.sh [/path/to/saaios-runtime]
# Optional: SAAIOS_AB=1 to also seed A/B layout + BOOT_OK oneshot.
# If saaios-console sits next to the runtime binary, it is installed too.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SRC="${1:-$ROOT_DIR/target/release/saaios-runtime}"
BIN_DST="/usr/local/bin/saaios-runtime"
CONSOLE_SRC="$(dirname "$BIN_SRC")/saaios-console"
CONSOLE_DST="/usr/local/bin/saaios-console"
STAGE_SRC="$(dirname "$BIN_SRC")/saaios-stage"
STAGE_DST="/usr/local/bin/saaios-stage"
UNIT_SRC="$ROOT_DIR/deploy/systemd/saaios-runtime.service"
UNIT_DST="/etc/systemd/system/saaios-runtime.service"
STAGE_UNIT_SRC="$ROOT_DIR/deploy/systemd/saaios-stage.service"
STAGE_UNIT_DST="/etc/systemd/system/saaios-stage.service"
BOOT_OK_UNIT_SRC="$ROOT_DIR/deploy/systemd/saaios-boot-ok.service"
BOOT_OK_UNIT_DST="/etc/systemd/system/saaios-boot-ok.service"
LIB_DST="/usr/local/lib/saaios"

if [[ ! -f "$BIN_SRC" ]]; then
  echo "binary not found: $BIN_SRC" >&2
  echo "Build first: cargo build -p saaios-runtime -p console-tui -p console-web --release" >&2
  echo "Or cross:    ./deploy/cross-pi.sh && ./deploy/package.sh target/aarch64-unknown-linux-gnu/release" >&2
  exit 1
fi

id -u saaios >/dev/null 2>&1 || useradd --system --home /var/lib/saaios --shell /usr/sbin/nologin saaios
install -d -o saaios -g saaios -m 0755 /var/lib/saaios
install -m 0755 "$BIN_SRC" "$BIN_DST"
if [[ -x "$CONSOLE_SRC" ]]; then
  install -m 0755 "$CONSOLE_SRC" "$CONSOLE_DST"
  echo "installed console: $CONSOLE_DST"
else
  echo "note: saaios-console not found at $CONSOLE_SRC (skipped)"
fi
if [[ -x "$STAGE_SRC" ]]; then
  install -m 0755 "$STAGE_SRC" "$STAGE_DST"
  echo "installed stage:   $STAGE_DST"
else
  echo "note: saaios-stage not found at $STAGE_SRC (skipped)"
fi
install -m 0644 "$UNIT_SRC" "$UNIT_DST"
if [[ -f "$STAGE_UNIT_SRC" ]]; then
  install -m 0644 "$STAGE_UNIT_SRC" "$STAGE_UNIT_DST"
fi

install -d -m 0755 "$LIB_DST"
install -m 0755 "$ROOT_DIR/deploy/ab/"*.sh "$LIB_DST/"
install -m 0644 "$BOOT_OK_UNIT_SRC" "$BOOT_OK_UNIT_DST"

if [[ "${SAAIOS_AB:-0}" == "1" ]]; then
  SAAIOS_AB_ROOT=/var/lib/saaios/ab "$LIB_DST/layout.sh"
  SAAIOS_AB_ROOT=/var/lib/saaios/ab "$LIB_DST/install-slot.sh" A "$BIN_SRC"
  chown -R saaios:saaios /var/lib/saaios/ab
fi

install -d -m 0755 /etc/saaios
if [[ ! -f /etc/saaios/saaios.toml ]]; then
  install -m 0644 "$ROOT_DIR/deploy/saaios.toml" /etc/saaios/saaios.toml
fi

systemctl daemon-reload
systemctl enable saaios-runtime.service
systemctl enable saaios-boot-ok.service
systemctl restart saaios-runtime.service
systemctl start saaios-boot-ok.service || true
systemctl --no-pager --full status saaios-runtime.service || true

echo
echo "Installed."
echo "Config:  /etc/saaios/saaios.toml"
echo "Socket:  /run/saaios/saaios.sock"
echo "Audit:   /var/lib/saaios/audit.jsonl"
echo "Runtime: $BIN_DST"
if [[ -x "$CONSOLE_DST" ]]; then
  echo "TUI:     SAAIOS_SOCK=/run/saaios/saaios.sock saaios-console"
fi
if [[ -x "$STAGE_DST" ]]; then
  echo "Stage:   SAAIOS_SOCK=/run/saaios/saaios.sock saaios-stage  (http://127.0.0.1:7420)"
fi
echo "A/B:     SAAIOS_AB=1 ./deploy/install.sh  (status via TUI h or /usr/local/lib/saaios/status.sh)"
