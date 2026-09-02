# SM-A127F/DSN — recovery

**Gate:** no custom image on the phone until (1) stock firmware for this **model + CSC + binary** is on disk with checksums, and (2) Download mode has been entered once successfully.

## This is not MediaTek

| Wrong | Right |
|-------|--------|
| BROM / preloader | Samsung **Download mode** |
| SP Flash Tool | **Odin** or **Heimdall** |
| `fastboot flashing unlock` | **OEM unlock** in Download mode (Vol+) |

ISP / test-point / EasyJTAG exists for dead eMMC (see Martview pinout thread). That is **board-level repair**, not a bring-up step.

## Everyday unbrick (software)

1. Download **exact** stock from [samfw.com/firmware/SM-A127F](https://samfw.com/firmware/SM-A127F) matching `A127FXXU<n>…` on the phone. Do not downgrade BL across a higher **U** (anti-rollback).
2. Enter Download mode: power off → hold **Vol+ and Vol−** → plug USB.
3. Flash **BL + AP + CP + CSC** (full wipe) with Odin, or Heimdall equivalents.
4. If it boots to stock recovery asking for factory reset — do it.

Linux: community `heimdall_flash_stock.sh` from uluruman’s super.img thread is the usual path; Odin remains the most battle-tested.

## Modes

| Mode | Keys (USB connected unless noted) |
|------|-----------------------------------|
| Download | Vol+ + Vol− + plug USB |
| Stock recovery | Vol+ + Power (USB in helps on a12s) |
| Soft reboot from download | Vol− + Power |

After a custom recovery flash, community requires **reboot to recovery, not system**, or stock recovery overwrites TWRP. We are **not** installing TWRP in Phase 0–1.

## What “restore” means for us

```text
images/stock/<build>/
  boot.img          # extracted from AP
  vbmeta.img
  recovery.img
  super.img         # huge; optional until SUPER is touched
  BL/AP/CP/CSC tars # full Odin set
  SHA256SUMS
```

Phase 1 rollback: flash **only** stock `boot.img` (+ stock `vbmeta.img` if we patched it).

PARAM nologo rollback: Odin **BL** = `saaios-up_param-stock-restore.tar` (only `up_param.bin.lz4`). Do not re-flash the full stock BL zip for a logo.

If that fails: full 4-file stock.

## TWRP / GSI (do not use as our OS)

They prove the bootloader can run non-stock `boot`/`recovery`/`super`:

- TWRP / OrangeFox: physwizz (a12s). Some reports: **a127f/DSN** flaky with older afaneh92 images — match binary U.
- GSI: Magisk-patched AP, or Linux **super.img repack** (uluruman) keeping VB on other partitions.
- Custom kernels from Samsung source: uluruman (touch/MTP), Project-Xed (KernelSU). Project-Xed is the **maze** currently in SaaiOS Images — not stock. Clean DXJ6 td4150 is `os/third_party/td4150_oss_dxj6/`. See [kernel-touch.md](kernel-touch.md).

Useful as **existence proofs**. Not our userspace.

## postmarketOS

Wiki page `Samsung Galaxy A12 Nacho (samsung-a12s)` exists; there is **no** solid official aport. One public attempt stalled on missing packages. We do not depend on pmOS.

## Dead device last resorts (human, optional)

1. Full stock Odin.
2. Different USB port / cable / another PC (Windows + official USB driver).
3. Do **not** flash a random BL from a higher/lower U.
4. ISP solder only if eMMC is unreachable — out of OS-track scope.

## Human checklist before Phase 1

- [ ] Photo of Download mode line: OEM LOCK, FRP, KG, binary
- [ ] Stock 4-file firmware downloaded, `sha256sum` stored
- [ ] `boot.img` + `vbmeta.img` extracted and copied off-phone
- [ ] `make restore` **not** run until those files exist
