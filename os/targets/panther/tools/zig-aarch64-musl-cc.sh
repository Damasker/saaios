#!/bin/sh
set -eu

# Plain zig-cc wrapper for cross-compiling ordinary C libraries (autotools,
# meson) to aarch64-linux-musl for the panther target. Unlike
# zig-aarch64-musl.sh (used for rustc, which brings its own musl sysroot and
# needs -nostdlib to avoid double-linking startup code), this one keeps
# zig's normal musl headers and libc linking so plain C build systems find
# standard headers like <arpa/inet.h>.

exec "${ZIG:-zig}" cc -target aarch64-linux-musl "$@"
