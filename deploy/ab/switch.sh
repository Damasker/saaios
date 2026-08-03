#!/usr/bin/env bash
# Point current -> A or B and restart the runtime unit (if present).
# Refuses to leave the previous slot without BOOT_OK unless --force.
# Usage: ./deploy/ab/switch.sh A|B [--force]
set -euo pipefail

AB_ROOT="${SAAIOS_AB_ROOT:-/var/lib/saaios/ab}"
SLOT="${1:-}"
FORCE="${2:-}"

if [[ "$SLOT" != "A" && "$SLOT" != "B" ]]; then
  echo "usage: $0 A|B [--force]" >&2
  exit 1
fi

if [[ ! -x "$AB_ROOT/$SLOT/bin/saaios-runtime" ]]; then
  echo "slot $SLOT has no binary; run install-slot.sh first" >&2
  exit 1
fi

PREV="$(readlink "$AB_ROOT/current" 2>/dev/null || true)"
if [[ -n "$PREV" && "$PREV" != "$SLOT" && "$FORCE" != "--force" ]]; then
  if [[ ! -f "$AB_ROOT/$PREV/BOOT_OK" ]]; then
    echo "refusing switch: previous slot $PREV has no BOOT_OK (use --force to override)" >&2
    exit 1
  fi
fi

ln -sfn "$SLOT" "$AB_ROOT/current"
echo "$SLOT" >"$AB_ROOT/current_slot"
# Clear BOOT_OK on the new active slot until boot-ok.sh runs.
rm -f "$AB_ROOT/$SLOT/BOOT_OK"

echo "current -> $SLOT"

if command -v systemctl >/dev/null 2>&1 && systemctl list-unit-files saaios-runtime.service >/dev/null 2>&1; then
  systemctl restart saaios-runtime.service || true
  echo "restarted saaios-runtime.service"
  echo "After healthy start, run: ./deploy/ab/boot-ok.sh"
fi
