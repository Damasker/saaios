#!/bin/bash
L=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
SY=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/extracted-syms.txt
OD=aarch64-linux-gnu-objdump
echo "########## search for missing builders ##########"
grep -iE 'Normal|IntPs|NetworkStart|BuildNs|StartNetwork|OnlineMode|OperMode|OperationMode|SetPsService|NetworkMode|SvcDomain|ServiceDomain|SetDual|InitAttach|SetAttach' "$SY" | grep -iE 'Build|Fill|_ZN' | sort -u
echo
echo "########## disasm target builders (id+len+body) ##########"
dis() { echo "===== $1 @$2 ====="; s=$(( $2 )); e=$(( $2 + ${3:-160} )); $OD -d --start-address=$s --stop-address=$e "$L" 2>/dev/null | grep -E ':\s+[0-9a-f]{8}\s' | grep -iE 'mov\s+w[0-9]|movk|strb|str |strh|sturb|stur|bl |add\s+x1'; }
dis BuildSetDeviceService 0x238a30 120
dis BuildGetDeviceService 0x238ab0 120
dis BuildSetVoiceOperation 0x22c030 140
dis BuildGetVoiceOperation 0x22c0c0 120
dis BuildGetPsService 0x236d00 120
dis BuildSetDualNetworkAndAllowData 0x2375a0 180
dis BuildSetDSNetworkType 0x236d80 290
