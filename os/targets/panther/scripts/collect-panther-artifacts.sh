#!/bin/sh
# Pull the proprietary SAAIOS_PANTHER_ARTIFACTS subset and the stock
# init_boot/vendor_boot images from a rooted stock panther over adb.
# Read-only on the phone: pulls files, dd-reads boot partitions of the
# stock slot. Never writes partitions, EFS or NV. Output stays local
# (os/targets/panther/artifacts is gitignored) -- do not commit it.
#
# Repo-built or fetched entries of artifacts.example.manifest
# (busybox-arm64, saaios-wpa_*-arm64, saaios-*-panther-tcp,
# saaios-test-tone.wav) are not on the phone and are reported as skipped.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
manifest="$script_dir/../artifacts.example.manifest"
out=${1:-"$script_dir/../artifacts/panther"}
slot=${STOCK_SLOT:-_b}
adb=${ADB:-adb}

not_on_phone="busybox-arm64 saaios-wpa_supplicant-arm64 saaios-wpa_cli-arm64
saaios-console-panther-tcp saaios-runtime-panther-tcp saaios-test-tone.wav"

mkdir -p "$out"

"$adb" root >/dev/null 2>&1 || true
"$adb" wait-for-device
if [ "$("$adb" shell id -u | tr -d '\r')" = 0 ]; then
  sush() { "$adb" shell "$1"; }
else
  sush() { "$adb" shell "su -c '$1'"; }
fi

missing=""
skipped=""
while IFS= read -r name; do
  case $name in ''|'#'*) continue ;; esac
  case " $(echo $not_on_phone) " in
    *" $name "*) skipped="$skipped $name"; continue ;;
  esac
  # GKI modules (rfkill, bluetooth, bt*, hci_uart) also exist in vendor_dlkm,
  # but the GKI kernel only accepts the signed system_dlkm copies.
  src=$(sush "find /system_dlkm/lib/modules /vendor /vendor_dlkm /odm -name '$name' -type f 2>/dev/null | head -n 1" | tr -d '\r')
  if [ -z "$src" ]; then
    missing="$missing $name"
    continue
  fi
  sush "cat '$src' > /data/local/tmp/saaios-collect.bin && chmod 0644 /data/local/tmp/saaios-collect.bin"
  "$adb" pull /data/local/tmp/saaios-collect.bin "$out/$name" >/dev/null
  printf 'pulled %s <- %s\n' "$name" "$src"
done < "$manifest"
sush "rm -f /data/local/tmp/saaios-collect.bin"

for part in init_boot vendor_boot; do
  sush "dd if=/dev/block/by-name/$part$slot of=/data/local/tmp/$part.img bs=4M 2>/dev/null && chmod 0644 /data/local/tmp/$part.img"
  "$adb" pull "/data/local/tmp/$part.img" "$out/$part.img" >/dev/null
  sush "rm -f /data/local/tmp/$part.img"
  printf 'pulled %s.img <- %s%s\n' "$part" "$part" "$slot"
done

(cd "$out" && sha256sum -- * > SHA256SUMS.local)

printf 'wrote %s\n' "$out"
[ -z "$skipped" ] || printf 'not on phone (build/fetch separately):%s\n' "$skipped"
[ -z "$missing" ] || { printf 'NOT FOUND on phone:%s\n' "$missing"; exit 2; }
