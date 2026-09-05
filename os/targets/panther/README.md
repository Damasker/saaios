# Pixel 7 (panther) native SaaiOS target

This target boots a standalone static Linux userspace on the Google Pixel 7.
Android framework and Android userspace are not started. The production entry
point is `src/native-init.c`; display, input, Wi-Fi, Bluetooth, storage, audio
and the SaaiOS runtime are brought up directly from PID 1.

The target is pinned to the Pixel firmware base `CP2A.260705.006`. Slot A was
used for SaaiOS during bring-up, while slot B remained the stock Android
fallback.

## Repository contents

- `src/` — production PID 1, DRM UI, Wi-Fi, Bluetooth, time and input tools.
- `scripts/` — runtime audio, brightness and network helpers.
- `config/` — non-secret wpa_supplicant build/runtime defaults.
- `tools/` — small hardware diagnostics used during bring-up.
- `patches/` — source patches required for third-party components.
- `build-native-c-image.sh` — builds static programs and the init_boot image.
- `build-wifi-vendor-boot.sh` — injects matching signed modules and firmware
  into a stock vendor_boot image.
- `artifacts.example.manifest` — expected local artifact names.

Binary firmware, signed Google modules, stock boot images, per-device
calibration and credentials are intentionally not committed. Extract them from
the matching factory image/device into one local directory and point
`SAAIOS_PANTHER_ARTIFACTS` at it.

## Build prerequisites

- Zig with an `aarch64-linux-musl` target, or set `ZIG`.
- Magiskboot, provided through `MAGISKBOOT`.
- Stock `init_boot` and `vendor_boot` from `CP2A.260705.006`.
- TinyALSA checkout at `4e466e8f90e5c15c029a18461cc384c0f7777193`,
  with `patches/tinyalsa-period-write.patch` applied.
- The files listed in `artifacts.example.manifest`.

Example:

```sh
export SAAIOS_PANTHER_ARTIFACTS=/secure/local/panther-artifacts
export MAGISKBOOT=/opt/magiskboot
export STOCK_INIT_BOOT=/secure/local/init_boot.img
export STOCK_VENDOR_BOOT=/secure/local/vendor_boot.img
export TINYALSA_DIR=/src/tinyalsa

./os/targets/panther/build-wifi-vendor-boot.sh
./os/targets/panther/build-native-c-image.sh
```

Outputs default to `dist/panther/`, which is ignored by Git.

## Flashing

```text
fastboot flash init_boot_a dist/panther/saaios-panther-init_boot.img
fastboot flash vendor_boot_a dist/panther/saaios-panther-vendor_boot.img
fastboot --set-active=a
fastboot reboot
```

The device must have an unlocked bootloader. Verify the exact model and stock
firmware revision before flashing. Do not flash these images to any device
other than `panther`.

## Private runtime state

The following files are created only on the phone and must never be committed:

- `/metadata/saaios/wifi.conf`
- `/metadata/saaios/bluetooth.keys`
- `/metadata/saaios/bluetooth.linkkeys`
- `/metadata/saaios/bluetooth.devices`
- saved time, brightness and per-device calibration values

The detailed live bring-up record is in
`docs/os/targets/panther/README.md`.
