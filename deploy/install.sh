#!/usr/bin/env bash
set -euo pipefail

# Minimal appliance install for Linux hosts (incl. Raspberry Pi OS).
# Usage: sudo ./deploy/install.sh [/path/to/saaios-runtime]
# Optional: SAAIOS_AB=1 to also seed A/B layout + BOOT_OK oneshot.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN_SRC="${1:-$ROOT_DIR/target/release/saaios-runtime}"
BIN_DST="/usr/local/bin/saaios-runtime"
UNIT_SRC="$ROOT_DIR/deploy/systemd/saaios-runtime.service"
UNIT_DST="/etc/systemd/system/saaios-runtime.service"
BOOT_OK_UNIT_SRC="$ROOT_DIR/deploy/systemd/saaios-boot-ok.service"
BOOT_OK_UNIT_DST="/etc/systemd/system/saaios-boot-ok.service"
LIB_DST="/usr/local/lib/saaios"

if [[ ! -f "$BIN_SRC" ]]; then
  echo "binary not found: $BIN_SRC" >&2
  echo "Build first: cargo build -p saaios-runtime --release" >&2
  exit 1
fi

id -u saaios >/dev/null 2>&1 || useradd --system --home /var/lib/saaios --shell /usr/sbin/nologin saaios
install -d -o saaios -g saaios -m 0755 /var/lib/saaios
install -m 0755 "$BIN_SRC" "$BIN_DST"
install -m 0644 "$UNIT_SRC" "$UNIT_DST"

install -d -m 0755 "$LIB_DST"
install -m 0755 "$ROOT_DIR/deploy/ab/boot-ok.sh" "$LIB_DST/boot-ok.sh"
install -m 0755 "$ROOT_DIR/deploy/ab/"*.sh "$LIB_DST/"
install -m 0644 "$BOOT_OK_UNIT_SRC" "$BOOT_OK_UNIT_DST"

if [[ "${SAAIOS_AB:-0}" == "1" ]]; then
  SAAIOS_AB_ROOT=/var/lib/saaios/ab "$LIB_DST/layout.sh"
  SAAIOS_AB_ROOT=/var/lib/saaios/ab "$LIB_DST/install-slot.sh" A "$BIN_SRC"
  chown -R saaios:saaios /var/lib/saaios/ab
fi

systemctl daemon-reload
systemctl enable saaios-runtime.service
systemctl enable saaios-boot-ok.service
systemctl restart saaios-runtime.service
systemctl start saaios-boot-ok.service || true
systemctl --no-pager --full status saaios-runtime.service || true

echo
echo "Installed."
echo "Socket: /run/saaios/saaios.sock"
echo "Audit:  /var/lib/saaios/audit.jsonl"
echo "TUI:    SAAIOS_SOCK=/run/saaios/saaios.sock saaios-console"
echo "A/B:    SAAIOS_AB=1 ./deploy/install.sh  (or /usr/local/lib/saaios/layout.sh)"
