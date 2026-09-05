#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
artifacts=${SAAIOS_PANTHER_ARTIFACTS:?set SAAIOS_PANTHER_ARTIFACTS}
zig=${ZIG:-zig}
magiskboot=${MAGISKBOOT:?set MAGISKBOOT}
stock=${STOCK_INIT_BOOT:?set STOCK_INIT_BOOT}
tinyalsa_dir=${TINYALSA_DIR:?set TINYALSA_DIR}
source_dir="$script_dir/src"
scripts_dir="$script_dir/scripts"
config_dir="$script_dir/config"
build_dir=${BUILD_DIR:-/var/tmp/saaios-panther/native-c}
bin_dir=${BIN_DIR:-"$repo_root/dist/panther/bin"}
output=${OUTPUT:-"$repo_root/dist/panther/saaios-panther-init_boot.img"}
native_init="$bin_dir/saaios-native-init-arm64"
drm_splash="$bin_dir/saaios-drm-splash-arm64"
touch_monitor="$bin_dir/saaios-touch-monitor-arm64"
wifi_scan="$bin_dir/saaios-wifi-scan-arm64"
sntp_sync="$bin_dir/saaios-sntp-sync-arm64"
bt_init="$bin_dir/saaios-bt-init-arm64"
bt_scan="$bin_dir/saaios-bt-scan-arm64"
bt_pair="$bin_dir/saaios-bt-pair-arm64"
bt_gatt_probe="$bin_dir/saaios-bt-gatt-probe-arm64"
reboot_bootloader="$bin_dir/reboot-bootloader-arm64"
wpa_supplicant="$artifacts/saaios-wpa_supplicant-arm64"
wpa_cli="$artifacts/saaios-wpa_cli-arm64"
tinyplay="$bin_dir/saaios-tinyplay-arm64"
tinymix="$bin_dir/saaios-tinymix-arm64"

mkdir -p "$bin_dir" "$(dirname -- "$output")"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    "$source_dir/native-init.c" -o "$native_init"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    -I/usr/include/libdrm \
    "$source_dir/drm-splash.c" -o "$drm_splash"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    "$source_dir/touch-monitor.c" -o "$touch_monitor"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    "$source_dir/wifi-scan.c" -o "$wifi_scan"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    "$source_dir/sntp-sync.c" -o "$sntp_sync"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    "$source_dir/bt-init.c" -o "$bt_init"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    "$source_dir/bt-scan.c" -o "$bt_scan"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    "$source_dir/bt-pair.c" -o "$bt_pair"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    "$source_dir/bt-gatt-probe.c" -o "$bt_gatt_probe"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    "$source_dir/reboot-bootloader.c" -o "$reboot_bootloader"

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    -I"$tinyalsa_dir/include" \
    "$tinyalsa_dir/tinyplay.c" "$tinyalsa_dir/pcm.c" \
    "$tinyalsa_dir/pcm_hw.c" "$tinyalsa_dir/pcm_plugin.c" \
    "$tinyalsa_dir/snd_utils.c" -o "$tinyplay" -ldl -lpthread

"$zig" cc -target aarch64-linux-musl -static -Os -s \
    -I"$tinyalsa_dir/include" \
    "$tinyalsa_dir/tinymix.c" "$tinyalsa_dir/mixer.c" \
    "$tinyalsa_dir/mixer_hw.c" "$tinyalsa_dir/mixer_plugin.c" \
    "$tinyalsa_dir/snd_utils.c" -o "$tinymix" -ldl -lpthread

rm -rf "$build_dir"
mkdir -p "$build_dir"
cp "$stock" "$build_dir/boot.img"

cd "$build_dir"
"$magiskboot" unpack boot.img

set -- ramdisk.cpio \
    "rm -r .backup" \
    "rm -r overlay.d" \
    "add 0755 init $native_init" \
    "mkdir 0755 saaios" \
    "add 0755 saaios/busybox $artifacts/busybox-arm64" \
    "add 0755 saaios/saaios-runtime $artifacts/saaios-runtime-panther-tcp" \
    "add 0755 saaios/saaios-console $artifacts/saaios-console-panther-tcp" \
    "add 0755 saaios/drm-splash $drm_splash" \
    "add 0755 saaios/touch-monitor $touch_monitor" \
    "add 0755 saaios/wifi-scan $wifi_scan" \
    "add 0755 saaios/sntp-sync $sntp_sync" \
    "add 0755 saaios/bt-init $bt_init" \
    "add 0755 saaios/bt-scan $bt_scan" \
    "add 0755 saaios/bt-pair $bt_pair" \
    "add 0755 saaios/bt-gatt-probe $bt_gatt_probe" \
    "add 0755 saaios/wpa_supplicant $wpa_supplicant" \
    "add 0755 saaios/wpa_cli $wpa_cli" \
    "add 0755 saaios/tinyplay $tinyplay" \
    "add 0755 saaios/tinymix $tinymix" \
    "add 0755 saaios/audio-init.sh $scripts_dir/audio-init.sh" \
    "add 0755 saaios/audio-test.sh $scripts_dir/audio-test.sh" \
    "add 0755 saaios/audio-volume.sh $scripts_dir/audio-volume.sh" \
    "add 0755 saaios/display-brightness.sh $scripts_dir/display-brightness.sh" \
    "add 0644 saaios/test-tone.wav $artifacts/saaios-test-tone.wav" \
    "mkdir 0755 saaios/firmware" \
    "add 0644 saaios/firmware/BCM.hcd $artifacts/BCM.hcd" \
    "add 0644 saaios/wpa_supplicant.conf $config_dir/wpa_supplicant.conf" \
    "add 0755 saaios/udhcpc.script $scripts_dir/udhcpc.script" \
    "add 0755 saaios/wifi-action.sh $scripts_dir/wifi-action.sh" \
    "add 0755 saaios/reboot-bootloader $reboot_bootloader" \
    "add 0644 saaios/focal_touch.ko $artifacts/focal_touch.ko" \
    "mkdir 0755 lib" \
    "mkdir 0755 lib/firmware" \
    "add 0644 lib/firmware/focaltech_ts_fw.bin $artifacts/focaltech_ts_fw.bin" \
    "add 0644 lib/firmware/focaltech_testconf.ini $artifacts/focaltech_testconf.ini"

for applet in sh touch cat uname ps dmesg mount ls ip ifconfig reboot sync; do
    set -- "$@" "ln busybox saaios/$applet"
done

"$magiskboot" cpio "$@"
"$magiskboot" repack boot.img "$output"

printf '%s\n' "$output"
