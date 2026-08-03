#!/usr/bin/env bash
# Show A/B slot status.
set -euo pipefail

AB_ROOT="${SAAIOS_AB_ROOT:-/var/lib/saaios/ab}"

echo "AB_ROOT=$AB_ROOT"
if [[ ! -d "$AB_ROOT" ]]; then
  echo "layout not created (run layout.sh)"
  exit 0
fi

CURRENT="$(readlink "$AB_ROOT/current" 2>/dev/null || echo '?')"
echo "current -> $CURRENT"
for s in A B; do
  bin="missing"
  [[ -x "$AB_ROOT/$s/bin/saaios-runtime" ]] && bin="present"
  ok="no"
  [[ -f "$AB_ROOT/$s/BOOT_OK" ]] && ok="yes ($(cat "$AB_ROOT/$s/BOOT_OK"))"
  mark=""
  [[ "$s" == "$CURRENT" ]] && mark=" *"
  echo "  slot $s$mark: binary=$bin BOOT_OK=$ok"
done
