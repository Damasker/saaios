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

- [opensource.samsung.com](https://opensource.samsung.com/uploadSearch?searchValue=SM-A127F) — four Android 13 SWA zips. Closest to this unit’s **A127FXXSDDXJ2** is **A127FXXSDDXJ6**. Host zip: `/srv/media/SM-A127F_SWA_13_Opensource.zip` (242026746 bytes). Clean td4150 extract: `os/third_party/td4150_oss_dxj6/` (17 source files, `CONFIG_LOCALVERSION=""`, no KernelSU). Plan: [kernel-touch.md](kernel-touch.md).
- Community **maze** currently built into SaaiOS Images: [maazm7d/kernel_samsung_a12](https://github.com/maazm7d/kernel_samsung_a12) — **a12s / Exynos 850 / S5E3830**, defconfig `exynos850-a12snsxx`, **not** Helio A125F. KernelSU/SUSFS fork (`-Project-xed-Ksun-4.c11`), **not** a clean XJ2 dump. Clone: `os/third_party/kernel_samsung_a12`. Driver: `drivers/input/touchscreen/synaptics/td4150/`. Opcode patches in `os/patches/syna-tcm-*.patch` are **history** — do not port the maze onto OSS. Canonical: [kernel-touch.md](kernel-touch.md).
- USB Wi‑Fi dongle (not this phone’s onboard combo): [aircrack-ng/rtl8188eus](https://github.com/aircrack-ng/rtl8188eus) — RTL8188EUS/EU/ETV, monitor/injection. Deprecated; prefer [lwfinger/rtw88](https://github.com/lwfinger/rtw88). Does **not** replace synaptics. Notes: [wifi-usb.md](wifi-usb.md).
- XDA: kernel source question for A127F/DSN Exynos 850.

## Mainline / pmOS

- [exynos850-mainline](https://github.com/exynos850-mainline) (A13-first).
- postmarketOS wiki title *Samsung Galaxy A12 Nacho (samsung-a12s)* — page exists; no mature official port as of this dossier.
- Public pmbootstrap attempt on A127F failed on missing aport `samsung-a12`.

## Device tree (Android)

- [MizProject/android-device-samsung_a12s](https://github.com/MizProject/android-device-samsung_a12s) — Lineage-oriented DT, not our OS.

## Gaps

Filled in [unit.md](unit.md) / [kernel-touch.md](kernel-touch.md): bootloader **A127FXXSDDXJ2**, CSC **XID** / OLE, binary **D**, fb0 (no DRM), touch IC **TD4150**.

Still missing (do not invent):

- Stock Android `dmesg` on **this** unit / same DTBO (probe → first touch).
- PIT from Heimdall vs the public GPT dump.
- UART presence.
