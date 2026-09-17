#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd)
build=${TMPDIR:-/tmp}/saaios-render-preview-$$
trap 'rm -rf "$build"' EXIT INT TERM
mkdir -p "$build/first" "$build/second"

cc=${CC:-cc}
"$cc" -std=c11 -Wall -Wextra -Werror \
    -I/usr/include/libdrm \
    "$root/os/targets/panther/src/drm-splash.c" \
    -o "$build/drm-splash-preview" -lm

export SAAIOS_FONT_DIR="$root/os/targets/panther/assets/fonts"
"$build/drm-splash-preview" --preview "$build/first"
"$build/drm-splash-preview" --preview "$build/second"

first_count=$(find "$build/first" -type f -name '*.ppm' | wc -l)
test "$first_count" -eq 12

for first in "$build/first"/*.ppm; do
    name=$(basename "$first")
    cmp "$first" "$build/second/$name"
    test "$(wc -c < "$first")" -gt 100000
done

echo "render-preview: 12 deterministic screens passed"
