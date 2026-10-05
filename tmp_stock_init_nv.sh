#!/system/bin/sh
echo '=== cmdline ==='
tr '\0' ' ' < /proc/1139/cmdline; echo
tr '\0' ' ' < /proc/1155/cmdline; echo
tr '\0' ' ' < /proc/1158/cmdline; echo
tr '\0' ' ' < /proc/992/cmdline; echo

echo '=== cbd.rc ==='
cat /vendor/etc/init/cbd.rc
echo '=== init.modem.rc ==='
cat /vendor/etc/init/init.modem.rc
echo '=== rfsd.rc ==='
cat /vendor/etc/init/rfsd.rc
echo '=== rild_exynos.rc ==='
cat /vendor/etc/init/rild_exynos.rc

echo '=== fstab efs/modem ==='
grep -E 'efs|modem' /vendor/etc/fstab.* 2>/dev/null | head -40

echo '=== nv sizes crc ==='
# size + first 64 bytes hex of tmp (payload) only, no identity parse
for f in /mnt/vendor/efs/nv_normal.bin.tmp /mnt/vendor/efs/nv_protected.bin.tmp /mnt/vendor/efs/nv_normal.bin /mnt/vendor/efs/nv_protected.bin; do
  echo "-- $f"
  wc -c "$f"
  dd if="$f" bs=64 count=1 2>/dev/null | xxd -p | head -4
done

echo '=== md5 files exist only ==='
cat /mnt/vendor/efs/nv_normal.bin.md5
echo
cat /mnt/vendor/efs/nv_protected.bin.md5
echo

echo '=== sit-ril log dir ==='
ls -l /data/vendor/radio/sit-ril 2>/dev/null | head -20

echo '=== mounts ==='
mount | grep -E 'efs|modem|rfs'
