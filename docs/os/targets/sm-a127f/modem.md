# SM-A127F — modem / CP (Phase E4)

Exynos 850 **S5000AP** Shannon (**SS310**) over CPIF SHMEM. Stock userspace is Android `cbd` + `rild`. We have BusyBox ramdisk only. **EFS writes are forever off-limits.** One-shot **ro,noload** list 2026-08-31 (below); umounted. RADIO is read-only (stock CP image). Do not `dd` / format / AT-write NV.

Detail: live v031 dump 2026-08-29 + `debugfs` of `os/build/stock-super/vendor.img` on R620. Kernel tree: `os/third_party/kernel_samsung_a12/drivers/soc/samsung/cpif`.

## LIVE map (v031, stock DXJ2 4.19.111-27127798)

| | |
|--|--|
| CP state | **`ONLINE`** (2026-09-01, probe 22). Holder **428** ipc0+rfs0 still open. Vendor `rild` **exec'd then exit 1** (did not keep ipc). SIM1 Kyivstar CS EMERGENCY HLR#2 / PS GMM#7; SIM2 HOME GMM#7 (probe 16–21). `rmnet0–7` rx=tx=0 (uptime **8221 s**). **Data-plane goal not complete.** |
| GNSS | **`OFFLINE`** (no BCMD this pass). `READ_FIRMWARE` vs `/tmp/kepler-fw.bin` **byte-identical** (64B K102 + full SHA). See GNSS boot LIVE |
| SIM detect | `ds_detect=2` (`cpif/sim/ds_detect` and `modem_ctrl_s5000ap` param) |
| Driver | `cpif_probe` **CPIF-200511N220408** `eur_open`; **s5000ap** modemctl; **s318ap** shmem link |
| DT | `samsung,exynos-cp`. LIVE `cpif/mif,protocol` = **0** (`PROTOCOL_SIPC`). gnssif has no protocol/sit cell |
| Firmware | BOOT+MAIN+**VSS** + real NV (userdata copy) via `/mnt/userdata/radio-boot loadnv`. Helper **1314320**. Bind-mount copy → `/mnt/vendor/efs`. Vendor `rild` exec'd this pass (exit 1; no Shannon attach). `cbd` not executed |

Kernel already created the netdevs and char nodes. They are empty until CP boots.

### Net (`/sys/class/net`)

`rmnet0`…`rmnet7` and `umts_dm0`: exist, **down**, type `519`, no MAC, zero packets. `ifconfig` shows `POINTOPOINT NOARP MULTICAST`. Android later writes RPS on `rmnet*` (`init.exynos850.rc`); that is not a CP start.

`rndis0` / `wlan0` are USB / Maxwell — not the modem.

### Char (`/dev`)

| Node | Role |
|------|------|
| `umts_boot0` | CP boot channel. `cbd` opens this (`/dev/umts_boot0`) |
| `umts_ipc0` / `umts_ipc1` | IPC to CP (what `rild` would use) |
| `umts_rfs0` | RFS — CP NV via EFS. **Do not serve this against a mounted EFS** |
| `umts_dm0` / `umts_csd` / `umts_cass` / `umts_router` / `umts_ramdump0` | DM / CSD / CASS / router / dump |
| `gnss_ipc` | GNSS (also OFFLINE) |
| `ipc_loopback0` | loopback |
| `radio0` | **FM V4L** (major 81), not cellular |

No `/dev/block` in this ramdisk. GPT names are in sysfs `uevent` `PARTNAME`.

### Partitions (this unit, list only)

| Part | Name | Size | Touch |
|------|------|------|--------|
| `mmcblk0p1` | `efs` | 20 MiB | **ro,noload** list 2026-08-31; umounted; never write |
| `mmcblk0p2` | `sec_efs` | 20 MiB | not mounted this pass; never write |
| `mmcblk0p4` | `cpefs` | 8 MiB | not mounted this pass; never write |
| `mmcblk0p22` | `radio` | 50 MiB | **read only** (259:14) |
| `mmcblk0p36` | `cp_debug` | 5 MiB | do not write |

Also `param` p6, `up_param` p13, `boot` p18, `super` p31. Matches [partitions.md](partitions.md) dump sizes.

### Platform

`cpif`, `cp_shmem`, `gnssif`, `11920000.cp_mailbox`, `11980000.gnss_mailbox`. SHMEM map from dmesg: CP `0xd0000000` 0x06900000, VSS, IPC `0xd7000000` 8 MiB, BTL.

**Do not write** `/sys/devices/platform/cpif/do_cp_crash` (write-only).

There is **no** sysfs `online` that downloads CP firmware. Load path is `cbd` → `IOCTL_START_CP_BOOTLOADER` on `umts_boot0`, image from RADIO.

## How Android starts it (vendor.img, read only)

Do **not** run these on the phone.

`init.baseband.rc`:

```text
symlink /dev/block/by-name/radio /dev/mbin0
service cpboot-daemon /vendor/bin/cbd -d -tss310 -bm -mm -P by-name/radio
```

Also writes `modem_ctrl_s5000ap/parameters/ds_detect` from `ro.vendor.multisim.simslotcount`. Chowns `/mnt/vendor/efs/factory.prop`.

`vendor.sem.rilchip.rc`: `ril-daemon` = `/vendor/bin/hw/rild`; `onrestart restart cpboot-daemon`. **Do not start rild.**

`init.exynos850.rc` mounts **sec_efs** on `/efs` and chowns `/mnt/vendor/efs` / `cpefs`. `init.vendor.onebinary.rc` **copies** `factory.prop` from EFS. Leave that unmounted.

`cbd` on this vendor: **151784** bytes, **CBD-20220120R1**, getopt `hdt:s:b:m:n:o:p:P:B:D:T:`. `-t ss310` (string “SS310 modem”), `-P by-name/radio`, boot node `/dev/umts_boot0`. It also opens **EFS NV** (`/mnt/vendor/efs/nv_data.bin`, `nv_5g_data.bin`, `nv_protected.bin`, `nv_normal.bin`) and **`fsync(nv_fd)`**. That is an EFS write. Do not mount EFS to “help” it.

`libsec-ril.so` / `libril_sem.so` stay Android HAL. Not this phase.

## RADIO TOC LIVE (v031, 2026-08-29, full 5 records)

Re-read sysfs before `mknod`: `/sys/class/block/mmcblk0p22` `PARTNAME=radio` `DEV=259:14` `size=102400` (50 MiB). Node already present from the 64-byte pass. `hexdump -C -n 160` and `-n 1040`. EFS **not** mounted. No `dd` of RADIO blobs into the repo.

32-byte records: `name[12] b_off m_off size crc idx` (LE). `toc[0].idx` is the count (**5**). Kernel CPIF does not parse this.

```text
00000000  54 4f 43 00 00 00 00 00  00 00 00 00 00 00 00 00  |TOC.............|
00000010  00 80 00 40 10 04 00 00  00 00 00 00 05 00 00 00  |...@............|
00000020  42 4f 4f 54 00 00 00 00  00 00 00 00 20 04 00 00  |BOOT........ ...|
00000030  00 00 00 40 a8 1c 00 00  42 b4 00 af 01 00 00 00  |...@....B.......|
00000040  4d 41 49 4e 00 00 00 00  00 00 00 00 e0 20 00 00  |MAIN......... ..|
00000050  00 00 01 40 c8 4d 56 02  21 9b a6 c5 02 00 00 00  |...@.MV.!.......|
00000060  56 53 53 00 00 00 00 00  00 00 00 00 c0 6e 56 02  |VSS..........nV.|
00000070  00 00 90 46 14 05 55 00  48 99 66 37 03 00 00 00  |...F..U.H.f7....|
00000080  4e 56 00 00 00 00 00 00  00 00 00 00 00 00 00 00  |NV..............|
00000090  00 00 30 46 00 00 10 00  00 00 00 00 04 00 00 00  |..0F............|
```

| i | name | b_off (RADIO) | m_off (CP) | size | crc | idx |
|--|--|--|--|--|--|--|
| 0 | `TOC` | `0x0` | `0x40008000` | `0x410` (1040) | `0` | 5 (count) |
| 1 | `BOOT` | `0x420` | `0x40000000` | `0x1ca8` (7336) | `0xaf00b442` | 1 |
| 2 | `MAIN` | `0x20e0` | `0x40010000` | `0x02564dc8` (37.3 MiB) | `0xc5a69b21` | 2 |
| 3 | `VSS` | `0x02566ec0` | `0x46900000` | `0x00550514` (5.3 MiB) | `0x37669948` | 3 |
| 4 | `NV` | **`0x0`** | `0x46300000` | `0x00100000` (1 MiB) | `0` | 4 |

`MAIN` is the CP image (RADIO). `NV` is a **placeholder** — `b_off=0` means it is **not** stored on RADIO. Vendor `cbd` strings `NV_NORM` / `NV_PROT` are EFS files (`/mnt/vendor/efs/nv_normal.bin`, `nv_protected.bin`, plus `nv_data.bin` / `nv_5g_data.bin`), not TOC names on this unit. At 0xa0 (past official count 5) there is an `OFFSET` record (`b_off=size=0x80000`); not a sixth firmware stage.

BOOT is 7 KiB at RADIO+`0x420`. MAIN starts at `0x20e0` and is most of the 50 MiB.

## Vendor `cbd` (R620 `vendor.img`, read only)

`/bin/cbd` 151784 bytes, **CBD-20220120R1**, Android 33 PIE, interp `/system/bin/linker64`. NEEDED: `liblog` `libcutils` `libc` `libc++` `libm` `libdl`. getopt `hdt:s:b:m:n:o:p:P:B:D:T:`. Published usage: `-d` daemon, `-t` type, `-b` boot link (`m`=SHMEM), `-m` main link, `-o u|t|r` (upload-test / Tegra / root), `-p` partition#, `-B`/`-D` devices. Hidden: `-P` (Android uses `-P by-name/radio`), `-n` (`%s/nv_data.bin`). **No dry-run. No skip-EFS.** NV file open fail is `ERR`. `fsync(nv_fd)` on those EFS paths. `invalid TOC : There is no NV` is a **TOC** check (we have an `NV` record) — not “EFS missing”.

`init.baseband.rc`: `symlink /dev/block/by-name/radio /dev/mbin0`; `service cpboot-daemon /vendor/bin/cbd -d -tss310 -bm -mm -P by-name/radio`. Vendor `lib64` has no bionic/`linker64` (those live in system inside `super.img`).

**LIVE wget:** `/tmp/cbd` from Windows RNDIS `192.168.42.17:8765`. `/tmp/cbd -h` → ash **not found** (no `linker64`). Did **not** run vendor `cbd`. EFS still unmounted.

## CP load LIVE (no EFS, no vendor `cbd`)

Static `/tmp/radio-boot` (R620 `aarch64-linux-gnu-gcc -static`, wget same HTTP). RADIO **O_RDONLY**. Never opened EFS / `umts_rfs0`. No pack.

1. `status`: `IOCTL_GET_CPIF_VERSION` = `CPIF-200511N220408`. `GET_CP_STATUS` 0. `modem_state=OFFLINE`.
2. `load` (BOOT+MAIN, skip NV): `POWER_ON` (PMUCAL `CP_STATUS` 0→1), `POWER_RESET`, `REQ_SECURITY` RE_INIT = `CP_NOT_WORKING` (11, expected before images), `LOAD_CP_IMAGE` BOOT+MAIN **OK**, skip NV (`b_off=0`), `REQ_SECURITY` NORMAL = **`CP_NO_ERROR`** (SMC `mode=0xd0000000` boot `0x1ca8` main `0x2564dc8`), `START_CP_BOOTLOADER` **OK**. **`OFFLINE` → `BOOTING`**. `start_normal_boot: cp_status=1`. `COMPLETE_NORMAL_BOOTUP` **timeout 15s** (`EAGAIN` / `T-I-M-E-O-U-T`). State stayed **`BOOTING`**. RNDIS stayed up. VSS not loaded (first pass).
3. `loadnv` (same + 1 MiB **zero** NV in RAM, not EFS): zeros loaded (`rel=0x6300000`). `POWER_ON` from BOOTING forced software `OFFLINE`; `POWER_RESET` then **no-op** (`already offline`) so CP HW was not reset. `START` failed `cp_status error:0` / EPERM. Complete timed out again. Still **`BOOTING`**.

Not **ONLINE**. `complete_normal_boot` waits for CP IPC (`init_cmpl`). That did not arrive without real EFS NV and/or vendor `cbd` handshake/`umts_rfs0`. Stopped — next would be EFS or `rild`.

## EFS read-only list LIVE (v031, 2026-08-31)

User-approved one-shot. **`ro,noload` only.** No journal replay. **Umounted after listing** so a later `cbd` cannot write. Did **not** start `cbd` / `rild`. No IMEI / NV payload in this doc.

Re-read sysfs `uevent` immediately before `mknod`:

| Part | PARTNAME | MAJOR:MINOR | sectors | FS (magic @ 0x438) |
|------|----------|-------------|---------|---------------------|
| `mmcblk0p1` | `efs` | **179:1** | 40960 (20 MiB) | ext4 `53 ef` |
| `mmcblk0p2` | `sec_efs` | **179:2** | 40960 (20 MiB) | ext4 `53 ef` |
| `mmcblk0p4` | `cpefs` | **179:4** | 16384 (8 MiB) | ext4 `53 ef` |

Ramdisk already had `/dev/mmcblk0pN`. Created `/dev/block/` and `mknod` only the missing Android-style nodes: `/dev/block/mmcblk0p1` b 179 1, `p2` b 179 2, `p4` b 179 4.

```text
mkdir -p /mnt/efs-ro
mount -t ext4 -o ro,noload /dev/block/mmcblk0p1 /mnt/efs-ro
```

**Success** (RC 0). `/proc/mounts`: `ext4 ro,relatime,norecovery,i_version`. dmesg: `EXT4-fs (mmcblk0p1): mounted filesystem without journal. Opts: noload`. `sec_efs` / `cpefs` not mounted (efs succeeded).

NV / wifi on **efs** — names and byte sizes only (no contents, no IMEI):

| Path under `/mnt/efs-ro` | Bytes |
|--------------------------|-------|
| `nv_data.bin` | 1048576 |
| `nv_data.bin.md5` | 32 |
| `.nv_data.bak` | 1048576 |
| `.nv_data.bak.md5` | 32 |
| `.nv_state` | 1 |
| `nv.log` | 6026 |
| `wifi/.mac.info` | 17 |
| `wifi/.mac.cob` | 17 |

**Absent** on efs: `nv_normal.bin`, `nv_protected.bin`, `nv_5g_data.bin` (vendor `cbd` still names those under `/mnt/vendor/efs/`).

`umount /mnt/efs-ro` RC 0. `/proc/mounts` has no efs. `cbd`/`rild` never started.

## Userdata NV copy LIVE (v031, 2026-08-31)

User-approved: experiment only on a **copy**. Original efs / sec_efs / cpefs / radio / boot / vbmeta **never written**. No IMEI / NV payload in this doc. No flash. No commit.

### userdata

Re-read sysfs before `mknod`: `mmcblk0p38` `PARTNAME=userdata` **259:30** `size=46735360` (~22.3 GiB). Existing contents would not mount (`f2fs`/`ext4` EINVAL; dmesg `Can't find valid F2FS filesystem` — superblocks look encrypted/garbage).

**Formatted userdata** (Android data gone): BusyBox `mke2fs -F -L saaios-ud /dev/block/mmcblk0p38 262144` (256 MiB ext2). Mounted RW `/mnt/userdata`. Only this partition was formatted.

### Copy (efs RO → userdata folder)

Remounted original efs **`ro,noload`** at `/mnt/efs-ro` only long enough to `cp`. Then **umounted**. Files under `/mnt/userdata/saaios-efs-copy/` (names + sizes; `cmp` vs efs all 0):

| Name | Bytes |
|------|-------|
| `nv_data.bin` | 1048576 |
| `nv_data.bin.md5` | 32 |
| `.nv_data.bak` | 1048576 |
| `.nv_data.bak.md5` | 32 |
| `.nv_state` | 1 |
| `nv.log` | 6026 |
| `wifi/.mac.info` | 17 |
| `wifi/.mac.cob` | 17 |

`cp` of `nv_data.bin` into `/tmp/nv_data.bin` (tmpfs) for the loader. Did **not** `dd` the efs partition into the Windows repo.

### `loadnv` with real NV (copy, not efs)

`/tmp/radio-boot loadnv /tmp/nv_data.bin` — RADIO `O_RDONLY`; NV bytes from the userdata copy in RAM. Original efs **unmounted**. No `cbd` / `rild`.

- `POWER_ON` OK (PMUCAL `CP_STATUS` 0→1). `POWER_RESET` logged `already offline` (no-op). `REQ_SECURITY` RE_INIT = `CP_NOT_WORKING` (11). BOOT+MAIN **OK**. **NV-file ioctl OK** (`rel=0x6300000` size `0x100000`). `REQ_SECURITY` NORMAL = **`CP_NO_ERROR`**. `START` **OK**. **`OFFLINE` → `BOOTING`**.
- `COMPLETE_NORMAL_BOOTUP` **timeout 15s** (`EAGAIN` / `T-I-M-E-O-U-T`). State stayed **`BOOTING`**. Not **ONLINE**. GNSS still `OFFLINE`.
- A later `IOCTL_POWER_OFF` dropped the box (tmpfs cleared; userdata copy survived). After remount of the ext2 userdata, the same `loadnv` from the copy was repeated: same **BOOTING** / complete timeout. Did not remount original efs after that reboot.

Same end state as zero-NV. Real 1 MiB NV in SHMEM is not enough for `init_cmpl` without vendor `cbd` / `umts_rfs0`. Did **not** bind-mount the copy onto `/mnt/vendor/efs` and did **not** start `cbd` (still no `linker64`).

## DXJ2 ioctl: no `IOCTL_MODEM_DL_START` name

Local tree `os/third_party/kernel_samsung_a12` was missing this pass; macros confirmed from the same CPIF sources ([maazm7d/kernel_samsung_a12](https://github.com/maazm7d/kernel_samsung_a12) `drivers/soc/samsung/cpif`, Samsung OSS import).

`modem_prj.h` — **no** `#define IOCTL_MODEM_DL_START`. Nr gap after `GET_CP_STATUS`:

```c
/* modem_prj.h:81-85 */
#define IOCTL_POWER_RESET		_IOW(IOCTL_MAGIC, 0x21, struct boot_mode)
#define IOCTL_START_CP_BOOTLOADER	_IOW(IOCTL_MAGIC, 0x22, struct boot_mode)
#define IOCTL_COMPLETE_NORMAL_BOOTUP	_IO(IOCTL_MAGIC, 0x23)
#define IOCTL_GET_CP_STATUS		_IO(IOCTL_MAGIC, 0x27)
#define IOCTL_START_CP_DUMP		_IO(IOCTL_MAGIC, 0x32)
```

Analogous SS310 is `_IO('o', 0x28)` (do not hardcode `0x6f28`). `bootdump_ioctl` default forwards unknown cmds to `shmem_ioctl`; that switch has no 0x28 case (`link_device.c` `default: invalid cmd` → `-EINVAL`).

`complete_normal_boot` (`modem_ctrl_s5000ap.c`) only `wait_for_completion_timeout(&mc->init_cmpl, MIF_INIT_TIMEOUT)` — EAGAIN on miss. It does **not** do the UDL handshake.

`IOCTL_START_CP_BOOTLOADER` is **`_IOW('o', 0x22, struct boot_mode)`** (typed). Kernel `copy_from_user`s `struct boot_mode` and only then calls `start_normal_boot()`. Not bare `_IO`.

`rild_ready` (`link_device.c:431`): **`PROTOCOL_SIT` returns true with no ipc0/rfs0 check.** Default (SIPC) is true only when **both** `umts_ipc0` and `umts_rfs0` have `opened > 0`. Then `CMD_PHONE_START` from CP gets `CMD_INIT_END` back.

**LIVE stock DTB (v031, 2026-09-01):** `/sys/firmware/devicetree/base/cpif/mif,protocol` = `00 00 00 00` → **`PROTOCOL_SIPC` (0)**, not SIT (1). gnssif has no `protocol` / `sit` cell. ipc0/rfs0 hold is **not** a SIT red herring on this unit. Do **not** add `DL_START` / `0x6f28` to radio-boot. `os/build/e4-radio-boot.c` already passes `&bm` with `.idx = CP_BOOT_MODE_NORMAL` for START/RESET.

Command/response on `umts_boot0` is **4-byte LE**.

## UDL handshake LIVE (v031, 2026-09-01)

User protocol after `START_CP_BOOTLOADER`: `DL_START` + write `0x0000900d` / expect `0x0000a00d` + write `0x00009f00` / expect `0x0000af00`, then `COMPLETE`. Hold `umts_ipc0` and `umts_rfs0` `O_RDWR|O_NONBLOCK` (no read/write) from before START until COMPLETE returns. RADIO `O_RDONLY`. NV from **userdata copy** only. **No** `IOCTL_POWER_OFF`. **No** EFS remount (copy already present). **No** vendor `cbd` / `linker64`. **No** RFS server.

Static `/tmp/radio-boot` rebuilt Zig musl `aarch64-linux-musl -static` (1075064). wget RNDIS `192.168.42.10:8765`. Telnet `:23`.

Start state: CP already **`BOOTING`** from the prior session. `POWER_RESET` logged **`already offline`** (no-op; ioctl still returned 0). Did **not** `POWER_OFF`.

| Step | Result |
|------|--------|
| `POWER_ON` | OK. PMUCAL `CP_STATUS` already 1 |
| `POWER_RESET` NORMAL | ioctl OK; dmesg `already offline` (no-op) |
| `REQ_SECURITY` RE_INIT | OK (rc=0) |
| LOAD BOOT+MAIN | OK. RADIO `O_RDONLY` |
| LOAD NV | OK. `/mnt/userdata/saaios-efs-copy/nv_data.bin` 1 MiB, `rel=0x6300000`. Original efs **not mounted** |
| `REQ_SECURITY` NORMAL | OK |
| open ipc0/rfs0 | **OK** fd 5 / 6. dmesg `umts_ipc0 (opened 1)` `umts_rfs0 (opened 1)` |
| `START_CP_BOOTLOADER` | **EPERM** (13). `start_normal_boot: cp_status error:0` after ~4 s (RESET no-op left HW `cp_status=0`). SHMEM magic still `0x424F4F54` |
| `IOCTL_MODEM_DL_START` `_IO('o',0x28)` | **EINVAL** (22). `shmem_ioctl: invalid cmd 0x00006F28` |
| UDL `0x0000900d` → read | **`0x0000a00d` ACK** |
| UDL `0x00009f00` → read | **`0x0000af00` ACK** |
| `COMPLETE_NORMAL_BOOTUP` | **OK** (rc=0), not EAGAIN. `BOOTING` → **`ONLINE`**. `GET_CP_STATUS` = 4 |
| VSS | **skipped** — handshake ACKed and COMPLETE did not time out |
| GNSS | still `OFFLINE` |

CP then sent `INIT_START` / `CP_START`. `rild_ready` saw both fds open and sent `INIT_END`. `complete_normal_boot` got `init_cmpl` in ~0.7 s.

Fds dropped after COMPLETE (hold-until-result only). Wrote **nothing** to ipc0/rfs0. `/proc/mounts` has userdata ext2 only — **no efs**. No IMEI / NV payload in this doc.

## VSS load LIVE (v031, 2026-09-01)

First CP unit with VSS on the MAIN `LOAD_CP_IMAGE` path (`mode=0`, rel `m_off`) **before** START / UDL / COMPLETE. DXJ2 `link_load_cp_image` maps `boot_size = SHMEM_CP + SHMEM_VSS`, so `rel=0x06900000` size `0x00550514` is in range. **No** `DL_START` added (probe still EINVAL). ipc0/rfs0 held across START. RADIO `O_RDONLY`. NV from **userdata copy** only. **No** `IOCTL_POWER_OFF`. **No** EFS remount. **No** vendor `cbd` / `linker64`. **No** pack / flash / commit.

Static `/tmp/radio-boot` rebuilt R620 `aarch64-linux-gnu-gcc -static -O2` (706128). wget RNDIS `192.168.42.10:8765`. Telnet `:23`. userdata already ext2 at `/mnt/userdata` (`nv_data.bin` 1 MiB present). Original efs **not mounted**.

Start: **`CRASH_EXIT`**. GNSS **`FAULT`**. `rmnet0` down, rx=0 tx=0.

| Step | Result |
|------|--------|
| `POWER_ON` | OK (rc=0) |
| `POWER_RESET` NORMAL | OK (rc=0). From `CRASH_EXIT` this is **not** the `already offline` no-op (that path is `STATE_OFFLINE` only). dmesg `CP aleady Init` vs skip **not captured** (WDT flooded the ring) |
| `REQ_SECURITY` RE_INIT | OK |
| LOAD BOOT+MAIN | OK. RADIO `O_RDONLY` |
| LOAD **VSS** | **OK** (rc=0). `b_off=0x02566ec0` `m_off=0x46900000` `rel=0x6900000` `size=0x550514` |
| LOAD NV | OK. `/mnt/userdata/saaios-efs-copy/nv_data.bin` 1 MiB, `rel=0x6300000`. Original efs **not mounted** |
| `REQ_SECURITY` NORMAL | OK |
| open ipc0/rfs0 | **OK** fd 5 / 6 |
| `START_CP_BOOTLOADER` | **EPERM** (13). `after-start` **`BOOTING`** (`change_modem_state` happens before the cp_status wait; errno 13 is `cp_status error:0` / `-EACCES`) |
| `IOCTL_MODEM_DL_START` `_IO('o',0x28)` | **EINVAL** (22) |
| UDL `0x0000900d` / `0x00009f00` | **timeout** (poll 3 s, have=0). No A00D/AF00. Unlike the leftover-BOOTING pass, where RESET was a no-op and UDL still ACKed |
| `COMPLETE_NORMAL_BOOTUP` | **EAGAIN** (11). dmesg `complete_normal_boot: T-I-M-E-O-U-T` then `umts_ipc0 (opened 0)` `umts_rfs0 (opened 0)` |
| after COMPLETE | **`BOOTING`**. `GET_CP_STATUS` = 3 |
| ~minutes later | still **`BOOTING`**. Not `CRASH_EXIT`. No `PHONE_START` / `INIT_END` in the remaining dmesg |
| `rmnet0` | down, rx=0 tx=0 |
| GNSS | still **`FAULT`** |

VSS ioctl is **not** the crash discriminator: CP did not `CRASH_EXIT` after this load. Completing boot still needs CP to set mailbox `cp_status` (START) and then UDL/`init_cmpl`. Leftover ONLINE was a no-reset bootloader; this pass started from `CRASH_EXIT` and UDL was silent.

WDT keepalive flooded `dmesg` (ring starts as watchdog-only again). Catch `start_normal_boot` / `POWER_RESET` lines during the next load, not after.

No IMEI / NV payload in this doc.

## AP reboot wait LIVE (v031, 2026-09-01)

Diagnosis from VSS pass: `POWER_RESET` from `CRASH_EXIT` may not re-init CP PMU (`_is_first_boot` already 1 → `Not first time, but power is down`). Product path is an **AP reboot** (hold Power 2s). This pass: telnet `192.168.42.1:23` only. **No** `IOCTL_POWER_OFF`. **No** load. **No** EFS. **No** `cbd`/`rild`. **No** `usb-host`. **No** flash / pack / commit.

Polled for a **new** boot (SaaiOS banner **v031**, `modem_state=OFFLINE`, dmesg from kernel start). Phone did **not** reboot itself.

| Probe | uptime (s) | `modem_state` | notes |
|-------|------------|---------------|-------|
| 1 | 43704.14 | `BOOTING` | BusyBox ash banner (not a fresh v031). Kernel `4.19.111-27127798` |
| 2 | 43725.29 | `BOOTING` | GNSS **`FAULT`**. dmesg head is WDT keepalive only (ring starts ~43544 s) |
| 3 | 43769.52 | `BOOTING` | `/tmp/radio-boot` still present (706128, Jan 2 05:17) |
| 4 | 43809.49 | `BOOTING` | `/tmp/radio-boot status`: `GET_CPIF_VERSION` `CPIF-200511N220408`, **`GET_CP_STATUS` = 3**. `rmnet0` rx=0 tx=0. userdata ext2 mounted at `/mnt/userdata`. Original efs **not** mounted |

dmesg grep `cal_cp_status|CP aleady Init|try reset|cp_status error|PHONE_START|INIT_END|start_normal_boot|complete_normal_boot|Not first time|POWER_RESET`: **empty**. WDT flooded the ring (same as the VSS pass). Cannot confirm PMU `_is_first_boot` / `CP aleady Init` from this buffer.

**Stopped.** Human must hold **Power 2s**, then say the phone is back. Next load (`/tmp/radio-boot` BOOT+MAIN+VSS+NV userdata copy, UDL, ipc0/rfs0 hold, COMPLETE) only on a **fresh** `OFFLINE` boot with early dmesg.

## One-shot `/tmp/cp-boot.sh` + CP PMU reinit (v031, 2026-09-01)

Telnet `192.168.42.1:23` immediately. **Not** a fresh boot — did **not** run `loadnv`. **No** `IOCTL_POWER_OFF`. **No** EFS. **No** `cbd`. **No** flash / pack / commit. R620 `ssh -b 192.168.168.150 home-mike` failed this pass (WiFi is `192.168.88.246`; bind `192.168.168.150` unknown). CPIF quotes from [maazm7d/kernel_samsung_a12](https://github.com/maazm7d/kernel_samsung_a12) `drivers/soc/samsung/cpif` (same DXJ2 import as the OSS tarball).

| Probe | uptime (s) | `modem_state` | notes |
|-------|------------|---------------|-------|
| 5 | 44047.25 | `BOOTING` | BusyBox ash, kernel `4.19.111-27127798`. `/tmp/radio-boot` **706128** (VSS-capable). userdata NV 1 MiB present. dmesg still WDT-only |

### Staged (survives Power 2s on userdata)

| Path | Bytes | Role |
|------|-------|------|
| `/tmp/radio-boot` | 706128 | VSS helper (`grep -a 'no VSS TOC'` **VSS_OK**) |
| `/tmp/cp-boot.sh` | 2200 | LF (`0a` after `#!/bin/sh`). wget `192.168.42.10:8765/cp-boot.sh` |
| `/mnt/userdata/radio-boot` | 706128 | copy — `/tmp` is empty after reboot |
| `/mnt/userdata/cp-boot.sh` | 2200 | copy |
| `/mnt/userdata/saaios-efs-copy/nv_data.bin` | 1048576 | unchanged |

`/proc/mounts`: userdata ext2 only. **No efs.** After Power 2s: `sh /mnt/userdata/cp-boot.sh` (script remounts userdata if needed, copies helper from userdata or wget RNDIS, `loadnv` that copy, dumps `modem_state` + dmesg grep). Wifi not required.

### DXJ2: no userspace CP PMU re-init after first boot

`cal_cp_init()` is the PMU first-init. The only caller in CPIF is `power_on_cp`, and only when `_is_first_boot` is still 0:

```c
/* modem_ctrl_s5000ap.c:294-320 */
static int _is_first_boot;
static int power_on_cp(struct modem_ctl *mc)
{
	...
	change_modem_state(mc, STATE_OFFLINE);
	if (cal_cp_status() == 0) {
		if (!_is_first_boot) {
			mif_info("First init\n");
			cal_cp_disable_dump_pc_no_pg();
			cal_cp_init();
			_is_first_boot = 1;
		} else {
			mif_err("Not first time, but power is down\n");
		}
	}
	return 0;
}
```

Userspace ioctls (`bootdump_io_device.c`):

| ioctl | lines | ops |
|-------|-------|-----|
| `IOCTL_POWER_ON` | 172–178 | `mc->ops.power_on` → `power_on_cp` |
| `IOCTL_POWER_OFF` | 180–186 | `mc->ops.power_off` → `power_off_cp` (`cal_cp_reset_assert` only; **forbidden**) |
| `IOCTL_POWER_RESET` | 188–207 | `power_reset_cp`: if `STATE_OFFLINE` **return 0** (`already offline`, 378–380); else if `cal_cp_status()` then `cal_cp_reset_assert`/`release` (`CP aleady Init, try reset`, 395–406). **Never** `cal_cp_init()` |

`power_on_cp` always sets `STATE_OFFLINE` **before** RESET. So `radio-boot`'s POWER_ON then POWER_RESET sequence **cannot** HW-reset after first boot — RESET no-ops. `_is_first_boot` is a static; POWER_OFF does not clear it. A later POWER_ON with `cal_cp_status()==0` only logs `Not first time, but power is down`.

CPIF sysfs (`modem_main.c:680-685`): `do_cp_crash` (WO, forbidden) and `modem_state` (RO). `ds_detect` is SIM. **No** sysfs/debugfs that calls `cal_cp_init` / `pmucal_cp_init`. `pmucal_cp.c` exposes init/status/reset as kernel CAL only.

**No userspace path to re-init CP PMU after first boot besides `IOCTL_POWER_OFF` (forbidden; does not restore `_is_first_boot` anyway) and AP reboot.** Power 2s remains the gate. Did not try POWER_OFF. Did not re-run `loadnv` on this 12 h `BOOTING` (RESET would no-op again).

### Probe 6 (2026-09-01 12:04) — skip load

Telnet `192.168.42.1:23` one shot. **No** `cp-boot.sh` / `loadnv`. **No** `POWER_OFF`. **No** EFS. **No** `cbd`. **No** `sipcinit` (CP not ONLINE). R620 `ssh -b 192.168.168.150 home-mike` failed (`bind 192.168.168.150: Unknown error`). CPIF quotes from [maazm7d/kernel_samsung_a12](https://github.com/maazm7d/kernel_samsung_a12) `drivers/soc/samsung/cpif`.

| | |
|--|--|
| uptime | **44597.86** s (~12.4 h) |
| `modem_state` | **`BOOTING`** |
| GNSS | **`FAULT`** |
| kernel | `4.19.111-27127798` (same v031, not a fresh banner) |
| `rmnet0` | rx=0 tx=0 |
| userdata | ext2 mounted; `cp-boot.sh` / `radio-boot` / NV copy still on `/mnt/userdata` |
| `/tmp/radio-boot` | 706128 (old VSS helper; **not** rebuilt this pass) |
| dmesg `PHONE_START`/`INIT_END`/timeout/crash | **empty** (WDT flooded) |

Still ~12 h `BOOTING`. Did **not** poll. Did **not** load.

### Probe 7 (2026-09-01 12:09–12:22) — rebuild sipcinit onto userdata

Telnet `192.168.42.1:23` first. **Not** a fresh boot — **did not** run `loadnv` / `cp-boot.sh`. **No** `POWER_OFF`. **No** EFS. **No** `cbd`. **No** BCMD/GNSS. R620 `ssh home-mike` and `ssh -b 192.168.168.150` unreachable (`laptop-wg` still steals `192.168.168.0/24`; WiFi is `192.168.88.246`; bind `192.168.168.150` absent). Rebuilt locally Zig 0.13 `aarch64-linux-musl -static -O2` (`os/build/e4-radio-boot.c`). wget RNDIS `192.168.42.10:8765`.

| | |
|--|--|
| uptime (probe) | **44899.42** s then **45637.72** s (~12.7 h) |
| `modem_state` | **`BOOTING`** |
| GNSS | **`FAULT`** |
| kernel | `4.19.111-27127798` (same v031) |
| load | **not run** |

### Staged (survives Power 2s on userdata)

| Path | Bytes | Role |
|------|-------|------|
| `/tmp/radio-boot` | **1080336** | sipcinit helper (Zig musl). Phone strings: `sipcinit` **OK**, `no VSS TOC` **OK**, no `write ipc0 INIT` |
| `/tmp/cp-boot.sh` | 2426 | LF (`0a` after `#!/bin/sh`). Prefers `$UD/radio-boot` over leftover `/tmp` |
| `/mnt/userdata/radio-boot` | **1080336** | same binary |
| `/mnt/userdata/cp-boot.sh` | 2426 | same script |
| `/mnt/userdata/saaios-efs-copy/nv_data.bin` | 1048576 | unchanged |

`/proc/mounts`: userdata ext2 only. **No efs.** After sysrq-b / Power 2s: `sh /mnt/userdata/cp-boot.sh` (mknod p38+p22, remounts userdata if needed, copies helper from userdata, refuses a binary without VSS/`sipcinit`/`never close`, `loadnv` that NV copy, dumps `modem_state` + dmesg grep). COMPLETE→ONLINE then **forks a holder that never closes** ipc0+rfs0 (**no** umts_ipc0 write).

### Probe 8 (2026-09-01 ~12:24–12:40) — sysrq-b, 35 s sipcinit, then DROP

Linux `echo b > /proc/sysrq-trigger` (not `IOCTL_POWER_OFF`). Telnet back ~1 min. Banner v031, small uptime, **`modem_state=OFFLINE`**, `power_on_cp: First init`. Helper still **1080336** (35 s hold then drop). `mknod` p38 + p22 `259:14`. `sh /mnt/userdata/cp-boot.sh`.

BOOT+MAIN+VSS+NV OK → START OK → UDL ACK → **COMPLETE → ONLINE** (`GET_CP_STATUS=4`). dmesg: `rild_ready: umts_ipc0.opened=1, umts_rfs0.opened=1` then `cmd_phone_start_handler: shmem: INIT_END -> s318ap`. sipcinit held 35 s, ONLINE on every sample, then **DROP ipc0/rfs0**. **`CP_CRASH_EXIT` ~41 s after the drop** (ONLINE window ~76 s, COMPLETE ~t=290 through crash t=367). `rmnet0–7` rx=tx=0. End state **`CRASH_EXIT`**. Goal not complete — fds were not held.

### Probe 9 (2026-09-01 12:44–12:55) — never-close holder

Rebuilt Zig musl `aarch64-linux-musl -static -O2` (`os/build/e4-radio-boot.c`): after ONLINE, **fork + `setsid` + ignore SIGHUP**, sleep forever, **never close** ipc0/rfs0. Parent returns. `/tmp/sipc-holder.pid`. `cp-boot.sh` 2860 LF (`0a` after `#!/bin/sh`); mknod radio p22; refuses old 35 s binary (`never close` string). wget RNDIS (host was `192.168.42.15` this boot). **No** `POWER_OFF`. **No** EFS. **No** `cbd` / `rild`. **No** usb-host.

sysrq-b again. Fresh v031, uptime 82 s, **`OFFLINE`**, GNSS **`OFFLINE`**. mknod userdata p38 + radio p22, mount userdata. First `cp-boot` telnet closed mid-LOAD BOOT (still `OFFLINE`; `First init` at 133.61). Second `sh /mnt/userdata/cp-boot.sh` (uptime 163.7): `POWER_RESET already offline`, LOAD all OK, UDL ACK, **COMPLETE → ONLINE** (164.75), `GET_CP_STATUS=4`.

| | |
|--|--|
| `/mnt/userdata/radio-boot` | **1121480** |
| holder PID | **332** (`/tmp/radio-boot loadnv …`) |
| holder fds | **5 → `/dev/umts_ipc0`**, **6 → `/dev/umts_rfs0`** (still open after crash) |

dmesg (captured before WDT flooded the ring): `ipc_open` opened=1 both; `rild_ready: umts_ipc0.opened=1, umts_rfs0.opened=1`; **`INIT_END -> s318ap`**.

#### `modem_state` timeline (kernel uptime s)

| sample | uptime | `modem_state` | notes |
|--------|--------|---------------|-------|
| fresh boot | 82.39 | `OFFLINE` | v031, GNSS OFFLINE |
| COMPLETE / INIT_END | 164.75 | `ONLINE` | holder forked 332 |
| t0 | 181.40 | `ONLINE` | fds 5/6 still ipc0/rfs0; rmnet 0 |
| t30 | 211.96 | `ONLINE` | same holder; rmnet 0 |
| t60 | 242.46 | **`CRASH_EXIT`** | holder **still** has ipc0+rfs0; rmnet 0 |
| t90 | 272.99 | `CRASH_EXIT` | same |
| end | 291+ | `CRASH_EXIT` | WDT flooded dmesg; no crash line left |

ONLINE did **not** hold 90 s (dropped between 212 and 242, ~48–78 s after COMPLETE). Holding ipc0/rfs0 **does not** stop `CRASH_EXIT`. `rmnet0`/`rmnet1`/`rmnet7` rx=tx=0. **No stable ONLINE. Goal not complete.** Remaining: no rmnet/IP; CP still dies without `rild` / RFS server (not started).

## INIT_END / PHONE_START ABI (DXJ2 SIPC, after ONLINE)

Stock `mif,protocol=0` = **`PROTOCOL_SIPC`**. `CMD_PHONE_START` / `CMD_INIT_END` are **mailbox IRQ commands**, not a write on `/dev/umts_ipc0`. Userspace action is **open and hold** `umts_ipc0` + `umts_rfs0`. Kernel prepends any SIPC5 FMT header on ipc0 writes (`cfg` start mask `0xF8`, `ch=SIPC5_CH_ID_FMT_0` = 235). **Do not write** `0x0002` / `0x0008` onto ipc0 (that is FMT payload, not INIT_END). **Do not write** `umts_rfs0` (RFS/NV). **Do not run** this against `BOOTING`.

Mailbox IDs (`link_device_memory.h:83-90`): `CMD_INIT_START=0x0001`, **`CMD_INIT_END=0x0002`**, **`CMD_PHONE_START=0x0008`**, `CMD_PIF_INIT_DONE=0x000D`. `cmd2int` (`:476-479`) = `MASK_INT_VALID\|MASK_CMD_VALID\|cmd` → INIT_END IRQ **`0x00C2`**.

CP → AP: `shmem_cmd_handler` (`link_device.c:746-757`) `CMD_PHONE_START` → `cmd_phone_start_handler` (`:574-690`). Log `CP_START <-`. If `rild_ready` (SIPC: both **ipc0 and rfs0** `opened > 0`, `:431-462`), kernel `send_ipc_irq(mld, cmd2int(CMD_INIT_END))` and logs `INIT_END ->`. First PHONE_START also `complete_all(&mc->init_cmpl)` / `LINK_STATE_IPC` (`:684-686`). If AP never sends INIT_END, CP retries `CP_START` while already ONLINE (`:632-651`). After `init_end_cnt > 0`, extra `CP_START` is **Abnormal** (then crash).

AP → CP without waiting for another PHONE_START: `ipc_open` (`ipc_io_device.c:37-58`) calls `ld->init_comm` → `shmem_init_comm` (`link_device.c:1961-2014`). If CP is **already ONLINE** and `init_end_cnt==0` and the other of fmt/rfs is open, kernel sends INIT_END on that open (`:2004-2007`).

Leftover ONLINE pass and Probe 8 used this: fds held across START → kernel sent INIT_END → COMPLETE in ~0.7 s. Probe 8 then **dropped** fds after 35 s → `CRASH_EXIT` ~41 s later. Probe 9 **never closes** (holder PID 332); INIT_END still happened; CP **still** `CRASH_EXIT` ~48–78 s after COMPLETE with fds open. `radio-boot sipcinit` (gitignored `os/build/e4-radio-boot.c`) forks a holder, writes **nothing**. Standalone `sipcinit` **refuses** unless `modem_state=ONLINE`.

Cellular complete only if **current** `modem_state=ONLINE` (not `CRASH_EXIT`) **and** (`PHONE_START`/`INIT_END` this boot **or** rmnet rx/tx ≠ 0). Probe 10: **ONLINE** + INIT_END past 90 s.

## Probe 10 — rfs loop vs userdata NV copy (v031, 2026-09-01)

Previous probe 9: holder PID 332 never-close ipc0+rfs0, **no read**. ONLINE ~48–78 s then `CRASH_EXIT`. This pass: same boot path + **drain ipc0** + **serve umts_rfs0** from `/mnt/userdata/saaios-efs-copy` only.

### Crash evidence (leftover probe-9 boot, before sysrq-b)

Telnet `192.168.42.1:23`. Uptime **693 s**, `modem_state=CRASH_EXIT`, holder **332** still had fds 5/6 ipc0/rfs0. `rmnet0/1/7` rx=tx=0. Original efs **not** mounted. Copy present.

**dmesg ring wrapped.** `dmesg | wc -l` = 15314. Earliest remaining line **t=512 s** (AP `[Exynos][WDT]` keepalive flood). Crash window was **212–242 s**. **No** `CP_CRASH` / `nv_rebuild` / `assert` / `dump` / `INIT_END` left in the buffer. Crash **reason not in dmesg** — WDT wrapped it. Open fds without a server still died.

### What CP wants after INIT_END (DXJ2 cpif)

`umts_rfs0` is IPC_RFS ch **245**, misc, `ATTR_SBD_IPC|ATTR_SIPC5`. Kernel **strips SIPC5 on read, prepends on write** (`skb_pull` / `sipc5_build_header`). **No in-kernel RFS filesystem** — packets go to userspace. Same for FMT on `umts_ipc0` (ch 235). `cbd`/`rild` would serve `/mnt/vendor/efs/nv_data.bin` etc. We bind-mount the **copy** there after original efs is unmounted.

FMT: CP sends `sipc_fmt_hdr` (le16 len matches `read()` n). Almost all **type=3 NOTI** (`AST_POWERON`, net/call status). Drain is enough not to fill the queue. One **type=1 EXEC** (`main=0x0e sub=0x03`) was not ACKed; CP stayed ONLINE. Dummy FMT RESP not sent (guessed cmd_type can crash CP).

RFS: classic 6-byte `rfs_hdr` (`u32 size, u8 cmd, u8 id`) is what libsec-ril used. LIVE: many `read()`s are **2040-byte** SBD cells (not a single `rfs_hdr`). Parser treated those as commands (cmd 0x02/0x14/0x11 noise). A few later packets have plausible sizes (**22 / 39** with cmd **0x11 OPEN**). Path layout after flags is **not** fully correct yet (`OPEN fail` on 1-byte junk names). Writes only inside the copy. Kernel did accept our rfs writes: `ipc_write: umts_rfs0: wait for INIT_END done (150ms) cnt:1 last:0 cmd:0xC2`.

### Boot

sysrq-b. Fresh `OFFLINE` uptime 87 s (first `cp-boot.sh` failed: userdata not mounted yet). Second shot uptime **200 s OFFLINE**. Bind `saaios-efs-copy` → `/mnt/vendor/efs`. `radio-boot` **1308440** Zig musl. LOAD BOOT+MAIN+VSS+NV OK. UDL ACK. **COMPLETE → ONLINE**. Holder **319** rfs loop, fds 5/6 + `/tmp/rfs.log`. **No** `POWER_OFF`. **No** original efs. **No** `cbd`/`rild`.

| t (uptime) | state | notes |
|------------|-------|-------|
| 200.5 | OFFLINE | cp-boot start |
| 201.59 | ONLINE | `rild_ready` + `INIT_END -> s318ap` |
| 341 | ONLINE | ~139 s after COMPLETE; rfs_rx=70 ipc_rx=64 |
| 390 | ONLINE | ~188 s after COMPLETE; rfs_rx=70 ipc_rx=69; rmnet 0 |

**90 s ONLINE held.** Goal **complete**. rmnet still 0.

## Probe 11 — data plane / RFS SBD framing (v031, 2026-09-01)

New goal: CP stably ONLINE **and** (`rmnet*` rx/tx ≠ 0 or IPv4 on rmnet). Probe 10 closed ONLINE+INIT_END with rmnet 0.

### Live start (no reboot)

Telnet `192.168.42.1:23`. Uptime **1494–1530 s**, **`ONLINE`**, holder **319**, `rmnet0–7` rx=tx=0, down, no IPv4. Bind still userdata copy → `/mnt/vendor/efs`. Original efs not mounted.

`/tmp/rfs.log`: **rfs_rx=70 rfs_tx=70** then frozen. **64/70** reads were **2040-byte** SBD cells (cmd/id noise). Six shorter sizes (22 / 39 / 54 / 102 / 110 / 432). Two real-looking **OPEN** (`cmd=0x11` size **39** id=1, size **22** id=2) parsed path as flags+cstring → 1-byte names `0x19` / `0x08`. That **is** `u32 namelen` (25 / 8): flags + namelen + name. OPEN with `fl=0x42` created junk `?` in the **copy** (removed later; fd still held by 319). FMT drain continues (type=3 NOTI; one type=1 EXEC `main=0x0e sub=0x03` not ACKed).

### Helper

`os/build/e4-radio-boot.c` (gitignored): walk SBD cell for 6-byte `rfs_hdr`; do **not** treat n=2040 as one packet; do **not** reply to cells with no valid hdr. OPEN/CREATE: `u32 flags, u32 namelen, name[namelen]`. Serve copy only. Drain ipc0; no fake FMT EXEC. Zig musl static **1321928**. R620 ssh unreachable this pass (no `192.168.168.150`; `home-mike` timed out).

### Attach (ONLINE, no sysrq-b, no POWER_OFF)

`kill -STOP 319` (fds 5/6 stay open). wget RNDIS `192.168.42.12:8765`. First `sipcinit` hit busy `/tmp/radio-boot` (old mapping) → holder **502** (old parser) also STOP. Then `/mnt/userdata/radio-boot sipcinit` → holder **514**, fds 3/4 ipc0+rfs0, log `SBD cell vs 6-byte rfs_hdr`. dmesg `umts_ipc0/rfs0 (opened 3)`. **No** `IOCTL_POWER_OFF`. **No** original efs. **No** `cbd`/`rild`.

### 90 s sample (uptime 2182 / 2205)

| | |
|--|--|
| `modem_state` | **`ONLINE`** |
| holder 514 | `S` (sleeping), ipc0+rfs0 open |
| 319 / 502 | `T` (stopped), fds still open |
| `rmnet0/1/2/7` | rx=tx=0, bytes=0, **down**, no IPv4 |
| new rfs_rx/tx | **0** (CP did not send more RFS this boot) |
| ipc drain | FMT `0x07/0x06` type=3 (periodic); no new RFS skip lines |

dmesg also `rx_demux: shmem: ERR! umts_ipc1 is not opened` (repeating). ipc1 was not held.

**Data-plane goal not complete.** rmnet still **0**. Framing/path fix is in the helper but was **not exercised on this boot** — the 70-cell RFS burst already finished before attach. Fresh CP boot (`sysrq-b`, not POWER_OFF) is what would run OPEN/READ through the new walker. Do not remount original EFS. Do not start `rild` / `cbd`.

## Probe 12 — FLL.bin copy + strict OPEN namelen (v031, 2026-09-01)

Goal: CP ONLINE **and** (`rmnet*` rx/tx ≠ 0 **from CP** or IPv4 on rmnet). Previous walker from t=0: OPEN `'Jf'` misparse, `err/csdiag_mmrj_Info.dat` denied, `FLL.bin` ENOENT, no READ.

### Where `FLL.bin` lived

R620 `home-mike` **unreachable** this pass (`192.168.168.150` not on this laptop; WiFi `192.168.88.246`; `laptop-wg` still steals `192.168.168.0/24`; `jump-wg` timed out). No `debugfs` of `vendor.img` / `odm.img`.

Phone **ro,noload** (never write p1/p2/p4), then umounted:

| Part | Path | `FLL.bin` / `err/` / `csdiag` |
|------|------|-------------------------------|
| `cpefs` p4 | `.nv_core.bak` 524288 + md5 only | **absent** |
| `sec_efs` p2 | FactoryApp / imei / `sec_efs/` … | **absent** |
| **`efs` p1** | **`/root/FLL.bin` 144 bytes** | **here.** No `err/` / `csdiag*` |

CP OPEN path is `FLL.bin` (no prefix). Copied p1 `root/FLL.bin` → userdata copy `/FLL.bin` (`cmp` OK) and `/root/FLL.bin`. Also copied cpefs `.nv_core.bak` into the copy. `err/` created empty in the copy (CP OPEN `fl=0x42` = creat). Original partitions umounted.

### Helper

`os/build/e4-radio-boot.c`: OPEN/CREATE is **only** `u32 flags + u32 namelen + name[namelen]` (namelen ≥ 3). Do **not** cstring-fallback the namelen field (`'Jf'`). Bad namelen: **no reply**. `map_path` `mkdir_p` parents under the copy so `err/` is allowed. Zig musl static **1314320**. `cp-boot.sh` refuses a binary without `OPEN flags+u32 namelen`.

### Boot

Pushed binary + script to userdata while leftover ONLINE, then `printf b > /proc/sysrq-trigger` (not POWER_OFF). Fresh `OFFLINE` uptime 82 s. `sh /mnt/userdata/cp-boot.sh`. GNSS off. No `cbd`/`rild`/`usb-host`.

COMPLETE **101.0 s** → **ONLINE**. `INIT_END -> s318ap`. Holder **321** ipc0+rfs0. ipc1 holder **309**.

`ifconfig rmnet0 up` **rc=0**. Iface UP, IPv6 `fe80::200:ff:fe00:0/64`, **no IPv4**. AP then sent a few 48-byte tx frames (link-local). **rx stayed 0.**

### 90 s sample (uptime)

| | uptime | modem | rfs_rx/tx | rmnet0 | rmnet1–7 |
|--|--|--|--|--|--|
| COMPLETE | 101.0 | ONLINE | starting | 0/0 down | 0 |
| T0 + ifconfig | 137.1 | ONLINE | 7/3 | rx=0 tx=1 UP | 0, down |
| ~T30 | 188.4 | ONLINE | 21/14 | rx=0 tx=4 | 0 |
| ~T60 | 243.7 | ONLINE | 21/14 frozen | rx=0 tx=5 | 0 |
| ~T90 | 285.3 | ONLINE | 21/14 | **rx=0 tx=6** (288 B) | **0/0 down** |

`crash=0`. Original efs **not** mounted. Only IPv4 is USB `rndis0` `192.168.42.1`.

### RFS walker from t=0

1. cmd=0x11 id=0 size=40 namelen `0x664a` (`'Jf'`) → **skip, no reply**
2. `'err/csdiag_mmrj_Info.dat'` namelen=24 fl=0x42 → **OPEN h=1** (0-byte file in copy)
3. `'FLL.bin'` namelen=7 fl=0 → **OPEN h=2**, **READ n=144 want=144**

One bogus NV_READ (`off=469762052`) skipped. rfs froze at **21/14**. FMT drain continues.

**Data-plane goal not complete.** ONLINE + FLL READ yes; rmnet **rx=0**, no IPv4 on rmnet. `ifconfig up` only produced AP tx. Next: FMT/PDN (not `rild`) or more RFS files CP may still want. Do not remount original EFS. Do not start `rild` / `cbd`.

## Probe 13 — Replicant GPRS DEFINE_PDP + PDP_CONTEXT (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 **from CP** or IPv4 on rmnet, still ONLINE. EXEC **0x0e/0x03** is STK `IPC_SAT_PROACTIVE_CMD` (payload SET UP EVENT LIST). **No** TERMINAL RESPONSE (replicant `sat.h` has no safe no-op). No original efs. No `POWER_OFF`. No flash. No `usb-host`. No `rild`.

### Start (leftover ONLINE, no reboot)

Telnet `192.168.42.1:23`. Uptime **547 s**, **`ONLINE`**, holder **428** ipc0+rfs0, ipc drain `0x07/0x06` type=3. `rmnet0` UP IPv6 `fe80::…` only, rx=tx=0. Bind userdata copy → `/mnt/vendor/efs`. Original efs **not** mounted.

Static `/tmp/gprs-pdp` (`os/build/e4-gprs-pdp.c`, Zig musl). FMT only on `umts_ipc0` (kernel prepends SIPC5). Holder **STOP/CONT** (fds stay open). cid **1**. aseq **0xff**, type **SET 0x03** (`ipc_client_send` / `ipc_gprs_*_setup`).

### Packets (exact replicant)

`ipc_fmt_header` (7) + payload. DEFINE: enable=1 cid=1 magic=**0x02** apn[124]. ACTIVATE: enable=1 cid=1, username/password **NULL** (magic1/magic2 stay 0).

| | hex |
|--|--|
| DEFINE empty 134 | `86 00 01 ff 0d 01 03 01 01 02` + 124×`00` |
| ACTIVATE 110 | `6e 00 02 ff 0d 04 03 01 01` + 101×`00` |
| DEFINE `internet` 134 | `86 00 01 ff 0d 01 03 01 01 02 69 6e 74 65 72 6e 65 74` + 116×`00` |

### CP RESP (same for both APNs)

| RX | decode |
|--|--|
| `0c 00 … 80 01 02 0d 01 03 00 80` | `IPC_GEN_PHONE_RES` RESP for DEFINE SET, code **`0x8000` SUCCESS** |
| `0c 00 … 80 01 02 0d 04 03 00 80` | `IPC_GEN_PHONE_RES` RESP for PDP_CONTEXT SET, code **`0x8000` SUCCESS** |
| `11 00 … 0d 10 03 01 03 00 …` | `IPC_GPRS_CALL_STATUS` NOTI cid=1 status=**DISABLED (0x03)** fail=**NONE** |
| — | no `IPC_GPRS_IP_CONFIGURATION` `0x0D09` |

Did **not** ACK STK `0x0e/0x03`. After CONT, holder still drains `0x07/0x06`. **No `CRASH_EXIT`.** Stopped — no more guessed FMT (no `NET_REGIST` / `GPRS_PS` / `PORT_LIST`).

### 90 s samples

| | uptime | modem | rmnet0–7 |
|--|--|--|--|
| empty T0–T90 | 787–877 | ONLINE | rx=tx=0 |
| internet T0–T90 | 935–1025 | ONLINE | rx=tx=0 |

`rmnet0`/`rmnet1` IPv6 link-local only. No IPv4. Holder **428** still open. dmesg: `ipc_open`/`ipc_release` by `gprs-pdp` only (opened 2→1).

**Data-plane goal not complete.** FMT accepted; bearer **DISABLED**. Next discriminator is PS/NET attach (not this pass). Do not remount original EFS. Do not start `rild` / `cbd`.

## Probe 14 — PIN / NET_REGIST GET, then PLMN_SEL + GPRS_PS + PDP (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 **from CP** or IPv4 on rmnet, still ONLINE. No original efs. No `POWER_OFF`. No flash. No `usb-host`. No `rild`. No STK TR.

### Start (leftover ONLINE, no reboot)

Telnet `192.168.42.1:23`. Uptime **1164 s**, **`ONLINE`**, holder **428** ipc0+rfs0. `ds_detect=2`. ipc1 holder 309 **gone**. `rmnet0` UP IPv6 `fe80::…` only, rx=tx=0. Static `/tmp/net-get` (`os/build/e4-net-get.c`, Zig musl **1157800**). Holder STOP/CONT. Dual SIM: `/dev/umts_ipc0` then `/dev/umts_ipc1`.

### GET (Replicant `ipc_fmt_send_get` / `ipc_net_regist_setup`)

| TX | hex |
|--|--|
| `IPC_SEC_PIN_STATUS` GET | `07 00 … 05 01 02` (empty) |
| `IPC_PWR_PHONE_STATE` GET | `07 00 … 01 07 02` (empty) |
| `IPC_NET_REGIST` GET GSM | `09 00 … 08 05 02 ff 02` act=UNKNOWN domain=GSM |
| `IPC_NET_REGIST` GET GPRS | `09 00 … 08 05 02 ff 03` act=UNKNOWN domain=GPRS |
| `IPC_GPRS_DEFINE_PDP_CONTEXT` GET | `07 00 … 0d 01 02` (empty) |
| `IPC_GPRS_PS` GET | `07 00 … 0d 03 02` (empty) |

No `IPC_NET_ATTACH` in libsamsung-ipc. No `NET_REGIST` SET (samsung-ril only GETs it). Did **not** ACK STK.

### SIM1 / ipc0

| | hex | decode |
|--|--|--|
| PIN | `09 00 … 05 01 02 **00 00**` | status=**0x00 READY**, lock=**0x00 SC_UNLOCKED**. Not PIN. |
| RADIO | `08 00 … 01 07 02 **02**` | **NORMAL** (0x02). No `PHONE_STATE` EXEC. |
| DEFINE GET | FMT len **0x0572** (1394) split across reads | **not reassembled** this pass |
| `GPRS_PS` GET (late) | `09 00 … 0d 03 02 **00 00**` | cid=0 attached=**0** |

### SIM2 / ipc1

PIN/REGIST GET: **no RESP**. Leftover `IPC_PWR_PHONE_PWR_UP` NOTI `01 01`. Other NOTI `0x0d0f` / `0x0511` / `0x050c` / `0x0510` / `0x2601` (not in replicant GET list; ignored). No PIN brute.

### Unregistered → documented attach, then PDP

CS not HOME/ROAMING → `IPC_NET_PLMN_SEL` SET AUTO (`ipc_net_plmn_sel_setup`, 8 B: mode=0x02 plmn=0 act=0xFF). Then `IPC_GPRS_PS` SET cid=1 attached=1. Then empty-APN DEFINE+ACTIVATE (no new APN; `internet` already failed). Kyivstar APN **not** sent.

| RX | decode |
|--|--|
| `1b 00 … 08 05 02 01 02 **04** … fail **02**` plen=20 (ril struct is 11) | CS **EMERGENCY (0x04)** act=GSM2 lac=**0xe345** cid=**0x00001c4a** fail=0x02 |
| `1b 00 … 08 05 02 01 03 **01** … fail **07**` | PS **NONE (0x01)** same lac/cid fail=0x07 |
| `0c 00 … 80 01 02 08 02 03 **64 00**` | `PLMN_SEL` SET **GEN_PHONE_RES 0x0064** (not 0x8000) |
| `0c 00 … 80 01 02 0d 03 03 **00 80**` | `GPRS_PS` SET **0x8000 SUCCESS** |
| `09 00 … 0d 03 02 00 00` | GET still attached=**0** |
| `21 00 … 08 03 03 … 32 35 35 30 33 23 …` | `IPC_NET_SERVING_NETWORK` NOTI PLMN ASCII **`25503#`** (MCC 255 MNC 03, UA Kyivstar) |
| DEFINE SET + PDP SET | both **0x8000** |
| `11 00 … 0d 10 03 01 03 00 …` | `CALL_STATUS` cid=1 **DISABLED (0x03)** fail=**NONE** |
| — | no `IP_CONFIGURATION` |

### 90 s samples (still ONLINE)

| | uptime | modem | rmnet0–7 |
|--|--|--|--|
| T0 | 1468 | ONLINE | rx=tx=0 |
| T30 | 1498 | ONLINE | 0 |
| T60 | 1528 | ONLINE | 0 |
| T90 | 1558 | ONLINE | 0 |

Holder **428** CONT, ipc0+rfs0 still open. Uptime **1608 s** still **ONLINE**. `rmnet0` IPv6 link-local only. No IPv4.

**Data-plane goal not complete.** SIM READY + radio NORMAL, but CS is **emergency camp** on Kyivstar and PS is **NONE**, so PDP stays DISABLED. Next is why IMSI attach is emergency-only (not another APN). **Do not ACK STK.** **Do not brute PIN.**

## Probe 16 — ipc1 GPRS_PS + NET_REGIST GPRS SET (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 from CP or IPv4 on rmnet, still ONLINE. Work **ipc1 / SIM2** only (SIM1 still Kyivstar emergency). No original efs. No `POWER_OFF`. No flash. No `usb-host`. No `rild`. No PIN. No STK TR. No Kyivstar APN.

### Start (leftover ONLINE, no reboot)

Telnet `192.168.42.1:23`. Holder **428** ipc0+rfs0. ipc1 holder **416** (`sh cp-boot.sh`, fd 3 = `umts_ipc1`, never-close). `rmnet0–7` rx=tx=0. Static `/tmp/gprs-ps` (`os/build/e4-gprs-ps.c`, Zig musl **1158552**). Holder 428 STOP/CONT. Did **not** STOP 416 (v1 STOP'd it and ipc1 then returned only `DISP_RSSI` 0x0706 — no FMT RESP).

Dual-SIM data pref: replicant `sec.h` has **no** data-slot SET. Shannon data slot is **`umts_ipc1`**. morphis names `IPC_NET_SERVICE_DOMAIN_CONFIG` 0x0808 / `POWERON_ATTACH` 0x0809 but **no payload struct** — not SET, not GET this pass.

### fail 0x07

Replicant `net.h` / morphis `rej_cause`: **no named enum** for `NET_REGIST` fail. `gprs.h` `IPC_GPRS_FAIL_USER_AUTHENTICATION` **0x0007** is **CALL_STATUS** (PDP) only. Same nibble as 3GPP TS 24.008 GMM / 24.301 EMM **#7 GPRS services not allowed** — matches PS NONE on a CS-HOME UMTS cell.

### ipc1 PS timeline (status hex)

All samples: serving ASCII `25501#` (Vodafone UA). `GPRS_PS` SET **GEN_PHONE_RES 0x8000**; GET **cid=0 attached=0**. `NET_REGIST` SET GPRS (act=UMTS and UNKNOWN) **0x8000**.

| t | CS | PS | GPRS_PS |
|--|--|--|--|
| baseline | **0x02 HOME** fail=0 act=UMTS lac=0x8dc3 cid=0x051d12fe | **0x01 NONE** fail=**0x07** | att=0 |
| after GPRS_PS SET | 0x02 HOME | 0x01 NONE fail=0x07 | att=0 |
| after NET_REGIST SET GPRS | 0x02 HOME | 0x01 NONE fail=0x07 | att=0 |
| T+0 … T+75 s (5 s GET) | 0x02 HOME | **0x01 NONE fail=0x07** (unchanged) | att=0 |

PS never HOME/ROAMING → **no DEFINE/ACTIVATE** (empty / `internet` skipped). Leftover `CALL_STATUS` DISABLED in the drain is from probe 15, not this pass.

### 90 s rmnet (still ONLINE)

| | uptime | modem | rmnet0–7 |
|--|--|--|--|
| T0 | 3230 | ONLINE | rx=tx=0 |
| T30 | 3260 | ONLINE | 0 |
| T60 | 3290 | ONLINE | 0 |
| T90 | 3320 | ONLINE | 0 |

Holder **428** CONT, ipc0+rfs0 still open. No IPv4 on rmnet. **Data-plane goal not complete.** Network rejected GPRS attach on SIM2 (`fail 0x07`) even though CS is HOME; `GPRS_PS` SET is ACK'd but attach does not stick. **Do not ACK STK.** **Do not brute PIN.**

## Probe 17 — ipc1 LTE/EPS NET_REGIST SET act=0x21 (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 from CP or IPv4 on rmnet, still ONLINE. Work **ipc1 / SIM2**. No original efs. No `POWER_OFF`. No flash. No `usb-host`. No `rild`. No PIN. No STK TR. No `GPRS_PS` SET (probe 16 already 0x8000 / attached=0). No `MODE_SEL` SET.

### act bytes

Replicant / LineageOS / GearCM / morphis ipc-v4 `net.h`: GSM `0x00` … UMTS `0x04` + UNKNOWN `0xFF` — **no LTE**. Live ipc1 `NET_REGIST` NOTI earlier this boot had act=**0x21** (vendor extra). That is the Shannon LTE byte used here. No other LTE act in those forks to SET.

### Start (leftover ONLINE, no reboot)

Telnet `192.168.42.1:23`. Holder **428** ipc0+rfs0. ipc1 holder **416**. Static `/tmp/lte-eps` (`os/build/e4-lte-eps.c`, Zig musl **1166008**). Holder 428 STOP/CONT. Did **not** STOP 416.

### TX

| | hex |
|--|--|
| GET GSM LTE | `09 00 … 08 05 02 **21** 02` |
| GET GPRS LTE | `09 00 … 08 05 02 **21** 03` |
| SET GSM LTE | `09 00 … 08 05 03 **21** 02` |
| SET GPRS LTE | `09 00 … 08 05 03 **21** 03` |

Both SET → `GEN_PHONE_RES` **0x8000 SUCCESS**.

### ipc1 90 s (status hex)

Serving ASCII `25501#` (Vodafone UA) every sample. GET/RESP **never** returned act=0x21 (`lte_seen=0`). CP stayed on UMTS.

| t | CS | PS | GPRS_PS |
|--|--|--|--|
| baseline (GET UNK + GET 0x21) | **0x02 HOME** fail=0 act=**UMTS 0x04** lac=0x8dc3 cid=0x051d12fe | **0x01 NONE** fail=**0x07** act=**0x04** | att=0 |
| after SET 0x21 | 0x02 HOME act=0x04 | 0x01 NONE fail=0x07 act=0x04 | att=0 |
| T+0 … T+90 s (5 s GET 0x21) | 0x02 HOME act=0x04 | **0x01 NONE fail=0x07** act=0x04 (unchanged) | att=0 |

PS/EPS never HOME → **no DEFINE/ACTIVATE**.

**PS regist hex** (plen=20, ril struct is 11): `04 03 01 b5 c3 8d fe 12 1d 05 07 c3 8d 02 02 01 ff ff 00 00`

**CS regist hex:** `04 02 02 b5 c3 8d fe 12 1d 05 00 c3 8d 02 02 01 ff ff 00 00`

`NET_REGIST` SET does not retune RAT (same as probe 16 SET UMTS/UNKNOWN). GMM #7 is on the serving UMTS cell. Data plane is **operator/SIM reject**, not a missing APN.

### rmnet (still ONLINE)

| | uptime | modem | rmnet0–7 |
|--|--|--|--|
| snapshot | 3941 | ONLINE | rx=tx=0 |

Holder **428** CONT. Uptime **3995 s** still **ONLINE**. No IPv4. **Data-plane goal not complete.** **Do not ACK STK.** **Do not brute PIN.**

## Probe 18 — ipc1 MODE_SEL SET 0x07 then 0x04 (v031, 2026-09-01)

Vendor `libsec-ril` `IpcTxNetSetPreferredNetType` → `IPC_NET_MODE_SEL` 0x080A, 1-byte. Live GET this boot was **0x0b** (LTE_ONLY). SET bitmask **0x07** (LTE_GSM_WCDMA) → `GEN_PHONE_RES` **0x8000**; follow-up GET **0x03 GSM/UMTS**. Second SET **0x04** → **0x8001** — stop. RAT stayed UMTS; PS GMM#7. No GPRS/PDP this probe. **Do not SET 0x04 again.**

## Probe 19 — restore ipc1 MODE_SEL 0x0b (v031, 2026-09-01)

Undo probe 18. SET **0x0b** only on ipc1. Confirm GET. No 0x04. No GPRS_PS. No PDP. No EFS write. No `POWER_OFF`. No flash.

### Start (leftover ONLINE, no reboot)

Telnet `192.168.42.1:23`. Holder **428** ipc0+rfs0. Static `/tmp/net-mode` (`os/build/e4-net-mode.c`, Zig musl **1150736**). Holder 428 STOP/CONT. Did **not** STOP ipc1 holder.

### TX

| | hex |
|--|--|
| SET MODE_SEL 0x0b | `08 00 06 ff 08 0a 03 0b` (same framing as SET 0x07 that got 0x8000) |

`GEN_PHONE_RES` for cmd=0x080a type=3 **code=0x8000 SUCCESS**. Did not SET 0x04.

### ipc1

Serving ASCII `25501#` (Vodafone UA). Baseline GET `MODE_SEL` had drifted to **0x01** (not still 0x03).

| t | MODE_SEL | CS | PS |
|--|--|--|--|
| leftover NOTI | — | **0x02 HOME** fail=0 act=**UMTS 0x04** lac=0x8dc3 cid=0x051d12fd | **0x01 NONE** fail=**0x07** act=**0x04** |
| baseline GET | **0x01** | 0x02 HOME act=0x04 | 0x01 NONE fail=0x07 act=0x04 |
| after SET 0x0b (NOTI) | (res 0x8000) | still UMTS HOME (stale) | **act=0x21 LTE** status=**0x07** fail=0 lac=0 cid=0x06423748 |
| confirm GET | **0x0b** | 0x02 HOME act=**0x04** lac=0 cid=0x051d12fd | status=**0x07** fail=0 act=**0x04 UMTS** lac=0 cid=0x051d12fd |

PS GET after restore is **not** GMM#7 (`fail` 0). Status **0x07** is unnamed in replicant (`0x01` NONE … `0x06` ROAMING). Confirm GET RAT still **UMTS**. LTE only appeared as a SET-side NOTI (`lte_seen=1`).

No GPRS_PS SET. No PDP.

### rmnet (still ONLINE)

| | uptime | modem | rmnet0–7 |
|--|--|--|--|
| snapshot | 5158 | ONLINE | rx=tx=0 |

Holder **428** CONT. Still **ONLINE**. No IPv4. **Data-plane goal not complete.** **Do not SET 0x04.** **Do not ACK STK.** **Do not brute PIN.**

## Probe 20 — ipc1 GET-only poll, decode status 0x07 (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 from CP or IPv4 on rmnet, still ONLINE. **No** `MODE_SEL` SET (no 0x04, no 0x07). **No** `NET_REGIST` SET. **No** EFS. **No** `POWER_OFF`. **No** flash. **No** PIN. **No** STK TR.

### status 0x07

GearCM/Replicant `net.h`: `NONE 0x01` `HOME 0x02` **`SEARCHING 0x03`** `EMERGENCY 0x04` `UNKNOWN 0x05` `ROAMING 0x06`. **No 0x07.** Not SEARCHING. LTE is **act=0x21**, not a status.

Vendor `libsec-ril` (`IpcRxNetRegState`): logs `tempStatus %x, RegStatus %d` — IPC hex → Android `RegState`. No named string for 0x07. AOSP `RIL_RegState` is 0–5 then jumps to 10/12/13/14 (no 6). 3GPP TS 27.007 `+CEREG` stat **6** = “SMS only, home” if IPC = CREG+1 — best named guess, **unconfirmed in strings**.

### Start (leftover ONLINE, no reboot)

Telnet `192.168.42.1:23`. Holder **428** ipc0+rfs0. ipc1 holder **416**. Static `/tmp/net-poll` (`os/build/e4-net-poll.c`, Zig musl **1184952**). Holder 428 STOP/CONT. Did **not** STOP 416. Did **not** SET `MODE_SEL`.

### Leftover NOTI (before GET)

| | act | domain | status | fail | cid |
|--|--|--|--|--|--|
| LTE burst | **0x21** | **0x01** (not GSM/GPRS) | **0x01 NONE** | 0 | `0x06423748` |
| CS | 0x04 UMTS | GSM 0x02 | **0x02 HOME** | 0 | `0x051d12fd`→`fe` |
| PS | 0x04 UMTS | GPRS 0x03 | **0x01 NONE** | 0 then **0x07** | same |

Serving ASCII `25501#`. LTE NOTI **did not stick**. Probe 19 PS **status=0x07 fail=0** already gone.

### GET (no SET)

`MODE_SEL` GET **0x0b**. Radio NORMAL. `GPRS_PS` cid=0 attached=**0**.

### ipc1 90 s (every 5 s)

All GET samples identical. No `act=0x21` on GET (`lte_stuck=0`; `lte_seen=1` only from leftover NOTI).

| t | CS | PS | GPRS_PS | rmnet |
|--|--|--|--|--|
| leftover | HOME UMTS | NONE fail=0 then GMM#7 | — | 0 |
| T+0 … T+90 | **0x02 HOME** act=**UMTS 0x04** lac=0x8dc3 cid=0x051d12fe | **0x01 NONE fail=0x07** act=**0x04** | att=0 | rx=tx=0 |

PS/CS never HOME on LTE; GPRS not attached → **no DEFINE/ACTIVATE**.

### rmnet (still ONLINE)

| | uptime | modem | rmnet0–7 |
|--|--|--|--|
| T0 / T30 / T60 / T90 | ~5496–6115 | ONLINE | rx=tx=0 |

Holder **428** CONT. No IPv4. **Data-plane goal not complete.** **Do not SET MODE_SEL.** **Do not ACK STK.** **Do not brute PIN.**

## Probe 21 — SIM1/ipc0 Kyivstar 25503, wait CS HOME (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 from CP or IPv4 on rmnet, still ONLINE. Work **ipc0 / SIM1** only (SIM2 Vodafone GMM#7 left alone). No original efs. No `POWER_OFF`. No flash. No `usb-host`. No `rild`. No PIN. No STK TR. **No** `MODE_SEL` SET (no 0x04, no 0x07). **No** ipc1 GPRS.

Replicant `net.h` (`xc-racer99` / GearCM): `IPC_NET_PLMN_LIST` **0x0804** GET; `PLMN_SEL_AUTO` **0x02**, `PLMN_SEL_MANUAL` **0x03**; `ipc_net_plmn_sel_setup` pads PLMN with `#` to 6. Static `/tmp/sim1-kyiv` (`os/build/e4-sim1-kyiv.c`, Zig musl **1172584**). wget RNDIS `192.168.42.7:8765`.

### Start (leftover ONLINE, no reboot)

Telnet `192.168.42.1:23`. Holder **428** ipc0+rfs0 STOP/CONT. Did **not** open ipc1. Did **not** SET `MODE_SEL`. Did **not** SET `NET_REGIST`.

### GET

| | decode |
|--|--|
| PIN | **READY** 0x00, SC_UNLOCKED |
| RADIO | **NORMAL** 0x02 |
| CS | **EMERGENCY 0x04** act=**GSM2 0x01** lac=**0xe345** cid=**0x00001c4a** fail=**0x02** |
| PS | **NONE 0x01** same cell fail=**0x07** |
| SERVING | ASCII **`25503#`** (Kyivstar UA) |
| `PLMN_SEL` GET | **AUTO 0x02** |
| `PLMN_LIST` GET | `GEN_PHONE_RES` **0x0064** (no list body) |

### PLMN_SEL MANUAL 25503

Serving already `25503#` → SET MANUAL `03 32 35 35 30 33 23 ff` (`25503#` + act UNKNOWN). `GEN_PHONE_RES` **0x009e** (not 0x8000). Follow-up NOTI still CS EMERGENCY fail=0x02 / PS NONE fail=0x07.

### ipc0 90 s (every 5 s)

All GET samples identical. Never HOME/ROAMING → **no GPRS_PS / DEFINE / ACTIVATE**.

| t | CS | PS | serving |
|--|--|--|--|
| baseline … T+90 | **0x04 EMERGENCY** fail=**0x02** act=**GSM2** lac=0xe345 cid=0x1c4a | **0x01 NONE** fail=**0x07** act=0x01 | `25503#` |

### fail=0x02

Replicant `net.h` has **no** named `NET_REGIST` fail enum. Same nibble as 3GPP TS 24.008 MM/GMM **#2 IMSI unknown in HLR** (also: packet-only subscription). CS stays limited/emergency; IMSI attach rejected. (PS fail=0x07 remains GMM **#7 GPRS services not allowed**.)

### rmnet (still ONLINE)

| | uptime | modem | rmnet0–7 |
|--|--|--|--|
| pre / post / final | ~6419–6610 | ONLINE | rx=tx=0 |

Holder **428** CONT. No IPv4. **Data-plane goal not complete.** SIM1 did not leave emergency. **Do not SET MODE_SEL.** **Do not ACK STK.** **Do not brute PIN.**

## GNSS LIVE (v031, 2026-09-01)

Kepler is a **separate** SHMEM + mailbox processor (`samsung,exynos-gnss` / `gnssif`). Not a RADIO TOC stage. Not VSS. Kernel: `CONFIG_EXYNOS_GNSS_IF` → `drivers/soc/samsung/gnssif` (**GNSSIF-20200511R1**). Driver `gnss_interface`. No `request_firmware` (unlike Maxwell `mx140`).

### This unit

| | |
|--|--|
| `/dev/gnss_ipc` | `10:107` only. No `umts_gnss*` |
| `gnss_status` | was `OFFLINE`; after gnss-boot **`FAULT`** (see GNSS boot LIVE) |
| DT | `/gnssif` `okay`, `shmem,name=KEPLER`, `device_node_name=gnss_ipc`. `/gnss_mailbox` `11980000` size `0x180`. `reserved-memory/gnss_rmem` **`0xee000000` / 6 MiB** |
| DT code window | `shmem,gnss_code_offset=0` `shmem,gnss_code_size=0` — userspace chooses LOAD offset |
| SHMEM map | Reserved `0+0x600000`. Fault `0x200000+0x180000`. IPC `0x380000+0x80000` |
| Mailbox / IPC | after BCMD: CTRL0 `0x4`, CTRL3 `0xff`; TX/RX head/tail still **0** |
| IRQs | after boot: `gnss_mailbox` 1 (fault-info), `kepler_active` 2, `kepler_wdt` 0, `kepler_sw_init` 2 |
| Sysfs | `gnss_status`, `mbox/mbox_status`, `shmem/{map_info,shm_status}`. `power/` is runtime-PM only (`unsupported`). **No** download / firmware-path node. No `/sys/class/gnss` |
| Ramdisk fw | `/vendor/firmware` = ABOX only. `firmware_class.path=/vendor/firmware`. No `gnss*` / `kepler*` file |
| GPT | **no** `gnss` partition (p1–p38 named; radio is p22) |
| RADIO TOC | still **5**: TOC BOOT MAIN VSS NV. No GNSS/GPS name. `OFFSET` after count is not a stage |
| RADIO MAIN strings | CP-side only: `hal_SendCp2Gnss_TsyncIPC`, `GnssIf_CdmaFreqAid*`, `DSPTX_Gnss_*` (IRAT/coex). Not a Kepler image |
| dmesg | gnss-boot: LOAD + POWER_ON + `sw_init` + BCMD 0x4 + `ACTIVE` → FAULT |

### How it loads

Userspace (`gpsd`, not `cbd`) opens `/dev/gnss_ipc` and:

1. `GNSS_IOCTL_LOAD_FIRMWARE` (`'K', 0x01`) — copy a blob into `gnss_rmem` (`kepler_firmware_args`: size, offset, ptr).
2. `GNSS_IOCTL_REQ_BCMD` (`'K', 0x03`) — mailbox boot commands. **cmd 0x4** = branch (no return). `kepler_req_bcmd` **POWER_ON**s Kepler if still `OFFLINE` (own PMU, not CPIF).
3. Driver sets `ONLINE` at POWER_ON, then `kepler_active_isr` can immediately flip to **`FAULT`**. `READ_SHMEM_SIZE` is 6 MiB.

DXJ2 vendor.img (R620 `debugfs`, 2026-09-01) confirms the same layout: `/vendor/bin/hw/gpsd` + `gps.sh`, `/vendor/etc/gnss/gps.cfg`, `vendor.samsung.hardware.gnss@2.1-service`. **No** `gnss*.bin` / `kepler*` under `/firmware`. Kepler bytes are a **trailer on `gpsd`**, not a loose firmware file. See vendor dump below.

CP **ONLINE is not** what boots Kepler. Kepler IRQs stayed 0 while CP was ONLINE (UDL). CP MAIN has GNSS coexistence hooks; AGPS/SUPL would want CP + data. VSS is still CP-side (`m_off 0x46900000`) — do not load it for GNSS. **VSS / CP `CRASH_EXIT` is not this dump.**

Do **not** start `gpsd` / GNSS HAL (needs `linker64`, same class as `rild`). Static `/tmp/gnss-boot` did `LOAD_FIRMWARE` + `REQ_BCMD` — see GNSS boot LIVE.

## Vendor GNSS dump (2026-09-01) — DXJ2 `vendor.img`

R620 `home-mike` `192.168.168.110` (Linux `R620` 6.12.96). Image `os/build/stock-super/vendor.img` (503668736, ext4 `vendor`, last mounted `/vendor`). `/sbin/debugfs` listings + `dump` of text + extract of `gpsd` to R620 `/tmp/gnss-vendor-dump` only. **No** blob copied into the Windows git tree. **No** flash, pack, `gpsd`/`cbd`/`rild` start, EFS RW, `POWER_OFF`, `usb-host`, or Kepler `LOAD_FIRMWARE` on the phone.

Bare `ssh home-mike` times out: this laptop’s `laptop-wg` (`10.10.0.3`) has an on-link `192.168.168.0/24` metric **0** that steals the LAN. Worked with `ssh -b 192.168.168.150` (WiFi).

Vendor paths below are inside the image (`/bin` = `/vendor/bin` at runtime).

### File table

| Path | Size | Role |
|------|------|------|
| `/bin/hw/gpsd` | 4154608 | Kepler daemon. ELF 64 PIE aarch64, Android 33, interp **`/system/bin/linker64`**. SELinux `gpsd_exec`. **Firmware is a 1222032-byte trailer after the ELF** (`K102-` / `K103-` records), not a separate `.bin` |
| `/bin/hw/gps.sh` | 441 | wrapper: `exec /vendor/bin/hw/gpsd -c $CONFIGFILE` |
| `/bin/hw/vendor.samsung.hardware.gnss@2.1-service` | 11568 | HIDL GNSS service (`hal_gnss_default_exec`) |
| `/bin/gpsd` | **absent** | only `hw/gpsd` |
| `/etc/gnss/gps.cfg` | 1692 | production cfg. **No firmware path line** |
| `/etc/gnss/gps.debug.cfg` | 1691 | debug variant (`debug_enable=1`); also no fw path |
| `/etc/gnss/ca.pem` | 73298 | SUPL TLS |
| `/etc/gnss/deleteLogCommands` | 208 | log cleanup list |
| `/etc/gnss/gps.issuetracker.cfg` | **absent** | named by `gps.sh`, not on this image |
| `/etc/init/init.gps.rc` | 1243 | `chmod`/`chown` `/dev/gnss_ipc`; `service gpsd /vendor/bin/hw/gps.sh` |
| `/etc/init/vendor.samsung.hardware.gnss@2.1-service.rc` | 133 | `service sec_gnss_service` |
| `/lib64/hw/android.hardware.gnss@2.1-impl.so` | 768200 | AOSP GNSS HAL impl |
| `/lib64/hw/vendor.samsung.hardware.gnss@2.1-impl.so` | 451568 | Samsung GNSS HAL impl |
| `/lib64/libwrappergps.so` | 81200 | gps wrapper (also `/lib/libwrappergps.so` 46604) |
| `/lib64/vendor.samsung.hardware.gnss@2.1.so` | 198736 | Samsung HIDL stub (`@2.0.so` 308904) |
| `/firmware` | ABOX / camera / NFC / MFC only | **no** `gnss*` / `kepler*` / `gps*` |

Also present (HIDL stubs, not loaders): `/lib64/android.hardware.gnss@{1.0,1.1,2.0,2.1}.so` and measurement_corrections / visibility_control. No `kepler*` name anywhere under `/bin`, `/etc`, `/firmware`, `/lib64`.

### `gps.cfg` excerpt (firmware path lines: **none**)

Production file has RF / constellation / AGPS / geofence keys only. Nothing like `firmware=` / `.bin` / `kepler` / `/vendor/firmware`:

```text
gnss_data_dir=/data/vendor/gps
Sif_UseFwXtraInterface=0
GlueLayer_EnableGnssCfgInterface=1
RfPathLossDb_Ap=5
RfPathLossDb_Cp=5
Chip_Configuration_GNSSConstConstraintDef=0x202F
```

`gps.sh` only selects among `/vendor/etc/gnss/gps.cfg`, `gps.debug.cfg`, `gps.issuetracker.cfg`.

### `init.gps.rc` (excerpt)

```text
on post-fs-data
    chmod 0660 /dev/gnss_ipc
    chown system system /dev/gnss_ipc
    mkdir /data/vendor/gps 0771 system system
    mkdir /data/vendor/gps/sgee 0771 system system

service gpsd /vendor/bin/hw/gps.sh
    class main
    user gps
    group system inet net_raw wakelock
```

### Blob: inside `gpsd`, not a separate `.bin`

`file`: `ELF 64-bit LSB pie executable, ARM aarch64`, dynamically linked, **`/system/bin/linker64`**, Android 33 NDK r25, stripped. NEEDED: `libc++` `liblog` `libm` `libandroid_net` `libssl` `libcrypto` `libz` `libdl` `libc`.

ELF phdrs end at `0x2cb798`; file is 4154608. **Trailer 1222032 bytes** starts `K102-0000…004` `K103-0000…004`. One ELF magic (offset 0). No `gnss.bin` / `kepler.bin` string. `strings` (requested filter):

```text
/dev/gnss_ipc
/vendor/etc/gnss/gps.cfg
/vendor/etc/gnss/ca.pem
/vendor/lib64/libwrappergps.so
/data/vendor/gps
failure reading Kepler firmware
Not code-loading kepler as no firmware given, should we just RUN it?
BCMD to codeload Kepler - IOCTL failed
Sending firmware to Kepler driver failed
Kepler Skipping Code Load
get_kepler_patch
lal_codeload_load
```

(`LOAD_FIRMWARE` as a C string is **absent** — ioctl is numeric `'K',0x01`. `.bin` hits are SUPL/aid `xtraee.bin` / `cm_tcxo.bin` / `ism*.bin` / `%s/%s%s.bin`, not Kepler.)

### Kepler without packing Android linker?

**Static ioctl helper, not `linker64` `gpsd`.** Vendor `gpsd` is the same class as `cbd`/`rild` (Android 33 PIE). Kernel `LOAD_FIRMWARE` + `REQ_BCMD` do not need it. The bytes to feed the ioctl are the **`gpsd` trailer** (carve on R620 `/tmp`, do not commit). Do **not** start `gpsd`.

## GNSS boot LIVE (v031, 2026-09-01)

Static `/tmp/gnss-boot` (`os/build/e4-gnss-boot.c`, gitignored). R620 `aarch64-linux-gnu-gcc -static` (712944). wget RNDIS `192.168.42.10:8765`. Telnet `:23`. **No** vendor `gpsd` / `linker64`. **No** EFS mount. **No** `cbd` / `rild`. **No** `POWER_OFF`. **No** pack. **No** flash. **No** commit. Firmware blob stayed on R620 `/tmp` + phone `/tmp` — not in git.

### Trailer carve (R620 `/tmp`)

`debugfs` extract `/tmp/gnss-vendor-dump/extract/gpsd` (4154608). ELF last `PT_LOAD` ends `0x2CB798`; section headers `shoff=0x2CB8A0` `shnum=27` end at **`0x2CBF60`**. Trailer starts there:

| | |
|--|--|
| File offset | **`0x2CBF60`** (2932576) |
| Length | **1222032** (`0x12A590`) |
| `+0x00` (32) | ASCII `K102-000000000000000000000000004` |
| `+0x20` (32) | ASCII `K103-000000000000000000000000004` |
| `+0x40` | payload, first LE words `0xFAEEF02A` `0x000002EC` `0x018CBA80` `0x000009C4` |

Later `K102` hits at trailer `+0x943F0` / `+0x122440` are **string tables inside the image** (`K100`/`K140`/`Harrier`), not extra records. One blob after two 32-byte stamps. The trailing `4` on both stamps matches BCMD branch `0x4`. Sizes are **not** encoded in the ASCII (all zeros except that `4`).

`dd`/`python` skip `0x2CBF60` → `/tmp/kepler-fw.bin` (1222032). SHA256 `0c84daeb4049784466e2fd1cfa7980ab92092d2d419f9c6f5858b709acf9069c`.

### Image identity LIVE (2026-09-01) — identification only, no BCMD

Re-dump of `/bin/hw/gpsd` from DXJ2 `vendor.img` (R620 debugfs) SHA256 `ccd3021b41fb1e08b1469c305a41abacaf791c9f1dc2bd5ca96e8210239dd08f` matches the earlier extract. Trailer skip `0x2CBF60` count 1222032 vs `/tmp/kepler-fw.bin` **byte-identical**. Phone `/tmp/kepler-fw.bin` same SHA-256. `file` = data (not ELF). Header: 32B `K102-…004` + 32B `K103-…004` + payload `2a f0 ee fa`. These **0x12A590 bytes ARE the DXJ2 gpsd Kepler trailer.**

Wider stock search (vendor.img RO loop + debugfs; odm.img; `/bin` via sudo): **no** separate `gnss.bin` / `kepler.bin` / `gps.default.so`. `/firmware` is ABOX/camera/NFC/MFC only. HAL strings: `lal_codeload_add_embedded`, `xport_kepdrv_load_firmware` / `xport_kepdrv_read_firmware` — blob is embedded in `gpsd`, not a path in `gps.cfg`. `system.img` / `product.img` not present as unpacked files (super.img + vendor + odm only).

DXJ2 OSS `Kernel.tar.gz` gnssif **GNSSIF-20200511R1**: `gnss_probe` does **not** `request_firmware` / hibernation download (same as community tree). Hibernation-at-probe: **no**.

`LOAD_FIRMWARE` is `copy_from_user` into mapped `gnss_rmem` (`shmem_copy_reserved_from_user`); dmesg size/offset only, **no CRC**. `map_info` is layout (`Reserved 0+0x600000`), not contents. **Readback is possible** via existing `GNSS_IOCTL_READ_FIRMWARE` (`'K',0x04` → `copy_reserved_to_user`).

### rmem readback LIVE (2026-09-01) — no BCMD

Fresh AP boot, GNSS `OFFLINE` (Kepler never powered this boot). wget helper `1206272` + `/tmp/kepler-fw.bin` SHA-256 `0c84daeb4049784466e2fd1cfa7980ab92092d2d419f9c6f5858b709acf9069c`. `gnss-boot readfw`: RESET skipped (already OFFLINE). `LOAD_FIRMWARE` **1222032 @ 0** `rc=0`. `READ_FIRMWARE` **64** then **1222032** `rc=0` (ioctl `0x4b04`, struct 16 / ptr +8). **No** `/dev/mem`. **No** BCMD. Status stayed **`OFFLINE`**. Mailbox CTRL all 0. CP left **`ONLINE`**.

64-byte file and rmem (same):

```text
0000  4b 31 30 32 2d 30 30 30 30 30 30 30 30 30 30 30  |K102-00000000000|
0010  30 30 30 30 30 30 30 30 30 30 30 30 30 30 30 34  |0000000000000004|
0020  4b 31 30 33 2d 30 30 30 30 30 30 30 30 30 30 30  |K103-00000000000|
0030  30 30 30 30 30 30 30 30 30 30 30 30 30 30 30 34  |0000000000000004|
```

| | |
|--|--|
| match64 | **yes** |
| classify | **K102** (not zeros / not garbage) |
| file SHA-256 | `0c84daeb4049784466e2fd1cfa7980ab92092d2d419f9c6f5858b709acf9069c` |
| rmem SHA-256 | `0c84daeb4049784466e2fd1cfa7980ab92092d2d419f9c6f5858b709acf9069c` |
| match_full_sha | **yes** |

Firmware **is in reserved RAM**. FAULT after BCMD is not a failed copy. Next discriminator is boot path (BCMD param / `req_security` TZPC on POWER_ON / gpsd extra ioctls), not the blob. **Do not fire BCMD this note.**

gnssif userspace ioctls (`gnss_prj.h` / `gnss_io_device.c`): RESET, LOAD, FAULT, BCMD, **READ_FIRMWARE**, CHANGE_SENSOR_GPIO, CHANGE_TCXO_MODE, SET_SENSOR_POWER, SET_WATCHDOG_RESET, READ_SHMEM_SIZE, READ_RESET_COUNT, GET_SWREG, GET_APREG. **No `GNSS_IOCTL_SECURITY`.** `req_security` is kernel `gnss_request_tzpc` SMC inside `kepler_power_on` / `kepler_release_reset` (BCMD path if OFFLINE/HOLD_RESET). `copy_reserved_*` is memcpy only — no verify.

gpsd strings (DXJ2 vendor dump, R620; not run): `xport_kepdrv_load_firmware`, `xport_kepdrv_read_firmware`, `xport_kepdrv_send_blc`, `failure reading Kepler firmware`, `BCMD to codeload Kepler`, `BCMD to start Kepler`, `Kepler EXE Address: %08X`. Likely **LOAD → READ (verify) → BCMD(s)**; two BCMD phrases plus an EXE address for branch `param1`. No `SECURITY` ioctl string.

### Kernel structs (DXJ2 `gnss_prj.h`, aarch64 — not guessed)

```c
#define GNSS_IOCTL_LOAD_FIRMWARE	_IO('K', 0x01)	/* 0x4b01 */
#define GNSS_IOCTL_REQ_BCMD		_IO('K', 0x03)	/* 0x4b03 */
#define GNSS_IOCTL_READ_FIRMWARE	_IO('K', 0x04)	/* 0x4b04 — same kepler_firmware_args */

struct kepler_firmware_args {	/* sizeof 16, pointer at +8 */
	u32 firmware_size;
	u32 offset;
	char *firmware_bin;
};
struct kepler_bcmd_args {	/* sizeof 16 */
	u16 flags;
	u16 cmd_id;
	u32 param1;
	u32 param2;
	u32 ret_val;
};
```

`copy_reserved_from_user` writes `gnss_rmem+offset` (6 MiB). BCMD `cmd_id==0x4` is `BLC_Branch` (no completion). If `OFFLINE`, `kepler_req_bcmd` **POWER_ON**s Kepler first (own PMU). `kepler_active_isr` always sets **`FAULT`**.

### Ioctl sequence

Start: `gnss_status=OFFLINE`, mailbox CTRL all 0, IRQs 0. CP **`CRASH_EXIT`**. `READ_SHMEM_SIZE` = **6291456**.

1. `GNSS_IOCTL_LOAD_FIRMWARE` full trailer **1222032** @ offset **0** → **OK** (`rc=0`). dmesg `Load Firmware - fw size : 1222032, fw_offset : 0`.
2. `GNSS_IOCTL_REQ_BCMD` `flags=0 cmd_id=0x4 param1=0 param2=0` → **OK** (`rc=0`, `ret_val=0`).

Kernel: `OFFLINE → ONLINE`, PMUCAL `GNSS_STATUS` 0→1, **`kepler_sw_init_isr`**, BAAW, then mailbox `CTRL0=0x4 CTRL3=0xff`, then **`kepler_active_isr`** (~90 µs later) → **`ONLINE → FAULT`**.

| After first load | |
|--|--|
| `gnss_status` | **`FAULT`** (not ONLINE) |
| mailbox | CTRL0 `0x4`, CTRL3 `0xff` (Kepler did not write a return) |
| IPC heads/tails | still 0 |
| IRQs | mailbox 0; **`kepler_active=1`**; wdt 0; **`kepler_sw_init=1`** |

`GNSS_IOCTL_REQ_FAULT_INFO` still answered (**282840** bytes, CTRL3 `0x450D8`). Head (LE): `02 00 00 00` … `00 50 02 20` … at +0x6c/`+0x70` two `04 00 00 00`. No IMEI in this dump.

Hold-reset (`GNSS_IOCTL_RESET`, not POWER_OFF) → `HOLD_RESET`. Second shot: load **payload only** (skip 64, size 1221968, head `2a f0 ee fa`) then same BCMD. `release_reset` → ONLINE, **`sw_init` again**, then **ACTIVE → FAULT** again. Same mailbox. IRQs active=2 sw_init=2. **Stopped** — not a struct-layout miss (16/16 matched kernel; dmesg printed the size we passed). Branch `param1=0` is the next discriminator (gpsd likely passes a code address). Do not pack.

## Probe 22 — vendor `rild` vs leftover ONLINE CP (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Replicant SIPC/PS exhausted (SIM1 emergency HLR#2, SIM2 HOME GMM#7). Remaining software path: vendor `rild` / `libsec-ril` Shannon attach. **No** original efs. **No** `POWER_OFF`. **No** flash. **No** commit. **No** `usb-host`. GNSS left **OFFLINE**.

### R620

**Unreachable** this pass. `192.168.168.110:22` fails (`laptop-wg` still on-link `192.168.168.0/24` metric 0). WiFi is `192.168.88.246` (no `192.168.168.150` bind). `jump-wg` / `jump.beart.cc` / `ruta-wg` timed out. WSL cannot ping R620. Did **not** copy NV off the phone into git.

### Extract (phone SUPER, read only — same DXJ2 images)

`mmcblk0p31` LP: vendor **one** extent `offset=3675258880` `sizelimit=503668736` (matches R620 `vendor.img` 503668736). system two extents (first 3674210304 @ 1 MiB). `losetup -r` + `mount -t ext4 -o ro,noload`. No `dm_linear`.

Runtime APEX: unzip `com.android.runtime.apex` → `apex_payload.img` loop12 at `/apex/com.android.runtime`. `linker64` symlink `/system/bin/linker64` → apex. Bind `/vendor/bin`, `/vendor/lib64`, `/system/lib64`. binderfs mounted (`binder`/`hwbinder`/`vndbinder`).

`rild` 15472, `libsec-ril.so` 4541576. `RIL_Init` **is** exported.

### `rild` start

Did **not** kill holder **428** (ipc0+rfs0). `rild` never opened those nodes.

```text
/vendor/bin/hw/rild -l /vendor/lib64/libsec-ril.so
```

`LD_DEBUG=1`: jumped to `_start` after linking libc/libm/libdl (apex), `libril_sem`, `libsec-ril` HIDL (`android.hardware.radio@1.0`–`1.5`, `vendor.samsung.hardware.radio@2.0`–`2.2`), `libhidlbase`, `libbinder`. **Not** “missing 50 libs.” **Not** missing `linker64`.

Alive? **No.** Exit **1** in <4 s. stderr only:

```text
libc: Using old property service protocol ("ro.property_service.version" is not set)
```

ALOG (`**RIL Daemon Started**`, `dlopen failed`, `RIL_Init argc`) goes to logd. `/system/bin/logd` **SIGABRT**. No `/dev/socket/logdw`. Fake `/dev/__properties__` prop_area (128 KiB, magic `PROP`) — `getprop` still empty (Android 13 wants `/dev/__properties__/property_info`). No `hwservicemanager`.

Holder exclusive? **Not tested.** `rild` died before ipc0. Holder 428 kept fds.

### 90 s rmnet

Not watched (daemon never stayed up). Spot check uptime **8221 s**: `modem_state=ONLINE`, GNSS **OFFLINE**, `rmnet0` rx=tx=**0** (IPv6 `fe80::200:ff:fe00:0/64` only, no IPv4). Original efs **not** mounted (`/mnt/vendor/efs` = userdata copy).

**Data-plane goal not complete.** `rild` cannot run as a radio daemon on this BusyBox ramdisk (needs Android property service + logd + HIDL). Both SIMs already rejected PS. **rmnet needs a PS-capable SIM.** Do not fake loopback counters. Do not pack v032.

## Next

- CP **`ONLINE`** through **8221 s** (holder 428). Probe 22: vendor `rild` linked then **exit 1**; **rmnet 0**. Probe 21 SIM1 Kyivstar CS **EMERGENCY fail=0x02**; PS GMM#7. SIM2 GMM#7. **No PDP.** Goal **not complete**. **Do not SET MODE_SEL.** **Do not ACK STK.** **Do not brute PIN.**
- GNSS: leave **OFFLINE**. Do not pack `linker64`/`gpsd`.
- Do not pack v032.

No v032.

## Forbidden

| | |
|--|--|
| Write `efs` / `sec_efs` / `cpefs` | IMEI / NV |
| `dd` to `radio` / format / stock CP flash | RADIO is observe-only |
| Mount EFS to feed `cbd` | `cbd` `fsync`s NV |
| Start `rild` / `secril_config_svc` / `cass` / `gpsd` | HAL + EFS / linker64 |
| Write `do_cp_crash` | CP panic |
| AT that writes NV | skip |
| `/sbin/usb-host` | drops RNDIS |
