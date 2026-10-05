#!/system/bin/sh
echo '=== extra rfsd cmdline ==='
tr '\0' ' ' < /proc/6479/cmdline 2>/dev/null; echo
ls -l /proc/6479/fd 2>/dev/null | grep umts || true

echo '=== dmesg cbd/upload ==='
dmesg 2>/dev/null | grep -iE 'cbd:|cpboot|s5100sit|boot_done|CP_BOOT|upload|nv_norm|nv_prot|rfsd:' | grep -viE 'imei|imsi|iccid' | head -60

echo '=== logbuffer cpif tail ==='
# skip if missing
ls /dev/logbuffer* 2>/dev/null | head
dumpsys meminfo rild_exynos 2>/dev/null | head -5
