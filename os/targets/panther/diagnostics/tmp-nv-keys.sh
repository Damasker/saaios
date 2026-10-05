#!/bin/bash
set -e
cd /mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw
IMG=factory-cp2a.260705.006-modem.img
BLOB=/mnt/c/Users/Admin/AppData/Local/Temp/candidate-quar.bin
echo "=== name keys in on-disk blob candidate.bin? ==="
for k in OPERATION_MODE SAEL3 GCFMODE PLMN_SEL SAE_FLASH UE_OP; do
  n=$(LC_ALL=C grep -a -c "$k" "$BLOB" || true)
  echo "  '$k' count=$n"
done
echo "--- any 4+ ascii near start of blob (first 2KB) ---"
LC_ALL=C grep -a -o -bE '[A-Za-z_]{4,}' <(dd if="$BLOB" bs=2048 count=1 2>/dev/null) | head -20
echo
echo "=== UE_OPERATION_MODE enum: strings around PS_MODE_2 (24620500..24621000) ==="
dd if="$IMG" bs=1 skip=24620400 count=900 2>/dev/null > /tmp/em.bin
strings -t d /tmp/em.bin
