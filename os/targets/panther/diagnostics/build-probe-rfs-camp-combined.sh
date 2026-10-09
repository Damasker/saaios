#!/bin/sh
# Host-side build only. Does not install to or contact the phone.
set -eu

if [ "$#" -ne 1 ] || [ ! -d "$1" ]; then
    printf 'usage: %s EXISTING_OUTPUT_DIRECTORY\n' "$0" >&2
    exit 64
fi
case "$1" in
    /*) output=$1 ;;
    *) printf 'output directory must be absolute\n' >&2; exit 64 ;;
esac
source_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
compiler=${SAAIOS_RFS_CROSS_CC:-aarch64-linux-gnu-gcc}

"$compiler" -O2 -static -ffunction-sections -fdata-sections \
    -Wl,--gc-sections -DPROBE_PREAMBLE -DPROBE_FULL_MAIN \
    -DPROBE_FIRMWARE_ONLY -DPROBE_COMPLETE -DPROBE_HANDOVER \
    -DPROBE_OWNER_HANDOFF -DPROBE_REPLAY -DPROBE_HANDOVER_EARLY \
    -include "$source_dir/probe-rfs-camp-combined-config.h" \
    "$source_dir/cp-boot-probe.c" \
    -o "$output/probe-handover-rfs-camp-combined"

sha256sum "$output/probe-handover-rfs-camp-combined"
