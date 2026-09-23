#!/bin/sh
# Read-only verification of existing userdata copies; never mount EFS.
# Pixel rfsd: MD5(file bytes || "Samsung_SIT_RIL"), no terminating NUL.
set -eu
for name in nv_normal nv_protected; do
    path=/data/saaios/var/efs-copy/$name.bin
    test -f "$path" && test ! -L "$path"
    test -f "$path.md5" && test ! -L "$path.md5"
    test "$(wc -c < "$path")" -eq 524288
    test "$(wc -c < "$path.md5")" -eq 32
    expected=$(tr 'A-F' 'a-f' < "$path.md5")
    case "$expected" in *[!0-9a-f]*|'') exit 1;; esac
    # Check reads independently: do not let a failed cat be hidden by printf.
    digest=$( (set -e; cat "$path"; printf '%s' 'Samsung_SIT_RIL') | md5sum)
    digest=${digest%% *}
    if test "$digest" != "$expected"; then
        echo "$name: validation FAILED" >&2
        exit 1
    fi
    echo "$name: factory checksum verified"
done
