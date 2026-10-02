#!/bin/sh
# Host build only; no phone installation or communication. Builds the
# mutually exclusive early camp-on SGC owner/probe pair, which dispatches the
# stock stage-1 sequence in order on the early radio trigger (0x0803 then
# 0x0802 raw 0): SetModemsConfig (0x093f) then the SGC (0x0404) then
# TrySetRadioPower(10) carrier SET (0x0800), before the settled baseline.
# Late-SGC, early-SGC, early-sequence, scan and passive binaries are unchanged;
# this script emits separately named artifacts only.
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
compiler=${SAAIOS_SGC_CROSS_CC:-aarch64-linux-gnu-gcc}

"$compiler" -O2 -static -Wall -Wextra -Werror -DSAAIOS_SGC_CAMP_ONCE \
    "$source_dir/modem-channel-owner.c" \
    -o "$output/modem-channel-owner-sgc-camp-once"

"$compiler" -O2 -static -ffunction-sections -fdata-sections \
    -Wl,--gc-sections -DPROBE_PREAMBLE -DPROBE_FULL_MAIN \
    -DPROBE_FIRMWARE_ONLY -DPROBE_COMPLETE -DPROBE_HANDOVER \
    -DPROBE_OWNER_HANDOFF -include "$source_dir/probe-sgc-camp-once-config.h" \
    "$source_dir/cp-boot-probe.c" \
    -o "$output/probe-handover-sgc-camp-once"

sha256sum "$output/modem-channel-owner-sgc-camp-once" \
          "$output/probe-handover-sgc-camp-once"
