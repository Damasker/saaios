#!/bin/bash
L=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libsitril.so
L2=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted/libril_sitril.so
OD=aarch64-linux-gnu-objdump
echo "=== callers of BuildSetDeviceService(0x238a30) in libsitril ==="
$OD -d "$L" 2>/dev/null | grep -nE 'bl\s+238a30' | head
echo "=== plt stub for BuildSetDeviceService ==="
$OD -d "$L" 2>/dev/null | grep -E '238a30\b' | grep -iE 'plt' | head
$OD -d "$L2" 2>/dev/null | grep -iE 'BuildSetDeviceService|BuildSetVoiceOperation|DeviceService|VoiceOperation' | head
echo "=== strings near DeviceService / VoiceOperation / device_service ==="
strings -a "$L" | grep -iE 'device.?service|voice.?operation|operation.?mode|normal.?service|limited.?service|SET_DEVICE|DEVICE_SERVICE' | head -40
echo "=== handler symbols DeviceService/VoiceOperation ==="
readelf -sW "$L" 2>/dev/null | grep -iE 'DeviceService|VoiceOperation|DsNetwork|DSNetwork' | grep -v Build | head
readelf -sW "$L2" 2>/dev/null | grep -iE 'DeviceService|VoiceOperation|SetDual' | head
