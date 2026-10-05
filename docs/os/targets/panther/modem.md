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

Firmware base historically `CP2A.260705.006` (SaaiOS slot A bring-up).
Stock reverse on **2026-10-05** used baseband
`g5300q-260317-260505-B-15346003` on slot **B** after the user flashed
stock. Slot A remains the SaaiOS image unless the user re-flashes.

**Working stock recipe (cbd + rfsd + rild + real EFS NV, LTE HOME +
`rmnet1`):** [modem-stock-reproduction.md](modem-stock-reproduction.md).
**Hardware next to the CP** (PCIe RCs, PMIC, GSA, GNSS, eSE):
[hardware-risks.md](hardware-risks.md).
Native `cp-boot` UDL below did **not** reach a bearer. Do not splice NV
into the `modem.bin` TOC.

## Success criterion

CP stably **ONLINE** (not `CRASH_EXIT`) **and** (`rmnet*` rx/tx ≠ 0 **and/or**
IPv4 on `rmnet*`). Wi-Fi is not a substitute. Stock already meets this
(2026-10-05): Kyivstar 25503 LTE CS+PS HOME, `SETUP_DATA_CALL` `NONE`
on `rmnet1`. SaaiOS must match that recipe, not a new SIT opcode.

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
  unless the user explicitly grants a new write
- `IOCTL_POWER_OFF` on `umts_boot0`
- inventing SIT opcodes or splicing NV into `modem.bin` TOC
- flashing anything except `panther`

Vendor `cbd` / `rfsd` / `rild_exynos` are **required** to match stock
cellular; see [modem-stock-reproduction.md](modem-stock-reproduction.md).
Do not auto-start them from a half-booted `cp-boot` UDL. Slot B was
the Android escape hatch; after the 2026-10-05 stock flash it is the
working control image.

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

No eighth live load was made on 2026-09-06: the requested MULTI/framing
mechanism is source-disproved. Later loads below did not retry MULTI.

## Ninth–twelfth loads LIVE (2026-09-21/22)

Signed CPIF was not in the flashed PID 1. Each boot: `insmod` `shm_ipc.ko`,
`cpif_page.ko`, `cpif.ko`, `cp_thermal_zone.ko` from `/lib/modules`, then
`mknod` `/dev/umts_boot0` from `/sys/class/cpif/umts_boot0/dev` (major 493).
Original EFS stayed unmounted. NV came from the userdata copy. No
`IOCTL_POWER_OFF`, no vendor `cbd` / `rild`. A stuck `BOOTING` or
`CRASH_EXIT` was cleared only with `echo b > /proc/sysrq-trigger`.

| Load | BIN geometry | Result |
|---|---|---|
| no `POWER_RESET`, one `0xC000` | wire `0xC018` | Inside `START`, before MAIN BIN: `NORM_RAW BAD CFG 0x00 (in:32 out:16 rest:16)`, 32 zero bytes, `CP_CRASH_REQ`. `BOOTING` → `CRASH_EXIT`. |
| `POWER_RESET`, one `0xC000` | wire `0xC018` | START ACK `0xC120`. CP consumed the frame (`TX head==tail`) and sent no `0xC12B` in 30s. Stayed `BOOTING`. No `BAD MSG`. |
| `POWER_RESET`, `SIT_CHUNK=0x7E8`, shrink so the wire frame never crosses `0x1FD000` | wire `0x800`, one short frame `0x7F0` at the first ring end | ACKed through MAIN offset `0x201a8b0` (16633 BIN ACKs). Next frame at `0x201b098` stayed queued (`TX head=0xAD000 tail=0xAC800`, one `0x800` frame). `RX head==tail==266144` = `(1+16633)*16`. No `BAD MSG`. Stayed `BOOTING`. Log: `/data/saaios/var/cp-boot-20260922-align.log`. |
| `POWER_RESET`, `0xC000`, wrap allowed, progress = TX consumed rather than BIN ACK | wire `0xC018` | 42 frames consumed, `acks=0`. Frame 42 at `0x1f8000` wrapped (`head=0x7418 tail=0x1f8400`) and was not consumed. Then the same zero-RX `BAD CFG` / `CP_CRASH_REQ` / `CRASH_EXIT`. Log: `cp-boot-20260922-c48.log`. |
| `POWER_RESET`, `0xC000`, shrink-to-ring-end, progress = TX consumed | first frames still `0xC018` (no shrink yet) | 3 frames consumed, `acks=0`. Frame 3 at `0x24000` did not wrap (`head=0x30070 tail=0x24058`) and was not consumed. `boot_stage 0x3FFF` at uptime 85.817s, first `BAD CFG 0x00 (in:32 …)` at 86.050s, then `CRASH_EXIT`. Log: `cp-boot-20260922-fit48.log`, `dmesg-fit48.txt`. |

`0xC000` is not a silent-ACK download. The CP can advance the TX tail for
those frames and still never send `0xC12B`, then publish zeros on RX. The
driver treats that as a bad CP message and issues `CP_CRASH_REQ`. The crash
is not specific to a wrapped frame: fit48 crashed on a frame that sat
entirely inside the ring. Repeating `0xC000` is not the next step.

The `0x7E8` ring-aligned path is the one the CP answers. It gets past the
old third-wrap stop (`0x5e49c8`) and stops later, still `BOOTING`, with one
unread `0x800` frame at payload offset `0x201b098` (~32.1 MiB of the
93 MiB MAIN image). That offset is not a ring wrap: post-wrap samples in
the same log show `TX head==tail==0`. It is 110744 bytes past 32 MiB, not
on that boundary. RX was not full (`0x200000` ring, 266144 bytes used).

A repeat of the aligned `0x7E8` load stalled at the same offset
(`0x201b098`, chunk 16633). A stage CRC (`0xA321`, TOC crc `0x68f46d27`)
written behind that unread frame produced no RX in 5s (`SIT UDL timeout
waiting 0x0000c320`, last read 0). After the probe the ring was
`TX head=708632 tail=706560`: the original `0x800` frame plus a 24-byte
padded CRC, neither consumed. The CP had stopped reading the ring, so a
command queued behind the stuck frame cannot be a test of CRC. State
stayed `BOOTING`. Log: `/data/saaios/var/cp-boot-20260922-crcprobe.log`.

The same stall with payload `0x3E8` (wire `0x400`, so about twice as many
frames) stopped at MAIN offset `0x201abf0`, chunk 33664, `last_good=0x201a808`.
That is 33663984 bytes, 1192 bytes short of the `0x7E8` high-water mark
`0x201b098`. Frame count scaled with the inverse of the chunk size, so this
is a byte offset near 33.66 MiB, not a 16633-frame cap and not a timeout.
The CRC probe behind that frame again timed out with 0 bytes RX. State
stayed `BOOTING`. Log: `/data/saaios/var/cp-boot-20260922-1k.log`.

Pausing 20s with the TX ring empty at offset `0x201a8b0` (head==tail==704512)
did not move the wall. The next frame, at `0x201b098` chunk 16633, again got
no `0xC12B`, and the CRC behind it again got no `0xC320`. No `BAD MSG`.
`boot_stage` reached `0x3FFF`. State stayed `BOOTING`. Log:
`/data/saaios/var/cp-boot-20260922-pause.log`.

Sending the stage CRC at offset `0x201b098` with the TX ring empty
(`head==tail==706560`, 16633 BIN ACKs, no further BIN frame queued) also
got no reply: `SIT UDL timeout waiting 0x0000c320 (last read 0 bytes)`.
The CP does not treat that offset as the end of MAIN. State stayed
`BOOTING`. No `BAD MSG`. Log:
`/data/saaios/var/cp-boot-20260922-crcwall.log`.

Payload `0xF00` (wire `0xF18`, ring-fit, `POWER_RESET`, BIN success only
on `0xC12B`) does ACK. The first MAIN BIN returned `0xC12B`. It then
stopped far earlier than the 2 KiB wall: `BIN ACK fail at 0xfb400
chunk=268 last_good=0xfa500`. The ring was empty (`TX head==tail==0xfdc48`,
1039432), so the CP consumed that frame and sent no ACK. No `BAD MSG`,
no `CP_CRASH_REQ`. `boot_stage` reached `0x3FFF`. State stayed `BOOTING`.
Offset `0xfb400` is about 0.98 MiB, not past `0x201b098`. VSS, APM, NV,
and COMPLETE were not issued. Log: `/data/saaios/var/cp-boot-20260922-f00.log`,
`dmesg-f00.txt`. AP sysrq-b after the stall.

## BOOT slice search (2026-09-22, no new load)

While `modem_state=OFFLINE`, `modem_a` (`sda19`) was mounted ext4
`ro,noload` only long enough to copy BOOT: file offset `0x410`, size
`0x16800`, to `/data/saaios/var/boot-slice.bin` (md5
`c66fdb1c1fb11096441836db75f31179`). The image was then unmounted.
Original EFS was not mounted. No `cp-boot load` was run after this search.

The slice is Thumb-2 CP boot code (PCIe link, dump, `SitRom,`,
`Mode=0x`, `Stage:`, `Msg Size Err`, `Frame Err`, `header`). It is not
an AArch64 literal pool of host commands.

| Looked for | Result |
|---|---|
| aligned `u32` `0x02000000` | one hit, at `0xd1a8`, inside a table of `0xff000000` flags and `0x0202xxxx` pointers. Not next to a UDL opcode. Not equal to the stop offset `0x201b098`. |
| `0x01f90000`, `0x0201b000`, `0x0201b098`, `0x00c00000` | no aligned hit |
| raw `u16` `0xA100`, `0xA301`, `0xA400`, `0xC100`, `0xC12B` | none |
| raw `u16` `0xA10B` / `0xA10D` at `0x47a8` / `0x47ae` | Thumb `add r1, pc, #imm`, six bytes apart, followed by the string `vref_mem_lv`. Not a command table. |
| raw `u16` `0xA00B` at `0xb86`, `0x20ee`, `0x2a00` | Thumb `add r0, pc, #imm` |
| ARM `MOVW`/`MOVT` of those opcodes | none that is a host command |

No extra UDL command and no size constant that is the `0x201b098` stop.
Guessing a new opcode and running another framed load would not be
evidence.

### `Msg Size Err` / `Frame Err` (same BOOT slice, no load)

Both strings are referenced from one Thumb function whose prologue is
`push` at file offset `0x1a24`. References are `add r0, pc`:

| String | File offset | ADR | Taken from |
|---|---:|---:|---|
| `Msg Size Err` | `0x1c18` | `0x1b0c` | `0x1aca` `blo` |
| `Frame Err` | `0x1c28` | `0x1b60` | `0x1b44` `bne` |
| `ChID Err` | `0x1c34` | `0x1b8a` | `0x1b4e` `bhs` |

`Msg Size Err` is a minimum, not a maximum:

```
0x1ac6  cmp.w r11, #0x10     ; f1bb 0f10
0x1aca  blo  0x1b0c          ; r11 < 16 → "Msg Size Err"
```

`Frame Err` checks the EXYNOS sync halfword, not a length:

```
0x1b20  movw  r1, #0xabcd
0x1b40  ldrh  r0, [r5]
0x1b42  cmp   r0, r1
0x1b44  bne   0x1b60         ; halfword != 0xABCD → "Frame Err"
```

The next check is the channel id, and it only allows 0 or 1:

```
0x1b4c  cmp   r0, #2
0x1b4e  bhs   0x1b8a         ; channel >= 2 → "ChID Err"
```

Boot channel `0xF1` would fail that check, so this function is the
normal IPC checker, not the UDL downloader. Inside this function there
is no compare against `0x800`, `0x1000`, `0x7e8`, `0xc000`, `0xc018`, or
`0x2000000`. The 16-bit compares here are `#16`, `#2`, and `#242`
(`0xF2`, equality).

A different function, which returns at `0x190c`, does clamp a value:

```
0x18a6  movw r2, #0xfe8
0x18ae  cmp  r1, r2
0x18b0  blo  0x18c6          ; if r1 >= 0xFE8, store 0xFE8
```

That is not on the path that prints `Msg Size Err` or `Frame Err`, and
the surrounding stores are small byte fields, not the SIT header. It is
not a demonstrated UDL frame cap, so `cp-boot` was not changed and no
load was run.

### `movw` of the UDL immediates

Thumb-2 `movw` (first halfword `0xF240`–`0xF24F` or `0xF640`–`0xF64F`) and
ARM `movw` (`(insn >> 20) & 0xFF == 0x30`) were scanned for
`0xA100`, `0xA10B`, `0xA301`, `0xA10D`, `0xA00B`, `0xA400`, `0xC100`,
`0xC10B`, `0xC300`, `0xC10D`, `0xC00B`, `0xC400`.

BOOT (`0x410`..`0x16c10`) has **zero** hits. The whole `modem.bin`
(98265168 bytes) has 196 Thumb hits and 90 ARM hits, all inside MAIN.
The first Thumb hit is at file offset 18223996.

The tight cluster at file offset 19788598 is a name table, not a
downloader. Each slot is `movw r0, #opcode` / `movw r3, #0x36a` for
sequential ids `0xA0FB`..`0xA124`, including `0xA10B` at 19788598 and
`0xA10D` at 19788646. There is no `cmp` in that table.

Within 160 bytes of every Thumb `movw` of `0xA10B`, `0xC10B`, `0xA301`,
or `0xA10D` the only compares are 16-bit and small: `#0`, `#1`, `#2`,
`#3`, `#8`, `#9`, `#31`, `#32`, `#35`, `#77`, `#114`, `#128`, `#148`.
There is no 32-bit `cmp` there, and none of those immediates is a frame
size of `0x800` / `0x7E8` / `0xC000` / `0xC018`. `cp-boot` was not
changed. No load was run.

Modem left **OFFLINE**.

## Stall logs at `0x201b098` (2026-09-22, one `0x7E8` load)

Fresh `OFFLINE` after `insmod` of `shm_ipc.ko`, `cpif_page.ko`, `cpif.ko`,
`cp_thermal_zone.ko`. Char nodes created from sysfs (`major:minor`):

| Node | dev |
|---|---|
| `umts_boot0` | 493:7 |
| `umts_dm0` | 493:4 |
| `umts_ipc0` | 493:0 |
| `umts_ipc1` | 493:1 |
| `umts_loopback` | 493:5 |
| `umts_rcs0` | 493:8 |
| `umts_rcs1` | 493:9 |
| `umts_rfs0` | 493:2 |
| `umts_router` | 493:3 |
| `umts_wfc0` | 493:10 |
| `umts_wfc1` | 493:11 |
| `umts_toe0` | 10:102 |
| `logbuffer_cpif` | 10:103 |
| `logbuffer_bd` | 10:111 |
| `logbuffer_cpm` | 10:113 |
| `logbuffer_maxfg` | 10:118 |
| `logbuffer_maxfg_monitor` | 10:117 |
| `logbuffer_maxq` | 10:116 |
| `logbuffer_pcie0` | 10:106 |
| `logbuffer_pcie1` | 10:105 |
| `logbuffer_ssoc` | 10:112 |
| `logbuffer_tcpm` | 10:115 |
| `logbuffer_ttf` | 10:104 |
| `logbuffer_tty18` | 10:110 |
| `logbuffer_usbpd` | 10:114 |

One load, ring-fit payload `0x7E8`, `POWER_RESET(NORMAL)`, BIN success
only on `0xC12B`. It stalled where the earlier `0x7E8` runs stalled:
`BIN ACK fail at 0x201b098 chunk=16633 last_good=0x201a8b0`, NORM_RAW
`head=708608` (`0xad000`) `tail=706560` (`0xac800`), one unread `0x800`
frame. `modem_state` stayed `BOOTING`. No `BAD MSG`. Log:
`/data/saaios/var/cp-boot-20260922-stalllog.log`.

Before reboot, dmesg lines matching `cpif`, `BAD`, `boot_stage`, `sit`,
or `udl` were saved to `/data/saaios/var/dmesg-stall-log.txt` (214 lines).
They record the usual power-on path and nothing about a download limit
or a missing command:

- `clear_boot_stage` then `boot_stage == 0xFF` then `boot_stage == 0x3FFF`
- `CP2AP_WAKEUP == 0x0` three times, then `0x1`
- `start_normal_boot`
- `shmem_enqueue_snapshot: invalid intr 0x0` twice during PCIe probe
- `bootdump_release: umts_boot0` when the loader exited
- the `sit` hits are the kernel IPv6 `sit` tunnel driver, not SIT UDL
- no `BAD`, no `udl`, no crash line

`/dev/umts_boot0` was not read (the loader had it). Every other new node
was polled for 300 ms. All `umts_*` returned no data. `logbuffer_cpif`
(887 bytes) is only the GPIO/PCIe power sequence, ending at
`DBG: doorbell: pcie_registered = 1`. The other `logbuffer_*` files
(pcie, tcpm, usbpd, tty, battery) contain no `udl`, `crash`, `frame`,
or `boot_stage` string. They do not name a limit or a missing command.

AP sysrq-b after the dump. Modules reloaded. `modem_state=OFFLINE`.
No second load.

## COMPLETE / FIN / READY at an empty ring (2026-09-22)

One `0x7E8` ring-fit load with `POWER_RESET(NORMAL)`. At MAIN offset
`0x201b098` (chunk 16633, previous frame ACKed) the next BIN was not
written. NORM_RAW was empty: `head==tail==706560`. Then, in order:

| Step | Result |
|---|---|
| `IOCTL_COMPLETE_NORMAL_BOOTUP` | `rc=-1 errno=11` (`EAGAIN`) |
| state after COMPLETE | `BOOTING` |
| FIN `0xA400`, wait 5s for `0xC400` | timeout, last read **0 bytes** |
| state after FIN | `BOOTING` |
| READY `0xA00B`, wait 5s for `0xC00B` | timeout, last read **0 bytes** |
| state after READY | `BOOTING` |
| `rmnet0` | `rx_bytes=0 tx_bytes=0`, no IPv4 (`EADDRNOTAVAIL`) |

No further BIN. No `rild`. Log:
`/data/saaios/var/cp-boot-20260922-wallprobe.log`. AP sysrq-b afterwards.
Modules reloaded. Modem returned to **OFFLINE**.

## No `POWER_RESET`, 3s wait, one `0xC000` BIN (2026-09-22)

`IOCTL_POWER_ON`, then no `IOCTL_POWER_RESET`. Once a second for 3s,
`modem_state` stayed `OFFLINE` and NORM_RAW was `TX head=tail=0`,
`RX head=tail=0`. dmesg had no `BAD CFG`, so `START` was issued.

| Step | Result |
|---|---|
| `LOAD_CP_IMAGE` BOOT `0x16800` | OK |
| `START_CP_BOOTLOADER` NORMAL | OK, `rc=0`. `OFFLINE` → `BOOTING`. Ring still `head=tail=0` on TX and RX. The earlier no-wait START crash did not repeat. |
| MAIN UDL START `0xA120` | ACK `0xC120` |
| one BIN, payload `0xC000`, one `write(0xC00C)`, no shrink | no `0xC12B` in 10s (last read 0). TX `head==tail==0xc028` (frame consumed). Then `bad_cfg=1`, `modem_state=CRASH_EXIT`. |
| after the crash | TX `head=tail=49192` (`0xc028`), RX `head=tail=32` |

No second frame. No `COMPLETE`. Log:
`/data/saaios/var/cp-boot-20260922-noreset.log`. AP sysrq-b afterwards.

Not yet reached: VSS, APM, NV, a successful `COMPLETE_NORMAL_BOOTUP`,
`ONLINE`, or any `rmnet` byte. Wi-Fi is not a substitute for that.

## No `POWER_RESET`, 3s settle, `0x7E8` ring-fit (log already on disk)

This load was already in `/data/saaios/var/cp-boot-20260922-nreset7e8.log`
(newest `cp-boot-*.log`, after `cp-boot-20260922-noreset.log`). It was
not repeated. `cp-boot.c` was not changed for it.

`IOCTL_POWER_ON`, no `IOCTL_POWER_RESET`. For 3s, `modem_state` stayed
`OFFLINE` and NORM_RAW TX/RX stayed `head=tail=0`. No `BAD CFG`, so
`START` ran.

| Step | Result |
|---|---|
| `LOAD_CP_IMAGE` BOOT | OK |
| `START_CP_BOOTLOADER` NORMAL | OK. `OFFLINE` → `BOOTING`. Ring still empty. |
| MAIN START `0xA120` | ACK `0xC120`, 4 bytes |
| MAIN BIN payload `0x7E8`, write `0x7F4`, success only on `0xC12B` | first ACK `0xC12B`, 4 bytes. One ring-fit shrink: payload `0x7E8` → `0x7D8` at TX head `0x1fc810` (wire ends on `0x1FD000`). |
| through chunk 16633 | ACKed. `last_good=0x201a8b0`. Ring empty at that point (`TX head==tail==706560`). |
| next frame, offset `0x201b098` | written, not ACKed. `SIT UDL timeout waiting 0x0000c12b (last read 0 bytes)`. `head=0xad000 tail=0xac800` (one `0x800` frame queued, `consumed=0`). |

The log line `ACKs continued past 0x201b098 next=0x201b098` is the
loader noticing that the next offset to send is `0x201b098`. It is the
ACK of the frame at `0x201a8b0`, not an ACK of the frame that starts at
`0x201b098`. That next frame is the one that timed out.

`bad_cfg=0`. `modem_state` stayed `BOOTING`. The loader stopped there:
no VSS, APM, NV, CRC, DONE, READY, FIN, or COMPLETE. The log has no
`rmnet` line. Same wall as the `POWER_RESET` + `0x7E8` runs
(`last_good=0x201a8b0`, unread frame at `0x201b098`). Skipping
`POWER_RESET` did not move it.

## No boot-write split knob (2026-09-23, no BIN load)

The wall did not move, so the next question was whether a factory
`write(0xC00C)` can be made into `0x800`-class EXYNOS frames without a
new UDL opcode and without `IOCTL_POWER_OFF`.

Local `s5300-src` `bootdump_write()` computes `cfg_sit` once per
`write()`, via `exynos_build_fr_config()`. That function returns
`EXYNOS_SINGLE` (`0xC000`) as soon as `format >= IPC_BOOT`, before it
looks at the byte count. The later `max_tx_size` loop can cut the skb
payload, but every fragment is headed with that same `cfg_sit`.
`bootdump_write()` never calls `modify_next_frame()`. `max_tx_size` is
copied once in `create_io_device()` from the DT property
`iod,max_tx_size`. Nothing in this tree assigns it later. `link_header`
is set once in `sipc5_init_io_device()` from `IO_ATTR_NO_LINK_HEADER`.

Live DT, `cpif/iodevs/io_device_8` (cells are big-endian):

| Property | Bytes | Value |
|---|---|---|
| `iod,name` | `75 6d 74 73 5f 62 6f 6f 74 30 00` | `umts_boot0` |
| `iod,attrs` | `00 00 02 00` | `0x200` |
| `iod,format` | `00 00 00 04` | `4` (`IPC_BOOT`) |
| `iod,ch` | `00 00 00 f1` | `0xF1` |
| `iod,max_tx_size` | absent | not in the node |
| `mif,protocol` | `00 00 00 01` | `1` (SIT; `bootdump_write` takes the `PROTOCOL_SIT` branch) |

`/lib/modules/cpif.ko` contains the string `iod,max_tx_size`, so the
signed module still has the DT reader. After `insmod` of `shm_ipc.ko`,
`cpif_page.ko`, `cpif.ko`, and `cp_thermal_zone.ko` the module
parameters are only `dflags`, `ds_detect`, and `wakeup_dflags`.
`/sys/class/cpif/umts_boot0` exposes `dev`, `power`, `subsystem`, and
`uevent`. There is no sysfs or module parameter that sets
`max_tx_size` or turns link-header insertion off.

A nonzero `max_tx_size` would still not produce EXYNOS MULTI on this
boot node: `IPC_BOOT` forces SINGLE, and the boot path does not update
the fragment config between skbs. Userspace also cannot prefix its own
MULTI header, because link-header insertion is on and cannot be
cleared at runtime. No such frame was sent. No `cp-boot load`.

Factory `cbd.dis` (`dist/panther/cbd-extract/cbd.dis`) expects that
one userspace write and does not split it:

| Site | What cbd does |
|---|---|
| `f2f8` / `f2fc` | chunk `0xC000`, or `0x7D00` only if the write-failure flag is set |
| `f2bc` | `memset` of `0xC00C` |
| `1fec0` | copy payload to `+12`, store `len = chunk+8`, one `__write_chk` of `chunk+12`; a short write is an error |

cbd never mentions `max_tx_size`. It relies on kernel `bootdump_write`
to add the EXYNOS header. With live `max_tx_size` absent (runtime 0)
and `format=IPC_BOOT`, that `write(0xC00C)` is one EXYNOS SINGLE of
length `0xC018`. cbd does not ask for MULTI, and this driver will not
emit MULTI for `umts_boot0`.

When this check started, cpif was not loaded (`modem_state` missing,
uptime about 329s). After the four `insmod`s: `modem_state=OFFLINE`,
`rmnet0` `rx_bytes=0` `tx_bytes=0`, no IPv4 address. No `POWER_ON`,
no `IOCTL_POWER_OFF`, no vendor `cbd` / `rild`, original EFS not
mounted.

## Ring, SHM, and the 48KB frame (2026-09-24, no BIN load)

Read-only. No `cp-boot load`. `modem_a` (`sda19`, 259:3) was mounted
`ext4` `ro,noload` only long enough to read `modem.bin`, then umounted.
Original EFS was not mounted.

Live `use_mem_map_on_cp` is disabled (`cp_shmem_probe`: use DT).
`/sys/devices/platform/cpif/legacy/region` and the probe / DT `reg`
cells agree:

| Region | Physical base | Size | Role |
|---|---:|---:|---|
| `cp_rmem` / IPC index 3 | `0xea400000` | `0x00800000` | IPC, cached |
| NORM_RAW TX ring | `0xea403000` | `0x001fd000` | IPC + buffer offset `0x3000` |
| NORM_RAW head/tail | `0xea400018` | 16 bytes | head offset `0x18` |
| NORM_RAW RX | `0xea600000` | `0x00200000` | after the TX ring |
| BOOT image slot | `0xea410000` | `0x16800` on the last power-on | `round_up(0x3000, 64K)` |
| `cp_rmem_1` | `0xe8000000` | `0x02000000` | PKTPROC `0x1c00000` at `0xe8000000` plus PKTPROC_UL `0x400000` at `0xe9c00000` |
| `cp_msi_rmem` / MSI index 11 | `0xf6200000` | `0x1000` | MSI regs, including `boot_stage` and `img_addr` |
| `cp_aoc_rmem` / VSS_AOC index 1 | `0x197fd000` | `0x3000` | AoC |

`ipc_base` in dmesg is a hashed `%pK` pointer, not a physical address.

On the `0xC000` crash (`/data/saaios/var/dmesg-fit48.txt`),
`cpif_pcie_iommu_enable_regions` identity-maps these windows
(`iova == paddr`, all `ret:0`):

| idx | Address | Size mapped |
|---:|---:|---:|
| 1 | `0x197fd000` | `0x3000` |
| 3 IPC | `0xea400000` | `0x800000` (the whole IPC region, ring included) |
| 7 PKTPROC | `0xe8000000` | `0x100000` only (`buff_rgn_offset`; the rest of the 32MB reservation is not this map) |
| 8 PKTPROC_UL | `0xe9c00000` | `0x400000` |
| 11 MSI | `0xf6200000` | `0x1000` |

Same log: `clear_boot_stage == 0x0`, then `boot_img addr:0xEA410000 size:0x16800`, then `boot_stage == 0xFF`, then `boot_stage == 0x3FFF`. The next cpif lines are `NORM_RAW BAD CFG 0x00` (32 zero bytes) and `CP_CRASH_REQ`. There is no sysmmu, S2MPU, or IOMMU fault line on that crash. A bounded read of `logbuffer_cpif` (10:103) on this later OFFLINE boot returned no bytes. The host PCIe outbound window in that same dmesg is `MEM 0x40000000..0x40feffff` translated to `0x14e00000`; the doorbell is BAR `0x40000000` plus `0x60000`. That is the EP register window, not a buffer for MAIN.

### How a boot TX address is given to the CP

`xmit_to_cp()` (`link_device.c`) sends a boot channel straight to the legacy ring:

```c
if (ld->is_bootdump_ch(ch))
    return xmit_to_legacy_link(mld, ch, skb, IPC_MAP_NORM_RAW);
```

`xmit_to_legacy_link()` (`link_device_memory_legacy.c`) `circ_write`s `skb->data` into `txq.buff`, which is `mld->base + legacy_raw_buffer_offset`. It does not pass a kernel virtual address, an `iommu` `dma_addr`, or a per-skb physical address to the CP. The CP is expected to read the bytes from the shared ring at `0xea403000`, using the head/tail words at `0xea400018`. That ring sits inside the identity-mapped IPC window, so it is not outside the inbound map.

The only boot pointer written for the CP is `set_cp_rom_boot_img()` (`modem_ctrl_s5100.c`): physical `cp_shmem_get_base(SHMEM_IPC) + boot_img_offset` goes to MSI `img_addr_lo` / `img_addr_hi`, and `boot_img_size` goes to `img_size`. On the PCIe path `link_load_cp_image()` forces that offset to `round_up(legacy_raw_buffer_offset, 64K)` and copies into the IPC region only. Live values: `0xEA410000`, size `0x16800` (the BOOT slice). MAIN is `0x5917acc` bytes. Remaining IPC space after `0x10000` is `0x7f0000`, so `LOAD_CP_IMAGE` cannot hold MAIN, and `m_offset` is an offset inside that IPC window, not `0x40010000`.

`bootdump_ioctl` has no other command that registers a host DMA buffer. Besides the ioctls already used, the handler also has `SILENT_RESET`, `TRIGGER_CP_CRASH`, `TRIGGER_KERNEL_PANIC`, `GET_LOG_DUMP`, `GET_CP_CRASH_REASON`, `HANDOVER_BLOCK_INFO`, `SET_SPI_BOOT_MODE`, and `GET_OPENED_STATUS`. None of them maps a MAIN buffer or changes the inbound window. There is no sysfs knob for it either. No load was run.

### MAIN header at file `0x16c10`

First bytes are a little-endian ARM vector table (`ldr pc, [pc, #0x88]`). The tag block at `+0x30` stores 4-character codes as big-endian constants. Values that are CP addresses were checked against the image: `VER` points at `g5300q-251202-260127-B-14784800`.

| File offset | Tag | Value |
|---:|---|---|
| `+0x30` | `LTE` | magic `0x4c544500` |
| `+0x34` | `VER` | `0x410eed54` → that version string |
| `+0x3c` | `DATE` | `0x410eedb0` → `2026-01-27T03:33-08:00` |
| `+0x44` | `MAP` | `0x40010000` (TOC `m_off`), next word `0` |
| `+0x50` | `MEM0` | `0x1f900004`, then two zero words |
| `+0x60` | `QVER` | `1` |
| `+0x68` | `USER` | `0x410eedc7` (the nearby string is `BUSER`) |
| `+0x70` | `QBID` | `0x40c2dab9` |
| `+0x78` | `QBCL` | `0x40c2dab8` |
| `+0x80` | `CVER` | `1` |
| `+0x88` | `cCAT` | `0x48bc6e8c` (above the end of MAIN, `0x45927acc`) |

ARM code starts at `+0xa0` (`e92d4000`). `MEM0` is the only `MEM*` tag in the header. `0x201b098` is not `0x1f900004` and not `0x2000000`.

`0x201b098 = 16633 * 0x7E8 - 0x10`. The `0x7E8` log has exactly one ring-fit shrink, `0x7E8` → `0x7D8`, which is those `0x10` bytes. The stall is the next chunk cursor, not a region end. `0x2000000` is exactly the `cp_rmem_1` reservation (PKTPROC plus PKTPROC_UL). Boot TX does not write that reservation, and the stall is `0x1b098` past it.

The 48KB failure is not explained as an AP DMA/IOMMU fault or as a ring outside the inbound window. The ring is inside the mapped 8MB IPC region. A `0xC000` write is copied into that ring like a `0x7E8` write. The CP then publishes zeros, and the driver raises `CP_CRASH_REQ`. There is no driver-exposed way to put MAIN, or the remainder after `0x201a8b0`, in a separate buffer the CP can DMA, and no way to change the inbound window without flashing.

Left `modem_state=OFFLINE`, `/dev/umts_boot0` is `493:7`, `rmnet0` `rx_bytes=0` `tx_bytes=0`, no IPv4. No `POWER_ON`, no sysrq.

## cbd vs cp-boot header, then CRC on an empty ring (2026-09-24)

Byte-level comparison of factory `cbd.dis` with `cp-boot.c`. The SIT
header already matched. Chunk size was not changed, and no `0xC000`
frame was sent.

### START

`cbd` `f270`–`f294` builds one command word and calls `eb70`:

```
f270  lsl  w8, w26, #4
f274  mov  w9, #0xa100
f278  and  w8, w8, #0xfff0
f27c  orr  w21, w8, w9          ; 0xA100 | ((idx<<4) & 0xFFF0)
f280  mov  w9, #0xc100
f284  orr  w19, w8, w9          ; expect 0xC100 | same bits
f294  bl   eb70
```

`eb70` stores that word at `sp+4` and, when it is non-zero, `write`s
exactly 4 bytes (`ebf8 mov w2, #4`). There is no larger START body.
MAIN idx 2 is `0xA120` / `0xC120`. `cp-boot` `sit_req_resp` writes the
same `uint32_t` (`sit_cmd(SIT_START, idx)`).

### BIN

Inside the loop, `f518` selects `min(remaining, block)` into `w21`.
`f51c` sets the offset to `total - remaining` (`sub w23, w25, w26`),
which is 0 on the first frame and advances by the size actually sent,
including a short frame. `w25` is the full stage size (`f360` divides
it by the block). It is not `m_off` `0x40010000` plus the offset.

```
f55c  strh w21, [sp, #86]       ; len = payload size
f560  stp  w25, w23, [sp, #88]  ; total, then 0-based offset
f568  strh w8,  [sp, #84]       ; cmd = 0xA10B | ((idx<<4) & 0xFFF0)
f588  bl   1fec0
```

`1fec0` reads the halfword at `+2`, copies that many payload bytes to
`+12`, rewrites `+2` to `chunk+8`, and `write`s `chunk+12`:

```
1fed4  ldrh w20, [x2, #2]
1feec  bl   memcpy
1fef0  add  w8, w20, #8
1ff08  strh w8, [x21, #2]
1ff0c  bl   __write_chk         ; length = chunk+12
```

After a good write, `f5c8 sub w26, w26, w8` subtracts that same chunk
from the remainder, so the next offset moves by the shrunk size.
`cp-boot.c` does the same at the BIN fill: `hdr->cmd`, `hdr->len = chunk`
then `hdr->len = chunk + 8`, `hdr->total = size`, `hdr->offset = off`,
and later `off += chunk`. MAIN cmd is `0xA12B`. Userspace still does
not add the EXYNOS wrap.

### CRC and ioctls before the first BIN

After the BIN loop, `f700`–`f770` store `{0xA301 | bits, TOC crc}` and
`write` 8 bytes. MAIN is `0xA321` plus TOC crc `0x68f46d27`, expect
`0xC320` (`f780 mov w8, #0xc300`, then `eb70` with request 0).
`cp-boot` uses that same 8-byte pair.

From START's return to the first `1fec0` the disassembly is memset,
lseek, and the header fill. No ioctl sits between
`START_CP_BOOTLOADER` and the first MAIN BIN. `cbd` does issue
`POWER_RESET` earlier in boot; this load kept it skipped, and
`POWER_OFF` was not added. No other missing pre-BIN ioctl was found.

### Load

One load from fresh `OFFLINE`: `POWER_ON`, 3s settle, no
`POWER_RESET`, `START`, payload `0x7E8`, ring-fit, BIN success only on
`0xC12B`. The new path returns before writing any frame whose offset
is `>= 0x201b098`. The live tail (the log file was not on disk after
the later reboot):

```
UDL MAIN stop before BIN off=0x201b098 chunk=16633 last_good=0x201a8b0
  head=0xac800 tail=0xac800 empty=1
legacy status before-crc: TX head=706560 tail=706560
modem_state before-crc: BOOTING
UDL MAIN empty-ring CRC only cmd=0xa321 crc=0x68f46d27 expect=0xc320 wait=10s
SIT UDL timeout waiting 0x0000c320 (last read 0 bytes)
legacy status after-crc: TX head=706584 tail=706560
modem_state after-crc: BOOTING
bad_cfg=0
rmnet0 rx_bytes=0 tx_bytes=0
rmnet0 ipv4 none errno=99
```

The ring was empty (`head==tail==706560`). The 8-byte CRC was published
(head advanced 24 bytes, the EXYNOS wrap plus the 8-byte body and
padding) and the tail did not move. No `0xC320`. No second BIN, no
DONE, READY, FIN, or COMPLETE. `bad_cfg=0`. State stayed `BOOTING`.

AP sysrq-b afterwards. Modules reloaded (`shm_ipc`, `cpif_page`,
`cpif`, `cp_thermal_zone`). `/dev/umts_boot0` is `493:7`.
`modem_state=OFFLINE`, `rmnet0` rx=tx=0, no IPv4. Original EFS stayed
unmounted.

## IPC wall dump already on disk (read 2026-09-29, no new load)

The descriptor check and the stop-before-`0x201b098` dump had already
finished on 2026-09-24. This session read the phone logs and did not
rebuild `cp-boot`, did not `POWER_ON`, and did not run another load.

Logs: `/data/saaios/var/cp-boot-20260924-ipcdump.log` and
`/data/saaios/var/cp-boot-ipc-20260924.txt`.

No MAIN `m_off`/`size` word (`0x40010000` / `0x5917acc`) was written
into the IPC header. The loader stopped before the BIN at `0x201b098`
(`chunk=16633`, `last_good=0x201a8b0`, TX `head==tail==0xac800`, empty).
The log line `ACKs continued past 0x201b098 next=0x201b098` is the ACK
of the frame at `0x201a8b0`. That next frame was not sent. This run
does not by itself prove the older stall; it stopped in front of it.

`/dev/mem` failed (`No such device or address`). The 256-byte IPC
image at `0xea400000` and the MSI block at `0xf6200000` were not read
(`ipc=-1`, `msi=-1`). The dump file is the sysfs snapshot only.

| Word | after START | at the stop |
|---|---|---|
| `GET_CP_STATUS` | 3 | 3 |
| `modem_state` | `BOOTING` | `BOOTING` |
| `bad_cfg` |  | 0 |
| `boot_stage` in dmesg | `0`, then `0xFF`, then `0x3FFF` | no new line |
| `ap2cp_msg` | 0 | 0 |
| `cp2ap_msg` | 0 | `0x83` |
| `ap2cp_united_status` | `0x4000` | `0x4000` |
| `cp2ap_united_status` | 0 | 0 |
| NORM_RAW TX | 0/0 | 706560/706560 |
| NORM_RAW RX | 0/0 | 266144/266144 |
| `rmnet0` |  | rx=0 tx=0, no IPv4 (`errno=99`) |

The CP had not left the UDL loop. It was still `BOOTING`, `bad_cfg=0`,
`boot_stage` unchanged, and the ring was empty because the next BIN
was withheld. `cp2ap_msg` `0x83` is not a new command and not an error
word that says the downloader has exited. The log ends at
`modem_state bin-fail: BOOTING`. No CRC, DONE, FIN, or COMPLETE in
this run.

### Later probes already passed the wall; not repeated

Separate logs from the same night, already on the phone, are a
different loader. They are evidence, not a load from this session.

| Log | What it records |
|---|---|
| `probe-b-preamble-20260924.log` | `UDL MAIN ACKs continued past 0x201b098 next=0x201b428 chunk=16634`, then `PROBE BOUND REACHED` at 36 MiB, `PROBE END result=2`, no CRC |
| `probe-b-fullmain-20260924.log` | `SIT UDL ack 0x0000c320` |
| `probe-handover-20260924.log` | `modem_state post-complete: ONLINE` ten times, then `modem_state probe-end: ONLINE`. No `rmnet` line |
| `probe-b-complete-20260924.log` | no `rx_bytes` line |

Those runs are the READY plus TOC preamble, then MAIN through CRC/DONE,
VSS/APM, the userdata NV copies, FIN, and COMPLETE, and later the
handover ioctl and status queries. Data registration in that note was
raw 0. None of that is nonzero `rmnet` rx/tx or an IPv4 address.
This session did not repeat any of them.

### No load on 2026-09-29

The dump does not name an untested mailbox, flag, or SIT word that
`cp-boot` fails to set. The CP was still inside UDL, so the stop was
not "the CP already left the loop." The preamble that later moved
`0x201b098` is already logged through `ONLINE`, and that `ONLINE`
session has no recorded `rmnet` byte. Repeating it would not be a new
mechanism.

The phone had rebooted before this read (uptime a few minutes).
`cpif` is not loaded: `/sys/devices/platform/cpif/modem_state` is
absent. `rmnet0` does not exist, so there is no rx/tx count and no
IPv4. No `insmod`, no sysrq, no `POWER_ON`. Original EFS was not
mounted. Vendor `cbd` and `rild` were not started.

## Stock reverse (2026-10-05)

User flashed stock Android and granted KernelSU `adb root`. Live
control: LTE HOME + data on `rmnet1`. Full process list, NV sizes,
init rc, RIL names, and SaaiOS gap table:
[modem-stock-reproduction.md](modem-stock-reproduction.md).

That capture is read-only of NV. The next SaaiOS cellular step is
vendor `cbd`/`rfsd`/`rild_exynos` with **both** EFS NV files verified,
not another `cp-boot` MAIN UDL geometry experiment.
