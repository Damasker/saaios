#!/bin/bash
set -e
LIB=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
OD=aarch64-linux-gnu-objdump
echo "=== log fmt strings (caller) ==="
for a in 0xc3df9 0xb4b59 0xcf97c 0xc8da8 0xc7da8 0xad e9b; do :; done
$OD -s -j .rodata --start-address=0xc3d80 --stop-address=0xc3e80 "$LIB"
echo "--- b4 b59 ---"
$OD -s -j .rodata --start-address=0xb4b00 --stop-address=0xb4bc0 "$LIB"
echo "=== translateNetworktype table @0xd8b84 (54 x int32) ==="
$OD -s -j .rodata --start-address=0xd8b84 --stop-address=0xd8c60 "$LIB"
echo "=== preferred-net table @0xd8c5c ==="
$OD -s -j .rodata --start-address=0xd8c5c --stop-address=0xd8d40 "$LIB"
