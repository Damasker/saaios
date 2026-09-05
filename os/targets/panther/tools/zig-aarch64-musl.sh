#!/bin/sh
set -eu

# rustc adds an A53 GNU ld workaround that Zig's linker does not accept. Pixel
# 7 does not use Cortex-A53 cores, so discard only that target-specific flag.
for argument in "$@"; do
    shift
    case "$argument" in
        -Wl,--fix-cortex-a53-843419|--target=aarch64-unknown-linux-musl)
            ;;
        *)
            set -- "$@" "$argument"
            ;;
    esac
done

exec "${ZIG:-zig}" cc -target aarch64-linux-musl -nostdlib "$@"
