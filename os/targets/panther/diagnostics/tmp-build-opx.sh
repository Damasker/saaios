#!/bin/bash
set -e
SRC=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics
OUT=/mnt/c/Users/Admin/Projects/saaios-som/os/targets/panther/diagnostics/build-out
mkdir -p "$OUT"
echo "=== host self-test build ==="
gcc -std=c11 -O2 -Wall -Wextra -Werror -pedantic -DSAAIOS_RFS_CAMP -DRFS_HOST_TEST \
    "$SRC/modem-rfs-full-quarantine-owner.c" -o "$OUT/st"
echo "=== run self-test ==="
"$OUT/st" self-test
echo "=== aarch64 static build ==="
aarch64-linux-gnu-gcc -std=c11 -O2 -static -Wall -Wextra -Werror -pedantic \
    -DSAAIOS_RFS_CAMP "$SRC/modem-rfs-full-quarantine-owner.c" \
    -o "$OUT/modem-rfs-camp-combined-owner"
echo "=== mode check + hash ==="
file "$OUT/modem-rfs-camp-combined-owner" | sed 's/,/\n/g' | head -3
sha256sum "$OUT/modem-rfs-camp-combined-owner"
