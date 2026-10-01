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

"$compiler" -std=c11 -O2 -static -Wall -Wextra -Werror -pedantic \
    "$source_dir/modem-rfs-one-grant-owner.c" \
    -o "$output/modem-rfs-one-grant-owner"

sha256sum "$output/modem-rfs-one-grant-owner"
