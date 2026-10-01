#!/bin/sh
# Manual provenance check only. Never use original EFS as an RFS destination.
set -eu
umask 077

fail() {
    printf 'ABORT %s\n' "$1" >&2
    exit 1
}

parent_dir=/dev/block
parent_identity=
work_dir=
mount_dir=
block_node=
node_identity=
verified=0
mode=
source_pin=
pin_dir=/run/saaios-rfs-one-grant
pin_file=$pin_dir/expected.sha256
run_identity=

run_is_tmpfs() {
    awk '$2 == "/run" { n++; if ($3 != "tmpfs") bad=1 }
         END { exit (n == 1 && !bad) ? 0 : 1 }' /proc/mounts
}

# Keep the private node and directory if unmount cannot be confirmed. A
# misleading PASS, or removing the mountpoint while EFS is mounted, is worse.
cleanup() {
    result=$?
    trap - 0 1 2 3 15
    if [ -n "$work_dir" ]; then
        current_parent=$(stat -c '%d:%i:%u:%a' "$parent_dir" 2>/dev/null) ||
            current_parent=
        if [ -z "$parent_identity" ] || [ ! -d "$parent_dir" ] ||
           [ -L "$parent_dir" ] ||
           [ "$current_parent" != "$parent_identity" ]; then
            printf 'ABORT /dev/block identity changed; private paths retained\n' >&2
            exit 1
        fi
        if [ -n "$mount_dir" ] && [ -d "$mount_dir" ]; then
            count=$(awk -v target="$mount_dir" '$5 == target { n++ } END { print n+0 }' \
                /proc/self/mountinfo) || {
                printf 'ABORT cannot confirm EFS unmount; private mountpoint retained\n' >&2
                exit 1
            }
            if [ "$count" -ne 0 ]; then
                if ! umount "$mount_dir" >/dev/null 2>&1; then
                    printf 'ABORT EFS unmount failed; private mountpoint retained\n' >&2
                    exit 1
                fi
                count=$(awk -v target="$mount_dir" \
                    '$5 == target { n++ } END { print n+0 }' \
                    /proc/self/mountinfo) || exit 1
                if [ "$count" -ne 0 ]; then
                    printf 'ABORT EFS still mounted; private mountpoint retained\n' >&2
                    exit 1
                fi
            fi
        fi
        if [ -n "$block_node" ] && { [ -e "$block_node" ] || [ -L "$block_node" ]; }; then
            current_identity=$(stat -c '%d:%i:%t:%T:%u:%a:%h' \
                "$block_node" 2>/dev/null) || current_identity=
            if [ -z "$node_identity" ] || [ -L "$block_node" ] ||
               [ ! -b "$block_node" ] ||
               [ "$current_identity" != "$node_identity" ]; then
                printf 'ABORT private EFS node identity changed; node retained\n' >&2
                result=1
            else
                rm "$block_node" || result=1
            fi
        elif [ -n "$node_identity" ]; then
            printf 'ABORT private EFS node disappeared\n' >&2
            result=1
        fi
        if [ -n "$mount_dir" ] && [ -d "$mount_dir" ]; then
            rmdir "$mount_dir" || result=1
        fi
        rmdir "$work_dir" || result=1
    fi
    if [ "$result" -eq 0 ] && [ "$verified" -eq 1 ] &&
       [ "$mode" = pin-read-only ]; then
        # Publish only after a confirmed unmount and complete private cleanup.
        count=$(awk -v dev="$dev_id" '$3 == dev { n++ } END { print n+0 }' \
            /proc/self/mountinfo) || count=-1
        current_run=$(stat -c '%d:%i:%u:%a' /run 2>/dev/null) ||
            current_run=
        if [ "$count" -ne 0 ] || [ -L /run ] ||
           [ "$current_run" != "$run_identity" ] || ! run_is_tmpfs ||
           [ -e "$pin_dir" ] || [ -L "$pin_dir" ] ||
           ! mkdir -m 700 "$pin_dir"; then
            printf 'ABORT cannot publish a fresh pin after EFS cleanup\n' >&2
            result=1
        elif [ -L "$pin_dir" ] ||
             [ "$(stat -c '%u:%a:%h' "$pin_dir" 2>/dev/null)" != 0:700:2 ]; then
            printf 'ABORT private pin directory identity mismatch\n' >&2
            result=1
        else
            # An O_EXCL-created temp becomes the final path only after its
            # contents and metadata have been checked. No partial final pin.
            pin_tmp=$(mktemp "$pin_dir/.expected.XXXXXX" 2>/dev/null) ||
                pin_tmp=
            if [ -z "$pin_tmp" ] || [ -e "$pin_file" ] || [ -L "$pin_file" ] ||
               ! printf '%s' "$source_pin" > "$pin_tmp" ||
               [ -L "$pin_tmp" ] ||
               [ "$(stat -c '%u:%a:%h:%s' "$pin_tmp" 2>/dev/null)" != \
                 0:600:1:64 ] || ! ln "$pin_tmp" "$pin_file" ||
               ! rm "$pin_tmp" ||
               [ "$(stat -c '%u:%a:%h:%s' "$pin_file" 2>/dev/null)" != \
                 0:600:1:64 ]; then
                printf 'ABORT private pin publication failed; directory retained\n' >&2
                result=1
            fi
        fi
    fi
    if [ "$result" -eq 0 ] && [ "$verified" -eq 1 ]; then
        printf 'PASS original EFS matches the four userdata files; EFS unmounted'
        if [ "$mode" = pin-read-only ]; then printf '; private pin ready'; fi
        printf '\n'
    fi
    exit "$result"
}
trap cleanup 0
trap 'exit 1' 1 2 3 15

[ "$#" -eq 1 ] || {
    printf 'usage: sh %s verify-read-only|pin-read-only\n' "$0" >&2
    exit 64
}
case "$1" in
    verify-read-only|pin-read-only) mode=$1 ;;
    *) printf 'usage: sh %s verify-read-only|pin-read-only\n' "$0" >&2; exit 64 ;;
esac
ulimit -c 0
[ "$(id -u)" = 0 ] || fail 'root is required'
if [ "$mode" = pin-read-only ]; then
    [ -d /run ] && [ ! -L /run ] &&
        [ "$(stat -c '%u:%a' /run 2>/dev/null)" = 0:755 ] ||
        fail '/run is not the expected private-pin parent'
    run_is_tmpfs ||
        fail '/run is not a dedicated tmpfs'
    run_identity=$(stat -c '%d:%i:%u:%a' /run 2>/dev/null) ||
        fail '/run identity unavailable'
    [ ! -e "$pin_dir" ] && [ ! -L "$pin_dir" ] ||
        fail 'private pin directory already exists'
fi

sys_part=/sys/block/sda/sda5
uevent=$sys_part/uevent
copy_dir=/data/saaios/var/efs-copy
[ -r "$uevent" ] && [ -r "$sys_part/dev" ] || fail 'sda5 sysfs identity unavailable'
[ -d "$copy_dir" ] && [ ! -L "$copy_dir" ] || fail 'userdata copy directory unavailable'

dev_id=$(cat "$sys_part/dev" 2>/dev/null) || fail 'sda5 device number unavailable'
case "$dev_id" in *:*) ;; *) fail 'invalid sda5 device number' ;; esac
major=${dev_id%%:*}
minor=${dev_id#*:}
case "$major" in ''|*[!0-9]*) fail 'invalid sda5 major number' ;; esac
case "$minor" in ''|*[!0-9]*) fail 'invalid sda5 minor number' ;; esac
case "$major" in 0|[1-9]*) ;; *) fail 'invalid sda5 major number' ;; esac
case "$minor" in 0|[1-9]*) ;; *) fail 'invalid sda5 minor number' ;; esac

uevent_exact() {
    key=$1
    expected=$2
    count=$(grep -c "^${key}=" "$uevent" 2>/dev/null) || return 1
    [ "$count" = 1 ] && grep -Fxq "$key=$expected" "$uevent" 2>/dev/null
}
uevent_exact PARTNAME efs &&
uevent_exact DEVNAME sda5 &&
uevent_exact DEVTYPE partition &&
uevent_exact MAJOR "$major" &&
uevent_exact MINOR "$minor" || fail 'sda5 is not the exact efs partition'

[ -r /proc/self/mountinfo ] && [ -r /proc/mounts ] || fail 'mount tables unavailable'
count=$(awk -v dev="$dev_id" '$3 == dev { n++ } END { print n+0 }' \
    /proc/self/mountinfo) || fail 'cannot inspect existing mounts'
[ "$count" -eq 0 ] || fail 'original EFS is already mounted'

[ -d "$parent_dir" ] && [ ! -L "$parent_dir" ] ||
    fail '/dev/block parent unavailable or linked'
[ "$(stat -c '%u:%a' "$parent_dir" 2>/dev/null)" = 0:700 ] ||
    fail '/dev/block parent owner or mode mismatch'
parent_identity=$(stat -c '%d:%i:%u:%a' "$parent_dir" 2>/dev/null) ||
    fail '/dev/block parent identity unavailable'
work_dir=$(mktemp -d "$parent_dir/saaios-efs-ro.XXXXXX" 2>/dev/null) ||
    fail 'cannot create private mount directory'
[ -d "$parent_dir" ] && [ ! -L "$parent_dir" ] &&
    [ "$(stat -c '%d:%i:%u:%a' "$parent_dir" 2>/dev/null)" = \
      "$parent_identity" ] || fail '/dev/block parent identity changed'
[ -d "$work_dir" ] && [ ! -L "$work_dir" ] &&
    [ "$(stat -c '%u:%a' "$work_dir" 2>/dev/null)" = 0:700 ] ||
    fail 'private directory identity mismatch'
mount_dir=$work_dir/original
block_node=$work_dir/efs-block
mkdir "$mount_dir" || fail 'cannot create private mountpoint'
mknod -m 400 "$block_node" b "$major" "$minor" ||
    fail 'cannot create private EFS node'
[ -b "$block_node" ] && [ ! -L "$block_node" ] &&
    [ "$(stat -c '%t:%T:%u:%a:%h' "$block_node" 2>/dev/null)" = \
      "$(printf '%x:%x:0:400:1' "$major" "$minor")" ] ||
    fail 'private EFS node major/minor or mode mismatch'
node_identity=$(stat -c '%d:%i:%t:%T:%u:%a:%h' "$block_node" 2>/dev/null) ||
    fail 'private EFS node identity unavailable'

mount -t f2fs -o ro,norecovery,nodiscard,nosuid,nodev,noexec,noatime \
    "$block_node" "$mount_dir" >/dev/null 2>&1 || fail 'read-only EFS mount failed'
awk -v target="$mount_dir" -v dev="$dev_id" '
    $5 == target { n++; if ($3 != dev) bad=1 }
    END { exit (n == 1 && !bad) ? 0 : 1 }
' /proc/self/mountinfo || fail 'mounted EFS device identity mismatch'
awk -v target="$mount_dir" '
    $2 == target {
        n++
        if ($3 != "f2fs") bad=1
        count=split($4, option, ",")
        for (i=1; i<=count; i++) seen[option[i]]=1
        if (!seen["ro"] || seen["rw"] || !seen["norecovery"] ||
            !seen["nodiscard"] || !seen["nosuid"] || !seen["nodev"] ||
            !seen["noexec"] || !seen["noatime"]) bad=1
    }
    END { exit (n == 1 && !bad) ? 0 : 1 }
' /proc/mounts || fail 'required EFS read-only options not active'

file_names='nv_protected.bin nv_normal.bin nv_protected.bin.md5 nv_normal.bin.md5'
snapshot_files() {
    for name in $file_names; do
        case "$name" in *.md5) expected_size=32 ;; *) expected_size=524288 ;; esac
        original=$mount_dir/$name
        copy=$copy_dir/$name
        [ -f "$original" ] && [ ! -L "$original" ] &&
            [ -f "$copy" ] && [ ! -L "$copy" ] || return 1
        original_info=$(stat -c '%d:%i:%s:%Y:%Z:%u:%a:%h' \
            "$original" 2>/dev/null) || return 1
        copy_info=$(stat -c '%d:%i:%s:%Y:%Z:%u:%a:%h' \
            "$copy" 2>/dev/null) || return 1
        [ "$(stat -c '%s' "$original" 2>/dev/null)" = "$expected_size" ] &&
            [ "$(stat -c '%u:%a:%h:%s' "$copy" 2>/dev/null)" = \
              "0:600:1:$expected_size" ] || return 1
        printf '%s|%s\n' "$original_info" "$copy_info"
    done
}
before=$(snapshot_files) || fail 'original or userdata file metadata refused'
for name in $file_names; do
    cmp -s "$mount_dir/$name" "$copy_dir/$name" ||
        fail "original/copy comparison failed: $name"
done
if [ "$mode" = pin-read-only ]; then
    original_sum=$(sha256sum "$mount_dir/nv_protected.bin") ||
        fail 'original protected-NV digest failed'
    copy_sum=$(sha256sum "$copy_dir/nv_protected.bin") ||
        fail 'userdata protected-NV digest failed'
    source_pin=${original_sum%% *}
    copy_pin=${copy_sum%% *}
    [ "${#source_pin}" -eq 64 ] && [ "$source_pin" = "$copy_pin" ] ||
        fail 'protected-NV digest mismatch'
    case "$source_pin" in *[!0-9a-f]*) fail 'invalid protected-NV digest' ;; esac
fi
after=$(snapshot_files) || fail 'original or userdata file metadata changed'
[ "$before" = "$after" ] || fail 'original or userdata file metadata changed'

verified=1
