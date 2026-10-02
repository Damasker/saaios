#!/bin/bash
set -e
LIB=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
OD=aarch64-linux-gnu-objdump
echo "=== BuildSetDualNetworkAndAllowData @0x2375a0 ==="
$OD -d --start-address=0x2375a0 --stop-address=0x237720 "$LIB" | sed -n '/>:/,$p'
