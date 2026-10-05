#!/bin/bash
IMG=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/fw/cdma-hunt/factory-td1a-vendor/vendor.img
echo "IMG=$IMG"
ls -l "$IMG"
echo "=== check ext4 ==="
dumpe2fs -h "$IMG" 2>/dev/null | grep -E 'Filesystem volume|Block size|Inode count' | head
echo "=== /lib64 sit/ril ==="
debugfs -R "ls -l /lib64" "$IMG" 2>/dev/null | grep -iE 'sit|ril'
echo "=== /lib sit/ril ==="
debugfs -R "ls -l /lib" "$IMG" 2>/dev/null | grep -iE 'sit|ril'
echo "=== search all for sitril ==="
debugfs -R "ls -R /lib64" "$IMG" 2>/dev/null | grep -iE 'sitril|sit-stream|sit_stream' | head -50
