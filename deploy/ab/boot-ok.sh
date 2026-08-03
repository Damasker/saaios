#!/usr/bin/env bash
# Mark the active A/B slot as BOOT_OK after a healthy start.
# Called manually or by saaios-boot-ok.service oneshot.
set -euo pipefail

AB_ROOT="${SAAIOS_AB_ROOT:-/var/lib/saaios/ab}"
SOCK="${SAAIOS_SOCK:-/run/saaios/saaios.sock}"
TIMEOUT_SECS="${SAAIOS_BOOT_OK_TIMEOUT:-30}"

SLOT="$(readlink "$AB_ROOT/current" 2>/dev/null || cat "$AB_ROOT/current_slot" 2>/dev/null || true)"
if [[ -z "$SLOT" ]]; then
  echo "no active slot under $AB_ROOT" >&2
  exit 1
fi

deadline=$((SECONDS + TIMEOUT_SECS))
while (( SECONDS < deadline )); do
  if [[ -S "$SOCK" ]]; then
    # Best-effort ping via socat/nc if available; socket presence is enough for v0.3.
    date -u +"%Y-%m-%dT%H:%M:%SZ" >"$AB_ROOT/$SLOT/BOOT_OK"
    echo "BOOT_OK written for slot $SLOT ($AB_ROOT/$SLOT/BOOT_OK)"
    exit 0
  fi
  sleep 1
done

echo "BOOT_OK failed: socket $SOCK not ready within ${TIMEOUT_SECS}s" >&2
exit 1
