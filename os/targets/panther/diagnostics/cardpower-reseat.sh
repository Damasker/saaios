#!/bin/sh
# ONE remote tray-analog: CardPower DOWN(4)→UP(1) via proven 0x024c helper.
# Does not VerifyPin (leave that to tray-bearer-chase VerifyPin-on-EDGE).
# No POWER_OFF / crash / EFS / cbd / rild. Stop if remain<=1 (helper enforces).
set -eu
BIN=${BIN:-/tmp/cardpower-reseat}
ALT=/data/saaios/bin/cardpower-reseat
LOG=${LOG:-/data/saaios/var/cardpower-reseat.log}
mkdir -p "$(dirname "$LOG")"
if [ ! -x "$BIN" ] && [ -x "$ALT" ]; then
  BIN=$ALT
fi
if [ ! -x "$BIN" ]; then
  echo "missing $BIN (build cardpower-reseat.c; opcode 0x024c already proven)"
  exit 1
fi
STATE=$(cat /sys/devices/platform/cpif/modem_state 2>/dev/null || echo DEAD)
echo "STATE=$STATE" | tee -a "$LOG"
[ "$STATE" = ONLINE ] || exit 1
# Free sit lock if a dead holder left it; do not kill a live tray-watch here —
# caller should pause chase / wait between rounds.
rm -f /run/saaios-sit-status.lock
{
  printf '=== cardpower-reseat %s ===\n' "$(date -Iseconds 2>/dev/null || date)"
  "$BIN"
} 2>&1 | tee -a "$LOG"
