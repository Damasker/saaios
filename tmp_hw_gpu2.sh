#!/system/bin/sh
echo '=== gpuinfo ==='
cat /sys/devices/platform/28000000.mali/gpuinfo
echo
echo '=== gpu_top ==='
ls /sys/devices/platform/28000000.mali/gpu_top
echo '=== mem ==='
cat /sys/devices/platform/28000000.mali/total_gpu_mem 2>/dev/null
cat /sys/devices/platform/28000000.mali/dma_buf_gpu_mem 2>/dev/null
echo '=== kbase attrs ==='
ls /sys/devices/platform/28000000.mali | head -60
echo '=== power / clk ==='
ls /sys/devices/platform/28000000.mali/power 2>/dev/null | head
cat /sys/devices/platform/28000000.mali/power/runtime_status 2>/dev/null
find /sys/devices/platform/28000000.mali -name '*freq*' -o -name '*governor*' -o -name '*opp*' 2>/dev/null | head -40
echo '=== devfreq ==='
ls /sys/class/devfreq 2>/dev/null
for d in /sys/class/devfreq/*; do
  n=$(basename $d)
  echo "-- $n gov=$(cat $d/governor 2>/dev/null) cur=$(cat $d/cur_freq 2>/dev/null) min=$(cat $d/min_freq 2>/dev/null) max=$(cat $d/max_freq 2>/dev/null)"
done
echo '=== firmware loaded ==='
dmesg 2>/dev/null | grep -iE 'mali|csffw|CSF firmware|G710|GPU ident' | grep -viE 'imei|serial' | head -40
echo '=== egl/vulk strings from getprop ==='
getprop ro.hardware.egl
getprop ro.hardware.vulkan
getprop ro.opengles.version
getprop persist.graphics.vulkan.disable
getprop debug.mali.*
getprop | grep -iE '^\[ro.hardware.egl|^\[ro.hardware.vulkan|^\[ro.opengles|^\[ro.gfx|^\[debug.mali|^\[vendor.mali|^\[graphics.gpu' 
echo '=== gpu service ==='
tr '\0' ' ' < /proc/973/cmdline 2>/dev/null; echo
ps -A | grep -iE 'gpu|mali|composer|surfaceflinger' | head
echo '=== mali0 sysfs ==='
ls /sys/class/misc/mali0 2>/dev/null
cat /sys/module/mali_kbase/version 2>/dev/null
ls /sys/module/mali_kbase/parameters 2>/dev/null | head
echo '=== firmware sizes ==='
ls -l /vendor/firmware/mali_csffw*.bin
echo '=== gles apk ==='
ls -l /vendor/lib64/egl/libGLES_mali.so /vendor/lib64/hw/vulkan.mali.so
echo '=== cooling gpu ==='
cat /sys/class/thermal/cooling_device24/type
cat /sys/class/thermal/cooling_device24/max_state
echo '=== pd ==='
ls /sys/devices/platform/18061e00.pd-g3d 2>/dev/null | head
ls /sys/devices/platform/18062000.pd-embedded_g3d 2>/dev/null | head
