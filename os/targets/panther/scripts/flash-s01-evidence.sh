#!/bin/sh
# Flash the locally built S01 Evidence image to Pixel 7 slot A only.
# Requires: device in fastboot, Android slot B left untouched.
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../../../.." && pwd)
image=${IMAGE:-"$repo_root/dist/panther/saaios-panther-init_boot.img"}
expected_sha_file="$repo_root/dist/panther/saaios-panther-init_boot.SHA256"
source_commit_file="$repo_root/dist/panther/saaios-panther-init_boot.SOURCE_COMMIT"

if ! command -v fastboot >/dev/null 2>&1; then
    echo "flash-s01-evidence: fastboot not found" >&2
    exit 1
fi
if [ ! -f "$image" ]; then
    echo "flash-s01-evidence: missing image: $image" >&2
    exit 1
fi

size=$(wc -c < "$image" | tr -d ' ')
if [ "$size" != "8388608" ]; then
    echo "flash-s01-evidence: refusing image size $size (want 8388608)" >&2
    exit 1
fi

actual_sha=$(sha256sum "$image" | awk '{print $1}')
if [ -f "$expected_sha_file" ]; then
    expected_sha=$(tr -d ' \n\r\t' < "$expected_sha_file")
    if [ "$actual_sha" != "$expected_sha" ]; then
        echo "flash-s01-evidence: SHA-256 mismatch" >&2
        echo "  expected $expected_sha" >&2
        echo "  actual   $actual_sha" >&2
        exit 1
    fi
fi

if ! fastboot devices | grep -q .; then
    echo "flash-s01-evidence: no fastboot device; boot Pixel 7 to bootloader and retry" >&2
    exit 1
fi

echo "flash-s01-evidence: image=$image"
echo "flash-s01-evidence: sha256=$actual_sha"
if [ -f "$source_commit_file" ]; then
    echo "flash-s01-evidence: source_commit=$(tr -d '\n' < "$source_commit_file")"
fi
echo "flash-s01-evidence: flashing init_boot_a only (slot B untouched)"

fastboot flash init_boot_a "$image"
fastboot reboot

echo "flash-s01-evidence: reboot issued; complete Issue #28 physical checklist"
