#!/bin/bash
set -e
cd /mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw
IMG=factory-cp2a.260705.006-modem.img
OUT=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics
echo "=== region around 2nd cluster 64744778 (base 64744600, 600 bytes) ==="
dd if="$IMG" of="$OUT/tmp-nv2.bin" bs=1 skip=64744600 count=600 2>/dev/null
strings -t d "$OUT/tmp-nv2.bin"
echo
echo "=== hex of 2nd cluster region (first 256B) ==="
xxd -s 0 -l 256 "$OUT/tmp-nv2.bin"
echo
echo "=== search operation-mode enum strings ==="
LC_ALL=C grep -a -o -bE 'OPER_MODE_[A-Z0-9_]+|UE_OP_MODE_[A-Z0-9_]+|OPERATION_MODE_[A-Z0-9_]+|PS_MODE_[0-9]|CS_PS_MODE[0-9]' "$IMG" | head -30
