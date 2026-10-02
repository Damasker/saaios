#!/bin/sh
# nv-revert.sh BACKUP TARGET  -- restore a backup onto TARGET (DESTRUCTIVE).
# GUARDED: refuses to run unless SAAIOS_NV_REVERT_CONFIRM=yes is exported AND
# the backup's recorded sha256 matches. Intended ONLY for post-authorization
# recovery of a scoped NV edit. Performs NO action without the confirm env var.
set -eu
BACKUP="${1:?usage: nv-revert.sh BACKUP TARGET}"
TARGET="${2:?usage: nv-revert.sh BACKUP TARGET}"
if [ "${SAAIOS_NV_REVERT_CONFIRM:-no}" != "yes" ]; then
  echo "NV_REVERT refused: set SAAIOS_NV_REVERT_CONFIRM=yes to proceed (no action taken)"
  exit 3
fi
[ -f "$BACKUP.sha256" ] || { echo "NV_REVERT refused: missing $BACKUP.sha256"; exit 4; }
WANT=$(cat "$BACKUP.sha256")
HAVE=$(sha256sum "$BACKUP" | cut -d' ' -f1)
[ "$WANT" = "$HAVE" ] || { echo "NV_REVERT refused: backup sha mismatch want=$WANT have=$HAVE"; exit 5; }
dd if="$BACKUP" of="$TARGET" bs=65536 conv=fsync 2>/dev/null
echo "NV_REVERT done target=$TARGET restored_from=$BACKUP sha256=$HAVE"
