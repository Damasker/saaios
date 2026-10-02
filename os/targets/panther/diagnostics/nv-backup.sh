#!/bin/sh
# nv-backup.sh SRC  -- READ-ONLY full backup of an NV blob/partition + sha256.
# Never writes to SRC. Output goes under /data/saaios/var/nv-backup.
# Usage: sh nv-backup.sh /dev/block/sdaN      (or a file path)
set -eu
SRC="${1:?usage: nv-backup.sh SRC}"
DST_DIR=/data/saaios/var/nv-backup
TS=$(date +%s)
BASE=$(basename "$SRC")
mkdir -p "$DST_DIR"
OUT="$DST_DIR/${BASE}.${TS}.bin"
# dd is a pure read of SRC; SRC is never opened for write.
dd if="$SRC" of="$OUT" bs=65536 conv=fsync 2>/dev/null
SHA=$(sha256sum "$OUT" | cut -d' ' -f1)
echo "$SHA" > "$OUT.sha256"
SZ=$(wc -c < "$OUT")
echo "NV_BACKUP ok src=$SRC out=$OUT size=$SZ sha256=$SHA"
