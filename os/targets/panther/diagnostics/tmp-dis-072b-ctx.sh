#!/bin/bash
set -e
LIB=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
OD=aarch64-linux-gnu-objdump
echo "=== translateNetworktype @0x236790 ==="
$OD -d --start-address=0x236790 --stop-address=0x2368d0 "$LIB" | sed -n '/>:/,$p'
echo
echo "=== DoSetDualNetworkTypeAndAllowData @0x19fe00 ==="
$OD -d --start-address=0x19fe00 --stop-address=0x1a0180 "$LIB" | sed -n '/>:/,$p'
