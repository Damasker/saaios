#!/bin/bash
L=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
OUT=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/extracted-syms.txt
readelf -sW "$L" 2>/dev/null | awk '{print $2,$4,$8}' > "$OUT"
echo "wrote $OUT lines=$(wc -l < $OUT)"
echo "=== candidate operational SET/GET builders ==="
grep -iE 'PsService|ServiceDomain|DeviceService|IntPs|VoiceOper|OperationMode|NetworkStart|NetworkNormal|DualNtw|DualNetwork|SetAttach|NormalStart|BuildSetPs|BuildGetPs|OpMode|SetDevice|SetService' "$OUT" | grep -iE 'Build|Fill' | sort -u
echo "=== ProtocolNetworkBuilder full ==="
grep -E 'ProtocolNetworkBuilder' "$OUT" | sort -u
echo "=== ProtocolPsBuilder full ==="
grep -E 'ProtocolPsBuilder' "$OUT" | sort -u
