#!/bin/sh
# Inject Mali kbase + CSF firmware into vendor_boot ramdisk.
# Does not change PID 1. GLES/Vulkan stay in artifacts/gpu (Android ABI).
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
artifacts=${SAAIOS_PANTHER_ARTIFACTS:?set SAAIOS_PANTHER_ARTIFACTS}
gpu=${SAAIOS_PANTHER_GPU_ARTIFACTS:-"$script_dir/artifacts/gpu"}
magiskboot=${MAGISKBOOT:?set MAGISKBOOT}
stock=${STOCK_VENDOR_BOOT:?set STOCK_VENDOR_BOOT}
build_dir=${BUILD_DIR:-/var/tmp/saaios-panther/vendor-boot-gpu}
output=${OUTPUT:-"$repo_root/dist/panther/saaios-panther-vendor_boot-gpu.img"}

test -f "$gpu/modules/mali_kbase.ko"
test -f "$gpu/modules/mali_pixel.ko"
test -f "$gpu/modules/gpu_cooling.ko"
test -f "$gpu/firmware/mali_csffw-r54p3.bin"

mkdir -p "$(dirname -- "$output")"
rm -rf "$build_dir"
mkdir -p "$build_dir"
cp "$stock" "$build_dir/vendor_boot.img"
cd "$build_dir"
"$magiskboot" unpack vendor_boot.img || test -f vendor_ramdisk/ramdisk.cpio

"$magiskboot" cpio vendor_ramdisk/ramdisk.cpio \
    "add 0644 lib/modules/mali_kbase.ko $gpu/modules/mali_kbase.ko" \
    "add 0644 lib/modules/mali_pixel.ko $gpu/modules/mali_pixel.ko" \
    "add 0644 lib/modules/gpu_cooling.ko $gpu/modules/gpu_cooling.ko" \
    "mkdir 0755 vendor/firmware" \
    "add 0644 vendor/firmware/mali_csffw-r54p3.bin $gpu/firmware/mali_csffw-r54p3.bin" \
    "add 0644 vendor/firmware/mali_csffw-r54p2.bin $gpu/firmware/mali_csffw-r54p2.bin" \
    "add 0644 vendor/firmware/mali_csffw-r54p1.bin $gpu/firmware/mali_csffw-r54p1.bin" \
    "add 0644 vendor/firmware/mali_csffw-r54p0.bin $gpu/firmware/mali_csffw-r54p0.bin" \
    "add 0644 vendor/firmware/mali_csffw-legacy-r56p0.bin $gpu/firmware/mali_csffw-legacy-r56p0.bin"

"$magiskboot" repack vendor_boot.img "$output"
printf '%s\n' "$output"
