#!/bin/bash
L=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
SY=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/extracted-syms.txt
OD=aarch64-linux-gnu-objdump
echo "=== stack / intps builder symbols ==="
grep -iE 'StackStatus|IntPs|SetIntPs|SetStack|GetStack' "$SY" | sort -u
full() { echo "===== $1 @$2 len$3 ====="; s=$(( $2 )); e=$(( $2 + $3 )); $OD -d --start-address=$s --stop-address=$e "$L" 2>/dev/null | grep -E ':\s+[0-9a-f]{8}\s' | grep -iE 'mov\s+w[0-9]|movk|strb|str |strh|stur|bl .*Init'; }
