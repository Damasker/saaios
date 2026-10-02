#!/bin/sh
# Host-side build only. Does not install to or contact the phone.
# Combined owner: RFS quarantine serving plus the active camp dispatche
# (0x093f -> 0x0404 -> 0x0800) on the early radio edge. Same source as the
# full-quarantine owner, compiled with -DSAAIOS_RFS_CAMP.
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

"$compiler" -std=c11 -O2 -static -Wall -Wextra -Werror -pedantic \
    -DSAAIOS_RFS_CAMP \
    "$source_dir/modem-rfs-full-quarantine-owner.c" \
    -o "$output/modem-rfs-camp-combined-owner"

sha256sum "$output/modem-rfs-camp-combined-owner"
