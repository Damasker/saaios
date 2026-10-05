#!/bin/bash
set -e
IMG=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/vendor.img
OUT=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/extracted
mkdir -p "$OUT"
for f in libsitril.so libril_sitril.so libsit_oem.so libsit_oem_proto.so; do
  debugfs -R "dump /lib64/$f $OUT/$f" "$IMG" 2>/dev/null
  echo "--- $f ---"
  ls -l "$OUT/$f"
  readelf -h "$OUT/$f" 2>/dev/null | grep -E 'Class|Machine|Type|section headers:' | head
  echo "sections:"
  readelf -S "$OUT/$f" 2>/dev/null | grep -E '\.text|\.rodata|\.data' | head
done
