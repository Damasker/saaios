# SM-A127F/DSN — boot chain

This is a **Samsung Exynos** chain. It is not MediaTek `BootROM → preloader → LK`.

```text
BootROM (on-chip, immutable)
    ↓
eMMC boot partitions (ROM2/ROM3, 4 MiB each)
    ↓
sboot (Samsung BL, Download mode lives here)
    ↓
EL3 / TrustZone (TZSW, TZAR, LDFW, KEYSTORAGE)
    ↓
Android Verified Boot (vbmeta, vbmeta_system)
    ↓
boot.img  = kernel Image + ramdisk + DTB/DTBO refs
    ↓
Linux 4.19 vendor kernel (first target)
    ↓
init (Android today → our PID 1 later)
```

## Stages

### BootROM

On-die. Loads from eMMC **boot partition 1** (`ROM2`, extCSD `PARTITION_CONFIG 0x48` on a public ISP dump). We never replace this.

### sboot / Download mode

Samsung first-stage BL. It blits JPEGs from **UP_PARAM** (not from `boot.img`):

- **Unlock warning** (`svb_orange.jpg`): orange state, Power to continue. This is the “sboot logo” people usually mean. Lives in PARAM; a custom ramdisk cannot remove it.
- **Boot splash** (`logo.jpg`): SAMSUNG / Galaxy A12. sboot paints it, then DECON keeps it until the kernel/init owns `fb0`. That leftover is not the Power-key warning.

Entry (human):

- Phone off, **Vol+ and Vol−**, plug USB to a PC → cyan “Downloading… Do not turn off target” (`download.jpg` in UP_PARAM).
- Unlock confirmation is a **long Vol+** on that screen (wipes userdata).

Tools: **Odin** (Windows) or **Heimdall** (Linux). Not `fastboot oem unlock` as on Pixel. Not SP Flash Tool.

PARAM-only Odin tars (`up_param.bin.lz4` alone) use the **BL** slot. Never flash sboot/TZ to change a splash. See [partitions.md](partitions.md) and [known-risks.md](known-risks.md) §16.

### TrustZone / Knox

`TZSW`, `TZAR`, `LDFW`, `KEYSTORAGE`, `KEYDATA`. Flashing these is out of scope forever unless a documented unbrick requires a **stock** BL package. Custom OS must live **after** sboot, in `boot` / ramdisk / (later) a partition we own.

### AVB

`VBMETA` + `VBMETA_SYSTEM`. Custom `boot.img` without a matching disable/patch of vbmeta → **boot loop** or AVB error. Community GSI guides always pair custom images with **patched vbmeta**.

Knox / RMM “Prenormal” can **re-lock** flashing until the device is online and OEM unlock is greyed-but-on. See [known-risks.md](known-risks.md).

### boot.img

Android boot image (not a raw Image.gz for Download mode). Typical a12s contents:

| Piece | Notes |
|-------|--------|
| Kernel | `Image` from `arch/arm64/boot/` of Samsung source |
| Ramdisk | gzip cpio; Android `init` today |
| Header | v1/v2 — **confirm with `unpack_bootimg` on stock** |
| DTB | separate `DTB` partition (8 MiB) **and/or** appended — **N** |
| DTBO | `DTBO` partition 8 MiB |

`vendor_boot.img` is **not** in the public GPT dump (BOOT + RECOVERY + DTB + DTBO). Treat vendor_boot as absent until the stock AP tarball is unpacked on this unit.

### Kernel

Vendor **Linux 4.19.111** from [Samsung Open Source](https://opensource.samsung.com/) search `SM-A127F`. Packages are **split by CSC** (RR / EUR / CIS) and **binary** (`A127FXXU3…`, `U4`, `U5`, `U7`, later `SD`/`UD`). Build kernel **only** against the same `U` as the phone.

Mainline: [exynos850-mainline](https://github.com/exynos850-mainline) is aimed more at Galaxy A13 than a12s. **Out of scope until vendor kernel + our init console works.**

## What we will put in boot.img (Phase 1)

```text
our kernel (vendor 4.19 + tiny config delta)
        ↓
our ramdisk (BusyBox init, no zygote)
        ↓
console
```

sboot stays stock. super/system stays stock until we have restore. First experiment replaces **only** `BOOT` (and vbmeta if AVB demands it), never BL.

## Human capture (no write)

From stock, after USB debug:

```text
adb shell getprop ro.boot.flash.locked
adb shell getprop ro.boot.vbmeta.device_state
adb shell getprop ro.bootloader
```

From Download mode: photo of the text block (FRP, OEM lock, KG state, binary).
