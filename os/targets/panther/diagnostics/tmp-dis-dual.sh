#!/bin/bash
L=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
OD=aarch64-linux-gnu-objdump
full() { echo "===== $1 @$2 len$3 ====="; s=$(( $2 )); e=$(( $2 + $3 )); $OD -d --start-address=$s --stop-address=$e "$L" 2>/dev/null | grep -E ':\s+[0-9a-f]{8}\s'; }
full BuildSetDualNetworkAndAllowData 0x2375a0 192
echo
full DoSetDualNetworkTypeAndAllowData 0x19fe00 360
echo "=== strings: dual network / allowdata / ps type ==="
strings -a "$L" | grep -iE 'dual.?network|allow.?data|ps.?type|SET_DUAL|NtwType|network.?type.*allow' | head -30
