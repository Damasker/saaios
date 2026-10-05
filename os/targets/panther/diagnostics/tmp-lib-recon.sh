#!/bin/bash
L=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
echo "=== dynsym count ==="
readelf -sW "$L" 2>/dev/null | wc -l
echo "=== reloc sections ==="
readelf -SW "$L" 2>/dev/null | grep -iE 'rela|relr|\.dynsym|\.dynstr'
echo "=== symbols w/ PsService / ServiceDomain / NetworkStart / DeviceService / IntPs / VoiceOp / OperationMode ==="
readelf -sW "$L" 2>/dev/null | grep -iE 'PsService|ServiceDomain|NetworkStart|NetworkNormal|DeviceService|IntPs|VoiceOper|OperationMode|OpMode|ModemConfig|DualNtw|AttachApn|Builder' | head -80
