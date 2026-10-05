#!/bin/bash
L=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
OD=aarch64-linux-gnu-objdump
dis() { # name start len
  echo "===== $1  @$2 len$3 ====="
  s=$(( $2 )); e=$(( $2 + $3 ))
  $OD -d --start-address=$s --stop-address=$e "$L" 2>/dev/null | grep -E ':\s+[0-9a-f]{8}\s'
}
# anchors
dis BuildRadioPower 0x236350 164
dis BuildAllowData 0x236c70 144
dis BuildSetNetworkSelectionAuto 0x236600 128
