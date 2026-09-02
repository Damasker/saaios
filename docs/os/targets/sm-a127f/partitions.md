# SM-A127F/DSN — partitions

Layout below is from a public **ISP/eMMC GPT dump** of an A127F Exynos 850 (Samsung eMMC `DP6DAB`, ~64 GB). Treat sizes as **reference**, not gospel. Re-dump PIT/GPT from *this* phone before any write.

Source: EasyJTAG GPT scan, Martview thread “A127F A12 [Exynos 850] ISP PIN OUT…” (2021). See [sources.md](sources.md).

## eMMC hardware

| Area | Size (that dump) |
|------|------------------|
| User (ROM1) | ~58–59 GiB |
| Boot1 / Boot2 (ROM2/ROM3) | 4 MiB each — **sboot lives here** |
| RPMB | 16 MiB |

Boot config: boot from ROM2 (boot partition 1).

## GPT (user area)

Do **not** flash: `EFS`, `SEC_EFS`, `CPEFS`, `TZ*`, `KEY*`, `RADIO` (unless stock CP package), or a **full BL** package (sboot + TZ). `UP_PARAM` is optional and PARAM-only — see below.

| Name | Size (dump) | Role | Touch? |
|------|-------------|------|--------|
| EFS / SEC_EFS / CPEFS | 20 / 20 / 8 MiB | IMEI, network, EFS | never |
| PARAM / UP_PARAM | 8 MiB | sboot JPEG splashes | optional; **PARAM-only** tar |
| LDFW / TZSW / TZAR / KEYSTORAGE | small | TZ | never |
| **BOOT** | **44 MiB** | kernel + ramdisk | Phase 1 |
| **RECOVERY** | **53 MiB** | recovery | only after stock copy |
| **DTB** | 8 MiB | device tree | if kernel needs it |
| **DTBO** | 8 MiB | DT overlay | if kernel needs it |
| RADIO | 50 MiB | modem | stock CP only |
| **VBMETA** | 512 KiB | AVB | likely with custom boot |
| VBMETA_SYSTEM | 512 KiB | AVB system | GSI later |
| METADATA | 32 MiB | FBE / metadata | careful |
| **SUPER** | **~5.29 GiB** | system + vendor + product + odm (dynamic) | not Phase 1 |
| PRISM / OPTICS | 880 / 24 MiB | Samsung overlays | ignore |
| CACHE | 200 MiB | cache | wipe ok |
| USERDATA | rest (~51 GiB) | data | OEM unlock already wipes |

There is **no** standalone `SYSTEM` partition. Android 11+ dynamic partitions: OS lives **inside SUPER**.

There is **no** `vendor_boot` in this GPT. Confirm when unpacking the unit’s AP tar.

## Stock firmware packages (Odin)

Typical 4-file set from SamFW / samfirmware for `SM-A127F`:

| Slot | Contains (this firmware) |
|------|--------------------------|
| **BL** | `sboot.bin.lz4`, **`up_param.bin.lz4`**, `ldfw.img.lz4`, `tzsw.img.lz4`, `keystorage.bin.lz4`, `tzar.img.lz4`, `vbmeta.img.lz4`. No `param.bin`. |
| AP | boot, recovery, super, dtbo, vbmeta, … |
| CP | radio/modem |
| CSC / HOME_CSC | cache, omr, pit; CSC wipes data |

Nologo / restore tars for the unlock warning go in the **BL** slot but contain **only** `up_param.bin.lz4`. Odin maps by inner filename. Putting that tar in AP is wrong. Putting the full stock BL zip in BL writes sboot/TZ — never for a logo change.

### UP_PARAM file list (stock A127FXXSDDXJ2)

`up_param.bin` is a ustar (839680 bytes; lz4 712735). 27 JPEGs:

| File | Size | Pixels | Role |
|------|------|--------|------|
| `svb_orange.jpg` | 66808 | 624×1200 | **Unlock warning** — orange, “PRESS POWER KEY TO CONTINUE” |
| `booting_warning.jpg` | 49917 | 624×292 | Unofficial-software banner (same family) |
| `logo.jpg` | 43161 | 720×1600 | SAMSUNG / Galaxy A12 / Knox boot splash (DECON leftover source) |
| `letter.jpg` | 21708 | 720×1600 | SAMSUNG wordmark splash |
| `download.jpg` | 31062 | 720×680 | Cyan “Downloading… Do not turn off target” — **keep** |
| `download_error.jpg` | 27659 | 486×514 | Flash error — **keep** |
| `warning.jpg` | 97841 | 720×1260 | Teal custom-OS Download-mode prompt — **keep** |
| `warning_svb.jpg` | 105845 | 720×1262 | Teal SVB/custom-OS prompt — **keep** |
| `device_unlock.jpg` | 131921 | 720×1252 | “Unlock bootloader?” — **keep** |
| `device_lock.jpg` | 66471 | 720×824 | lock-state art — **keep** |
| `broken_cable.jpg`, `secure_error.jpg`, `lpm.jpg`, `low_battery_alert.jpg`, `android_logo.jpg`, `setting_logo.jpg`, `SUD_0.jpg`…`SUD_10.jpg` | small | various | charging / SUD overlays / errors — **keep** |

Nologo blanks the first four with same-dimension black JPEGs. Host: `make -f os/Makefile up_param`. Flash: human, Odin BL. Rollback: `saaios-up_param-stock-restore.tar`.

**Restore = flash all four** matching the phone’s **binary U** (or HOME_CSC to try keeping data — not relied on).

## What we still need from this unit

Human, after USB debug, **read-only**:

```text
adb shell su -c "ls -l /dev/block/by-name"   # if rooted; else skip
# or from Download mode: heimdall print-pit
```

Save PIT as `targets/sm-a127f/artifacts/unit.pit` (not in git if huge; checksum in unit.md).

**This unit GPT `PARTNAME` (v031 sysfs, 2026-08-29):** `efs` p1, `sec_efs` p2, `cpefs` p4, `param` p6, `up_param` p13, `boot` p18, `radio` p22 (50 MiB, 259:14), `cp_debug` p36, `super` p31. Sizes match the table above. Ramdisk has **no** `/dev/block` nodes — list via `/sys/block/mmcblk0/mmcblk0p*/uevent` only. Never write EFS/RADIO.

## Policy

Phase 1 writes, if any: `BOOT` ± `VBMETA`.  
Rollback image: original `boot.img` + original `vbmeta.img` extracted from the **same** AP the phone runs.

Optional later: `UP_PARAM` via a tar that contains **only** `up_param.bin.lz4` (Odin BL slot). Rollback: stock `up_param.bin.lz4` from the same BL zip. Never sboot/TZ/EFS.
