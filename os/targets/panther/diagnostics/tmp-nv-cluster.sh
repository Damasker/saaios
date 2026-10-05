#!/bin/bash
set -e
cd /mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw
IMG=factory-cp2a.260705.006-modem.img
OUT=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics
dd if="$IMG" of="$OUT/tmp-nvname.bin" bs=1 skip=24644300 count=1200 2>/dev/null
echo "=== NV name cluster @0x177xxxx (base 24644300) ==="
strings -t d "$OUT/tmp-nvname.bin"
