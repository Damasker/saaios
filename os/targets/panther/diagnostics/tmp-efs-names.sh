#!/bin/sh
# Read-only names and sizes on the original EFS. No file contents.
set -eu
uevent=/sys/block/sda/sda5/uevent
grep -qx 'PARTNAME=efs' "$uevent"
grep -qx 'DEVNAME=sda5' "$uevent"
if grep -q 'sda5' /proc/mounts; then
    echo 'ABORT already-mounted'
    exit 1
fi
dev=$(cat /sys/block/sda/sda5/dev)
major=${dev%%:*}
minor=${dev#*:}
work=$(mktemp -d /dev/block/saaios-efs-ro.XXXXXX)
mount_dir=$work/original
node=$work/efs-block
mkdir "$mount_dir"
mknod -m 400 "$node" b "$major" "$minor"
cleanup() {
    umount "$mount_dir" 2>/dev/null || true
    rm -f "$node"
    rmdir "$mount_dir" 2>/dev/null || true
    rmdir "$work" 2>/dev/null || true
}
trap cleanup EXIT
mount -t f2fs -o ro,norecovery,nodiscard,nosuid,nodev,noexec,noatime \
    "$node" "$mount_dir"
echo "MOUNTED ro"
total=0
match=0
# Names and sizes only. Skip the walk's own output of non-matching files.
/saaios/busybox find "$mount_dir" -type f | while IFS= read -r path; do
    total=$((total + 1))
    base=${path##*/}
    case "$base" in
        *nv*|*NV*|*tmp*|*temp*|*prot*|*md5*|*cal*|*unsigned*)
            sz=$(stat -c '%s' "$path")
            rel=${path#"$mount_dir"/}
            printf '%s %s\n' "$sz" "$rel"
            match=$((match + 1))
            ;;
    esac
done
echo "DONE"
