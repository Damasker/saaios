#!/bin/bash
L=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
OD=aarch64-linux-gnu-objdump
full() { echo "===== $1 @$2 len$3 ====="; s=$(( $2 )); e=$(( $2 + $3 )); $OD -d --start-address=$s --stop-address=$e "$L" 2>/dev/null | grep -E ':\s+[0-9a-f]{8}\s'; }
full InitRequestHeader 0x292e90 120
full BuildSetDeviceService 0x238a30 120
full BuildSetVoiceOperation 0x22c030 140
full BuildSetDSNetworkType 0x236d80 300
full BuildNvWriteItem 0x22be30 8
full DoOemSetPsService 0x19dfd0 400
