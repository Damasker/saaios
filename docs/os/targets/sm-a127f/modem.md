# SM-A127F — modem / CP (Phase E4)

Exynos 850 **S5000AP** Shannon (**SS310**) over CPIF SHMEM. Stock userspace is Android `cbd` + `rild`. We have BusyBox ramdisk only. **EFS writes are forever off-limits.** One-shot **ro,noload** list 2026-08-31 (below); umounted. RADIO is read-only (stock CP image). Do not `dd` / format / AT-write NV.

Detail: live v031 dump 2026-08-29 + `debugfs` of `os/build/stock-super/vendor.img` on R620. Kernel tree: `os/third_party/kernel_samsung_a12/drivers/soc/samsung/cpif`.

## LIVE map (v031, stock DXJ2 4.19.111-27127798)

| | |
|--|--|
| CP state | **`CRASH_EXIT`** (2026-09-02, probe 48 — leftover, **no AP reboot**). Uptime **43739** s (same v031 as probe 47). Last **ONLINE** was probe 44. Holder **428** left alive. `rmnet*` rx=tx=**0**. GET-only `ps-p48` **staged, not run**. **`IOCTL_POWER_OFF` not used.** **`loadnv` not run.** **Data-plane goal not complete.** |
| GNSS | **`OFFLINE`** (no BCMD this pass). `READ_FIRMWARE` vs `/tmp/kepler-fw.bin` **byte-identical** (64B K102 + full SHA). See GNSS boot LIVE |
| SIM detect | `ds_detect=2` (`cpif/sim/ds_detect` and `modem_ctrl_s5000ap` param) |
| Driver | `cpif_probe` **CPIF-200511N220408** `eur_open`; **s5000ap** modemctl; **s318ap** shmem link |
| DT | `samsung,exynos-cp`. LIVE `cpif/mif,protocol` = **0** (`PROTOCOL_SIPC`). gnssif has no protocol/sit cell |
| Firmware | BOOT+MAIN+**VSS** + real NV (userdata copy) via `/mnt/userdata/radio-boot loadnv`. Helper **1314320**. Bind-mount copy → `/mnt/vendor/efs`. Vendor `rild` **stayed up** (probe 23). `cbd` not executed |

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

## Probe 23 — property_service + stub logd + rild vs leftover ONLINE (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Keep vendor `rild` alive (it may send attach replicant missed). **No** original efs write. **No** `POWER_OFF`. **No** flash. **No** commit. **No** `usb-host`. GNSS left **OFFLINE**. Did **not** fake rmnet counters.

### Stubs

No in-tree mini-propdaemon. Static musl `/tmp/minird` (Zig 0.13 `aarch64-linux-musl`, 1058344). **ContextsPreSplit** file `/dev/__properties__` (128 KiB, magic `PROP`, version `0xfc6ed0ab`) with `ro.property_service.version=2`. Unix `/dev/socket/property_service` (old SETPROP + SETPROP2). Stub `/dev/socket/logdw` (dgram, dump, no abort). Real `/system/bin/logd` **not** used.

`getprop ro.property_service.version` → **2**.

### `rild` (same vendor bind / APEX linker as probe 22)

Did **not** kill holder **428**. `/mnt/vendor/efs` is userdata **p38** copy (not original efs p1).

1. First exec: `dlopen` `libsec-ril.so` → `libsqlite.so` → **`libandroidicu.so` not found**. Mounted `com.android.i18n.apex` (loop13).
2. With i18n on `LD_LIBRARY_PATH`: `RIL_Init` completed, opened **ipc0+ipc1+rfs0**, then ~30 s **`Init process for SecRilProxy is stucked`** waiting `hwservicemanager.ready`.
3. Real `hwservicemanager`: bind `plat_hwservice_contexts` / `vendor_hwservice_contexts`, mount selinuxfs. **Alive** (PID 2243).
4. `rild` **2245** stayed up: `RIL_Init` / `RIL_register` (v15) / `RIL_register_socket` completed. HIDL `registerAsService` **fails** (`must be in VINTF manifest`). `ril.hasisim=0,0`, `ril.ICC_TYPE0/1=0`, `ril.phone.connected.*=false`.

### 90 s rmnet (rild alive 30 s+)

Nine samples, 10 s apart, uptime **9178–9259**, modem **ONLINE**, rild **alive** every sample:

`rmnet0–7` rx=tx=**0**. `ip -4` only `rndis0` `192.168.42.1/24`. No IPv4 on rmnet.

**Data-plane goal not complete.** rild is a live radio daemon now; Shannon attach still produced **no** rmnet. SIMs still not `hasisim`. Next hole is **VINTF manifest** (HIDL IRadio), not property_service/logd. Do not pack v032.

## Probe 24 — libsec-ril IpcTxPsAttach + vendor 3-byte GPRS_PS (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Pulled `/vendor/lib64/libsec-ril.so` **4541576** via TCP `192.168.42.7:8830` (`busybox nc`; size match). Disassembled dynsym on the host (no invented opcodes). **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0808/0x0809 SET. **No** 0x0D14 SET. **No** STK ACK. **No** PIN. Holder **428** STOP/CONT only (did **not** STOP 416). Did **not** commit the `.so`.

### Binary (this DXJ2 `libsec-ril.so`)

| Symbol | VA | What it actually sends |
|--|--|--|
| `IpcProtocol41Data::IpcTxPsAttach(uchar,bool,bool,DataDetachReason)` | `0x36a388` | FMT **len=10** cmd uint16 **0x030D** LE → group **0x0D** index **0x03** (`GPRS_PS`), type **SET 0x03**, payload **3 B** `{attach, flag, reason}` — **no cid**. Attach path: `01 00 00`. |
| `IpcTxSetLteAttachProfile` | `0x36b040` | **0x0D14** SET, FMT len **0x151** (337). GET only this pass. |
| `IpcTxSetMobileDataSetting(bool,bool)` | `0x36c5cc` | **0x0D1D** SET, len 9, payload **2 B** bools. |
| `IpcTxNetGetServiceDomain` | `0x3892b8` | **0x0808** GET, empty (len 7). |
| `IpcTxNetSetServiceDomain` | `0x3891ec` | **0x0808** SET, len 8, **1-byte** payload (enum maps to 0/2/3). **Not SET.** |
| `IpcTxGetDualStandbyPref` | `0x389a30` | **0x0816** GET empty. |

`GPRS_SUB_CMD_UNDEFINED` is a **printf** `GPRS_SUB_CMD_UNDEFINED(0x%x)`, not a name table (no RELA pointer run).

### Live ipc1 (SIM2 Vodafone 25501)

Static `/tmp/ps-vnd` (`os/build/e4-ps-vnd.c`, Zig musl **1022608**). wget `192.168.42.7:8831` (size match). mseq from **0xC0**.

| TX | result |
|--|--|
| GET `MODE_SEL` | **0x0b** (late RESP) |
| GET `NET_REGIST` CS/PS | CS **HOME** act=UMTS fail=0; PS **NONE fail=0x07** |
| SET `GPRS_PS` vendor **`01 00 00`** (mseq **0xC9**) | **no** matching `GEN_PHONE_RES` aseq=0xC9. Follow-up GET **cid=0 attached=0** |
| SET `0x0D1D` `01 01` | no matching RESP |
| GET `0x0D14` / `0x0816` | no matching RESP this drain |
| leftover `GEN_PHONE_RES` aseq=**9** `0x0D03` **0x8000** | **stale** (not this SET) |
| delayed GET RESP aseq **0xC0–0xC3** | **0x0808** body **`01`**; **0x0809** body **`01`**; `MODE_SEL` **0x0b**; `GPRS_PS` **00 00** — these match the **previous** GET-only sequence that had “no RESP in 5s” |

Replicant `IPC_CALL_OUTGOING` **0x0201** SET (voice, identity default, prefix international) then `CALL_LIST` GET then `CALL_RELEASE` **0x0203**. **No** `CALL_STATUS` / `CALL_LIST` body / `GEN_PHONE_RES` for those mseqs. Drain saw only `DISP` **0x0706** / **0x0701**. CS REGIST stayed HOME. **Did not** ACK STK (none). CP stayed **ONLINE**.

### rmnet

| | modem | rmnet0 |
|--|--|--|
| pre / post / final | ONLINE | rx=tx=**0** |

Holder **428** CONT, still alive. **Data-plane goal not complete.** Vendor 3-byte attach is ACK-ambiguous (ipc1 race vs PID **416** + delayed aseq). GMM **#7** unchanged. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808** (GET body **0x01** is not in the SET enum 0/2/3 map). Next discriminator: 0x0D14 GET with long wait / ipc1 vs 416, or GMM#7 as subscription. Do not pack v032.

## Probe 25 — 0x0D14 layout from `IpcTxSetLteAttachProfile` (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Decoded `IpcTxSetLteAttachProfile` @ **0x36b040** in pulled `libsec-ril.so` **4541576** (host `e4-lte-profile.c` + `e4-ril-disasm.c`). **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0D14 SET. **No** STK ACK. **No** PIN. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **16800** (had `/dev/umts_ipc1`; not 416). RNDIS host **192.168.42.7**. wget **8834** `/tmp/ps-p25` **1026448**.

### Prior GET 0x0D14 (probe 24 drain fix — do not redo)

20 s GET on ipc1 after wall-clock drain fix: TX GET `GPRS_LTE_ATTACH_APN_INFO` **0x0D14** → **GEN_PHONE_RES 0x8001** (unsupported), no body. Real CP answer (not ipc1/416 steal). GET **0x0D1D** also **8001**. GET **0x0D03** RESP plen=2 **cid=0 attached=0**.

### Binary: `IpcProtocol41Data::IpcTxSetLteAttachProfile`

Mangled: `(uchar, char const*, char const*, char const*, DataAuth, DataProtocol, DataProtocol, PcscfViaPco, uchar*, uchar*, bool)`. **No** BL xrefs (vtable). **No** default APN / user / pass / CID / P-CSCF bytes in this function. NULL APN → return **-1**, **no send**. `internet` / `lte_internet` / `lte_ia` live in `IpcTxSetDataProfile` (**0x0D1B**), not here. `IpcModemImplData::SetLteAttachProfile` @ **0x34deec** passes caller `DataCallSetup` (factory / KDI props only).

FMT send: `w2=0x151`, buffer at SP (memset 0 via `movi v0.16b` + STP Q).

| FMT off | field |
|--|--|
| 0–1 | length **0x0151** |
| 2–3 | mseq / aseq (send path) |
| 4–5 | **0x0D 0x14** |
| 6 | type **SET 0x03** |
| 7 | `(bool & 1) ? 3 : 0` |
| 8 | **CID** (arg1) |
| 9 | mapped **DataProtocol1** (table `0x00040605` after `proto-2`; default **2** if out of range) |
| 10–110 | **APN[101]** (strlen ≤ 100) |
| 111–126 | 16 B from `uchar*` arg B if non-NULL (P-CSCF) |
| 127–130 | 4 B from `uchar*` arg A if non-NULL |
| 131–132 | **0** |
| 133 | remapped proto **2 or 3** only if `PcscfViaPco==1` |
| 134–233 | **USER[100]** (strlen ≤ 100; NULL skips) |
| 234–333 | **PASS** memcpy 100 B from ptr; strlen must be ≤ **32**; NULL skips |
| 334 | mapped **DataAuth** (table `0x00010201` after `auth-1`; else 0) |
| 335 | mapped **DataProtocol2** |
| 336 | remapped proto2 only if `PcscfViaPco==1` |

Layout is field-complete. **Exact 337-byte blob is argument-filled.** Did **not** guess-fill. **0x0D14 SET not sent.**

Legacy twin `IpcTxSetLteAttachProfileLegacy` @ **0x36add4**: same cmd **0x0D14 SET**, FMT len **0xC9** (201). Also APN-arg.

### Related SET-only GPRS (GET is 8001)

| Symbol | VA | FMT |
|--|--|--|
| `IpcTxPsAttach` | `0x36a388` | **0x0D03 SET** len 10, **3 B** `{attach,flag,reason}` = `01 00 00` (no cid) |
| `IpcTxSetMobileDataSetting` | `0x36c5cc` | **0x0D1D SET** len 9, **2 B** bools (already sent `01 01`) |
| `IpcTxSetAlwaysOnPdn` | `0x36c714` | **0x0D22 SET** len 9, `{bool, MapDataProfile(enum)}`. Map @ **0x36a9c0** is a VZW/USC switch (returns 1/2/3/4/5/11) — **not fully recovered**, **0x0D22 SET not sent** |
| `IpcTxSetDataProfile` | `0x36aae4` | **0x0D1B SET** len **0xCB** (APN/profile names) |
| `IpcTxDefinePdpContext` | `0x369744` | len **0x95** (vendor; not replicant 134) |
| `IpcTxSetPdpContext` | `0x369ec8` | **0x0D04 SET** len **0xF8** |
| `IpcTxSetPdpContextLegacy` | `0x369c84` | **0x0D04 SET** len **0x70** |

### Live ipc1 (SIM2 25501) after killing sleep 16800

Static `/tmp/ps-p25` (`os/build/e4-ps-p25.c`, Zig musl **1026448**). mseq from **0xF0**. Wall-clock drain. **No SET 0x0D14.**

| TX | result |
|--|--|
| leftover NOTI | CS **HOME** UMTS fail=0; PS **NONE fail=0x07**; SERVING **25501** |
| GET `MODE_SEL` aseq **0xF0** | **0x0b** |
| GET `GPRS_PS` aseq **0xF3** | **cid=0 attached=0** |
| GET `0x0D22` aseq **0xF4** | **GEN_PHONE_RES 0x8001** (unsupported, no body) |
| GET `NET_REGIST` CS/PS | CS **HOME** act=UMTS fail=0; PS **NONE fail=0x07** |
| SET `GPRS_PS` `01 00 00` aseq **0xF5** | **GEN_PHONE_RES 0x0D03 0x8000 SUCCESS** (real; sleep thief gone) |
| SET `0x0D1D` `01 01` aseq **0xF6** | **GEN_PHONE_RES 0x0D1D 0x8003** |
| GET `GPRS_PS` after | **cid=0 attached=0** |
| GET `NET_REGIST` PS after | **NONE fail=0x07** |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** |

Holder **428** CONT, still alive. PID **416** still holds ipc1. **Data-plane goal not complete.** CP accepted vendor 3-byte attach (**8000**) but GMM **#7** unchanged — not an ipc1-steal miss. GET attach-profile / always-on are **unsupported**; SET 0x0D14 still needs a caller APN (none in this .so). **Do not SET MODE_SEL 0x04/0x07.** **Do not ACK STK.** Do not pack v032.

## Probe 26 — 0x0D1B / 0x0D14 SET + vendor PDP (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Decoded `IpcTxSetDataProfile` @ **0x36aae4** and sent vendor 0x0D14 / 0x0D1B / 0x0D01 / 0x0D04 blobs (APN **`internet`** is an exact cstring in this `.so`, not an operator guess). **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** STK ACK. **No** PIN. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **16907** (had `/dev/umts_ipc1`; not 416). RNDIS host **192.168.42.7**. wget **8835** `/tmp/ps-p26` **1093152**.

### Binary: `IpcProtocol41Data::IpcTxSetDataProfile` (0x0D1B SET len **0xCB**)

Mangled: `(char const*, char const*, char const*, DataAuth, DataProtocol, DataProfile, bool, PcscfViaPco, int, int, int)`. NULL APN → return **-1**, no send. `MapDataProfile` @ **0x36a9c0** writes byte[7] (VZW/USC / invalid → **1** for out-of-range enum). Profile **name** (not APN) is a 20-byte field from a switch on `DataProfile`:

| enum | name at [8..27] |
|--|--|
| 1–6 jump | `lte_tethered` / `lte_ims` (table) |
| 1001–1006 | `lte_emergency` `lte_embms` `lte_bip` `lte_cas` **`lte_ia`** `lte_mms` |
| else (e.g. 7) | **`lte_internet`** |

Exact cstring **`internet`** @ `0x10483d` (xrefs `0x243aa8` / `0x243b2c` in DataCallManager). `lte_ia` / `lte_internet` are profile-name strings inside this function.

| FMT off | field |
|--|--|
| 0–1 | length **0x00CB** |
| 2–3 | mseq / aseq |
| 4–5 | **0x0D 0x1B** |
| 6 | type **SET 0x03** |
| 7 | `MapDataProfile` (**1** for out-of-range) |
| 8–27 | profile name[20] |
| 28 | mapped **DataProtocol** (table `0x00040605` after `proto-2`; default **2**) |
| 29–129 | **APN[101]** |
| 130–161 | USER[32] (NULL skips) |
| 162–193 | PASS[32] (NULL skips) |
| 194 | mapped DataAuth (else 0) |
| 195–196 | proto remap if `PcscfViaPco==1` |
| 197–202 | three int16 stack args |

This pass: name **`lte_internet`**, APN **`internet`**, proto **2**, user/pass/auth/pcscf **0**.

### 0x0D14 SET (Probe 25 layout, APN now from `.so`)

CID=**1**, proto1 default **2**, APN **`internet`**, empty user/pass, auth **0**, no P-CSCF, last bool **0** so byte[7]=**0**. Layout check passed before send.

### Vendor PDP (not Replicant 134)

`IpcTxDefinePdpContext` @ **0x369744**: FMT **0x0D01 SET len 0x95**. Packet @ SP+0x60: `[7]=0x01` (from `0x0103010D`), CID @ **8**, proto default **2** @ **9**, APN[101] @ **10**, unconditional last-1 byte **0x01** @ **147**.

`IpcTxSetPdpContext` @ **0x369ec8**: FMT **0x0D04 SET len 0xF8**. byte[7]=**1** is the APN-copy path; CID @ **8**; present-flag **1** @ **9**; APN[101] @ **13**; auth @ **0xF5**. No separate `IpcTxActivatePdpContext` symbol — `ActivatePdpContext` has no dedicated IpcTx; **0x0D04** is the vendor SET.

### Live ipc1 (SIM2 25501) after killing sleep 16907

Static `/tmp/ps-p26` (`os/build/e4-ps-p26.c`, Zig musl **1093152**). mseq from **0x20**. Wall-clock drain.

| TX | result |
|--|--|
| leftover NOTI | CS **HOME** UMTS fail=0; PS **NONE fail=0x07**; SERVING **25501**; MODE_SEL **0x0b** |
| GET `MODE_SEL` aseq **0x20** | **0x0b** |
| GET `GPRS_PS` aseq **0x23** | **cid=0 attached=0** |
| GET `NET_REGIST` CS/PS | CS **HOME** act=UMTS fail=0; PS **NONE fail=0x07** |
| SET `0x0D1B` aseq **0x24** 0xCB `lte_internet`+`internet` | **GEN_PHONE_RES 0x0D1B 0x8000 SUCCESS** |
| SET `0x0D14` aseq **0x25** 0x151 APN `internet` | **GEN_PHONE_RES 0x0D14 0x8001** (unsupported) |
| SET `GPRS_PS` `01 00 00` aseq **0x26** | **GEN_PHONE_RES 0x0D03 0x8000 SUCCESS** |
| GET `GPRS_PS` / `NET_REGIST` PS | **attached=0**; PS **NONE fail=0x07** |
| SET `0x0D01` aseq **0x29** 0x95 | **GEN_PHONE_RES 0x0D01 0x8000 SUCCESS** |
| SET `0x0D04` aseq **0x2A** 0xF8 | **GEN_PHONE_RES 0x0D04 0x8000 SUCCESS**; leftover NOTI **0x0D10** `01 03` + zeros |
| SET `GPRS_PS` `01 00 00` aseq **0x2B** | **0x8000 SUCCESS** |
| GET final | **attached=0**; PS **NONE fail=0x07** |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / mid / post | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0**. IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`). PID **416** still holds ipc1. **Data-plane goal not complete.** CP accepted vendor data-profile / define / set-PDP (**8000**) but **0x0D14 is unimplemented** on this CP (SET=GET=**8001**). GMM **#7** unchanged — not an ipc1-steal miss and not a 0x0D14 layout miss. **Do not SET MODE_SEL 0x04/0x07.** **Do not ACK STK.** Do not pack v032.

## Probe 27 — `NetServiceDomainType` map + GET 0x0808/0x0809 (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Decoded `IpcTxNetSetServiceDomain` / `IpcRxNetServiceDomain` / `GetSubCommandName` in pulled `libsec-ril.so` **4541576** (host `e4-svc-dom-decode.c`). **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0808 SET (names ambiguous). **No** 0x0809 SET (no IpcTx). **No** STK ACK. **No** PIN. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **16943** (had `/dev/umts_ipc1`; not 416). RNDIS host **192.168.42.7**. wget **8836** `/tmp/ps-p27` **1088192**.

### Binary: `IpcTxNetSetServiceDomain(NetServiceDomainType)` @ 0x3891ec

Mangled: `_ZN16IpcProtocol41Net24IpcTxNetSetServiceDomainE20NetServiceDomainType`. FMT **0x0808 SET** len **8**, 1-byte payload. Twin pack in `BuildIpcNetSetServiceDomain` @ **0x3af1a4**. Prior note “maps to 0/2/3” treated `1a9f0529` as CSEL — it is **CSINC**.

```text
w9 = 3
cmp enum, #1
csinc w9, w9, wzr, eq    ; enum==1 → 3 ; else → 1
cmp enum, #2
csel  w9, enum, w9, eq   ; enum==2 → 2 ; else keep
strb w9, [pkt+7]
```

| `NetServiceDomainType` | SET byte |
|--|--|
| 1 | **0x03** |
| 2 | **0x02** |
| else (0, 3, …) | **0x01** |

`IpcRxNetServiceDomain` @ **0x38facc**: payload ∉ {1,2,3} → enum **-1**. Else table @ **0x153254** (3×u32):

| GET/SET byte | RX enum |
|--|--|
| 0x01 | **0** |
| 0x02 | **2** |
| 0x03 | **1** |

Bijection is recovered. **No** `CS_ONLY` / `PS_ONLY` / `CS_PS` / `COMBINED` strings. **No** BL callers with a constant enum (vtable only). `DoOemSetServiceDomain` passes the raw OEM byte. Two namings stay open (Replicant IPC 1/2/3 = CS/PS/COMBINED vs AOSP-style enum 0/1/2 = CS/PS/CS_PS, which would swap which of **2** or **3** is combined). **Did not SET 0x0808.**

`IpcModemImplNet::SetServiceDomain` @ **0x359c64** is a vtable hop to this IpcTx (w2 = enum). No IpcTx symbol for **0x0809**.

### 0x0809 POWERON_ATTACH

`IpcProtocol::GetSubCommandName` 12-byte NET cases (adjacent to known cmds):

| index | name |
|--|--|
| 0x08 | `NET_SERVICE_DOMAIN_CONFIG` |
| **0x09** | **`NET_POWERON_ATTACH`** |
| 0x0A | `NET_MODE_SEL` |

No `IpcTxNet*Poweron*` / no `MOVZ #0x0908` send path. **0x0809 SET payload not recovered. Not SET.**

### Leftover 0x0D10 `01 03` (probe 26)

String **`GPRS_CALL_STATUS`**. Handler **`IpcRxGprsCallStatus`** @ **0x36d29c** (CID + status; logs “CDMA Data call disconnected” / “Invalid CID” / “throttle timer from cp”). **`GPRS_IP_CONFIGURATION`** is a different cmd (`IpcRxIpConfiguration` @ **0x36cbf8**). So **0x0D10 NOTI is call/PDP status**, not IP config. Probe 26 body `01 03` after SET 0x0D04 CID **1** = cid=1 status=0x03 (not the 1 / 0x0A / 0x0B specials in this RX).

### Live ipc1 (SIM2 25501) after killing sleep 16943

Static `/tmp/ps-p27` (`os/build/e4-ps-p27.c`, Zig musl **1088192**). mseq from **0x40**. Wall-clock drain. **GET only.**

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | CS **HOME** UMTS fail=0; PS **NONE fail=0x07**; SERVING **25501** |
| GET `PHONE_STATE` **0x40** | **0x40** | **0x02** |
| GET `MODE_SEL` **0x41** | **0x41** | **0x0b** |
| GET `0x0808` **0x42** | **0x42** | body **`01`** (enum **0**) |
| GET `0x0809` **0x43** | **0x43** | body **`01`** |
| GET `GPRS_PS` **0x44** | **0x44** | **cid=0 attached=0** |
| GET `NET_REGIST` CS/PS | **0x46** (both) | CS **HOME** act=UMTS fail=0; PS **NONE fail=0x07** |

No `GEN_PHONE_RES` on these GETs (type-2 RESP with body). **No SET.**

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** |

Holder **428** CONT, still alive (`radio-boot`). PID **416** still holds ipc1. **Data-plane goal not complete.** GET domain still **0x01** (RX enum 0). Combined vs PS-only SET byte is **not named** in this `.so` — next pass needs a caller constant or OEM doc before SET 0x02/0x03. **Do not SET MODE_SEL.** **Do not ACK STK.** Do not pack v032.

## Probe 28 — SET 0x0808 CS_PS byte 0x02 (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 27 RX table is **0-based** (`0x01→enum 0`, `0x02→enum 2`, `0x03→enum 1`) matching Samsung `NetServiceDomainType` **CS=0, PS=1, CS_PS=2**, not Replicant IPC 1/2/3. AOSP `android.hardware.radio.network.Domain` is a **different** bitflag enum (`CS=1`, `PS=2`). Current GET **0x01** = CS-only explains ACK’d attach + GMM **#7**. Combined **CS_PS = enum 2 = SET byte 0x02**. **Never SET 0x03** (PS-only, can drop CS HOME). **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0808 SET **0x03** / **0x01** (0x01 only if restore). **No** 0x0D14 SET. **No** STK ACK. **No** PIN. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **16963** (had `/dev/umts_ipc1`; not 416). RNDIS host **192.168.42.7**. wget **8837** `/tmp/ps-p28` **1106976**.

Static `/tmp/ps-p28` (`os/build/e4-ps-p28.c`, Zig musl). mseq from **0x50**. FMT SET len **8**, 1-byte payload.

### Live ipc1 (SIM2 25501)

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** only (no STK) |
| GET `PHONE_STATE` **0x50** | **0x50** | **0x02** |
| GET `MODE_SEL` **0x51** | **0x51** | **0x0b** |
| GET `0x0808` **0x52** | **0x52** | body **`01`** (enum **0** CS) |
| GET `0x0809` **0x53** | **0x53** | body **`01`** |
| GET `GPRS_PS` **0x56** | **0x56** | **cid=0 attached=0** |
| GET `NET_REGIST` CS/PS | **0x55** (both) | CS **HOME** act=UMTS fail=0; PS **NONE fail=0x07** |
| SET `0x0808` **`02`** **0x58** | **0x58** | **GEN_PHONE_RES 0x0808 0x8000 SUCCESS** |
| GET `0x0808` **0x59** | **0x59** | body **`02`** (enum **2** CS_PS) |
| GET CS/PS / GPRS_PS | **0x5b** / **0x5c** | CS **HOME** UMTS fail=0; PS **NONE fail=0x07**; **attached=0** |
| SET `GPRS_PS` `01 00 00` **0x5d** | **0x5d** | **GEN_PHONE_RES 0x0D03 0x8000 SUCCESS** |
| GET CS/PS / GPRS_PS | **0x5f** / **0x60** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** after the 0x02 SET → **restore to 0x01 did not run**. Vendor **0x0D1B / 0x0D01 / 0x0D04** skipped (GMM still **#7**). Domain left at **0x02** (CS_PS).

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post-SET / post-attach / final | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0**. No IPv4 on rmnet (ioctl empty). IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** Combined domain is ACK’d and GET-confirmed; GMM **#7** is not a CS-only domain miss. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 29 — vendor PDP under CS_PS (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 28 left **0x0808 = 0x02** (CS_PS) and skipped vendor PDP because GMM was still **#7**. This pass sends **0x0D1B / 0x0D01 / 0x0D04** then `IpcTxPsAttach` under that domain. **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0808 SET **0x03** / **0x01** (0x01 only if restore). **No** 0x0D14 SET. **No** STK ACK. **No** PIN. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **16976** (had `/dev/umts_ipc1`; not 416). RNDIS host **192.168.42.7**. wget **8838** `/tmp/ps-p29` **1109144**.

### Binary: `GPRS_IP_CONFIGURATION` cmd id

`IpcRxIpConfiguration` @ **0x36cbf8** (size 832) has no packed FMT cmd (dispatcher RX). Twin TX **`IpcTxIpv6Configuration`** @ **0x36b31c** packs MOVZ **`0x090d`** → group **0x0D** index **0x09**. String **`GPRS_IP_CONFIGURATION`** @ `0x124490`. Host `e4-ip-cfg-decode.c`. Same id as Replicant `IPC_GPRS_IP_CONFIGURATION` **0x0D09**. GET **0x0816** (`IpcTxGetDualStandbyPref` @ **0x389a30**) is empty GET; SET layout not recovered — GET only.

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p29` (`os/build/e4-ps-p29.c`, Zig musl). mseq from **0x60**. Probe 26 layouts (`lte_internet` + APN `internet`, proto **2**).

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0838** / **0x0F36** (no STK) |
| GET `PHONE_STATE` **0x60** | **0x60** | **0x02** |
| GET `MODE_SEL` **0x61** | **0x61** | **0x0b** |
| GET `0x0808` **0x62** | **0x62** | body **`02`** (enum **2** CS_PS) |
| GET `0x0809` **0x63** | **0x63** | body **`01`** |
| GET `GPRS_PS` **0x66** | **0x66** | **cid=0 attached=0** |
| GET `NET_REGIST` CS/PS | **0x65** (both) | CS **HOME** act=UMTS fail=0; PS **NONE fail=0x07** |
| SET `0x0808` | — | **not sent** (already **0x02**) |
| SET `0x0D1B` **0x68** 0xCB `lte_internet`+`internet` | **0x68** | **GEN_PHONE_RES 0x0D1B 0x8000 SUCCESS** |
| SET `0x0D01` **0x69** 0x95 CID=1 | **0x69** | **GEN_PHONE_RES 0x0D01 0x8000 SUCCESS** |
| SET `0x0D04` **0x6A** 0xF8 APN-copy | **0x6A** | **GEN_PHONE_RES 0x0D04 0x8000 SUCCESS**; NOTI **0x0D10** cid=**1** st=**0x03** |
| SET `GPRS_PS` `01 00 00` **0x6B** | **0x6B** | **GEN_PHONE_RES 0x0D03 0x8000 SUCCESS** |
| GET CS/PS / GPRS_PS | **0x6d** / **0x6e** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |
| **0x0D09** IP config | — | **none** |
| GET `0x0816` **0x6F** | **0x6F** | RESP plen=2 body **`00 00`** |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **0x02** (CS_PS).

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post-PDP / final | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0**. No IPv4 on rmnet (ioctl empty). IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** Vendor PDP under combined domain is ACK’d the same as Probe 26 under CS-only: **0x0D10** `01 03`, no **0x0D09**. GMM **#7** is not a “PDP skipped while CS-only” miss. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 30 — SET DualStandbyPref SIM2 (v031, 2026-09-01)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 29 left GET **0x0816** **`00 00`** (possible “no data SIM”). This pass recovers **SET** from `libsec-ril.so` and selects **SIM2 / ipc1**. **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0808 SET (0x01 only if restore). **No** 0x0D14. **No** STK ACK. **No** PIN. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17002** (had `/dev/umts_ipc1`; not 416). RNDIS host **192.168.42.7**. wget **8839** `/tmp/ps-p30` **1107816**.

### Binary: `IpcTxSetDualStandbyPref` / `IpcTxSetDualStandbyPrefModem`

Host `e4-dsds-decode.c`. String **`NET_DUAL_STANDBY_PREF`** @ `0x117669`. No invented opcodes.

| Symbol | VA | size |
|--|--|--|
| `IpcProtocol41Net::IpcTxGetDualStandbyPref()` | **0x389a30** | 176 |
| `IpcProtocol41Net::IpcTxSetDualStandbyPref(DdsSwitchParam)` | **0x389ae0** | 272 |
| `IpcProtocol41Net::IpcTxSetDualStandbyPrefModem(int, DdsSwitchParam)` | **0x389bf0** | 268 |
| `IpcProtocol41Net::IpcRxDualStandbyPref` | **0x3905e8** | 96 |
| `DataCallManager::DoSetPreferredDataModem` | **0x23dc24** | 616 |

GET: MOVZ **`0x1608`** → cmd **0x0816**, type **GET 0x02**, FMT **len=7** (empty). Live RESP plen=2.

SET (both TX): MOVZ **`0x1608`**, type **SET 0x03**, FMT **len=9**, payload **2 B**:

| byte | meaning |
|--|--|
| 0 | **DDS slot / standbyPref**. `csinc` after `cmp modemId, #0`: **0** = modemId 0 / ipc0 / SIM1; **1** = modemId ≠ 0 / ipc1 / SIM2. `IpcRx` stores this byte as `currentDds`. |
| 1 | **cause**. `DdsSwitchParam==1` path (`OnGetDualStandbyPrefDone` `movz w22, #1` + string **SWITCH_PARAM_TEMPORARY**) always sends **1** (no `ril.dds.datacross.slotid`). `!=1` reads that property and **skips send** if unset (−1). |

Log `standbyPref=%d, cause=%d`. SIM2 SET used vendor **`01 01`**.

### Leftover NOTI names (Probe 29)

| cmd | name |
|--|--|
| **0x0838** | No `NET_*` string. Packed MOVZ **`0x3808`** in `NetworkRespBuilder::BuildSignalBarInfosResponse` and `BuildUnsolicited`. Type **`SignalBarInfos`** exists. |
| **0x0F36** | No MOVZ, no name string. Group **0x0F** is **IMEI** (`IMEI_CMD`, `IMEI_SUB_CMD_UNDEFINED`). Index **0x36** unnamed. |

This drain: leftover only DISP **0x0706** (no 0x0838 / 0x0F36).

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p30` (`os/build/e4-ps-p30.c`, Zig musl). mseq from **0x70**.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** only (no STK) |
| GET `PHONE_STATE` **0x70** | **0x70** | **0x02** |
| GET `MODE_SEL` **0x71** | **0x71** | **0x0b** |
| GET `0x0808` **0x72** | **0x72** | body **`02`** (CS_PS) |
| GET `0x0809` **0x73** | **0x73** | body **`01`** |
| GET `GPRS_PS` **0x76** | **0x76** | **cid=0 attached=0** |
| GET `NET_REGIST` CS/PS | **0x75** (both) | CS **HOME** act=UMTS fail=0; PS **NONE fail=0x07** |
| GET `0x0816` **0x78** | **0x78** | RESP plen=2 body **`00 00`** (slot=0 cause=0) |
| SET `0x0816` **0x79** `01 01` | **0x79** | **GEN_PHONE_RES 0x0816 0x8000 SUCCESS** |
| GET `0x0816` **0x7a** | **0x7a** | RESP plen=2 body **`01 00`** (slot=**1** cause=0) |
| GET CS/PS / GPRS_PS | **0x7c** / **0x7d** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |
| SET `GPRS_PS` `01 00 00` **0x7e** | **0x7e** | **GEN_PHONE_RES 0x0D03 0x8000 SUCCESS** |
| GET CS/PS / GPRS_PS | **0x80** / **0x81** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **0x02** (CS_PS). GET after SET does **not** echo cause=1 (RESP cause=0; slot stuck at **1**).

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post-SET / post-attach / final | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet (ioctl empty). IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** DDS slot **0→1** is ACK’d and GET-confirmed; GMM **#7** is not “no data SIM selected”. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 31 — vendor PLMN_SEL SET AUTO then PsAttach (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Hypothesis: stored GMM **#7** needs a new CS/PS registration under current CS_PS + DDS SIM2. Recovered SET from `libsec-ril.so` **4541576** (host `e4-plmn-decode.c`). **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0808 SET (0x01 only if restore). **No** 0x0816 SET. **No** 0x0D14. **No** STK ACK. **No** PIN. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17019** (had `/dev/umts_ipc1`; not 416). RNDIS host **192.168.42.7**. wget **8840** `/tmp/ps-p31` **1102808**.

### Binary: `IpcTxNetSetNetSelectionAuto` / `Manual` / `GetNetSelectionMode`

Host `e4-plmn-decode.c`. String **`NET_PLMN_SEL`** @ `0x10c391`. No invented opcodes. No direct BL (vtable). `SetNetworkSelectionAutoHandler::DoEvent` @ **0x2cc0cc** passes **NetSelectModeType 0** (w2=xzr). `IpcModemImplNet::SetNetSelectionAuto` @ **0x35971c** forwards that enum unchanged.

| Symbol | VA | size |
|--|--|--|
| `IpcProtocol41Net::IpcTxNetGetNetSelectionMode()` | **0x388050** | 176 |
| `IpcProtocol41Net::IpcTxNetSetNetSelectionAuto(NetSelectModeType)` | **0x388100** | 220 |
| `IpcProtocol41Net::IpcTxNetSetNetSelectionManual(char*, RadioTechnology)` | **0x388200** | 328 |
| `IpcProtocol41Net::IpcRxNetPlmnSelect` | **0x38ec4c** | 104 |
| `DataCallManager::DoGprsDetach` | **0x240548** | 224 |

GET: MOVZ **`0x0208`** → cmd **0x0802**, type **GET 0x02**, FMT **len=7** (empty). Live RESP plen=1 mode byte.

SET AUTO: MOVZ **`0x0208`**, type **SET 0x03**, FMT **len=15**, payload **8 B** (STUR XZR then two STRB):

| byte | meaning |
|--|--|
| 0 | **mode**. `csel` after `cmp enum, #3`: **0x02** if enum ≠ 3 (AutoHandler **0**); **0x05** if enum == 3 (no RIL caller recovered — **not sent**). |
| 1–6 | **PLMN** ASCII zeros |
| 7 | **act 0xFF** |

This is the vendor AUTO path, not the Replicant 8-byte guess. Same bytes happen to match Replicant AUTO; SIM1 previously got **0x0064**, this SIM2 SET got **0x8000**.

SET MANUAL (not sent): mode **0x03**, act from RadioTechnology table else **0xFF**, PLMN via memcpy of caller string (pad `#` / `0x23` at [5] if strlen==6).

`IpcTxNetGetAvailableNetworks` @ **0x388348** is empty GET **0x0804** (scan). **Not sent.**

### Detach IpcTx

**No** `IpcTxGprs*Detach*` / `IpcTxPsDetach` / `IpcTxGmm*`. The only FMT detach is **`IpcTxPsAttach`** @ **0x36a388** with attach=0 (3 B `{0, flag, reason}`), same cmd **0x0D03** already used as attach `01 00 00`. `DoGprsDetach` is a `DataCallManager` wrapper (RIL req **0x9d**, then vtable) — not a new IPC. JSON `IpcTxPsAttach` has `detach_reason` / `reattach_flag` strings; this CP is **PROTOCOL_SIPC**. **No detach SET this pass.**

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p31` (`os/build/e4-ps-p31.c`, Zig musl). mseq from **0x82**. Uptime **35279 s**.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701** only (no STK) |
| GET `PHONE_STATE` **0x82** | **0x82** | **0x02** |
| GET `MODE_SEL` **0x83** | **0x83** | **0x0b** |
| GET `0x0808` **0x84** | **0x84** | body **`01`** (enum **0** CS) — **reverted** since Probe 30 **`02`** |
| GET `0x0809` **0x85** | **0x85** | body **`01`** |
| GET `0x0816` **0x86** | **0x86** | RESP plen=2 body **`01 00`** (slot=1) |
| GET `PLMN_SEL` **0x87** | **0x87** | body **`02`** AUTO |
| GET `GPRS_PS` **0x8a** | **0x8a** | **cid=0 attached=0** |
| GET `NET_REGIST` CS/PS | **0x89** | CS **HOME** act=UMTS fail=0; PS **NONE fail=0x07** |
| SET `PLMN_SEL` **0x8c** `02 00 00 00 00 00 00 FF` | **0x8c** | **GEN_PHONE_RES 0x0802 0x8000 SUCCESS** |
| 25 s drain | — | DISP **0x0706** only (no CS/PS status change NOTI) |
| GET `PLMN_SEL` / `0x0808` / `0x0816` | **0x8d** / **0x8e** / **0x8f** | mode **`02`**; domain **`01`**; slot **`01 00`** |
| GET CS/PS / GPRS_PS | **0x91** / **0x92** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |
| SET `GPRS_PS` `01 00 00` **0x93** | **0x93** | **GEN_PHONE_RES 0x0D03 0x8000 SUCCESS** |
| NOTI `NET_REGIST` | — | CS **HOME**; PS **NONE GMM#7** (same) |
| GET CS/PS / GPRS_PS | **0x95** / **0x96** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at GET **`01`** (did **not** SET 0x0808 back to 0x02; that SET is a closed lever). Hypothesis precondition “current CS_PS” was **not** live — CP had already reverted to CS-only.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post-SET / post-attach / final | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet (ioctl empty). IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** Vendor PLMN AUTO is ACK’d **8000** (not SIM1’s **0x0064**) but does **not** clear stored GMM **#7** or produce a new PS registration. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 32 — CS_PS then same-fd PLMN AUTO (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 31 SET PLMN AUTO while GET **0x0808** was already **`01`** (CP had reverted Probe 30’s **`02`**); the “reselect with CS_PS + DDS SIM2” hypothesis was not tested. This pass re-SETs **0x0808 `02`** (Probe 28 layout), confirms GET **`02`**, leaves DDS unless drifted, then **same fd** SET PLMN AUTO + drain + GET domain + `IpcTxPsAttach`. **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0808 SET **0x03**. **No** 0x0D14. **No** STK ACK. **No** PIN. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17035** (had `/dev/umts_ipc1`; not 416). RNDIS host **192.168.42.7**. wget **8841** `/tmp/ps-p32` **1107592**.

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p32` (`os/build/e4-ps-p32.c`, Zig musl). mseq from **0xA0**. Uptime **35729 s**. One ipc1 open.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701** only (no STK) |
| GET `PHONE_STATE` **0xA0** | **0xA0** | **0x02** |
| GET `MODE_SEL` **0xA1** | **0xA1** | **0x0b** |
| GET `0x0808` **0xA2** | **0xA2** | body **`01`** (enum **0** CS) — still reverted vs Probe 30 |
| GET `0x0809` **0xA3** | **0xA3** | body **`01`** |
| GET `0x0816` **0xA4** | **0xA4** | RESP plen=2 body **`01 00`** (slot=1) — no drift |
| GET `PLMN_SEL` **0xA5** | **0xA5** | body **`02`** AUTO |
| GET `GPRS_PS` **0xA8** | **0xA8** | **cid=0 attached=0** |
| GET `NET_REGIST` CS/PS | **0xA7** | CS **HOME** act=UMTS fail=0; PS **NONE fail=0x07** |
| SET `0x0808` **`02`** **0xAA** | **0xAA** | **GEN_PHONE_RES 0x0808 0x8000 SUCCESS** |
| GET `0x0808` **0xAB** | **0xAB** | body **`02`** (enum **2** CS_PS) |
| GET `0x0816` / CS/PS / GPRS_PS | **0xAC** | slot **`01 00`**; CS **HOME**; PS **NONE GMM#7**; **attached=0** |
| SET `0x0816` | — | **not sent** (still **`01 xx`**) |
| SET `PLMN_SEL` **0xB0** `02 00 00 00 00 00 00 FF` | **0xB0** | **GEN_PHONE_RES 0x0802 0x8000 SUCCESS** |
| 20 s drain | — | GEN only (no CS/PS status-change NOTI) |
| GET `PLMN_SEL` / `0x0808` / `0x0816` | **0xB1** / **0xB2** / **0xB3** | mode **`02`**; domain **`02`** (**stayed CS_PS**); slot **`01 00`** |
| GET CS/PS / GPRS_PS | **0xB5** / **0xB6** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |
| SET `GPRS_PS` `01 00 00` **0xB7** | **0xB7** | **GEN_PHONE_RES 0x0D03 0x8000 SUCCESS** |
| NOTI `NET_REGIST` | — | CS **HOME**; PS **NONE GMM#7** (same) |
| GET CS/PS / GPRS_PS | **0xB9** / **0xBA** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at GET **`02`** (CS_PS). **0x0808 stayed `02` across PLMN SET** (did not revert on this fd).

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post-SET / post-PLMN / post-attach / final | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet (ioctl empty). IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** CS_PS + DDS SIM2 + vendor PLMN AUTO on one fd is ACK’d and GET-confirmed; GMM **#7** is not “PLMN AUTO while CS-only”. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 33 — vendor MO Originate (v031, 2026-09-02)

Not the rmnet goal. Dest number is **not recorded here**. Probe 32 already finished (CS_PS stayed `02` across PLMN; GMM **#7**; rmnet 0). This pass recovered vendor TX from `libsec-ril.so` **4541576** (host `e4-call-decode.c` / `e4-call-args.c`). **No** `IpcTxCallOutgoing` / `IpcTxSetCallOutgoing` symbol — the packer is **`IpcProtocol41Call::IpcTxCallOriginate(char*, ClirType, CallType, int)`** @ **0x3601e4** size **400**. Replicant 90-byte **SET** `0x0201` was the wrong type and length.

| Symbol | VA | size | FMT |
|--|--|--|--|
| `IpcTxCallOriginate` | **0x3601e4** | 400 | len **99** (`0x63`), cmd packed **`0x0102`** → **0x0201**, type **EXEC 0x01** (not SET). Payload: LE u16 CallType map (voice `CallType=1` → table[0] **`0x0100`** @ `0x151a30`), Clir byte, strlen (cap **0x52**), prefix **0x11** if number[0] is `+` else **0x21**, number, last byte 4th int. |
| `IpcTxCallRelease` | **0x360a9c** | 176 | len **7**, cmd **`0x0302`** → **0x0203**, type **EXEC 0x01** |
| `IpcTxCallGetCallList` | **0x360374** | 176 | len **7**, cmd **`0x0602`** → **0x0206**, type **GET 0x02** |
| `IpcRxCallStatus` | **0x361fb4** | 268 | live NOTI **0x0205** |

No invented opcodes. No dest digits in this file. **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0808 SET **0x03**. **No** STK ACK (`0x0E0A` NOTI seen, not `0x0E03`). Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17052**. RNDIS host **192.168.42.7**. wget **8842** `/tmp/call-mo` **1102920**.

### Live ipc1 (SIM2 25501)

Static `/tmp/call-mo` (`os/build/e4-call-mo.c`, Zig musl). mseq from **0xC0**. International prefix path (`0x11`), voice **0x0100**, Clir **0**. Ring drain **25 s**, then vendor RELEASE.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701** / SERVING / CS **HOME** (no STK **0x0E03**) |
| GET `PHONE_STATE` **0xC0** | **0xC0** | **0x02** |
| GET `MODE_SEL` **0xC1** | **0xC1** | **0x0b** |
| GET `NET_REGIST` CS/PS | **0xC3** | CS **HOME** UMTS fail=0; PS **NONE GMM#7** |
| GET `CALL_LIST` **0xC5** | **0xC5** | RESP plen=1 body **`00`** |
| EXEC `0x0201` **0xC6** len 99 | **0xC6** | **GEN_PHONE_RES 0x0201 0x8000 SUCCESS** |
| NOTI `0x0205` (CALL_STATUS) | — | three during ring: `00 01 …` (b1=**1**) |
| GET `CALL_LIST` mid **0xC7** | **0xC7** | still plen=1 **`00`** |
| EXEC `0x0203` **0xC8** | **0xC8** | **GEN_PHONE_RES 0x0203 0x8005** |
| NOTI `0x0205` | — | `00 00 …` (b1=**0**) |
| GET CS / `CALL_LIST` | **0xCA** / **0xCC** | CS **HOME**; LIST **`00`** |

CS stayed **HOME**. CP **ONLINE**. Other NOTI while ringing: **0x0E0A**, **0x0908** / **0x0909**, **0x0B0D** JSON (PLMN **25501**, no dest digits logged here). **Do not ACK STK.**

### rmnet

Still **0**. This pass is CS identity only. Holder **428** CONT. Check the **other phone** for a missed call or operator SMS (that is how the Samsung SIM’s own MSISDN is learned). **Do not write that number here.**

## Probe 34 — live GET after MO + unused IpcTxGetPdpContext / IpcTxGetImsi (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 33 (vendor MO) left CS **HOME**; this pass is GET-first, then the next unused recovered IpcTx. **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0808 SET (already **`02`**; 0x01 only if restore). **No** 0x0D14. **No** 0x0D22 (MapDataProfile still VZW/USC-only). **No** STK ACK. **No** PIN. **No** second MO. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17296** (GET) then **17331** (unused IpcTx). RNDIS host **192.168.42.7**. wget **8844** `/tmp/ps-p34` **1100552**; wget **8845** `/tmp/ps-p35` **1109416**.

### Binary: unused IpcTx (host `e4-p33-decode.c`)

`libsec-ril.so` **4541576**. No invented opcodes. **No** `IpcTxGetMsisdn` / `IpcTxSimGetMsisdn`. String **`NET_SUBSCRIBER_NUM`** exists; no MOVZ **`0x0608`** send path. **MSISDN IpcTx not recovered.** `IpcTxSetAlwaysOnPdn` still **0x0D22 SET** len 9 with `MapDataProfile` VZW/USC — **not fully recovered, not SET.** No attach IpcTx besides **`IpcTxPsAttach` 0x0D03** (closed).

| Symbol | VA | size | FMT |
|--|--|--|--|
| `IpcProtocol41Data::IpcTxGetPdpContext()` | **0x36a1f4** | 116 | len **7**, packed **`0x040d`** → **0x0D04**, type **GET 0x02**, empty. Twin IilData @ **0x3b1640**. |
| `IpcProtocol41Misc::IpcTxGetImsi()` | **0x37b04c** | 176 | len **7**, packed **`0x020a`** → **0x0A02**, type **GET 0x02**, empty. String **`MISC_ME_IMSI`**. |
| `IpcTxGetPsiQueryInfo` | **0x36a268** | 288 | empty GET **0x0D26** (packed **`0x260d`**). **Not sent** (picked 0x0D04). |

### Live ipc1 (SIM2 25501) — GET first (`/tmp/ps-p34`, mseq **0xE0**)

Uptime **37052 s**. One ipc1 open.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701** only (no STK) |
| GET `PHONE_STATE` **0xE0** | **0xE0** | **0x02** |
| GET `MODE_SEL` **0xE1** | **0xE1** | **0x0b** |
| GET `0x0808` **0xE2** | **0xE2** | body **`02`** (enum **2** CS_PS) — **stayed** since Probe 32 / MO |
| GET `0x0809` **0xE3** | **0xE3** | body **`01`** |
| GET `0x0816` **0xE4** | **0xE4** | RESP plen=2 body **`01 00`** (slot=1) |
| GET `GPRS_PS` **0xE7** | **0xE7** | **cid=0 attached=0** |
| GET `NET_REGIST` CS/PS | **0xE6** | CS **HOME** act=UMTS fail=0; PS **NONE fail=0x07** |

GMM still **#7**, rmnet **0** — not a win. **No SET.**

### Live unused IpcTx (`/tmp/ps-p35`, mseq **0xF0**)

Baseline GET same as above (CS_PS **`02`**, DDS **`01 00`**, CS **HOME**, PS **NONE GMM#7**, attached=0). **0x0808 SET skipped** (still **`02`**).

| TX | aseq | result |
|--|--|--|
| GET `0x0D04` **0xF9** | **0xF9** | type-2 RESP plen=3 body **`01 18 00`** (no IPv4; no GEN) |
| GET `0x0A02` **0xFA** | **0xFA** | type-2 RESP plen=16 (length-prefixed ASCII IMSI; MCC/MNC **25501** matches SERVING; **digits not recorded**) |
| GET CS/PS / GPRS_PS | **0xFC** / **0xFD** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **`02`**. No GEN on these GETs.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre GET / post unused GET | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet (ioctl empty). IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** Vendor PDP GET after the MO is a 3-byte stub, not IP config. GMM **#7** is not “PDP context unread”. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 35 — IpcRx 0x0D04 `01 18 00` / CALL_STATUS 0x03 + AlwaysOn SET (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Decode GET **0x0D04** body **`01 18 00`**, name **0x0D10** st=**0x03**, finish `MapDataProfile` / AlwaysOn, list `IpcTxNetSetPreferredNetType` SET bytes (do not send 0x04/0x07). **No** original efs. **No** `POWER_OFF`. **No** `MODE_SEL` SET. **No** 0x0808 SET (already **`02`**). **No** 0x0D14. **No** STK ACK. **No** PIN. **No** second MO. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17340**. RNDIS host **192.168.42.7**. wget **8846** `/tmp/ps-p36` **1102504**.

### Binary: `IpcRxPdpContext` @ 0x36dc08 (GET 0x0D04 `01 18 00`)

Host `e4-p35-decode.c`. `libsec-ril.so` **4541576**. Type byte `[6]==SET 0x03` skips parse. GET/NOTI: **`[7]` = count**. Records are **2 B** starting at `[8]`: loop `x23=[9]`, `ldurb [x23,#-1]` + `ldrb [x23]`, step +2.

| body | IpcRx field |
|--|--|
| `01` | count **1** |
| `18` | record[0] raw (stored; **not** the 1 / 0x0A / 0x0B status map) |
| `00` | record[1] status → **not** {1, 0x0A, 0x0B} → mapped **0** (inactive) |

Status remap (same constants as CALL_STATUS): **0x01→4**, **0x0A→5**, **0x0B→6**, else **0**. Live **0x00** = inactive — no IP. **0x18 is not a 3GPP SM/ESM cause** (those skip #24 / reserved between #8 and #25). If read as a NAS/GMM cause it would be **#24 Not authorized for this CSG**; this RX does **not** run the CallStatus fail mapper on that byte. After SET CID **1**, GET count=1 + raw **0x18** + status **0** = one inactive context with CP reason **0x18**, not an IPv4 stub.

### Binary: `IpcRxGprsCallStatus` @ 0x36d29c (st=**0x03**)

`[7]` CID, `[8]` status, `[9]` end-reason (mapped @ **0x36d5c0** “Data call end reason” when not connected), `[0xa]`/`[0xb]` extras, `[0xc]` throttle.

| `[8]` | map | flags | log |
|--|--|--|--|
| **0x01** | 4 | connected (w23=1) | `CDMA Data call(%d)` |
| **0x0A** | 5 | — | (no disconnected) |
| **0x0B** | 6 | — | (no disconnected) |
| **else (incl. 0x03)** | 0 | disconnected (w24=1) | **`CDMA Data call(%d) disconnected`** |

**st=0x03 = disconnected** (default/else; not a named 1 / 0x0A / 0x0B special). Probe 26/29 NOTI `01 03` = cid **1** disconnected.

### Binary: `MapDataProfile` @ 0x36a9c0 (size 292) — recovered

Byte jump table @ **0x151cdb**. Non-VZW/USC (this SIM) path:

| `DataProfile` | return |
|--|--|
| 0 | VZW/USC → **3**, else **1** |
| 1 | **1** (Invalid Profile) |
| 2 | VZW/USC → **3**, else **1** |
| 3 | **2** |
| 4, 5 | **4** |
| 1003 (`0x3eb`) | **0x0B** |
| 1004 (`0x3ec`) | **5** |
| 1006 (`0x3ee`) | VZW/USC → **3**, else **1** |
| else | **1** + log `Invalid Profile(%d)` |

`IpcTxSetAlwaysOnPdn` @ **0x36c714**: **0x0D22 SET** len **9**, `{bool, MapDataProfile}`. `DoAlwaysOnPdn` default profile **1** → map **1**. Layout complete. Twin `IpcTxSetDataCallEstablish` @ **0x36b93c** is **`CDMA_DATA_CALL_ESTABLISH`** packed **0x0203** → cmd **0x0302** SET 1-byte (1=on, 2=off) — **not sent** (CDMA, not EPS).

No vendor **EPS / LTE attach** IpcTx besides closed **0x0D03** and unimplemented **0x0D14**. String **`GPRS_LTE_ATTACH_APN_INFO`** only. **`EPS_ATTACH`** none. **0x0809** SET still not recovered.

### Binary: `IpcTxNetSetPreferredNetType` SET bytes (do not send)

`ConvertPreferredNetTypeToIpcWithBitmask` @ **0x387db4**. Lookup @ **0x153e9c** enum **0..0x21** (default **0x2f** if enum>0x21). Optional OR **0x04** / **0x24** for enum 0–9. Bytes the binary can emit:

`01 02 03 04 08 0a 0b 10 11 12 13 18 19 1a 1b 20 24 27 2c 2f 37 3f 40 48 4a 4b 58 59 5a 5b 6c 6f 7f`

**0x07** is **not** in the table; it is **0x03|0x04** from the LTE-bit OR (Probe 18). Live GET **0x0b** = table[9]. **Never SET 0x04 or 0x07.** Prefer GET-only.

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p36` (`os/build/e4-ps-p36.c`, Zig musl **1102504**). mseq from **0x10**. Leftover NOTI: DISP **0x0706** / **0x0701**, CS **HOME**, PS **NONE GMM#7**, SERVING **25501**, act **0x04→0x03** (UMTS→GSM; cid changed). No STK.

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0x10** | **0x10** | **0x02** |
| GET `MODE_SEL` **0x11** | **0x11** | **0x0b** |
| GET `0x0808` **0x12** | **0x12** | body **`02`** (CS_PS) — **stayed** |
| GET `0x0809` **0x13** | **0x13** | **`01`** |
| GET `0x0816` **0x14** | **0x14** | **`01 00`** |
| GET CS/PS / GPRS_PS | **0x16** / **0x17** | CS **HOME** act=**GSM 0x03** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `0x0808` | — | **not sent** (already **`02`**) |
| SET `0x0D22` **`01 01`** **0x19** | **0x19** | **GEN_PHONE_RES 0x0D22 0x8000 SUCCESS** |
| GET `0x0D26` **0x1a** | **0x1a** | **GEN_PHONE_RES 0x0D26 0x8001** |
| GET CS/PS / GPRS_PS | **0x1c** / **0x1d** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **`02`**. No **0x0D09** / **0x0D10** after AlwaysOn.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post-SET | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet (ioctl empty). IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** AlwaysOn SET is ACK’d (**8000**) but does not clear GMM **#7**. GET 0x0D22 was **8001** (probe 25); SET-only on this CP. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 36 — MODE_SEL SET 0x0a LTE_WCDMA (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Confirm `0x0a` in `IpcTxNetSetPreferredNetType` from **this** `libsec-ril.so`, then SET if safe. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL **0x04/0x07**. **No** 0x0808 SET **0x03**. **No** 0x0D14. **No** STK ACK. **No** PIN. **No** second MO. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17353**. RNDIS host **192.168.42.7**. wget **8847** `/tmp/ps-p37` **1111104**.

### Binary: `0x0a` is LTE_WCDMA, not AOSP 10 global

Host `e4-p36-decode.c`. `ConvertPreferredNetTypeToIpcWithBitmask` @ **0x387db4** loads `table[enum]` @ **0x153e9c**. IPC bits on this .so: GSM **0x01**, UMTS **0x02**, CDMA **0x04**, LTE **0x08**, TD-SCDMA **0x10**, EvDo **0x20**, NR **0x40**. No `LTE_ONLY` / `LTE_WCDMA` cstrings (enum names from AOSP `RIL_PreferredNetworkType` vs table).

| AOSP enum | name | IPC emit | bits |
|--|--|--|--|
| 9 | LTE_GSM_WCDMA | **0x0b** | GSM\|UMTS\|LTE (live GET before this pass) |
| 10 | LTE_CDMA_EVDO_GSM_WCDMA (global) | **0x2f** | GSM\|UMTS\|CDMA\|LTE\|EvDo |
| 11 | LTE_ONLY | **0x08** | LTE |
| 12 | LTE_WCDMA | **0x0a** | **UMTS\|LTE** (no GSM, no CDMA) |
| 5 | CDMA_ONLY | **0x04** | CDMA (forbidden SET) |

**0x0a is not AOSP 10.** AOSP 10 emits **0x2f**. **0x0b** was already GSM+UMTS+LTE (not LTE_ONLY). **0x0a** is UMTS+LTE — valid for this Exynos UMTS/LTE unit, not CDMA-only. SET **0x0a** to leave GSM camp (Probe 35 act=GSM under 0x0b). Restore **0x0b** only if CS leaves HOME/ROAMING.

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p37` (`os/build/e4-ps-p37.c`, Zig musl **1111104**). mseq from **0x30**. Leftover NOTI: DISP **0x0706** only (no STK **0x0E03**).

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0x30** | **0x30** | **0x02** |
| GET `MODE_SEL` **0x31** | **0x31** | **0x0b** |
| GET `0x0808` **0x32** | **0x32** | **`02`** CS_PS |
| GET `0x0809` **0x33** | **0x33** | **`01`** |
| GET `0x0816` **0x34** | **0x34** | **`01 00`** |
| GET CS/PS / GPRS_PS | **0x36** / **0x37** | CS **HOME** act=**GSM 0x03** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `MODE_SEL` **0x0a** **0x39** | **0x39** | **GEN_PHONE_RES 0x080A 0x8000 SUCCESS**. FMT `08 00 39 ff 08 0a 03 0a` |
| GET `MODE_SEL` **0x3a** | **0x3a** | **0x0a** |
| 20 s drain | — | LTE NOTI act=**0x21** PS st=**0x07** fail=0; then CS **HOME UMTS 0x04**; PS **NONE** fail=0 then **GMM#7**. SERVING **25501**. No STK. |
| GET `0x0808` **0x3e** | **0x3e** | **`01`** (CS-only; drifted after MODE_SEL) |
| SET `0x0808` **`02`** **0x3f** | **0x3f** | **GEN 0x0808 0x8000**. GET **`02`**. CS **HOME UMTS** |
| SET `GPRS_PS` `01 00 00` **0x44** | **0x44** | **GEN_PHONE_RES 0x0D03 0x8000 SUCCESS** |
| GET CS/PS / GPRS_PS | **0x46** / **0x47** | CS **HOME** UMTS fail=0; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **restore MODE_SEL 0x0b did not run**. **0x0808 restore 0x01 did not run.** Domain left at **`02`**. MODE_SEL left at **0x0a**.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post-SET / post-attach | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet (ioctl empty). IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** SET **0x0a** retuned CS GSM→UMTS and produced an LTE NOTI, but GMM **#7** returned. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 37 — LTE-window GMM + MODE_SEL SET 0x2f (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Sample GMM **while** LTE (act=**0x21**) is indicated — Probe 36 saw an LTE NOTI then sampled after UMTS fallback. Same ipc1 fd; MODE_SEL already **0x0a**. Immediate GET `NET_REGIST` PS + `GPRS_PS` + rmnet on any NOTI/RESP act=**0x21**. If no LTE in ~25s and GET still **0x0a**, SET **0x2f** (this `.so` table[10] **LTE_CDMA_EVDO_GSM_WCDMA** = GSM\|UMTS\|CDMA\|LTE\|EvDo, AOSP-10 global — **not** CDMA-only **0x04**). Keep **0x0808 `02`**. Restore **0x0a**/**0x0b** only if CS drops. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL **0x04/0x07**. **No** 0x0808 SET **0x03**. **No** 0x0D14. **No** STK ACK. **No** PIN. **No** second MO. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17367**. RNDIS host **192.168.42.7**. wget **8848** `/tmp/ps-p38` **1115704**.

### Binary: `0x2f` is AOSP-10 global, not CDMA-only

Re-ran host `e4-p36-decode.c` on `libsec-ril.so` **4541576** before SET. `ConvertPreferredNetTypeToIpcWithBitmask` @ **0x387db4** table @ **0x153e9c**:

| AOSP enum | name | IPC emit | bits |
|--|--|--|--|
| 9 | LTE_GSM_WCDMA | **0x0b** | GSM\|UMTS\|LTE |
| 10 | LTE_CDMA_EVDO_GSM_WCDMA (global) | **0x2f** | GSM\|UMTS\|CDMA\|LTE\|EvDo |
| 12 | LTE_WCDMA | **0x0a** | UMTS\|LTE (Probe 36 SET) |
| 5 | CDMA_ONLY | **0x04** | CDMA (**forbidden SET**) |

**0x2f includes GSM+UMTS+LTE** (plus CDMA+EvDo). Not CDMA-only. Unused emit, not 0x04/0x07. SET allowed after no LTE under 0x0a.

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p38` (`os/build/e4-ps-p38.c`, Zig musl **1115704**). mseq from **0x50**. Leftover NOTI: DISP **0x0706** / **0x0701**, CS **HOME UMTS**, PS **NONE GMM#7**, SERVING **25501**. No STK.

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0x50** | **0x50** | **0x02** |
| GET `MODE_SEL` **0x51** | **0x51** | **0x0a** |
| GET `0x0808` **0x52** | **0x52** | **`02`** CS_PS |
| GET `0x0809` **0x53** | **0x53** | **`01`** |
| GET `0x0816` **0x54** | **0x54** | **`01 00`** |
| GET CS/PS / GPRS_PS | **0x55** / **0x56** / **0x57** | CS **HOME** UMTS fail=0; PS **NONE GMM#7**; **attached=0** |
| 25 s drain under **0x0a** | — | **no** act=**0x21**. No LTE-window GET. |
| SET `MODE_SEL` **0x2f** | — | **GEN_PHONE_RES 0x080A 0x8000 SUCCESS** |
| GET `MODE_SEL` | — | **0x0b** (CP folded global → GSM\|UMTS\|LTE; not stored as 0x2f) |
| SET `0x0808` **`02`** | — | **GEN 0x0808 0x8000** (drifted after MODE_SEL; `set0808=1`) |
| 25 s drain after 0x2f | — | **no** act=**0x21**. No LTE-window GET. |
| GET `0x0808` **0x63** | **0x63** | **`02`** |
| GET CS/PS / GPRS_PS | **0x64** / **0x65** / **0x66** | CS **HOME** UMTS fail=0; PS **NONE fail=0**; **attached=0** |

**GMM#7 during LTE: not sampled** (`lte_seen=0`). No NOTI/RESP with act=**0x21** in either 25 s wall-clock drain, so the immediate PS/GPRS GET never fired. Baseline still had GMM **#7**; final PS fail=**0** (still **NONE**, not attached). CS stayed **HOME** → **restore MODE_SEL 0x0a/0x0b did not run**. Domain left at **`02`**. MODE_SEL left at **0x0b**.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / mid / post | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet (ioctl empty). IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** SET **0x2f** is ACK’d (**8000**) but this CP reports **0x0b**; no LTE window this pass. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 38 — raw PS regist hex + fail=0 wait (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 37 final GET was PS **NONE fail=0** (baseline still GMM#7). This pass dumps **raw NET_REGIST PS** so fail=0 is not a parse miss, then waits **50 s** with GET every ~5 s. If still NONE: SET `PsAttach` `01 00 00`. Vendor PDP only if PS HOME/ROAMING. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL SET (left **0x0b**). **No** 0x0808 SET (already **`02`**). **No** 0x0D14. **No** STK ACK. **No** PIN. **No** second MO. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17382**. RNDIS host **192.168.42.7**. wget **8849** `/tmp/ps-p39` **1138320**.

### Raw NET_REGIST PS (fail=0 is real)

FMT **n=27** plen=**20**, fail at byte **[17]** **present** (`fail_missing=0`). Same layout as Probe 16 GMM#7 sample; only the fail nibble differs.

| when | act | st | body (20 B) | fail |
|--|--|--|--|--|
| leftover | — | (no PS NOTI; DISP **0x0706** only) | — | — |
| baseline GET | **UMTS 0x04** | **NONE 0x01** | `04 03 01 b5 c3 8d 01 13 1d 05 **00** c3 8d 02 02 01 ff ff 00 00` | **0x00** |
| T+0 … T+50 s (10 GET) | **0x04** | **NONE** | same body, fail **00** every sample | **0** |
| NOTI after PsAttach | **0x04** then **GSM 0x03** | **NONE** | `04 03 01 … **07** …` then `03 03 01 b5 c3 8d c7 d8 00 00 **07** …` | **0x07** |
| final GET | **GSM 0x03** | **NONE** | `03 03 01 b5 c3 8d c7 d8 00 00 **07** c3 8d 02 02 00 ff ff 00 00` | **0x07** |

CS body stayed HOME fail=0 (`04 02 02 … 00 …` then `03 02 02 … 00 …`). GMM **#7** was **cleared** for ~50 s (searching, still NONE), then **re-asserted** by `IpcTxPsAttach`. Not a short-packet parse of fail=0.

### Live ipc1 (SIM2)

Static `/tmp/ps-p39` (`os/build/e4-ps-p39.c`, Zig musl **1138320**). mseq from **0x70**.

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0x70** | **0x70** | **0x02** |
| GET `MODE_SEL` **0x71** | **0x71** | **0x0b** |
| GET `0x0808` **0x72** | **0x72** | **`02`** CS_PS — **no SET** |
| GET `0x0809` / `0x0816` | **0x73** / **0x74** | **`01`** / **`01 00`** |
| GET CS/PS / GPRS_PS | **0x75** / **0x76** / **0x77** | CS **HOME UMTS** fail=0; PS **NONE fail=0**; **attached=0** |
| 50 s poll (~5 s) | **0x79…** | 11× PS **NONE fail=0** act=UMTS. **lte_seen=0**. No HOME/ROAMING |
| SET `GPRS_PS` `01 00 00` **0x97** | **0x97** | **GEN 0x0D03 0x8000**. NOTI PS **GMM#7**. CS **HOME** UMTS→GSM |
| GET final | **0x9b** / **0x9c** / **0x9d** | CS **HOME GSM**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **0x0808 restore 0x01 did not run**. MODE_SEL restore did **not** run. Domain left at **`02`**. Vendor **0x0D1B / 0x0D01 / 0x0D04** skipped (PS never camped).

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / wait / post-attach | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet. IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** fail=0 was a quiet window, not attach. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 39 — speaker tone + vendor MO 40 s (v031, 2026-09-02)

Not the rmnet goal. Dest number is **not recorded here**. Probe 38 already finished (fail=0 was real for 50 s; attach brought GMM#7 back; rmnet 0). User heard nothing on the other phone after Probe 33 (CALL_LIST stayed `00`). This pass: verify originate packing, ring **40 s**, play **local** `/sbin/beep` on the LIVE v026 speaker path so the Samsung itself is audible. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL SET. **No** 0x0808 SET **0x03**. **No** STK ACK. **No** PIN. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17397**. RNDIS host **192.168.42.7**. wget **8850** `/tmp/call-p39` **1145176**.

### Originate packing (not BCD)

`IpcTxCallOriginate` @ **0x3601e4** `memcpy`s the `char*` after prefix. Live pack: nlen=**13**, prefix=**0x11** (first byte `+` / **0x2b**), CallType **0x0100**, Clir **0**, ASCII digits after `+`. **digits_ok=1**. Same layout as Probe 33 — packing was already correct. Not GSM 7-bit BCD.

### Speaker (local, not CP downlink)

No CP voice PCM path recovered this pass (no ABOX call FE / `IpcTx` audio). Forked **`/sbin/beep`** five times during the ring (pcmC0D1p / RDMA1 / SIFS1 / UAIF1 / SMA1303). Each child printed **`beep: ok rdma1 sifs1 tonegen 57600 frames`** and exit **0**. **spk_ok=1**. Did **not** toggle Codec Enable, SMA I2C reset, Force AMP Power Down, or `ABOX SPUS ASRC3`. This is an 880 Hz local square, not network ringtone / in-call audio.

### Live ipc1 (SIM2)

Static `/tmp/call-p39` (`os/build/e4-call-p39.c`, Zig musl). mseq from **0xE0**. Leftover: CS **HOME UMTS** fail=0; PS **NONE GMM#7**; SERVING **25501**. No STK **0x0E03**.

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0xE0** | **0xE0** | **0x02** |
| GET `MODE_SEL` **0xE1** | **0xE1** | **0x0b** |
| GET CS/PS / GPRS_PS | **0xE2** / **0xE3** / **0xE4** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| GET `CALL_LIST` **0xE5** | **0xE5** | plen=1 **`00`** |
| EXEC `0x0201` **0xE6** len 99 | **0xE6** | **GEN 0x0201 0x8000**. NOTI **0x0205** b1=**1** (several) |
| GET `CALL_LIST` mid-ring **0xE7** | **0xE7** | plen=**22** count=**1** — prefix **0x11** nlen=**13** first=**0x2b** (ASCII international; dest **not** logged). **Not empty.** |
| GET CS/PS mid | — | CS **HOME**; PS **NONE GMM#7**; **attached=0** |
| GET `CALL_LIST` end-of-40s | **0xF3** / **0xF7** | plen=1 **`00`** again |
| EXEC `0x0203` **0xF8** | **0xF8** | **GEN 0x0203 0x8005**. NOTI **0x0205** b1=**0** |
| GET CS / LIST after | **0xF9** / **0xFC** | CS **HOME**; LIST **`00`** |

Mid-ring CALL_LIST had one entry (unlike Probe 33). It was gone by T+40 s before RELEASE. That is a CP call object, still **not** proof the other phone rang. **SMS skipped**: CALL_LIST was not empty; `IpcTxSendSms` @ **0x39c538** size **324** named but FMT/PDU **not** recovered — no guessed PDU.

### rmnet

Still **0**. Holder **428** CONT. CP **ONLINE**. **Do not ACK STK.** Dest number stays out of this file.

## Probe 40 — GET-only GMM#7 timeline, no attach (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 38 hypothesis: local `PsAttach` **caused** GMM#7 after a real fail=0 window. This pass is **GET-only** until PS HOME/ROAMING. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL SET (left **0x0b**). **No** 0x0808 SET (already **`02`**). **No** 0x0D14. **No** `PsAttach` while NONE. **No** PDP (PS never HOME). **No** STK ACK. **No** PIN. **No** MO. **No** `/sbin/beep`. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17416**. RNDIS host **192.168.42.7**. wget **8851** `/tmp/ps-p40` **1138560**.

### Live ipc1 (SIM2)

Static `/tmp/ps-p40` (`os/build/e4-ps-p40.c`, Zig musl **1138560**). mseq from **0x10**. Leftover: DISP **0x0706** only (no PS NOTI).

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0x10** | **0x10** | **0x02** |
| GET `MODE_SEL` **0x11** | **0x11** | **0x0b** |
| GET `0x0808` **0x12** | **0x12** | **`02`** CS_PS — **no SET** |
| GET `0x0809` / `0x0816` | **0x13** / **0x14** | **`01`** / **`01 00`** |
| GET CS/PS / GPRS_PS | **0x15** / **0x16** / **0x17** | CS **HOME UMTS** fail=0; PS **NONE fail=0x07**; **attached=0** |
| 100 s poll (~8 s) | **0x19…0x3f** | **13** GET + baseline + final = **15**× PS **NONE fail=0x07** act=UMTS. **lte_seen=0**. **No HOME/ROAMING**. **No attach.** |
| GET final | **0x40** / **0x41** / **0x42** | CS **HOME UMTS**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **0x0808 restore 0x01 did not run**. MODE_SEL restore did **not** run. Domain left at **`02`**. Vendor **0x0D1B / 0x0D01 / 0x0D04** skipped (PS never camped). `did_attach=0` `did_pdp=0` `fail7_no_att=0` `saw_fail0=0`.

### Raw NET_REGIST PS (GMM#7 sticky)

FMT **n=27** plen=**20**, fail at byte **[17]** **present** (`fail_missing=0`). Same layout as Probe 38; fail nibble stayed **07**.

| when | act | st | body (20 B) | fail |
|--|--|--|--|--|
| leftover | — | (no PS NOTI; DISP **0x0706** only) | — | — |
| baseline GET | **UMTS 0x04** | **NONE 0x01** | `04 03 01 b5 c3 8d fe 12 1d 05 **07** c3 8d 02 02 01 ff ff 00 00` | **0x07** |
| T+0 … T+100 s (13 GET) | **0x04** | **NONE** | same body, fail **07** every sample | **7** |
| final GET | **0x04** | **NONE** | same | **0x07** |

CS body stayed HOME fail=0 (`04 02 02 … 00 …`) UMTS the whole window (did **not** drop to GSM — that drop in Probe 38 was after attach). GMM **#7** did **not** clear in 100 s without attach, and fail **never** returned to 0 (`saw_fail0=0`). The Probe 38 fail=0 window was **not** reproduced after the earlier attach.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / wait / post | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet. IPv4 only on **rndis0**. Holder **428** CONT, still alive (`radio-boot`, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** GET-only does not lift GMM#7. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 41 — vendor SMS EXEC 0x0401 (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. User: second phone saw **no** missed call / SMS from Probe 39 MO. Probe 40 GET-only already showed GMM#7 sticky — this pass **snapshots PS once** (no 100 s wait, **no PsAttach**), then SMS from SIM2/ipc1. Dest number is **not** logged here. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL SET. **No** 0x0808 SET (already **`02`**). **No** 0x0D14. **No** STK ACK. **No** PIN. **No** MO. **No** `/sbin/beep`. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17473**. RNDIS host **192.168.42.7**. wget **8852** `/tmp/sms-p41` **1130240**.

### IpcTxSendSms @ 0x39c538 (324 B)

`libsec-ril.so` 4541576. Mangled `_ZN16IpcProtocol41Sms12IpcTxSendSmsEhiihhPcm`. FMT **recovered from stores**, not guessed:

| store | meaning |
|--|--|
| `movz w8, #0x104` + `strh [sp,#4]` | packed cmd **0x0104** → group **0x04** index **0x01** = **`0x0401` SMS_SEND_MSG** |
| `strb 0x01 [sp,#6]` | type **EXEC 0x01** (not SET) |
| `strb` arg1 / arg4 → `[sp,#7]` / `[sp,#8]` | prefix; `DoSendSms` defaults both **0x01** |
| BLR vtable+0x78 → `[sp,#9]` | converter result; default **0x01** (GSM `IpcRxSendMsg` network_type **1 or 2**) |
| `strb` pdulen `[sp,#0xa]` | n = SCA+TPDU |
| `bl 0x409ab0` dest=`sp+0xb` n=`ulong` cap **0x100** | memcpy body |
| `strh` length = pdulen + **0x0b** | FMT length |

String `SMS_SEND_MSG` @ `0x135c1a`. `ConvertToIpcCmd` RIL **0x191** → group **0x4**. `IpcRxSendMsg` @ `0x39e0ac`: payload `[7]` = network_type (1 or 2). `DoSendSms` requires TP-DA EXT=1 (**TOA 0x91**); `"Use default SMSC"` → SCA first byte **0x00**. TPDU is 23.040 SMS-SUBMIT (FO **0x11**, VP relative).

### Live ipc1 (SIM2)

Static `/tmp/sms-p41` (`os/build/e4-sms-p41.c`, Zig musl **1130240**). mseq from **0x50**. Leftover: DISP **0x0706** / **0x0701** only (no PS NOTI).

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0x50** | **0x50** | **0x02** |
| GET `MODE_SEL` **0x51** | **0x51** | **0x0b** |
| GET `0x0808` **0x52** | **0x52** | **`02`** CS_PS — **no SET** |
| GET `0x0809` / `0x0816` | **0x53** / **0x54** | **`01`** / **`01 00`** |
| GET CS/PS / GPRS_PS | **0x55** / **0x56** / **0x57** | CS **HOME UMTS** fail=0; PS **NONE fail=0x07**; **attached=0** |
| EXEC `SMS_SEND_MSG` **0x59** | **0x59** | GEN **0x0004** (not 0x8000). **No 0x0401 RX / no RP.** |
| GET after SMS / final | **0x5a…0x5f** | CS **HOME UMTS**; PS **NONE GMM#7**; **attached=0** |

SMS EXEC **was sent** (`sms_sent=1`). Type **EXEC**, not SET. flen=**32**, prefix `01 01 01`, lenb=`15`, SCA `00`, FO `11`, DA digits=12, TOA **0x91**. Header only: `20 00 59 ff 04 01 01 01 01 01 15 00 11 00 0c 91` (DA BCD omitted). 15 s drain after EXEC: only GEN + DISP **0x0701**. `sms_rx=0` `net=-1`. CS stayed **HOME** → **0x0808 restore 0x01 did not run**. MODE_SEL restore did **not** run. Domain left at **`02`**. **No PDP. No PsAttach.**

### Raw NET_REGIST PS (GET-only snapshot)

FMT **n=27** plen=**20**, fail at byte **[17]** **present**. Same body as Probe 40. Fail **never** 0 this pass (`saw_fail0=0`).

| when | act | st | body (20 B) | fail |
|--|--|--|--|--|
| leftover | — | (no PS NOTI) | — | — |
| baseline GET | **UMTS 0x04** | **NONE 0x01** | `04 03 01 b5 c3 8d fe 12 1d 05 **07** c3 8d 02 02 01 ff ff 00 00` | **0x07** |
| after SMS EXEC | **0x04** | **NONE** | same | **0x07** |
| final GET | **0x04** | **NONE** | same | **0x07** |

FAIL HIST **n=3**, all **NONE fail=7** act=UMTS. CS body stayed HOME fail=0 UMTS. SMS EXEC did **not** change PS/GMM.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** |

No IPv4 on rmnet. IPv4 only on **rndis0**. Holder **428** CONT (`radio-boot`, post **R** then **S**, ipc0+rfs0). PID **416** still holds ipc1. **Data-plane goal not complete.** GEN **0x0004** means CP did not accept SMS_SEND_MSG (no RP). **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 42 — SET 0x2f then GET-only 120 s, no attach (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 37: SET **0x2f** from MODE_SEL **0x0a** → GET folded **0x0b**, then PS **NONE fail=0**. Probe 38 attached during that window and **reintroduced GMM#7**. Probe 40 GET-only while already #7 never returned fail=0. This pass re-SET **0x2f** (same FMT as Probe 37) from already-folded **0x0b**, then GET-only **120 s** — **no PsAttach, no 0x0D03, no PDP** unless PS HOME/ROAMING. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL **0x04/0x07**. **No** 0x0808 SET **0x03**. **No** 0x0D14. **No** STK ACK. **No** PIN. **No** MO. **No** SMS. **No** `/sbin/beep`. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17496**. RNDIS host **192.168.42.7**. wget **8853** `/tmp/ps-p42` **1134040**.

### Live ipc1 (SIM2)

Static `/tmp/ps-p42` (`os/build/e4-ps-p42.c`, Zig musl **1134040**). mseq from **0x20**. Leftover: DISP **0x0706** / **0x0701** only (no PS NOTI).

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0x20** | **0x20** | **0x02** |
| GET `MODE_SEL` **0x21** | **0x21** | **0x0b** |
| GET `0x0808` **0x22** | **0x22** | **`02`** CS_PS — **no SET** |
| GET `0x0809` / `0x0816` | **0x23** / **0x24** | **`01`** / **`01 00`** |
| GET CS/PS / GPRS_PS | **0x25** / **0x26** / **0x27** | CS **HOME UMTS** fail=0; PS **NONE fail=0x07**; **attached=0** |
| SET `MODE_SEL` **0x2f** **0x29** | **0x29** | **GEN 0x080A 0x8000 SUCCESS**. FMT `08 00 29 ff 08 0a 03 2f` |
| GET `MODE_SEL` **0x2a** | **0x2a** | **0x0b** (CP folded again; not stored as 0x2f) |
| GET `0x0808` **0x2b** | **0x2b** | **`02`** — no re-SET |
| 120 s poll (~8 s) | **0x2c…** | **15** GET + baseline + final = **17**× PS **NONE fail=0x07** act=UMTS. **lte_seen=0**. **No HOME/ROAMING**. **No attach. No PDP.** |
| GET final | — | CS **HOME UMTS**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **0x0808 restore 0x01 did not run**. MODE_SEL restore did **not** run. Domain left at **`02`**. Vendor **0x0D1B / 0x0D01 / 0x0D04** skipped (PS never camped). `did_attach=0` `did_pdp=0` `fail7_no_att=0` `saw_fail0=0` `set2f=1`.

### Raw NET_REGIST PS (fail=0 not reproduced)

FMT **n=27** plen=**20**, fail at byte **[17]** **present** (`fail_missing=0`). Same body as Probe 40/41. Fail nibble stayed **07** through SET 0x2f and the whole 120 s.

| when | act | st | body (20 B) | fail |
|--|--|--|--|--|
| leftover | — | (no PS NOTI; DISP **0x0706** / **0x0701**) | — | — |
| baseline GET | **UMTS 0x04** | **NONE 0x01** | `04 03 01 b5 c3 8d fe 12 1d 05 **07** c3 8d 02 02 01 ff ff 00 00` | **0x07** |
| T+0 … T+120 s (15 GET) | **0x04** | **NONE** | same body, fail **07** every sample | **7** |
| final GET | **0x04** | **NONE** | same | **0x07** |

CS body stayed HOME fail=0 (`04 02 02 … 00 …`) UMTS the whole window. GMM **#7** did **not** clear after re-SET **0x2f** from already-folded **0x0b**. The Probe 37 fail=0 window was after SET **0x2f** from **0x0a**, not from **0x0b**. Re-SET does not recreate it.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / wait / post | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet. IPv4 only on **rndis0**. Holder **428** CONT (`radio-boot`, post **R**). PID **416** still holds ipc1. **Data-plane goal not complete.** SET **0x2f** is ACK’d (**8000**) but this CP still reports **0x0b**; GET-only 120 s after that SET never saw fail=0. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 43 — SET 0x0a then 0x2f, GET-only fail=0 wait (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 42 hole: SET **0x2f** from already-folded **0x0b** left GMM#7 (`saw_fail0=0`). Probe 37 fail=0 was after SET **0x2f** when GET-before was **0x0a**. This pass recreates **0x0a then 0x2f**, GET PS immediately, then GET-only — **no PsAttach, no 0x0D03, no PDP** unless PS HOME/ROAMING. fail=0 → **110 s**; fail stays 0x07 → 30 s then stop. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL **0x04/0x07**. **No** 0x0808 SET **0x03**. **No** 0x0D14. **No** STK ACK. **No** PIN. **No** MO. **No** SMS. **No** `/sbin/beep`. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17516**. RNDIS host **192.168.42.7**. wget **8854** `/tmp/ps-p43` **1145008**.

### Live ipc1 (SIM2)

Static `/tmp/ps-p43` (`os/build/e4-ps-p43.c`, Zig musl **1145008**). mseq from **0x60**. Leftover: DISP **0x0706** only (no PS NOTI).

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0x60** | **0x60** | **0x02** |
| GET `MODE_SEL` **0x61** | **0x61** | **0x0b** |
| GET `0x0808` **0x62** | **0x62** | **`02`** CS_PS |
| GET `0x0809` / `0x0816` | **0x63** / **0x64** | **`01`** / **`01 00`** |
| GET CS/PS / GPRS_PS | **0x65** / **0x66** / **0x67** | CS **HOME UMTS** fail=0; PS **NONE fail=0x07**; **attached=0** |
| SET `MODE_SEL` **0x0a** **0x69** | **0x69** | **GEN 0x080A 0x8000 SUCCESS**. FMT `08 00 69 ff 08 0a 03 0a` |
| GET `MODE_SEL` **0x6a** | **0x6a** | **0x0a** |
| 5 s drain + GET `0x0808` | — | LTE NOTI act=**0x21** PS st=**0x07** fail=0. **0x0808 drifted `01`** |
| SET `0x0808` **`02`** **0x6c** | **0x6c** | **GEN 0x0808 0x8000**. GET **`02`**. CS **HOME UMTS**. PS **NONE fail=0** |
| SET `MODE_SEL` **0x2f** **0x71** | **0x71** | **GEN 0x080A 0x8000 SUCCESS**. FMT `08 00 71 ff 08 0a 03 2f`. GET-before was **0x0a** |
| GET `MODE_SEL` **0x72** | **0x72** | **0x0b** (CP folded again) |
| GET PS immediately | **0x74** | UMTS PS st=**0x07** fail=**0** (not GMM#7). `fail_after_2f=0` `saw_fail0=1` |
| GET `0x0808` **0x76** | **0x76** | **`01`** again — re-SET **`02`** **0x77** GEN **8000** |
| 110 s poll (~8 s) | — | **14** GET PS **NONE fail=0** act=UMTS. **lte_seen=1** (NOTI after both SETs; **lte_gets=0** during wait). **No HOME/ROAMING**. **No attach. No PDP.** |
| GET final | — | CS **HOME UMTS**; PS **NONE fail=0**; **attached=0** |

CS stayed **HOME** → **0x0808 restore 0x01 did not run**. MODE_SEL restore did **not** run. Domain left at **`02`**. Vendor **0x0D1B / 0x0D01 / 0x0D04** skipped (PS never camped). `did_attach=0` `did_pdp=0` `fail7_no_att=0` `saw_fail0=1` `set0a=1` `set2f=1`.

### Raw NET_REGIST PS (Probe 37 fail=0 reproduced)

FMT **n=27** plen=**20**, fail at byte **[17]** **present** (`fail_missing=0`). Baseline fail nibble **07**; after SET **0x0a** it cleared and stayed **00**.

| when | act | st | body (20 B) | fail |
|--|--|--|--|--|
| leftover | — | (no PS NOTI; DISP **0x0706**) | — | — |
| baseline GET | **UMTS 0x04** | **NONE 0x01** | `04 03 01 b5 c3 8d fe 12 1d 05 **07** c3 8d 02 02 01 ff ff 00 00` | **0x07** |
| NOTI after SET 0x0a | **LTE 0x21** | **0x07** | `21 03 07 00 00 00 48 37 42 06 **00** …` | **0** |
| GET after re-SET 0x0808 | **0x04** | **NONE** | `04 03 01 b5 c3 8d fe 12 1d 05 **00** c3 8d 02 02 01 ff ff 00 00` | **0** |
| NOTI after SET 0x2f | **LTE 0x21** | **0x07** | same LTE body, fail **00** | **0** |
| GET PS after 0x2f | **0x04** | **0x07** | `04 03 07 00 00 00 fe 12 1d 05 **00** …` | **0** |
| T+0 … T+110 s (14 GET) | **0x04** | **NONE** | `04 03 01 b5 c3 8d fe 12 1d 05 **00** c3 8d 02 02 01 ff ff 00 00` | **0** |
| final GET | **0x04** | **NONE** | same | **0** |

CS body stayed HOME fail=0 (`04 02 02 … 00 …`) UMTS the whole window. GMM **#7** **cleared** after SET **0x0a** (same path as Probe 36/37) and did **not** return during 110 s GET-only. PS stayed **NONE** — fail=0 is searching, not camp. **Did not attach** (Probe 38: attach reintroduced #7).

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / wait / post | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet. IPv4 only on **rndis0**. Holder **428** CONT (`radio-boot`, post **R**). PID **416** still holds ipc1. **Data-plane goal not complete.** SET **0x0a** sticks; SET **0x2f** still folds to **0x0b**; fail=0 window is real again but GET-only does not camp PS. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 44 — vendor PDP in fail=0 window (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 43 holes: (1) PS may not have been GET during LTE; (2) vendor PDP never ran in the fail=0 window (only under GMM#7). This pass recreates **0x0a then 0x2f**, arms LTE GET on act=**0x21**, then SET **0x0D1B / 0x0D01 / 0x0D04** as soon as fail=**0** (LTE or UMTS) — **no PsAttach, no 0x0D03**. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL **0x04/0x07**. **No** 0x0808 SET **0x03**. **No** 0x0D14. **No** STK ACK. **No** PIN. **No** MO. **No** SMS. **No** `/sbin/beep`. Holder **428** STOP/CONT only (did **not** STOP 416). Killed leftover **sleep** PID **17533**. RNDIS host **192.168.42.7**. wget **8855** `/tmp/ps-p44` **1146752**.

### Live ipc1 (SIM2)

Static `/tmp/ps-p44` (`os/build/e4-ps-p44.c`, Zig musl **1146752**). mseq from **0x80**. `g_win` armed before leftover so act=**0x21** fires GET.

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0x80** | **0x80** | (baseline; CS later HOME) |
| GET `MODE_SEL` **0x81** | **0x81** | **0x0b** |
| GET `0x0808` **0x82** | **0x82** | **`02`** CS_PS |
| GET CS/PS / GPRS_PS | **0x85** / **0x86** / **0x87** | CS **HOME UMTS** fail=0; PS **NONE fail=0**; **attached=0** |
| SET `MODE_SEL` **0x0a** **0x89** | **0x89** | **GEN 0x080A 0x8000 SUCCESS** |
| GET `MODE_SEL` **0x8d** | **0x8d** | **0x0a** |
| LTE NOTI + **8×** LTE-window GET | — | act=**0x21** fail=**0**; PS st=**0x07** then **NONE**; **attached=0**; rmnet **0**. **0x0808 drifted `01`** |
| SET `0x0808` **`02`** **0xa4** | **0xa4** | **GEN 0x0808 0x8000**. GET **`02`**. CS **HOME UMTS**. PS **NONE fail=0** |
| SET `MODE_SEL` **0x2f** **0xa9** | **0xa9** | **GEN 0x080A 0x8000 SUCCESS**. GET-before was **0x0a** |
| GET `MODE_SEL` **0xaa** | **0xaa** | **0x0b** (CP folded again) |
| GET PS immediately | **0xac** | UMTS PS st=**0x07** fail=**0**. `fail_after_2f=0` `saw_fail0=1` |
| LTE NOTI after 0x2f | — | act=**0x21** st=**0x07** fail=**0**. `lte_gets` already **8** — no 9th GET |
| GET `0x0808` **0xae** | **0xae** | **`01`** again — re-SET **`02`** **0xaf** GEN **8000** |
| SET `0x0D1B` **0xb4** 0xCB `lte_internet`+`internet` | **0xb4** | **GEN 0x0D1B 0x8000 SUCCESS** |
| SET `0x0D01` **0xb5** 0x95 CID=1 APN `internet` | **0xb5** | **GEN 0x0D01 0x8000 SUCCESS** |
| SET `0x0D04` **0xb6** 0xF8 | **0xb6** | **GEN 0x0D04 0x8000 SUCCESS**. NOTI **0x0D10** cid=**1** st=**0x03**. **No 0x0D09** |
| GET final | — | CS **HOME UMTS**; PS **NONE fail=0**; **attached=0** |

CS stayed **HOME** → **0x0808 restore 0x01 did not run**. MODE_SEL restore did **not** run. Domain left at **`02`**. `did_attach=0` `did_pdp=1` `fail7_no_att=0` `saw_fail0=1` `set0a=1` `set2f=1`. **GEN_0D03=0xffff** (GET only).

### LTE-window PS (hole 1)

Immediate GET on act=**0x21** (`lte_gets=8`). GMM/fail **during LTE**:

| when | act | st | fail | attached | rmnet |
|--|--|--|--|--|--|
| NOTI after SET 0x0a | **LTE 0x21** | **0x07** | **0** | — | — |
| LTE GET #1 | UMTS then more LTE | **0x07** / **NONE** | **0** | **0** | 0 |
| LTE GET #2–8 (after GET 0x0a / 0x0808 drain) | **LTE 0x21** CS+PS | **NONE** | **0** | **0** | 0 |
| NOTI after SET 0x2f | **LTE 0x21** | **0x07** | **0** | — | (GET cap exhausted) |
| GET after 0x2f | **UMTS 0x04** | **0x07** | **0** | **0** | 0 |

`lte_fail=0` `lte_st=7` `lte_att=0`. Fail stayed **0** on every LTE sample. PS never HOME. No IPv4.

### Vendor PDP in fail=0 (hole 2)

Fired after 0x0a+0x2f + re-SET **0x0808 `02`**, current GET **UMTS NONE fail=0** (`pdp_act=0x04` `pdp_fail=0` `pdp_st=1`). Probe 26/29 layouts.

| cmd | GEN | follow-up |
|--|--|--|
| **0x0D1B** | **0x8000** | — |
| **0x0D01** | **0x8000** | — |
| **0x0D04** | **0x8000** | **0x0D10** cid=**1** st=**0x03** (**disconnected**, Probe 36 map). **0x0D09 absent** (`ipcfg=0`) |

After PDP: PS still **NONE fail=0**, **attached=0**, rmnet **0**, no IPv4. ACK’d profile/define/set does not bring a bearer without camp or attach.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / LTE GET / post PDP | ONLINE | rx=tx=**0** |

`/proc/net/dev`: all `rmnet*` bytes=packets=**0** (`rmnet0/1/2` tx drop=1 only). No IPv4 on rmnet. IPv4 only on **rndis0**. Holder **428** CONT (`radio-boot`, post **R**). PID **416** still holds ipc1. **Data-plane goal not complete.** SET **0x0a** sticks; SET **0x2f** still folds to **0x0b**; fail=0 window is real and vendor PDP **ACKs** but **0x0D10=disconnected** and no IP. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 45 — vendor PDP APN `www.vodafone.net.ua` aborted (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 44: `.so` APN **`internet`** GEN **8000** then **0x0D10 st=0x03** / no **0x0D09**. This pass: same vendor **0x0D1B / 0x0D01 / 0x0D04** layouts, APN **`www.vodafone.net.ua`** (SIM2 PLMN **25501** operator APN; not a binary guess). Profile name still **`lte_internet`**. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL **0x04/0x07**. **No** 0x0808 SET **0x03**. **No** 0x0D14. **No** 0x0D03. **No** STK ACK. **No** PIN. **No** MO. **No** SMS. **No** `/sbin/beep`. RNDIS host **192.168.42.7** (Ethernet 6). Binary built: wget **8856** `/tmp/ps-p45` **1147256** (`os/build/e4-ps-p45.c`, Zig musl). **Not pushed. Not run.**

### Abort

Telnet `192.168.42.1:23` before wget: `modem_state` = **`CRASH_EXIT`** (re-read twice). Uptime **43163 s**. Holder **428** `radio-boot` state **D**, fds **ipc0 + rfs0** still open, `/tmp/sipc-holder.pid` **428**. PID **416** still `sh`. GNSS **OFFLINE**. `rmnet0` rx=tx=**0**. **Did not** STOP 428. **Did not** open ipc1. **Did not** SET MODE_SEL / 0x0808 / 0x0D1B / 0x0D01 / 0x0D04. **Did not** wget. **Did not** `POWER_OFF` / reload.

| | result |
|--|--|
| fail / LTE | **not sampled** (no ipc1) |
| PDP GEN 0x0D1B/0x0D01/0x0D04 | **not sent** |
| 0x0D10 / 0x0D09 | **not sent** |
| attached | **not sampled** |
| rmnet | rx=tx=**0** |
| CP | **`CRASH_EXIT`** |
| holder 428 | alive **D**, ipc0+rfs0, CONT (not T) |

CS/PS/0x0808/MODE_SEL unknown after the crash. Last live sample remains Probe 44: fail=**0**, LTE fail=**0**, PDP **8000**, **0x0D10** cid=1 st=**0x03**, no **0x0D09**, **attached=0**. Crash happened **after** Probe 44 returned ONLINE and **before** this pass opened ipc1. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032. **Do not write `do_cp_crash`.**

## Probe 46 — CRASH_EXIT recover blocked (v031, 2026-09-02)

Goal: restore **`ONLINE`** (this pass). Overall `rmnet*` rx/tx ≠ 0 still **not** the pass goal. Probe 45 aborted at **`CRASH_EXIT`** before wget. **No** original efs. **No** `POWER_OFF`. **No** MODE_SEL SET. **No** PDP SET. **No** `PsAttach`. **No** MO. **No** SMS. **No** `/sbin/beep`. **No** GNSS. Did **not** STOP 428. Did **not** STOP 416. RNDIS host **192.168.42.7** (Ethernet 6, `ipconfig` confirmed). Telnet `192.168.42.1:23`. HTTP **8857** unused (no binary).

### Live (re-read)

| | |
|--|--|
| uptime | **43324** s then **43455** s (same v031, not a fresh boot) |
| `modem_state` | **`CRASH_EXIT`** (twice + `radio-boot status`) |
| `GET_CP_STATUS` | **2** (`CPIF-200511N220408`) |
| GNSS | **`OFFLINE`** |
| holder **428** | `radio-boot` **D**, `/tmp/sipc-holder.pid` **428**. fds **5=`umts_ipc0`** **6=`umts_rfs0`** **7=`/tmp/rfs.log`** + copy `err/csdiag_callfail_count.dat`. wchan **`msleep`**. stack **`ipc_poll`** (`s318ap.state == CRASH_EXIT`) |
| PID **416** | `sh` **S**, fd **3=`umts_ipc1`**. Left running |
| `rmnet0`/`rmnet1` | rx=tx=**0** |
| userdata | ext2 `/mnt/userdata`. NV copy 1 MiB. `/tmp/radio-boot` **1314320** (same as userdata) |
| efs | **not** original p1/p2/p4. Bind is userdata copy → `/mnt/vendor/efs` |

### Crash reason

dmesg ring **wrapped**. `/tmp/dm46.txt` **1828627** B. Earliest remaining line **t=43237.82** — already holder **428** `ipc_poll: umts_ipc0/rfs0: s318ap.state == CRASH_EXIT` + WDT keepalive. **8296** `CRASH_EXIT` lines (poll flood). **0** `CP_CRASH`. **0** `INIT_END`. **0** `PHONE_START`. **0** `nv_rebuild`. **0** `forced`. Probe 45 already saw `CRASH_EXIT` at **43163** s — that window is **gone**. **Reason not in dmesg.** sysfs `cpif/` has `modem_state` + **`do_cp_crash`** (WO; **not written**).

### Recovery — not run

Proven **ONLINE** on this unit is only **fresh AP `OFFLINE`** (`sysrq-b` / Power 2s, `First init`) then `/mnt/userdata/cp-boot.sh` → `radio-boot loadnv` BOOT+MAIN+**VSS**+NV userdata copy, UDL, hold ipc0+rfs0, COMPLETE. That **kills holder 428**. **Not done.**

`radio-boot loadnv` **from leftover `CRASH_EXIT`** was already tried (VSS load LIVE, 2026-09-01): `POWER_ON` forces software `OFFLINE`, `POWER_RESET` does **not** `cal_cp_init()`, `START` **EPERM** `cp_status error:0`, UDL **timeout**, `COMPLETE` **EAGAIN**, stayed **`BOOTING`**. **Not a proven ONLINE path.** **Not re-run** (would also `POWER_ON` and, on fail, DROP a new ipc0/rfs0 pair).

`IOCTL_POWER_OFF` is the other PMU path and is **forbidden**. **Not used.**

| attempted | |
|--|--|
| live GET sysfs / `radio-boot status` | yes (read-only) |
| `loadnv` / VSS reload / `POWER_RESET` | **no** (not proven from `CRASH_EXIT`) |
| `IOCTL_POWER_OFF` | **no** (**avoided**) |
| sysrq-b / Power 2s | **no** (would kill 428) |
| MODE_SEL / PDP / PsAttach / MO / SMS | **no** |
| write `do_cp_crash` | **no** |
| ipc1 GET | **no** (not ONLINE) |
| wget / HTTP 8857 | **no** |

### rmnet

Still **0**. CP **`CRASH_EXIT`**. Holder **428** left **D**, ipc0+rfs0 open. **POWER_OFF avoided: yes.** **Data-plane goal not complete.** Blocker: no userspace CP PMU re-init after first boot except **`IOCTL_POWER_OFF`** (forbidden) or **AP reboot** (kills 428). **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032. **Do not write `do_cp_crash`.**

## Probe 47 — leftover CRASH_EXIT, still waiting on AP reboot (v031, 2026-09-02)

Goal: restore **`ONLINE`** then GET-only ipc1 (PHONE_STATE / MODE_SEL / 0x0808 / NET_REGIST CS/PS / GPRS_PS / rmnet). **No SET** this pass. Probe 46 left **`CRASH_EXIT`** and asked for a human AP reboot. **No** original efs. **No** `POWER_OFF`. **No** `loadnv`. **No** MODE_SEL SET. **No** PDP SET. **No** `do_cp_crash`. Did **not** STOP 428. Did **not** STOP 416. RNDIS host **192.168.42.7** (Ethernet 6, `ipconfig` confirmed). Telnet `192.168.42.1:23`. HTTP **8858** unused (no binary).

### Live

| | |
|--|--|
| uptime | **43576** s (same v031 boot as probe 46 at 43324–43455; **not** a fresh AP reboot) |
| `modem_state` | **`CRASH_EXIT`** |
| `GET_CP_STATUS` | **2** (`CPIF-200511N220408`) |
| GNSS | **`OFFLINE`** (`gnss_status`) |
| holder **428** | `radio-boot`, `/tmp/sipc-holder.pid` **428**. fds **5=`umts_ipc0`** **6=`umts_rfs0`** **7=`/tmp/rfs.log`**. wchan **`msleep`**. Left running |
| PID **416** | `sh` fd **3=`umts_ipc1`**. Left running |
| `rmnet0` | **down**, rx=tx=**0**. `rmnet1` rx=tx=**0** |
| userdata | ext2 `/mnt/userdata`. `cp-boot.sh` + `radio-boot` + NV copy present. Bind copy → `/mnt/vendor/efs` (same p38; **not** original p1/p2/p4) |

### Leftover-crash path (read-only, not tried)

`/mnt/userdata/cp-boot.sh` is the proven ONLINE path: after a **fresh** AP `OFFLINE` (`First init`), `radio-boot loadnv` BOOT+MAIN+**VSS**+NV userdata copy, UDL, hold ipc0+rfs0, COMPLETE, fork never-close holder + rfs loop, also hold ipc1. **Not run** on this leftover `CRASH_EXIT` (probe 46 already recorded: `POWER_ON` → software `OFFLINE`, no `cal_cp_init`, START EPERM, UDL timeout).

cpif sysfs re-listed: `modem_state` (RO), **`do_cp_crash`** (WO; **not written**), `sim/ds_detect`, `shmem/` (`force_use_memcpy`, `tx_period_ms`, `rb_info`, …), `napi/`, `power/` (Linux runtime PM). **No** unused sysfs that calls `cal_cp_init` / PMU re-init. `power/control` is **not** CP `IOCTL_POWER_OFF` and is **not** a leftover-crash recovery. **Did not write any of them.**

### Recovery — not run

| attempted | |
|--|--|
| live GET sysfs / `radio-boot status` | yes (read-only) |
| `loadnv` / VSS reload / `POWER_RESET` | **no** |
| `IOCTL_POWER_OFF` | **no** (**avoided**) |
| sysrq-b / Power 2s | **no** (human still has not rebooted) |
| write `do_cp_crash` / `power/control` / shmem | **no** |
| MODE_SEL / PDP / ipc1 GET | **no** (not ONLINE) |
| wget / HTTP 8858 | **no** |

### rmnet

Still **0**. CP **`CRASH_EXIT`**. Holder **428** left open. **POWER_OFF avoided: yes.** **Data-plane goal not complete.** Still waiting on **human AP reboot**. After Power 2s / sysrq-b wait for RNDIS + telnet, then exactly:

```
sh /mnt/userdata/cp-boot.sh
```

If userdata is not mounted yet (seen on prior fresh boots), wait a few seconds and run the same line again. Confirm `modem_state=OFFLINE` before that line. **Not** `POWER_OFF`. **Not** `loadnv` on leftover `CRASH_EXIT`. **Do not SET MODE_SEL 0x04/0x07.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032. **Do not write `do_cp_crash`.**

## Probe 48 — GET-only helper staged, not run — waiting AP reboot (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. **No SET** this pass (no MODE_SEL **0x0a/0x2f**, no PDP, no PsAttach, no MO, no SMS). Probe 47 left **`CRASH_EXIT`** waiting on a human AP reboot. **No** original efs. **No** `POWER_OFF`. **No** `loadnv`. Did **not** STOP 428. Did **not** open ipc1. RNDIS host **192.168.42.7** (Ethernet 6). Telnet `192.168.42.1:23`.

### Live (one telnet)

| | |
|--|--|
| uptime | **43739.60** s (same v031 boot as probe 47 at 43576; **not** a fresh AP reboot) |
| `modem_state` | **`CRASH_EXIT`** |
| ipc1 GET | **not run** (not ONLINE) |
| wget | **not run** |

### Host staging (not pushed)

Static GET-only helper **`os/build/e4-ps-p48.c` → `os/build/ps-p48`** Zig musl **1100032**. Refuses unless `modem_state=ONLINE`. STOP/CONT holder 428 only. Dumps PHONE_STATE, MODE_SEL, 0x0808, 0x0816, NET_REGIST CS+PS hex, GPRS_PS, all `rmnet*` rx/tx. **Zero SETs** (`send_small` refuses non-GET). Leftover drain **5 s**. Serving **`http://192.168.42.7:8858/ps-p48`**. **Staged, not run — waiting AP reboot.**

### Recovery — not run

| attempted | |
|--|--|
| live GET `modem_state` + `/proc/uptime` | yes (one telnet) |
| `loadnv` / VSS reload / `POWER_RESET` | **no** |
| `IOCTL_POWER_OFF` | **no** (**avoided**) |
| sysrq-b / Power 2s | **no** (human still has not rebooted) |
| write `do_cp_crash` | **no** |
| MODE_SEL / 0x0808 / PDP / PsAttach / ipc1 GET | **no** (not ONLINE) |
| wget / run `ps-p48` | **no** |

### rmnet

Still **0**. CP **`CRASH_EXIT`**. Holder **428** left open. **POWER_OFF avoided: yes.** **Data-plane goal not complete.** After Power 2s / sysrq-b wait for RNDIS + telnet, then:

```
sh /mnt/userdata/cp-boot.sh
```

Then wget the staged helper (host already listening):

```
busybox wget -O /tmp/ps-p48 http://192.168.42.7:8858/ps-p48
```

Confirm `modem_state=ONLINE` before running `/tmp/ps-p48`. **Not** `POWER_OFF`. **Not** `loadnv` on leftover `CRASH_EXIT`. **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032. **Do not write `do_cp_crash`.**

## Next

- CP **`CRASH_EXIT`**. Holder **428** alive. **rmnet 0**. Goal **not complete**.
- Still waiting on **human AP reboot** (Power 2s / sysrq-b), then `sh /mnt/userdata/cp-boot.sh`. **Not** `POWER_OFF`. **Not** `loadnv` on this leftover `CRASH_EXIT`.
- Probe 48 GET-only `ps-p48` is **staged** on **8858** (`/tmp/ps-p48` after wget). Run only after CP is **ONLINE**.
- Probe 45 binary (`ps-p45` APN `www.vodafone.net.ua`) still waits for a later **ONLINE** window that allows SET. This pass: **no SET**.
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
