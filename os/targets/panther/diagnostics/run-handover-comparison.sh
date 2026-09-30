#!/bin/sh
# One explicit comparison after reviewed B firmware/module preparation.
set -eu
[ "$#" -eq 1 ] && [ "$1" = run-once ] || exit 64
[ "$(cat /sys/devices/platform/cpif/modem_state)" = OFFLINE ]
[ "$(sha256sum /lib/modules/cpif.ko | cut -d ' ' -f 1)" = 8cdd21d771189af08035dc6b8fc2b90708a83a520ccb0a45570836a0bce1e79c ]
for name in umts_boot0 umts_ipc0 umts_rfs0; do
    number=$(cat "/sys/class/cpif/$name/dev")
    [ ! -e "/dev/$name" ] || { echo 'Existing node; refusing replacement'; exit 1; }
    mknod "/dev/$name" c "${number%:*}" "${number#*:}"
done
grep -qx 'PARTNAME=persist' /sys/class/block/sda1/uevent
number=$(cat /sys/class/block/sda1/dev)
[ "$number" = '8:1' ]
if awk -v d="$number" '$3==d {f=1} END {exit !f}' /proc/self/mountinfo; then exit 1; fi
audit_dir=$(mktemp -d /dev/saaios-handover.XXXXXX)
mkdir "$audit_dir/mount"
cleanup() {
    if grep -q " $audit_dir/mount " /proc/mounts; then
        umount "$audit_dir/mount" || return
    fi
    rm -f "$audit_dir/partition"
    rmdir "$audit_dir/mount" "$audit_dir"
}
trap cleanup EXIT
mknod "$audit_dir/partition" b 8 1
mount -t ext4 -o ro,noload,nosuid,nodev,noexec "$audit_dir/partition" "$audit_dir/mount"
signature="$audit_dir/mount/modem/cpsha"
[ ! -L "$audit_dir/mount/modem" ] && [ ! -L "$signature" ]
[ -f "$signature" ] && [ "$(wc -c < "$signature")" -eq 64 ]
awk -v p="$audit_dir/mount" '$2==p {print "persist: " $3 " " $4}' /proc/mounts
/tmp/probe-handover boot-b-with-verified-nv-handover "$signature"
