#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
artifacts=${SAAIOS_PANTHER_ARTIFACTS:?set SAAIOS_PANTHER_ARTIFACTS}
magiskboot=${MAGISKBOOT:?set MAGISKBOOT}
stock=${STOCK_VENDOR_BOOT:?set STOCK_VENDOR_BOOT}
build_dir=${BUILD_DIR:-/var/tmp/saaios-panther/vendor-boot}
output=${OUTPUT:-"$repo_root/dist/panther/saaios-panther-vendor_boot.img"}

mkdir -p "$(dirname -- "$output")"

rm -rf "$build_dir"
mkdir -p "$build_dir"
cp "$stock" "$build_dir/vendor_boot.img"

cd "$build_dir"
"$magiskboot" unpack vendor_boot.img || test -f vendor_ramdisk/ramdisk.cpio

"$magiskboot" cpio vendor_ramdisk/ramdisk.cpio \
    "add 0644 lib/modules/rfkill.ko $artifacts/rfkill.ko" \
    "add 0644 lib/modules/cfg80211.ko $artifacts/cfg80211.ko" \
    "add 0644 lib/modules/bcmdhd4389.ko $artifacts/bcmdhd4389.ko" \
    "add 0644 lib/modules/bluetooth.ko $artifacts/bluetooth.ko" \
    "add 0644 lib/modules/btqca.ko $artifacts/btqca.ko" \
    "add 0644 lib/modules/btbcm.ko $artifacts/btbcm.ko" \
    "add 0644 lib/modules/hci_uart.ko $artifacts/hci_uart.ko" \
    "add 0644 lib/modules/cl_dsp-core.ko $artifacts/cl_dsp-core.ko" \
    "add 0644 lib/modules/cs40l26-core.ko $artifacts/cs40l26-core.ko" \
    "add 0644 lib/modules/cs40l26-i2c.ko $artifacts/cs40l26-i2c.ko" \
    "add 0644 lib/modules/snd-soc-cs40l26.ko $artifacts/snd-soc-cs40l26.ko" \
    "mkdir 0755 vendor/firmware" \
    "add 0644 vendor/firmware/fw_bcmdhd.bin $artifacts/fw_bcmdhd.bin" \
    "add 0644 vendor/firmware/bcmdhd.cal $artifacts/bcmdhd.cal" \
    "add 0644 vendor/firmware/bcmdhd_clm.blob $artifacts/bcmdhd_clm.blob" \
    "add 0644 vendor/firmware/aoc.bin $artifacts/aoc.bin" \
    "add 0644 vendor/firmware/cs35l41-dsp1-spk-cali.bin $artifacts/cs35l41-dsp1-spk-cali.bin" \
    "add 0644 vendor/firmware/cs35l41-dsp1-spk-cali.wmfw $artifacts/cs35l41-dsp1-spk-cali.wmfw" \
    "add 0644 vendor/firmware/cs35l41-dsp1-spk-diag.bin $artifacts/cs35l41-dsp1-spk-diag.bin" \
    "add 0644 vendor/firmware/cs35l41-dsp1-spk-diag.wmfw $artifacts/cs35l41-dsp1-spk-diag.wmfw" \
    "add 0644 vendor/firmware/cs35l41-dsp1-spk-prot.bin $artifacts/cs35l41-dsp1-spk-prot.bin" \
    "add 0644 vendor/firmware/cs35l41-dsp1-spk-prot.wmfw $artifacts/cs35l41-dsp1-spk-prot.wmfw" \
    "add 0644 vendor/firmware/R-cs35l41-dsp1-spk-cali.bin $artifacts/R-cs35l41-dsp1-spk-cali.bin" \
    "add 0644 vendor/firmware/R-cs35l41-dsp1-spk-diag.bin $artifacts/R-cs35l41-dsp1-spk-diag.bin" \
    "add 0644 vendor/firmware/R-cs35l41-dsp1-spk-prot.bin $artifacts/R-cs35l41-dsp1-spk-prot.bin" \
    "add 0644 vendor/firmware/cs40l26.wmfw $artifacts/cs40l26.wmfw" \
    "add 0644 vendor/firmware/cs40l26-calib.wmfw $artifacts/cs40l26-calib.wmfw" \
    "add 0644 vendor/firmware/cs40l26.bin $artifacts/cs40l26.bin" \
    "add 0644 vendor/firmware/cs40l26-svc.bin $artifacts/cs40l26-svc.bin" \
    "add 0644 vendor/firmware/cs40l26-dvl.bin $artifacts/cs40l26-dvl.bin" \
    "add 0644 vendor/firmware/fast_switch1.txt $artifacts/fast_switch1.txt" \
    "add 0644 vendor/firmware/fast_switch2.txt $artifacts/fast_switch2.txt" \
    "add 0644 vendor/firmware/fast_switch3.txt $artifacts/fast_switch3.txt" \
    "add 0644 vendor/firmware/fast_switch4.txt $artifacts/fast_switch4.txt"

"$magiskboot" repack vendor_boot.img "$output"
printf '%s\n' "$output"
