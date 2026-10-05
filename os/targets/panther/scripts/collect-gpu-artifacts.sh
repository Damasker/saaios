#!/bin/sh
# Pull the stock Mali kit over adb into os/targets/panther/artifacts/gpu.
# Does not insmod, flash, or change PID 1. Requires a rooted panther on
# CP2A.260705.006 matching docs/os/targets/panther/gpu-kit.sha256.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
out=${1:-"$script_dir/../artifacts/gpu"}
adb=${ADB:-adb}

mkdir -p "$out/firmware" "$out/modules/deps" "$out/egl" "$out/hw"

pull() {
  src=$1
  dest=$2
  "$adb" pull "$src" "$dest"
}

pull /vendor/lib/modules/mali_kbase.ko "$out/modules/mali_kbase.ko"
pull /vendor/lib/modules/mali_pixel.ko "$out/modules/mali_pixel.ko"
pull /vendor/lib/modules/gpu_cooling.ko "$out/modules/gpu_cooling.ko"

for fw in mali_csffw-r54p3.bin mali_csffw-r54p2.bin mali_csffw-r54p1.bin \
          mali_csffw-r54p0.bin mali_csffw-legacy-r56p0.bin; do
  pull "/vendor/firmware/$fw" "$out/firmware/$fw"
done

pull /vendor/lib64/egl/libGLES_mali.so "$out/egl/libGLES_mali.so"
pull /vendor/lib64/libOpenCL.so "$out/egl/libOpenCL.so"
pull /vendor/lib64/libOpenCL-pixel.so "$out/egl/libOpenCL-pixel.so"
pull /vendor/lib64/libgpudataproducer.so "$out/egl/libgpudataproducer.so"
pull /vendor/lib64/hw/vulkan.mali.so "$out/hw/vulkan.mali.so"
pull /vendor/lib64/hw/mapper.pixel.so "$out/hw/mapper.pixel.so"
pull /vendor/lib64/hw/android.hardware.graphics.allocator-aidl-impl.so \
  "$out/hw/android.hardware.graphics.allocator-aidl-impl.so"

for ko in systrace.ko google_bcl.ko exynos-pmu-if.ko exynos-pd.ko itmon.ko \
          cmupmucal.ko exynos_pm_qos.ko bts.ko dss.ko pixel_stat_sysfs.ko \
          pixel_stat_mm.ko slc_pt.ko ect_parser.ko; do
  pull "/vendor_dlkm/lib/modules/$ko" "$out/modules/deps/$ko"
done

printf '%s\n' "wrote $out"
