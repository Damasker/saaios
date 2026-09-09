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
    # Accept bare hash or `sha256sum` two-field lines; take first token only.
    expected_sha=$(awk 'NF { print $1; exit }' "$expected_sha_file")
    if [ -z "$expected_sha" ] || [ "$actual_sha" != "$expected_sha" ]; then
        echo "flash-s01-evidence: SHA-256 mismatch" >&2
        echo "  expected $expected_sha" >&2
        echo "  actual   $actual_sha" >&2
        exit 1
    fi
fi

device_lines=$(fastboot devices | awk 'NF >= 2')
device_count=$(printf '%s\n' "$device_lines" | awk 'NF { c++ } END { print c+0 }')
if [ "$device_count" -eq 0 ]; then
    echo "flash-s01-evidence: no fastboot device; boot Pixel 7 to bootloader and retry" >&2
    exit 1
fi
if [ "$device_count" -ne 1 ] && [ -z "${ANDROID_SERIAL:-}" ]; then
    echo "flash-s01-evidence: refusing multiple fastboot devices; set ANDROID_SERIAL" >&2
    printf '%s\n' "$device_lines" >&2
    exit 1
fi

# Pixel 7 codename is panther; refuse any other bootloader product.
product=$(fastboot getvar product 2>&1 | sed -n 's/^.*product:[[:space:]]*//p' | head -n 1 | tr -d '\r')
if [ "$product" != "panther" ]; then
    echo "flash-s01-evidence: refusing product '$product' (want panther / Pixel 7)" >&2
    exit 1
fi

if [ "${S01_FLASH_CONFIRM:-}" != "1" ]; then
    echo "flash-s01-evidence: set S01_FLASH_CONFIRM=1 to flash init_boot_a on panther" >&2
    exit 1
fi

echo "flash-s01-evidence: image=$image"
echo "flash-s01-evidence: sha256=$actual_sha"
echo "flash-s01-evidence: product=$product"
if [ -f "$source_commit_file" ]; then
    echo "flash-s01-evidence: source_commit=$(tr -d '\n' < "$source_commit_file")"
fi
echo "flash-s01-evidence: flashing init_boot_a only (slot B untouched)"

fastboot flash init_boot_a "$image"
fastboot reboot

echo "flash-s01-evidence: reboot issued; complete Issue #28 physical checklist"
