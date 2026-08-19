# SM-A127F/DSN — sources

Phase 0 used public material only. No phone was modified.

## Model / SoC

- Wikipedia, *Samsung Galaxy A12* — A12 = Helio P35 (`SM-A125*`); A12 Nacho = Exynos 850 (`SM-A127*`).
- Device specifications / IMEI.org listings for SM-A127F/DSN.

## On-device silicon samples

- Device Info HW: [item 99921](https://www.deviceinfohw.ru/devices/item.php?item=99921) (NVT-ts, sm5714, s2mpu12, Mali-G52).
- Device Info HW: [item 78967](https://www.deviceinfohw.ru/devices/item.php?item=78967) (synaptics_tcm, LCM `0xba6220`).

BOM differs. Do not freeze a single touch IC.

## Partitions / eMMC

- Martview: [A127F A12 Exynos 850 ISP pinout / GPT dump](https://www.martview-forum.com/threads/a127f-a12-exynos-850-isp-pin-out-frp-reset-and-dump-file-read.53562/) — GPT names and sizes; EasyJTAG log.

## Unlock, TWRP, GSI, kernels

- XDA [ROOT/TWRP/KERNEL/GSI SM-A127F (a12s)](https://xdaforums.com/t/root-twrp-kernel-gsi-for-sm-a127f-a12s.4352297/) — OEM unlock; DSN TWRP warning.
- XDA physwizz [TWRP A127](https://xdaforums.com/t/twrp-a127-physwizz.4537997/).
- XDA [GSI via super.img repack](https://xdaforums.com/t/installing-gsi-by-repacking-super-img-on-sm-a127f-and-sm-a325f-linux.4365511/) (uluruman; tested SM-A127F/DSN).
- XDA [boot.img touch/MTP from Samsung OSS](https://xdaforums.com/t/boot-img-for-sm-a127f-and-sm-a125f-with-the-touch-and-mtp-fixed.4536735/).
- XDA [Project-Xed kernel](https://xdaforums.com/t/kernel-a127f-project-xed-kernelsu-next-susfs.4735546/).
- Firmware index: [samfw.com/firmware/SM-A127F](https://samfw.com/firmware/SM-A127F).
- This unit's BL zip `BL_A127FXXSDDXJ2_…tar.md5.zip` — inner `up_param.bin.lz4` is a ustar of JPEGs (`svb_orange.jpg` = Power-key unlock warning).

## Kernel source

- [opensource.samsung.com](https://opensource.samsung.com/) search `SM-A127F` (region zips: RR/EUR/CIS; match binary).
- XDA: kernel source question for A127F/DSN Exynos 850.

## Mainline / pmOS

- [exynos850-mainline](https://github.com/exynos850-mainline) (A13-first).
- postmarketOS wiki title *Samsung Galaxy A12 Nacho (samsung-a12s)* — page exists; no mature official port as of this dossier.
- Public pmbootstrap attempt on A127F failed on missing aport `samsung-a12`.

## Device tree (Android)

- [MizProject/android-device-samsung_a12s](https://github.com/MizProject/android-device-samsung_a12s) — Lineage-oriented DT, not our OS.

## Gaps (fill from the unit)

- Exact `ro.bootloader` / CSC / U-number.
- PIT from Heimdall vs this GPT dump.
- `/sys/class/drm` vs fbdev on stock kernel.
- Touch IC on this board.
- UART presence.
