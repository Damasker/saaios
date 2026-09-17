# Pixel 7 (panther) cellular

Tensor G2 (`gs201`) Shannon **s5300** over Samsung CPIF, PCIE link
(`cpif_probe: s5300: PCIE link created`). Stock Android boots the CP with
vendor `cbd` and then `rild`. SaaiOS native userspace does **not** start
either. The live map is below. A static `cp-boot` helper has had **seven**
OFFLINE boot attempts (2026-09-06). PCIE-legal BIN chunk stays **`0x7E8`**
(EXYNOS+SIT+payload = `SZ_2K`). First MAIN BIN **ACK `0xC12B`** is real
(no `CRASH_EXIT`). The waiter then got **3053** further `0xC12B` ACKs
and stalled at the same MAIN offset **`0x5e49c8`** (`3053 * 0x7E8`)
with **0 bytes** in RXQ. A 100 ms guard immediately before the exact third
legacy-ring wrap did not change the stop. State **BOOTING**. `rmnet` still
0. Not ONLINE.

Firmware base: `CP2A.260705.006`. Slot A = SaaiOS. Slot B = stock Android.

## Success criterion

CP stably **ONLINE** (not `CRASH_EXIT`) **and** (`rmnet*` rx/tx ≠ 0 **and/or**
IPv4 on `rmnet*`). Wi-Fi is not a substitute.

## Live map (read-only)

PID 1 creates `/dev/umts_*` from `/sys/class/cpif` and `/sys/class/misc`
(no Android `ueventd`) and forks `/saaios/modem-probe`. Console command is
the same binary.

Reports:

- `/run/saaios-modem.state` — one token for STATUS (`OFFLINE`, `ONLINE`, …)
- `/run/saaios-modem.txt` — full dump
- `/data/saaios/var/modem-probe.txt` — same dump on userdata

The probe does **not** boot the CP, start `cbd`/`rild`, mount EFS, or issue
`IOCTL_POWER_OFF`.

## CPIF modules

Hypothesis confirmed: Pixel CPIF does **not** autoload. The signed Google
modules already sit in the running `vendor_boot` ramdisk at `/lib/modules`.
Android first-stage init loaded them from `modules.load`; native PID 1 did
not. `vendor_boot` was **not** rebuilt or reflashed.

On-device (SaaiOS `vendor_boot_a`, `/lib/modules`):

| File | In `modules.load` | Notes |
|---|---|---|
| `shm_ipc.ko` | yes (after `spi-s3c64xx.ko`) | softdep `pre: spi-s3c64xx` |
| `cpif_page.ko` | yes | |
| `cpif.ko` | yes | deps include `pcie-exynos-gs`, `google_modemctl`, `shm_ipc` |
| `cp_thermal_zone.ko` | yes | |
| `boot_device_spi.ko` | no | **absent** from this ramdisk |
| `exynos_dit.ko` | no | **absent** from this ramdisk |

`modules.load` order used on the live phone and in `native-init.c`:
`shm_ipc` → `cpif_page` → `cpif` → `cp_thermal_zone`. PID 1 loads them after
touch (`spi-s3c64xx`), audio (`google_modemctl`), and Wi-Fi (`pcie-exynos-gs`).

DT node `/sys/firmware/devicetree/base/cpif` is `samsung,exynos-cp` and exists
even before the driver binds. `do_cp_crash` is write-only — **do not write it**.

`umts_*` / `oem_*` char devices register under **`/sys/class/cpif`** (major
493), not `/sys/class/misc`. Only `umts_toe0` and `logbuffer_cpif` are misc
(10:106 / 10:107).

## First live dump — boot, no CPIF load (2026-09-05)

`/run/saaios-modem.state` = `NO_CPIF`. `init_boot_a` had the probe but PID 1
did not insmod CPIF.

```
SaaiOS panther modem-probe (read-only)
goal: CP ONLINE and (rmnet rx/tx != 0 or IPv4 on rmnet*)
forbidden: efs writes, IOCTL_POWER_OFF, cbd, rild, dd modem

modem_state: not found
modules:
  google_modemctl 16384 1 aoc_alsa_dev_util, Live 0xffffffd712859000 (O)
dir /sys/class/misc:
  (none)
dir /sys/class/misc:
  (none)
dir /dev:
  (none)
dir /dev:
  (none)
node /dev/logbuffer_cpif: missing (No such file or directory)
node /dev/umts_boot0: missing (No such file or directory)
node /dev/umts_ipc0: missing (No such file or directory)
node /dev/umts_rfs0: missing (No such file or directory)
net:
  (no rmnet/umts/vnet interfaces)
gpt names:
  sda7 modem_userdata
  sda29 modem_b
  sda19 modem_a
  sda5 efs
  sda6 efs_backup
ds_detect /sys/devices/platform/cpif/sim/ds_detect: missing (No such file or directory)

summary_state=NO_CPIF
```

## Second live dump — after insmod of signed vendor_boot modules (2026-09-05)

Loaded on the running kernel (no reboot, no flash, no `cbd`):

```
insmod /lib/modules/shm_ipc.ko
insmod /lib/modules/cpif_page.ko
insmod /lib/modules/cpif.ko
insmod /lib/modules/cp_thermal_zone.ko
```

Then `mknod` from `/sys/class/cpif/*/dev` and `/sys/class/misc/{logbuffer_cpif,umts_toe0}/dev`.
`cpif_probe` created `umts_boot0` / `umts_ipc0` / `umts_rfs0` and `rmnet0`–`rmnet29`.
`modem_state` = **OFFLINE**. `ds_detect` = **2**.

```
SaaiOS panther modem-probe (read-only)
goal: CP ONLINE and (rmnet rx/tx != 0 or IPv4 on rmnet*)
forbidden: efs writes, IOCTL_POWER_OFF, cbd, rild, dd modem

sysfs /sys/devices/platform/cpif/modem_state: OFFLINE
modules:
  cp_thermal_zone 20480 0 - Live 0xffffffd712f35000 (O)
  cpif 311296 0 - Live 0xffffffd712ee8000 (O)
  cpif_page 20480 0 - Live 0xffffffd712ee2000 (O)
  shm_ipc 28672 1 cpif, Live 0xffffffd712eda000 (O)
  google_modemctl 16384 2 cpif,aoc_alsa_dev_util, Live 0xffffffd712859000 (O)
dir /sys/class/misc:
  umts_toe0
dir /sys/class/misc:
  (none)
dir /dev:
  umts_toe0
  umts_wfc1
  umts_wfc0
  umts_router
  umts_rfs0
  umts_rcs1
  umts_rcs0
  umts_loopback
  umts_ipc1
  umts_ipc0
  umts_dm0
  umts_boot0
dir /dev:
  (none)
node /dev/logbuffer_cpif: empty
node /dev/umts_boot0: empty
node /dev/umts_ipc0: empty
node /dev/umts_rfs0: empty
net:
  rmnet0..rmnet29 oper=down flags=0x1090 rx=0 tx=0
  umts_dummy oper=down flags=0x1090 rx=0 tx=0
gpt names:
  sda7 modem_userdata
  sda29 modem_b
  sda19 modem_a
  sda5 efs
  sda6 efs_backup
ds_detect /sys/devices/platform/cpif/sim/ds_detect: 2

summary_state=OFFLINE
```

`/sys/class/cpif` also has `oem_ipc0`–`oem_ipc7`, `oem_test`, `multipdp`,
`umts_dm0`, `umts_router`, `umts_loopback`, `umts_rcs0`/`1`, `umts_wfc0`/`1`.
Do **not** write `do_cp_crash`.

`init_boot_a` was then rebuilt so PID 1 loads this stack after Wi-Fi. After
that reboot, `/run/boot.log` has `shm_ipc`/`cpif_page`/`cpif` ready, `radio
cpif nodes created: 21`, `radio misc nodes created: 2`, and the probe dump
is the same **OFFLINE** picture (`umts_boot0` present, `ds_detect=2`).
`vendor_boot` was not flashed. Slot B was not touched.

`native-init.c` `setup_cpif()` runs after Wi-Fi (`shm_ipc` → `cpif_page`
→ `cpif` → `cp_thermal_zone`, then `mknod`). The image that was running
through the sixth load did **not** contain it (fresh sysrq boots were
`NO_CPIF` until hand-`insmod`). `init_boot_a` only was packed from the
current `native-init` and flashed (2026-09-06). **Not** `vendor_boot`,
**not** slot B. Live `/run/boot.log` after that reboot:

```
saaios-init: module shm_ipc.ko ready
saaios-init: module cpif_page.ko ready
saaios-init: module cpif.ko ready
saaios-init: module cp_thermal_zone.ko ready
saaios-init: radio cpif nodes created: 21
saaios-init: radio misc nodes created: 2
```

`modem_state=OFFLINE`, `umts_boot0` present. PID 1 CPIF load is
persistent again. No further flash is required for the load path.

## Android partitions (do not mount originals RW)

Stock `fstab.modem` (gs201):

| GPT name | Android mount | FS | Role |
|---|---|---|---|
| `efs` | `/mnt/vendor/efs` | f2fs | live NV — **never write original** |
| `efs_backup` | `/mnt/vendor/efs_backup` | f2fs | backup NV — same rule |
| `modem_userdata` | `/mnt/vendor/modem_userdata` | f2fs | CP scratch |
| `modem` (slot) | `/mnt/vendor/modem_img` | ext4 **ro** | CP image; `cbd -P by-name/modem[_a]` |

NV for any later loader must come from a **userdata copy**, never from the
live `efs` / `efs_backup` originals. Do not `dd` or format `modem`.

## `modem_a` image (sda19, slot A, 2026-09-06, read-only)

200 MiB GPT `modem_a` (`/sys/block/sda/sda19`, major:minor **259:3**).
ext4. Mounted once as `mount -t ext4 -o ro,noload /dev/block/sda19
/mnt/modem_img` (no by-name nodes on SaaiOS). **Umounted immediately**
after listing. Partition was not `dd`'d into git. `cbd` was not started.

Filesystem (Android would mount this at `/mnt/vendor/modem_img`):

```
/mnt/modem_img/
  images/
    g5300q-251202-260127-B-14784800/
      modem.bin          98265168
      pw_token_db            419
      pw_token_db.csv        445
      confpack/
        cfg.db           1409024
        cfg.sha2              35
        build.info           309
        release-label         35
        confseqs/           2046 carrier seq files (names only; not dumped)
        manifests/
        *_symbolic_link_mapping
      uecap/               carrier UE-capability .binarypb files
  lost+found/
```

`g5300q-251202-260127-B-14784800` is the live CP2A.260705.006 Shannon
**g5300q** build (matches kernel `s5300` / `create_modemctl_device: s5300
is created!!!`).

### Shannon TOC inside `modem.bin`

32-byte records: `name[12] b_off m_off size crc idx` (LE). First 512 bytes:

| name | b_off | m_off | size | in modem.bin? |
|---|---|---|---|---|
| `TOC` | 0 | `0x410` | 0 | header (`idx=7`) |
| `BOOT` | `0x410` | 0 | `0x16800` (90 KiB) | yes |
| `MAIN` | `0x16c10` | `0x40010000` | `0x05917acc` (93 256 396) | yes |
| `VSS` | `0x0592e6dc` | `0x4f900000` | `0x0047cc0c` (4 706 316) | yes |
| `APM` | `0x05dab2e8` | `0x0202e000` | `0xb498` (46 232) | yes |
| `NV_NORM` | **0** | `0x4d600000` | `0x80000` (512 KiB) | **no — EFS** |
| `NV_PROT` | **0** | `0x4d680000` | `0x80000` (512 KiB) | **no — EFS** |
| `REPLAY` | **0** | `0x5a400000` | `0x80000` (512 KiB) | **no — scratch** |
| `INFO` | `0x05db6780` | `0x40009000` | `0xd0` (208) | yes |

`b_off=0` on `NV_NORM` / `NV_PROT` / `REPLAY` means those bytes are **not**
on the modem image. Newer Pixel TOC vs A12: `APM` + `NV_NORM`/`NV_PROT`
instead of a single `NV` placeholder.

## Stock `cbd` argv (gs201 / panther)

Not in AOSP `init.gs201.rc`. Lives in proprietary `vendor/etc/init/cbd.rc`.
Same service line on panther dumps from TD1A (Android 13) through AP3A
(Android 15) ([dumps.tadiphone.dev panther AP3A.241005.015](https://dumps.tadiphone.dev/dumps/google/panther/-/raw/panther-user-15-AP3A.241005.015-12366759-release-keys/vendor/etc/init/cbd.rc)):

```
service cpboot-daemon /vendor/bin/cbd -d -t ${ro.vendor.cbd.modem_type} -P by-name/${vendor.cbd.partition} -s 2
```

Expanded on this device:

```
/vendor/bin/cbd -d -t s5100sit -P by-name/modem_a -s 2
```

Facts behind the expansion:

- `ro.vendor.cbd.modem_type=s5100sit` (gs201 `device.mk` and vendor
  `build.prop`). Kernel still names the modem **s5300**; SIT type string
  is `s5100sit`.
- `on fs`: `setprop vendor.cbd.partition modem`
- `on property:ro.boot.slot_suffix=*`: `setprop vendor.cbd.partition
  modem${ro.boot.slot_suffix}` → slot A = `modem_a`
- `-P` is a **by-name fragment**, not `radio` and not `/mnt/vendor/modem_img`
- **no** `-b` / `-m` link flags. CBD v2 + `CBD_PROTOCOL_SIT` + `s5100sit`
  imply PCIE (confirmed live: `cpif_probe: s5300: PCIE link created`)
- `-s 2` matches live `ds_detect=2` (dual SIM)
- `fstab.modem` still mounts `by-name/modem` (slotselect) **ro ext4** at
  `/mnt/vendor/modem_img`. `cbd` then reads
  `/mnt/vendor/modem_img/images/<build>/modem.bin`

`-P` must stay `by-name/modem` or `by-name/modem_a`. Do **not** point it
at `radio`.

## NV files `cbd` expects vs userdata-copy plan

`cbd` logs `CP NV file = /mnt/vendor/efs/nv_normal.bin` (Pixel Tensor
crash logs). Live Pixel EFS also carries `nv_protected.bin` plus `.md5`
siblings. TOC `NV_NORM` / `NV_PROT` (`b_off=0`, 512 KiB each) are those
EFS files, not blobs inside `modem.bin`.

| Path `cbd` / RFS opens | Source | SaaiOS rule |
|---|---|---|
| `/mnt/vendor/efs/nv_normal.bin` | live `efs` (`sda5`) | **never** the original |
| `/mnt/vendor/efs/nv_protected.bin` | live `efs` (`sda5`) | **never** the original |
| `*.md5` next to those | live `efs` | copy with the bins |
| `/mnt/vendor/efs_backup/…` | `sda6` | **never** the original |
| `/mnt/vendor/modem_userdata/replay` | `sda7` scratch | optional later; not efs |

The userdata copy is on the phone (below). Do not bind-mount it onto a
live original efs path. Do not start vendor `cbd`.

Originals stay unmounted.

## `boot_device_spi.ko` — not required on panther

- **Absent** from this `vendor_boot` `/lib/modules` (no file, not in
  `modules.load`, not in `modules.dep`)
- DT leftover exists: `/sys/firmware/devicetree/base/spi@10D20000/cpboot_spi@0`
  (`compatible`, `reg`, `spi-max-frequency`)
- Live `cpif_probe` already did `MODEM:s5300 LINK:PCIE` and `s5300: PCIE
  link created` after `insmod cpif.ko`

SPI boot is unused on this PCIE Shannon. Do not hunt an unsigned rebuild.

## Forbidden (still)

- writes to live `efs` / `efs_backup` / `cpefs` / RADIO-class originals
- `IOCTL_POWER_OFF` on `umts_boot0`
- auto-start of vendor `cbd` or `rild`
- flashing anything except `panther`; slot B stays the Android escape hatch

## NV copy LIVE (2026-09-06, one-shot ro)

`sda5` efs is `8:5`. This kernel's F2FS **rejects** `noload`
(`Unrecognized mount option "noload"`). Mounted once as
`mount -t f2fs -o ro,norecovery /dev/block/sda5 /mnt/efs-ro`.
`/proc/mounts` showed `f2fs ro,...norecovery...`. **Never rw.** Copied
four files to `/data/saaios/var/efs-copy/` mode **0600**, then
**umounted immediately**. `/proc/mounts` has no efs / sda5.

Names and byte sizes only (no contents, no IMEI):

| Name | Bytes |
|---|---|
| `nv_normal.bin` | 524288 |
| `nv_protected.bin` | 524288 |
| `nv_normal.bin.md5` | 32 |
| `nv_protected.bin.md5` | 32 |

Also present on the original (not copied): `nv_normal.bin.tmp` 476784,
`nv_protected.bin.tmp` 189452. No other `nv_*.bin`. `cbd`/`rild` not
started. Original efs left unmounted.

## Protocol

Live DT `/sys/firmware/devicetree/base/cpif/mif,protocol` = `00 00 00 01`
→ **PROTOCOL_SIT (1)**, not A12 SIPC (0). Compatible `samsung,exynos-cp`.

## Ioctl numbers (confirmed live)

Source: LineageOS `android-16` `radio/samsung/s5300/modem_prj.h` (same
tree as live module path
`../../private/google-modules/radio/samsung/s5300/`). Live
`/lib/modules/cpif.ko` strings match those log names
(`IOCTL_POWER_ON`, `IOCTL_START_CP_BOOTLOADER`, `IOCTL_COMPLETE_NORMAL_BOOTUP`,
`IOCTL_REQ_SECURITY`, `IOCTL_GET_CPIF_VERSION`, plus
`load_cp_image is null` for `IOCTL_LOAD_CP_IMAGE` which is `mif_debug`).

Factory `vendor/bin/cbd` is now on the host at
`dist/panther/cbd-extract/raw-cbd` (AP3A tadiphone dump, Android 35 PIE,
interp `/system/bin/linker64`). **Strings and disassembly only — never
executed.** Ioctl numbers were **not** guessed: live
`IOCTL_GET_CPIF_VERSION` is **OK** `CPIF-20220408R1`.

| Name | Macro |
|---|---|
| `IOCTL_POWER_ON` | `_IO('o', 0x19)` |
| `IOCTL_POWER_OFF` | `_IO('o', 0x20)` — **never issue** |
| `IOCTL_POWER_RESET` | `_IOW('o', 0x21, struct boot_mode)` |
| `IOCTL_START_CP_BOOTLOADER` | `_IOW('o', 0x22, struct boot_mode)` |
| `IOCTL_COMPLETE_NORMAL_BOOTUP` | `_IO('o', 0x23)` |
| `IOCTL_GET_CP_STATUS` | `_IO('o', 0x27)` |
| `IOCTL_LOAD_CP_IMAGE` | `_IOW('o', 0x40, struct cp_image)` |
| `IOCTL_REQ_SECURITY` | `_IOW('o', 0x53, struct modem_sec_req)` |
| `IOCTL_GET_CPIF_VERSION` | `_IOR('o', 0x56, struct cpif_version)` |

PCIE `link_load_cp_image`: copies into a small staging buffer and sets
`boot_img_size = img.size` **before** the range check. MAIN (~93 MiB)
does not fit. Failed later loads still clobber `boot_img_size`.

## First `cp-boot` attempt LIVE (2026-09-06, was OFFLINE)

Static `/data/saaios/cp-boot` (`os/targets/panther/src/cp-boot.c`, Zig
musl). `modem_a` mounted **ro** long enough to read `modem.bin`, then
umounted. NV from userdata copy only. Original efs **unmounted**. No
`cbd` / `rild`. **No** `IOCTL_POWER_OFF`.

| Step | Result |
|---|---|
| `GET_CPIF_VERSION` | **OK** `CPIF-20220408R1` |
| `POWER_ON` / `POWER_RESET` | **OK** |
| `REQ_SECURITY` | **EINVAL** — dmesg `security_req is null` (PCIE) |
| `LOAD_CP_IMAGE` BOOT | **OK** size `0x16800` m_off 0 |
| `LOAD_CP_IMAGE` MAIN/VSS/APM/NV | **EINVAL** — dmesg `PCIE: ERR! Invalid args` |
| `START_CP_BOOTLOADER` | ran; `OFFLINE → BOOTING`; then `check_cp_status` **EFAULT** (`boot_stage` stuck `0x1FF`) |
| SIT UDL `0xA110` | no RX (`NO data in RXQ`) |
| `COMPLETE_NORMAL_BOOTUP` | **EAGAIN** / `T-I-M-E-O-U-T` |

`set_cp_rom_boot_img` used `size:0x80000` (NV size) because the failed
NV `LOAD_CP_IMAGE` overwrote `boot_img_size` after BOOT succeeded.
`debug_cp_rom_boot_img`: `boot_stage:0x1FF err_report:0x400`.

After: **`modem_state=BOOTING`**, `GET_CP_STATUS=3`. Holder **650**
still has `umts_boot0` / `umts_ipc0` / `umts_rfs0` open. `rmnet0`–`3`
down, rx=tx=**0**. Not `CRASH_EXIT`. Not `ONLINE`.

Helper was then fixed to **LOAD BOOT only** via ioctl (do not clobber
`boot_img_size`). That leftover `BOOTING` was cleared by AP sysrq
reboot; the BOOT-only + SIT UDL run is the second attempt below.

`cp-boot` is compiled by `build-native-c-image.sh` as `/saaios/cp-boot`.
**Not** started from PID 1.

## How SIT UDL stages were identified

Not guessed. Factory panther `cbd` (AP3A, 149600 bytes) plus
`oberdfr/google-modules_radio_samsung_s5300` (`bootdump_io_device.c`,
`exynos_ipc.h`). `cbd` was **not** run.

Strings: `std_udl_stage_start`, `std_udl_stage_done`, `std_dl_send_bin`,
`std_dl_send_crc`, `std_udl_req_resp`, `std_boot_dload`,
`std_boot_finish_handshake`, `[stage %d] START fail (req:0x%X exp:0x%X)`,
`cmd = 0x%x, crc = 0x%x`.

Disassembly (`dist/panther/cbd-extract/cbd.dis`):

| Site | What |
|---|---|
| `eb70` | `std_udl_req_resp(fd, req, exp)`: write 4-byte LE if `req≠0`, `poll(POLLIN, 2000ms)`, read 4 bytes, compare |
| `f270` | START: `req = 0xA100\|((idx<<4)&0xFFF0)`, `exp = 0xC100\|…`; `idx==0xf` uses `0xA400/0xC400` |
| `f328` / `f5cc` | BIN cmd `0xA10B\|bits`, wait `0xC10B\|bits` (read-only, no 4-byte write) |
| `f55c`–`f568` | 12-byte header at `sp+0x54`: `u16 cmd`, `u16 chunk`, then `stp total, offset`; `1fec0` rewrites `len = chunk+8` and writes `chunk+12` |
| `f700` | CRC: 8 bytes `{0xA301\|bits, toc.crc}`, wait `0xC300\|bits` |
| `ee00` | DONE `0xA10D/0xC10D` (or READY `0xA00B/0xC00B` if a flag is 0), then FIN `0xA400/0xC400` |
| `f2f8` | default block `0xC000` (alt `0x7D00` on a flag) |

Pixel Tensor logs (`std_udl_stage_start` `0xA110`/`0xC110`) are **stage
idx=1** (`BOOT`). This `modem.bin` TOC uses **BOOT idx=1**, **MAIN
idx=2**, so MAIN START is `0xA120`/`0xC120`. Confirmed live.

### BIN frame (cbd `0x1fec0`, LE, packed)

Not invented. `std_dl_send_bin` at `f55c`–`f588` builds a 12-byte
header at `sp+0x54`, then `1fec0` copies payload and `write()`s once.

| Offset | Field | Endian | First MAIN chunk (idx=2) |
|---|---|---|---|
| +0 | `u16 cmd` | LE | `0xA12B` (`0xA10B \| (2<<4)`) |
| +2 | `u16 len` | LE | **`chunk+8`** after rewrite (`0xC008` for `0xC000`) |
| +4 | `u32 total` | LE | TOC size (`0x05917ACC`) |
| +8 | `u32 offset` | LE | byte offset in stage (`0`) |
| +12 | payload | raw | `chunk` bytes from `modem.bin` |

`len` includes the two `u32`s + payload (`chunk+8`). It does **not**
include the first 4 bytes (`cmd`+`len`). `1fec0` writes **`chunk+12`**
bytes in **one** `write()` and fails if the return is not exact.
Default chunk is **`0xC000`** (`f2f8`; alt `0x7D00` only after a prior
write failure). CRC is **not** in this header: later 8-byte
`{0xA301\|bits, toc.crc}` at `f700`.

Live first-BIN bytes this turn (userspace, before kernel wrap):

`2b a1 08 c0 cc 7a 91 05 00 00 00 00` then `0xC000` of MAIN.

### Kernel wrap (`bootdump_write`, s5300 `PROTOCOL_SIT`)

Live DT `cpif/iodevs/io_device_8` = `umts_boot0`:
`iod,attrs=0x00000200` (`ATTR_NO_CHECK_MAXQ` bit 9).
`ATTR_NO_LINK_HEADER` is bit 8 (`0x100`) and is **not** set, so
`iod->link_header = true`. `mif,protocol=1` (SIT). Channel `0xF1`,
format `IPC_BOOT`.

`bootdump_write` therefore prepends a 12-byte `exynos_link_header`
(`EXYNOS_HEADER_SIZE`) via `exynos_build_header` before the PCIE
send. RX strips the same header (`skb_pull` `EXYNOS_HEADER_SIZE`)
before userspace `read()`. **Userspace must not add EXYNOS** — `cbd`
`1fec0` `write()`s the SIT frame only. START (`0xA120`, 4 raw bytes)
was also wrapped this way and still ACKed `0xC120`, so the wrap is
not unique to BIN.

A12 lesson still holds for ipc/rfs SIPC5; boot0 is EXYNOS-on-SIT, not
SIPC5. Do not duplicate the wrap.

## Second `cp-boot` attempt LIVE (2026-09-06, fresh OFFLINE)

Leftover first-attempt `BOOTING` + holder 650: AP reboot via
`echo b > /proc/sysrq-trigger` (plain `reboot` is a no-op on this PID 1).
After ~29s: `modem_state=OFFLINE`, `/dev/umts_boot0` present, efs
**unmounted**, NV copy still `0600` on userdata, no holder.

Helper rebuilt (Zig musl static, 65552 bytes) to
`/data/saaios/cp-boot`. **Not** packed into PID 1. One `load` from
OFFLINE (pid 334). `modem_a` mounted **ro** only to read `modem.bin`,
then umounted. NV from userdata copy only. ipc0/rfs0 opened before
START and never closed.

### Sequence used

1. Refuse if efs mounted or state is BOOTING / ONLINE / CRASH_EXIT
2. Open `umts_boot0` O_RDWR (then O_NONBLOCK); open `umts_ipc0` +
   `umts_rfs0` O_RDWR\|O_NONBLOCK — **never close**
3. `POWER_ON`, `POWER_RESET` (NORMAL). Skip `REQ_SECURITY` (PCIE
   `security_req` is null)
4. `LOAD_CP_IMAGE` **BOOT only** (`0x16800`) — do not clobber
   `boot_img_size`
5. `START_CP_BOOTLOADER` → `OFFLINE` → `BOOTING`
6. UDL per TOC `idx` (chunk `0xC000`):

| Stage | idx | start / ACK | source |
|---|---|---|---|
| MAIN | 2 | `0xA120` / `0xC120` | modem.bin |
| VSS | 3 | `0xA130` / `0xC130` | modem.bin |
| APM | 4 | `0xA140` / `0xC140` | modem.bin |
| INFO | — | skipped | missing usable `b_off` on this pass |
| NV_NORM | 5 | `0xA150` / `0xC150` | userdata copy |
| NV_PROT | 6 | `0xA160` / `0xC160` | userdata copy |
| READY / FIN | — | `0xA00B`/`0xC00B` then `0xA400`/`0xC400` | handshake |

BIN frame written: `{u16 cmd=0xA10B\|(idx<<4), u16 len=chunk+8, u32
total, u32 offset}` + data. Then wait `0xC10B\|(idx<<4)`.

7. `COMPLETE_NORMAL_BOOTUP`
8. Holder fork keeps the three fds open

### Result

| Step | Result |
|---|---|
| `POWER_ON` / `POWER_RESET` | **OK** |
| `LOAD_CP_IMAGE` BOOT | **OK** |
| `START_CP_BOOTLOADER` | **OK**; `OFFLINE → BOOTING`; dmesg `boot_stage` `0xFF` then **`0x3FFF`** (was `0x1FF` when MAIN went through ioctl) |
| UDL MAIN START | **ACK `0xC120`** — protocol is real |
| UDL MAIN first BIN (`0xC000` @ off 0) | write OK; wait **`0xC12B` timed out** |
| UDL VSS / APM / NV START | all **timeout** (CP stuck after bad/unacked BIN) |
| UDL READY / FIN | timeout |
| `COMPLETE_NORMAL_BOOTUP` | **EAGAIN** / dmesg `T-I-M-E-O-U-T` |
| after | **`modem_state=BOOTING`**, `GET_CP_STATUS=3`, holder **370** |
| `rmnet0` | **rx=0 tx=0** (ifaces `rmnet0`–`29` exist, down) |
| crash | **not** `CRASH_EXIT` |

AP reboot via sysrq after the COMPLETE timeout. Fresh boot:
`modem_state=OFFLINE`, `umts_boot0` present, efs unmounted, NV copy
intact, no holder. **No** `IOCTL_POWER_OFF`. **No** `cbd` / `rild`.

## Third `cp-boot` attempt LIVE (2026-09-06, fresh OFFLINE)

Helper rebuilt (Zig musl static, 66064 bytes) so BIN write matches
`cbd` `1fec0` exactly: one `write()` of `chunk+12`, `len=chunk+8`, no
userspace EXYNOS, abort on first BIN fail (no VSS/COMPLETE). Pushed to
`/data/saaios/cp-boot`. One `load` from OFFLINE (pid 498). `modem_a`
ro then umount. NV userdata copy only. **No** `IOCTL_POWER_OFF`.

| Step | Result |
|---|---|
| `POWER_ON` / `POWER_RESET` | **OK** |
| `LOAD_CP_IMAGE` BOOT | **OK** |
| `START_CP_BOOTLOADER` | **OK**; `OFFLINE → BOOTING`; `boot_stage` `0xFF` then **`0x3FFF`** |
| UDL MAIN START | **ACK `0xC120`** |
| UDL MAIN first BIN | write `0xC00C`; hdr `2b a1 08 c0 cc 7a 91 05 00 00 00 00`; wait **`0xC12B` no ACK** |
| after first BIN | **`modem_state=CRASH_EXIT`** (was BOOTING on the second attempt) |
| `rmnet0` | **rx=0 tx=0** |
| COMPLETE / VSS / NV | **not issued** (aborted) |

AP reboot via `echo b > /proc/sysrq-trigger`. After ~100s uptime:
`modem_state=OFFLINE`, no holder, efs unmounted, `rmnet0` rx=tx=0.
**No** `IOCTL_POWER_OFF`. **No** `cbd` / `rild`. Do not write
`do_cp_crash`.

## `cbd` PCIE download (AP3A `raw-cbd`, not executed)

`std_dl_send_bin` / `1fec0` is the same for `s5100sit`. There is **no**
smaller PCIE default:

| Site | Fact |
|---|---|
| `f2f8` / `f2fc` | `csel` chunk = `0x7D00` if bss flag ≠ 0, else **`0xC000`** |
| `f6a4` | **only** store of that flag: `strb 1` after a `write()` failure |
| `f2bc` | memset `0xC00C` (header+payload buffer) |
| `1fec0` | `memcpy` payload to +12, `len = chunk+8`, **one** `__write_chk(chunk+12)` |
| `eb70` | START is a 4-byte `write` on the **same fd** |
| `f770` | CRC is a later 8-byte `write`, not ioctl |

`0x1000` sites (`15ca8`, `15f2c`, …) are not on this BIN path.

## Kernel wrap / max TX (live DT + s5300 `bootdump_write`)

Live `io_device_8` = `umts_boot0`: `iod,attrs=0x200`, `format=4`
(IPC_BOOT), `ch=0xF1`, **no** `iod,max_tx_size` / `ul_buffer_size`.
`bootdump_write` therefore does **not** split a userspace write
(`if (iod->max_tx_size) alloc_size = min(..., max_tx_size)`).

`exynos_build_header`: `len = EXYNOS_HEADER_SIZE + payload` (12 +
`tx_bytes`). `exynos_build_fr_config`: **`format >= IPC_BOOT` always
`EXYNOS_SINGLE`**. `ipc_write` SIT caps at `SZ_2K` and updates
multi-frame; **bootdump does not**. Therefore the stock `0xC00C`
userspace write is one EXYNOS SINGLE frame of length `0xC018`; the
earlier inference that this was illegal merely because it exceeds 2 KiB
was wrong. The 2 KiB cap belongs to `ipc_write`, not the boot path.

Boot/dump `ld->send` is `xmit_to_legacy_link` → NORM_RAW circ queue.
Live `legacy_raw_txq_size` = **`0x1FD000`** (~2 MiB), so a `0xC018`
skb fits the queue. Live `pktproc_ul_max_packet_size` = **`0x800`**
(2048), but boot traffic bypasses pktproc and that value does not limit
an EXYNOS boot frame. START (4 bytes + 12 EXYNOS) is also SINGLE and ACKs.

## Fourth `cp-boot` attempt LIVE (2026-09-06, fresh OFFLINE)

Helper rebuilt (Zig musl static, 66120 bytes): same 12-byte `cbd`
header (`len = chunk+8`, one `write()`, no userspace EXYNOS), but
`SIT_CHUNK = 0x7E8` so EXYNOS(12)+SIT hdr(12)+payload = `SZ_2K`.
Pushed to `/data/saaios/cp-boot`. One `load` from OFFLINE (pid 697).
`modem_a` ro then umount. NV userdata copy only. **No**
`IOCTL_POWER_OFF`. Signed CPIF modules were re-`insmod` first (they
were not loaded when this turn began); `umts_*` re-`mknod` from
sysfs. **No** `cbd` / `rild`.

| Step | Result |
|---|---|
| `POWER_ON` / `POWER_RESET` | **OK** |
| `LOAD_CP_IMAGE` BOOT | **OK** `0x16800` |
| `START_CP_BOOTLOADER` | **OK**; `OFFLINE → BOOTING`; `boot_stage` `0xFF` then **`0x3FFF`** |
| UDL MAIN START | **ACK `0xC120`** |
| UDL MAIN first BIN | write `0x7F4`; hdr `2b a1 f0 07 cc 7a 91 05 00 00 00 00` (`cmd=0xA12B` `len=0x7F0` total `0x05917ACC` off 0); **ACK `0xC12B`** |
| later `0xC12B` | **timeout** (dmesg `bootdump_read: NO data in RXQ`); fail line printed off `0x5e49c8` |
| after | **`modem_state=BOOTING`** (not `CRASH_EXIT`) |
| `rmnet0` | **rx=0 tx=0** |
| COMPLETE / VSS / NV | **not issued** |

Did **not** try a second chunk size on this boot. Log:
`/data/saaios/var/cp-boot-20260906-sz2k.log`.

## Why later `0xC12B` stalled (cbd + kernel + live log)

Not guesses. AP3A `cbd.dis` + `bootdump_read` + two live fails at the
same offset.

| Question | Evidence |
|---|---|
| Wait after every chunk? | **Yes.** `f504`–`f5d8`: while remaining ≠ 0, `1fec0` write then `eb70(fd, req=0, exp=0xC10B\|bits)`. |
| Drain extra RX before next write? | **No.** `eb70` is poll + one `read(4)`. No leftover consume. |
| ACK size | **4 bytes.** `eb70` `read(fd, buf, 4)`. Live START/first BIN: `read=4` value `0xC120` / `0xC12B`. Kernel already `skb_pull`s EXYNOS. Not 16. |
| CRC / DONE mid-MAIN at `0x5e49c8`? | **No.** `f700` CRC is after the BIN loop. `0x5e49c8` is `3053 * 0x7E8` (6.18 MiB of 93 MiB). Not 1 MiB / `0xC000` / `SZ_2K` aligned in a special way beyond being a legal chunk start. |
| Timeout too short / EAGAIN-as-timeout? | **Not the 6.18 MiB stop.** cbd `e2a0` is **one** `poll(POLLIN, 2000)` then blocking `read(4)`. Our fifth run waited a 30 s wall deadline; last RX **0 bytes**. |
| `bootdump_read` + `O_NONBLOCK` | **Does not honour nonblocking.** Empty read blocks in `bootdump_read` (~100 ms `NO data in RXQ` retries). A `read()` without prior `poll(POLLIN)` deadlocks the loader (fifth attempt: stuck after first ACK, never wrote chunk 1). |

`0x5e49c8` is therefore a **repeatable CP-quiet point** after thousands of
legal 2K BIN frames, not a leftover-desync and not a 4-byte-vs-16-byte
ACK bug.

## Fifth `cp-boot` attempt LIVE (2026-09-06) — waiter regression

Added a blocking `read()` drain before every BIN write. After first
`0xC12B`, drain sat in `bootdump_read` (wchan, 1960 `NO data` lines).
Never wrote chunk 1. `BOOTING`. sysrq-b. Log:
`/data/saaios/var/cp-boot-20260906-drainstall.log`.

Waiter fix: drain and ACK `read()` only after `poll` says `POLLIN`
(`poll(0)` for drain). Consume extras from that same burst. 30 s
deadline around poll, not a 2-strike EAGAIN abort.

## Sixth `cp-boot` attempt LIVE (2026-09-06, fresh OFFLINE)

Helper 68328 bytes. PID 1 still had no CPIF; signed modules `insmod`'d
and `umts_*` `mknod`'d. One `load` (pid 515) from OFFLINE. Same first
BIN frame. **No** `IOCTL_POWER_OFF`. **No** `cbd` / `rild`.

| Step | Result |
|---|---|
| START | **ACK `0xC120` `read=4`** |
| first BIN | hdr `2b a1 f0 07 …` write `0x7F4`; **ACK `0xC12B` `read=4`** |
| later BIN | **3053** ACKs; `last_good=0x5e41e0` (`3052 * 0x7E8`) |
| fail | off **`0x5e49c8`** (`3053 * 0x7E8`) chunk=3053; timeout last read **0** bytes |
| after | **`BOOTING`** (not `CRASH_EXIT`); `boot_stage` `0xFF` → **`0x3FFF`** |
| `rmnet0` | **rx=0 tx=0** |
| COMPLETE / VSS / NV | **not issued** |

Same fail offset as the fourth attempt. Waiter is no longer the
blocker. Log: `/data/saaios/var/cp-boot-20260906-ack3053.log`. sysrq-b.

## Next

Do **not** start `rild`. Do **not** `IOCTL_POWER_OFF`. Phone is
**OFFLINE** after the `init_boot_a` flash reboot. Next load only from
this fresh `OFFLINE`. Keep `0x7E8` and START `0xA120`. Do not invent
opcodes. Do not spray chunk sizes on one boot. Next discriminator is
why the CP goes quiet after 3053 legal 2K BIN ACKs at `0x5e49c8`
(still from cbd/kernel, not a new command).

## Seventh attempt — exact legacy-ring wrap (2026-09-06)

Matching source was read from Google
`kernel/google-modules/radio/samsung/s5300`, branch
`android-gs-pantah-6.1-android16`.

The earlier pktproc hypothesis is not the boot path:

- `xmit_to_cp()` sends every `is_bootdump_ch()` skb directly to
  `xmit_to_legacy_link(..., IPC_MAP_NORM_RAW)`.
- It does so before the pktproc-UL branch and intentionally sends no IPC
  doorbell for boot/dump.
- Live `/sys/devices/platform/cpif/pktproc_ul/{region,status}` showed both
  queues inactive while OFFLINE. pktproc UL is for packet-switched IPC after
  boot, not UDL on `umts_boot0`.
- `xmit_to_legacy_link()` checks byte-ring space, waits up to 20 × 50 ms on
  `-ENOSPC`, copies the whole skb with `circ_write()`, and advances `head`
  modulo the ring size. The userspace write had always returned in full, and
  there was no `NOSPC` log.

Live legacy layout:

```
RAW offset head:0x00000018 buff:0x00003000
RAW size txq:0x001FD000 rxq:0x00200000
```

The arithmetic identifies the boundary exactly:

- one BIN is `0x7E8` data + 12-byte SIT + 12-byte EXYNOS = `0x800`
- `0x1FD000 / 0x800 = 1018` frames exactly
- `3053 * 0x7E8 = 0x5E49C8` payload bytes
- `3053 * 0x800 = 0x5F6800` wire bytes
- the failed write completes frame 3054:
  `3054 * 0x800 = 0x5F7000 = 3 * 0x1FD000`
- the initial wrapped MAIN START occupies 16 ring bytes, so every 1018th BIN
  crosses from ring offset `0x1FC810` to `0x10`.

The helper was instrumented to print live legacy head/tail and sleep 100 ms
before each such crossing. It preserved opcodes, first-frame behavior, and
`0x7E8`; it added no ioctl and changed no stage sequencing.

The first two crossings were consumed and ACKed:

```
pre-wrap  chunk=1017: NORM_RAW head=2082832 tail=2082832
post-ACK  chunk=1018: NORM_RAW head=16 tail=16
pre-wrap  chunk=2035: NORM_RAW head=2082832 tail=2082832
post-ACK  chunk=2036: NORM_RAW head=16 tail=16
```

At the third crossing, the ring was empty before the write
(`head=tail=2082832`). After the write and 30-second ACK timeout:

```
NORM_RAW TX busy:0 head:16 tail:2082832
NORM_RAW RX head:48864 tail:48864
```

Therefore this is **not AP-side ring fullness, pktproc descriptor exhaustion,
or insufficient reclaim time**. AP successfully published the wrap-crossing
frame; CP did not advance the legacy tail and produced no RX. The RX count is
also exact: START ACK plus 3053 BIN ACKs, each a 16-byte EXYNOS-wrapped ring
frame, is `(1 + 3053) * 16 = 48864`.

Factory cbd has no transport-setup ioctl between
`IOCTL_START_CP_BOOTLOADER` and this BIN loop and no sleep in the loop. `-s 2`
writes SIM count 2 to `/sys/devices/platform/cpif/sim/ds_detect`; the live
value was already 2. Its loop is one write followed by one 4-byte ACK read.
The material remaining difference is frame geometry/count: cbd sends
`0xC000` payload writes, whereas the constrained path sends `0x7E8`.

Result: same `last_good=0x5E41E0`, failure at `0x5E49C8`,
`modem_state=BOOTING`, `rmnet0`–`rmnet3` rx=tx=0. Logs:

- `/data/saaios/var/cp-boot-20260906-wrapguard.log`
- `/data/saaios/var/dmesg-cp-boot-20260906-wrapguard.log`
- `/data/saaios/var/legacy-cp-boot-20260906-wrapguard.log`

The phone was restored with AP sysrq-b and confirmed **OFFLINE**, original
EFS unmounted, and rmnet counters zero.

Next evidence-based step is to determine why the CP bootloader stops accepting
the 3054th small EXYNOS/SIT frame (a CP-side frame-count/window limit is now
more likely than AP flow control). Pacing and `POLLOUT` cannot correct it:
the AP ring was empty before the failed write, and the write succeeded.

## Source-confirmed stock framing result (2026-09-06)

The exact historical source snapshot that introduced the live-reported
`CPIF-20220408R1` version (`8da0bb9`) and the current matching
`android-gs-pantah-6.1-android16` source agree:

- DT parsing reads optional `iod,max_tx_size` into
  `modem_io_t.ul_buffer_size`; `create_io_device()` copies that once into
  `iod->max_tx_size`. There is no ioctl, module parameter, or sysfs setter.
- The running DT is also the stock DT (SaaiOS changed `init_boot_a`, not the
  kernel DT): `umts_boot0` has no `iod,max_tx_size`, so its live zero is real,
  not a read from the wrong field.
- `IO_ATTR_NO_LINK_HEADER` is consumed only by `sipc5_init_io_device()` at
  probe to set `iod->link_header`. There is no runtime toggle.
- `bootdump_write()` computes the SIT config once for the whole `write()`.
  For `IPC_BOOT`, `exynos_build_fr_config()` unconditionally returns
  `0xC000` (SINGLE), before checking size. With `max_tx_size == 0`, one
  `write(0xC00C)` allocates one skb and adds one 12-byte EXYNOS header:
  total frame `0xC018`, EXYNOS `len=0xC018`.
- Even if boot0 had a nonzero DT max, `bootdump_write()` would split the skb
  payload but would reuse the same SINGLE config for every fragment. Unlike
  `ipc_write()`, it never calls `modify_next_frame()`. A DT max therefore
  would not produce EXYNOS MULTI on this boot path.
- `skbpriv(skb)->lnk_hdr = iod->link_header`; `xmit_to_cp()` identifies the
  boot channel and sends the complete skb directly through
  `xmit_to_legacy_link(..., IPC_MAP_NORM_RAW)`.

### EXYNOS header and MULTI encoding

The on-wire header is 12 bytes (the packed C struct names only 10 bytes; the
remaining two reserved bytes are not initialized by `exynos_build_header()`):

| Offset | Size | Field |
|---|---:|---|
| `0x00` | 2 | sync `0xABCD` |
| `0x02` | 2 | global frame sequence, incremented per frame |
| `0x04` | 2 | fragmentation config |
| `0x06` | 2 | this frame length including the 12-byte EXYNOS header |
| `0x08` | 1 | channel (`0xF1` for boot0) |
| `0x09` | 1 | per-channel sequence, incremented per frame |
| `0x0A` | 2 | reserved/unspecified |

Config is little-endian `u16`:

- SINGLE: `0xC000`.
- MULTI first/intermediate: high byte has bit 7 plus a 6-bit packet index;
  low byte is the number of following frames (`N - 1` initially).
- `modify_next_frame()` decrements the low frame index after each frame.
  The final frame clears bit 7 and sets bit 6, preserving the 6-bit packet
  index. There is no total/remaining byte-length field; RX gathers by packet
  index until the LAST bit.

This MULTI machinery is reachable on `ipc_write()` for SIT formats below
`IPC_BOOT`, not on `umts_boot0`.

### Factory `cbd` fd and write behavior

AP3A factory `cbd` disassembly confirms:

- Its boot-argument constructor opens the configured boot node once with
  `O_RDWR` and stores that fd at object offset `+8`.
- The s5100sit boot node string is `/dev/umts_boot0`; START, BIN, CRC, and the
  boot ioctls all load/use that same stored fd. There is no PCIe-download
  device or second transport fd for the SIT BIN loop.
- `1fec0` performs exactly one checked write of `chunk + 12`; default MAIN
  chunk `0xC000` therefore means one `write(0xC00C)`.
- A short write is an error, not a continuation. The error path sets the
  process-global fallback flag; a later stage/attempt selects `0x7D00`.
  It does not retry the same logical SIT frame through partial writes.
- No ioctl in the START/BIN loop changes max TX or link-header behavior.

### Consequence

Userspace cannot reproduce a kernel-managed MULTI boot packet by issuing
several writes: every write is a new `bootdump_write()` call, receives a new
EXYNOS SINGLE header and sequence, and is a separate SIT payload as seen by
the CP. Userspace also cannot supply its own EXYNOS MULTI headers because
boot0 link-header insertion cannot be disabled at runtime or through a
confirmed alternate boot node.

No eighth live load was made: the requested MULTI/framing mechanism is
source-disproved, so testing it would violate the one-load safety gate.
Phone remains **OFFLINE**, original EFS unmounted, and rmnet has no traffic.
The precise blocker is now the unexplained difference outside fragmentation:
factory accepts a legal `0xC018` SINGLE boot frame, while the helper's prior
nominally identical large BIN write reached `CRASH_EXIT`. Before another load,
the complete factory pre-UDL boot setup/argument sequence must be compared
against `cp-boot` (especially boot-mode/ioctl arguments and BOOT staging);
the transport should not be converted to guessed MULTI framing.
