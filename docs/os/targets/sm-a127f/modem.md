# SM-A127F — modem / CP (Phase E4)

Exynos 850 **S5000AP** Shannon (**SS310**) over CPIF SHMEM. Stock userspace is Android `cbd` + `rild`. We have BusyBox ramdisk only. **EFS writes are forever off-limits.** One-shot **ro,noload** list 2026-08-31 (below); umounted. RADIO is read-only (stock CP image). Do not `dd` / format / AT-write NV.

Detail: live v031 dump 2026-08-29 + `debugfs` of `os/build/stock-super/vendor.img` on R620. Kernel tree: `os/third_party/kernel_samsung_a12/drivers/soc/samsung/cpif`.

## LIVE map (v031, stock DXJ2 4.19.111-27127798)

| | |
|--|--|
| CP state | **`ONLINE`** (2026-09-02, probe 77, **same AP boot** as probe 64–76). Holder **326** (`radio-boot` ipc0+rfs0) **STOPPED** (`state=T`, fds still open — **not killed**). ipc1-holder **314** running. Vendor **`rild` 1317** alive (`umts_ipc0`/`ipc1`/`rfs0`). MODE_SEL left **`0x0a`**. **0x0808 `02`**. att=**0**. **ipc1 multi-open OK** errno=**0**. rmnet **rx=0** tx=AP IPv6 LLA leftover (p76 IFF_UP — not a PDN). **No IPv4**. Watch **50 s ONLINE**. **`IOCTL_POWER_OFF` not used.** **No leftover loadnv.** **Did not kill 326/314.** **Data-plane goal not complete.** |
| GNSS | **`OFFLINE`** (no BCMD this pass). `READ_FIRMWARE` vs `/tmp/kepler-fw.bin` **byte-identical** (64B K102 + full SHA). See GNSS boot LIVE |
| SIM detect | `ds_detect=2` (`cpif/sim/ds_detect` and `modem_ctrl_s5000ap` param) |
| Driver | `cpif_probe` **CPIF-200511N220408** `eur_open`; **s5000ap** modemctl; **s318ap** shmem link |
| DT | `samsung,exynos-cp`. LIVE `cpif/mif,protocol` = **0** (`PROTOCOL_SIPC`). gnssif has no protocol/sit cell |
| Firmware | BOOT+MAIN+**VSS** + real NV (userdata copy) via `/mnt/userdata/radio-boot loadnv`. Helper **1314320**. Bind-mount copy → `/mnt/vendor/efs`. Vendor `rild` **1317** up (probe 77; also probe 23). `cbd` not executed |

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

## Probe 49 — re-loadnv on already-ONLINE CP → WDT_RESET (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. **No SET** this pass (no MODE_SEL **0x0a/0x2f**, no PDP, no PsAttach, no MO, no SMS). Probe 48 left leftover **`CRASH_EXIT`** on the previous AP boot. Human AP reboot happened. **No** original efs. **No** `POWER_OFF`. Did **not** STOP holder **339**. Did **not** open ipc1 for GET. RNDIS host **192.168.42.3** (Ethernet 6, `ipconfig`; **not** 192.168.42.7). Telnet `192.168.42.1:23`. HTTP **8858** still Listen on **192.168.42.7** (stale bind; host no longer has that addr).

### Live (first telnet, before userdata)

| | |
|--|--|
| uptime | **381.44** s (fresh v031; **not** leftover 43k s) |
| `modem_state` | **`OFFLINE`** |
| GNSS | **`OFFLINE`** (`gnssif/gnss_status`) |
| `/tmp/sipc-holder.pid` | **absent** |
| userdata | **not** mounted (`/mnt/userdata` missing) |

### Userdata mount

Re-read sysfs before `mknod`: `mmcblk0p38` `PARTNAME=userdata` **259:30**; `mmcblk0p22` `PARTNAME=radio` **259:14**. Nodes `/dev/mmcblk0p38` / `/dev/mmcblk0p22` already present. Created `/dev/block/` copies. `mount -t ext2 /dev/mmcblk0p38 /mnt/userdata`. **Success.** `/mnt/userdata/cp-boot.sh` **4226**, `radio-boot` **1314320**, `saaios-efs-copy/nv_data.bin` 1 MiB. Original p1/p2/p4 **not** mounted. Uptime **439.45**, still **`OFFLINE`**.

### First ONLINE this boot (not this pass’s loadnv)

dmesg: **`radio-boot` 338** at **490.48** `IOCTL_POWER_ON` **`First init`**, START, `complete_normal_boot`, `INIT_END` **491.52**. Holder **339** (`/tmp/radio-boot loadnv` userdata NV copy; fds **5=`umts_ipc0`** **6=`umts_rfs0`** **7=`/tmp/rfs.log`**). `/tmp/rfs.log` shows IPC drain while **`tick modem=ONLINE`**. `/tmp/radio-boot` was **Text file busy** when this pass later `cp`’d. Something else ran `loadnv` after userdata was mounted and **before** this pass’s `cp-boot.sh`.

### This pass’s `cp-boot.sh` — should not have re-loadnv

`sh /mnt/userdata/cp-boot.sh` at uptime **523.47**. Script start already saw **`modem_state=ONLINE`**. Bind copy → `/mnt/vendor/efs` (p38 only). ipc1-holder **368**. Then **`$RB` loadnv** anyway (radio-boot **379**):

| | |
|--|--|
| `IOCTL_POWER_ON` / `POWER_RESET` | rc=0; dmesg `power_reset_cp: already offline` |
| LOAD BOOT+MAIN+**VSS**+NV (userdata copy) | OK |
| `IOCTL_START_CP_BOOTLOADER` | **EPERM** (13) |
| UDL A00D/AF00 | **FAIL** (timeout / EOF) |
| `COMPLETE_NORMAL_BOOTUP` | **EAGAIN** / `T-I-M-E-O-U-T` |
| after | **`WDT_RESET`**, `GET_CP_STATUS` **9** |
| DROP | 379 closed its ipc0/rfs0 pair (339 kept its fds) |

**Did not** run `loadnv` a third time. **Did not** `POWER_OFF`.

### After (re-read)

| | |
|--|--|
| uptime | **585.21** s |
| `modem_state` | **`WDT_RESET`** |
| GNSS | **`OFFLINE`** |
| holder **339** | `radio-boot` **D**, `/tmp/sipc-holder.pid` **339**. fds ipc0+rfs0. Left running |
| ipc1-holder **368** | holds `umts_ipc1`. Left running |
| ipc1 GET / `ps-p48` | **not run** (not ONLINE) |
| wget / HTTP 8858 | **not run** |
| MODE_SEL / 0x0808 / CS / PS / fail / attached | **not sampled** (no ipc1) |
| `rmnet0`–`rmnet7` | rx=tx=**0** (all eight) |
| IPv4 on rmnet | **no** |
| efs | bind is userdata **p38** copy only; **not** original p1/p2/p4 |

### Recovery — not run after WDT

Same leftover-crash rule as probes 46–48. `loadnv` from **`WDT_RESET`** is not a proven ONLINE path (`POWER_ON` + `POWER_RESET already offline` + START EPERM already happened this pass). **`IOCTL_POWER_OFF` forbidden.**

| attempted | |
|--|--|
| userdata mount + first `cp-boot.sh` | yes (re-loadnv on already-ONLINE — **caused WDT**) |
| second `loadnv` / VSS reload | **no** |
| `IOCTL_POWER_OFF` | **no** (**avoided**) |
| sysrq-b / Power 2s | **no** (need human AP reboot again) |
| write `do_cp_crash` | **no** |
| MODE_SEL / PDP / PsAttach / ipc1 GET | **no** (not ONLINE) |
| wget / run `ps-p48` | **no** |

### rmnet

Still **0**. CP **`WDT_RESET`**. Holder **339** left open. **POWER_OFF avoided: yes.** **Data-plane goal not complete.** After Power 2s / sysrq-b wait for RNDIS + telnet. Host RNDIS this boot is **192.168.42.3**. Confirm `modem_state=OFFLINE`, then **once**:

```
sh /mnt/userdata/cp-boot.sh
```

If the script start line already prints `modem_state=ONLINE`, **stop** — do **not** run `loadnv` again. Confirm `modem_state=ONLINE` before wget/run `ps-p48`. **Not** `POWER_OFF`. **Not** `loadnv` on leftover `WDT_RESET`. **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032. **Do not write `do_cp_crash`.**

## Probe 50 — GET-only `ps-p48` on fresh ONLINE boot (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. **No SET** this pass (no MODE_SEL **0x0a/0x2f**, no PDP, no PsAttach, no MO, no SMS). Probe 49 left **`WDT_RESET`**; human AP reboot happened. CP came **ONLINE** on this boot without this pass running `loadnv`. **No** original efs. **No** `POWER_OFF`. **No** second `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running (did **not** STOP/kill). Killed leftover **sleep** PID **328** (held ipc1; not 326). RNDIS host **192.168.42.16** (Ethernet 6, `ipconfig`). Telnet `192.168.42.1:23`. wget **8859** `/tmp/ps-p48` **1100032**.

### Live (first telnet)

| | |
|--|--|
| uptime | **643.98** s (fresh v031 boot after probe 49 WDT) |
| `modem_state` | **`ONLINE`** |
| `/tmp/sipc-holder.pid` | **338** (`radio-boot loadnv`, fds **5=`umts_ipc0`** **6=`umts_rfs0`** **7=`/tmp/rfs.log`**) |
| ipc1-holder | **326** (`sleep`, fd **3=`umts_ipc1`**) |

### GET-only ipc1 (`/tmp/ps-p48`)

Static `/tmp/ps-p48` (`os/build/e4-ps-p48.c`, Zig musl **1100032**). mseq from **0x90**. Leftover drain **5 s** saw LTE NOTI (act=**0x21**) then UMTS CS **HOME** + PS **NONE** with GMM **#7** appearing on NOTI before GETs.

| TX | aseq | result |
|--|--|--|
| GET `PHONE_STATE` **0x90** | **0x90** | **0x02** |
| GET `MODE_SEL` **0x91** | **0x91** | **0x0b** |
| GET `0x0808` **0x92** | **0x92** | **`01`** CS-only — **no SET** |
| GET `0x0816` **0x93** | **0x93** | **`00 00`** (slot=0 cause=0) |
| GET CS / PS / GPRS_PS | **0x94** / **0x95** / **0x96** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |

`sets=0`. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=S`, ipc0+rfs0). ipc1-holder **326** untouched.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present** (`fail_missing=0`).

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

0x0808 GET **`01`** (CS-only) — CP reverted from prior probes’ **`02`** CS_PS without this pass SETting. GMM **#7** sticky on PS; CS still **HOME**.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 51 — SET CS_PS + SIM2 DDS, no PsAttach (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 50 left **0x0808 `01`** CS-only and **0x0816 `00 00`**. This pass SETs **0x0808 `02`** (CS_PS, Probe 28 layout FMT len **8**) then **0x0816 `01 01`** (SIM2 DDS, Probe 30 layout FMT len **9**). **No** MODE_SEL SET. **No** PsAttach **0x0D03**. **No** PDP. **No** MO/SMS. **No** `POWER_OFF`. **No** second `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running (did **not** STOP/kill). Killed leftover **sleep** PID **394** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8860** `/tmp/ps-p51` **1111056**.

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p51` (`os/build/e4-ps-p51.c`, Zig musl **1111056**). mseq from **0xA0**. Uptime **822.48** s.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** only (no STK) |
| GET `PHONE_STATE` **0xA0** | **0xA0** | **0x02** |
| GET `MODE_SEL` **0xA1** | **0xA1** | **0x0b** — **no SET** |
| GET `0x0808` **0xA2** | **0xA2** | body **`01`** (enum **0** CS) |
| GET `0x0816` **0xA3** | **0xA3** | body **`00 00`** (slot=0 cause=0) |
| GET CS / PS / GPRS_PS | **0xA4** / **0xA5** / **0xA6** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `0x0808` **`02`** **0xA7** | **0xA7** | **GEN_PHONE_RES 0x0808 0x8000 SUCCESS** |
| SET `0x0816` **`01 01`** **0xA8** | **0xA8** | **GEN_PHONE_RES 0x0816 0x8000 SUCCESS** |
| GET `0x0808` **0xA9** | **0xA9** | body **`01`** — **CP reverted** despite SET SUCCESS |
| GET `0x0816` **0xAA** | **0xAA** | body **`01 00`** (slot=**1** cause=0) |
| GET CS / PS / GPRS_PS | **0xAB** / **0xAC** / **0xAD** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **restore to 0x01 did not run**. Both SETs ACK'd **0x8000**; GET confirms DDS **0→1**, but **0x0808** reads back **`01`** immediately (same revert pattern as Probe 31 after PLMN). **No PsAttach** this pass.

### Raw NET_REGIST (after SET)

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=R`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 52 — GET 0x0808 before DDS (v031, 2026-09-02)

Goal: disambiguate Probe 51 revert. Probe 51 SET **0x0808 `02`** then **0x0816 `01 01`** then GET **0x0808 `01`** — hypothesis: DDS reverts domain, or GET ran after revert. Probe 28 same-fd GET after **0x0808 SET** was **`02`**. This pass: one ipc1 fd, GET baseline **0x0808**, SET **0x0808 `02`**, drain, **GET 0x0808 before any 0x0816**; only if GET is **`02`**: SET **0x0816 `01 01`**, GET **0x0808** again, GET **0x0816**, NET_REGIST CS+PS, GPRS_PS, rmnet. **No** MODE_SEL SET. **No** PsAttach **0x0D03**. **No** PDP. **No** MO/SMS. **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running (did **not** STOP/kill). Killed leftover **sleep** PID **418** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8861** `/tmp/ps-p52` **1112760**.

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p52` (`os/build/e4-ps-p52.c`, Zig musl **1112760**). mseq from **0xB0**.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** only (no STK) |
| GET `PHONE_STATE` **0xB0** | **0xB0** | **0x02** |
| GET `MODE_SEL` **0xB1** | **0xB1** | **0x0b** — **no SET** |
| GET `0x0808` **0xB2** | **0xB2** | body **`01`** (enum **0** CS) |
| GET `0x0816` **0xB3** | **0xB3** | body **`01 00`** (slot=**1** cause=0 — carried from probe 51) |
| GET CS / PS / GPRS_PS | **0xB4** / **0xB5** / **0xB6** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `0x0808` **`02`** **0xB7** | **0xB7** | **GEN_PHONE_RES 0x0808 0x8000 SUCCESS** |
| GET `0x0808` **0xB8** (before 0x0816) | **0xB8** | body **`02`** (enum **2** CS_PS) — **sticks** (Probe 28 pattern) |
| SET `0x0816` **`01 01`** **0xB9** | **0xB9** | **GEN_PHONE_RES 0x0816 0x8000 SUCCESS** |
| GET `0x0808` **0xBA** | **0xBA** | body **`02`** — **DDS did not revert** |
| GET `0x0816` **0xBB** | **0xBB** | body **`01 00`** (slot=**1** cause=0) |
| GET CS / PS / GPRS_PS | **0xBC** / **0xBD** / **0xBE** | CS **HOME**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **restore to 0x01 did not run**. Probe 51 **`01`** readback was **ordering** (GET ran after both SETs on a boot where baseline was already CS-only); same-fd GET **immediately after 0x0808 SET** returns **`02`**, and DDS SET does **not** revert it. GMM **#7** still sticky on PS. **No PsAttach** this pass.

### Raw NET_REGIST (after SET)

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=R`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 53 — GET-only wait then vendor PDP under CS_PS=02 (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 52 left **0x0808 `02`** (CS_PS sticks) and **0x0816 slot=1**; PS still **NONE GMM#7**. This pass: confirm CS_PS + SIM2 DDS, **GET-only wait ~105 s** for PS HOME / fail=0 (no `0x0D03`, no MODE_SEL SET, no `0x0a`/`0x2f`), then vendor **0x0D1B / 0x0D01 / 0x0D04** APN **`internet`**. If that fails under confirmed **0x0808=02**: optional **`www.vodafone.net.ua`**. If wait stays GMM#7: **no PsAttach**; secondary PDP documented as under GMM#7 / no attach. **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running (did **not** STOP/kill). Killed leftover **sleep** PID **429** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8862** `/tmp/ps-p53` **1131160**.

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p53` (`os/build/e4-ps-p53.c`, Zig musl **1131160**). mseq from **0xC0**. CP already **`ONLINE`** (same boot as probes 50–52). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh`. GNSS left **OFFLINE**.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** only (no STK) |
| GET `PHONE_STATE` **0xC0** | **0xC0** | **0x02** |
| GET `MODE_SEL` **0xC1** | **0xC1** | **0x0b** — **no SET** |
| GET `0x0808` **0xC2** | **0xC2** | body **`02`** (enum **2** CS_PS) — skip SET |
| GET `0x0816` **0xC3** | **0xC3** | body **`01 00`** (slot=**1** cause=0) — skip SET |
| GET CS / PS / GPRS_PS | **0xC4** / **0xC5** / **0xC6** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| GET-only wait **105 s** (35 polls, CS+PS) | — | PS stayed **NONE GMM#7**; **saw_home=0** **saw_fail0=0**. **No 0x0D03** |
| GET `0x0808` confirm | **0x30** | still **`02`** |
| SET `0x0D1B` 0xCB `lte_internet`+`internet` | **0x31** | **GEN 0x0D1B 0x8000 SUCCESS** (PDP under GMM#7 / no attach) |
| SET `0x0D01` 0x95 CID=1 APN `internet` | **0x32** | **GEN 0x0D01 0x8000 SUCCESS** |
| SET `0x0D04` 0xF8 APN-copy | **0x33** | **GEN 0x0D04 0x8000 SUCCESS**. NOTI **0x0D10** cid=**1** st=**0x03**. **No 0x0D09** |
| SET `0x0D1B`/`0x0D01`/`0x0D04` APN `www.vodafone.net.ua` | **0x38** / **0x39** / **0x3A** | all **0x8000**. NOTI **0x0D10** cid=**1** st=**0x03**. **No 0x0D09** |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **`02`**. DDS left slot **1**. **No PsAttach.** GMM **#7** sticky for the full GET-only wait under confirmed CS_PS.

### Raw NET_REGIST (after wait + PDP)

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / mid / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=R`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 54 — unused `IpcTxSetUsageSetting` 0x0844 DATA_CENTRIC (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 53 left **0x0808 `02`**, **0x0816 slot=1**, PS **NONE GMM#7**, vendor PDP both APNs **0x0D10 st=0x03**. This pass: unused **`IpcTxSetUsageSetting` / `NET_USAGE_SETTING`**. Host `e4-p54-decode.c` on pulled `libsec-ril.so` **4541576**: **`IpcTxGetUsageSetting`** @ **0x38ca30** packed MOVZ **`0x4408`** → group **0x08** index **0x44** empty GET len **7** type **2**; **`IpcTxSetUsageSetting(RIL_UsageSetting)`** @ **0x38c97c** FMT len **8** type **SET 3**, 1-byte payload (AOSP **VOICE_CENTRIC=0** **DATA_CENTRIC=1**). String **`NET_USAGE_SETTING`**. Voice-centric UE can refuse PS. **No** MODE_SEL SET. **No** PsAttach **0x0D03**. **No** PDP. **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running (did **not** STOP/kill). Killed leftover **sleep** PID **448** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8863** `/tmp/ps-p54` **1108064**.

Not chosen this pass (decoded, unused, higher risk or no TX): **`GPRS_MS_CLASS`** string only (no MOVZ **`0x070d`** / no IpcTx); **`IpcTxNetGetDataRegState`** is still **0x0805** domain=PS; **`IpcTxSetFdInfo` 0x0D17** len 12; **`IpcTxSetDormancy` 0x0D0E** empty SET; **`IpcTxSetProcTypeInfo`** (index not fully packed). **Do not SET MODE_SEL 0x0a/0x2f.**

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p54` (`os/build/e4-ps-p54.c`, Zig musl **1108064**). mseq from **0xD0**. CP already **`ONLINE`** (same boot as probes 50–53). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** only (no STK) |
| GET `PHONE_STATE` **0xD0** | **0xD0** | **0x02** |
| GET `MODE_SEL` **0xD1** | **0xD1** | **0x0b** — **no SET** |
| GET `0x0808` **0xD2** | **0xD2** | body **`02`** (enum **2** CS_PS) — skip SET |
| GET `0x0816` **0xD3** | **0xD3** | body **`01 00`** (slot=**1** cause=0) — skip SET |
| GET CS / PS / GPRS_PS | **0xD4** / **0xD5** / **0xD6** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| GET `0x0844` **0xD7** | **0xD7** | **GEN_PHONE_RES 0x0844 0x8001** (unimplemented, no body) |
| SET `0x0844` **`01`** DATA_CENTRIC **0xD8** | **0xD8** | **GEN_PHONE_RES 0x0844 0x8001** (unimplemented) |
| GET `0x0844` **0xD9** | **0xD9** | **GEN 0x0844 0x8001** |
| GET `0x0808` / `0x0816` / CS/PS / GPRS_PS | **0xDA**… | still **`02`** / slot **1**; CS **HOME**; PS **NONE GMM#7**; **attached=0** |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **`02`**. DDS left slot **1**. **No PsAttach.** **0x0844** is **unimplemented on this CP** (GET and SET both **8001**), same class as **0x0D14** / GET **0x0D26**.

### Raw NET_REGIST (after SET)

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=S`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 55 — unused `IpcTxSetFdInfo` 0x0D17 / `IpcTxSetDormancy` 0x0D0E + GET `GPRS_MS_CLASS` 0x0D07 (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 54 left **0x0808 `02`**, **0x0816 slot=1**, PS **NONE GMM#7**. This pass: unused **`IpcTxSetFdInfo` / `GPRS_FD_INFORMATION`** and **`IpcTxSetDormancy` / `GPRS_DATA_DORMANT`**, plus GET-only **`GPRS_MS_CLASS`**. Host `e4-p55-decode.c` / `e4-p55-fd.c` on pulled `libsec-ril.so` **4541576**. **No** MODE_SEL SET. **No** PsAttach **0x0D03**. **No** PDP. **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running (did **not** STOP/kill). Killed leftover **sleep** PID **468** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8864** `/tmp/ps-p55` **1120080**.

### Binary

| Symbol | VA | cmd | FMT | payload |
|--|--|--|--|--|
| `IpcProtocol41Data::IpcTxSetFdInfo(int,int*)` | **0x36ba28** | packed MOVZ **`0x170d`** → **0x0D17** | len **12** type **SET 3** | `[7]`=mode byte (w1); `[8..11]`=4 packed timer bytes from `int*`. No `IpcTxGetFdInfo`. |
| `IpcModemImplData::SetFdInfo` | **0x34ee68** | passes caller `(int,int*)` through | — | `FastDormancyManager::SetTetheringForFD` / `OnFdSettingChanged`: mode **1**=Enable FD, mode **3**=Disable FD (`"Disable FD, tether state"` / `"Disable FD, sharedpref value is false."`). Disable path `stp xzr,xzr` then times=zeros. Vendor data-on SET: **`03 00 00 00 00`**. |
| `IpcProtocol41Data::IpcTxSetDormancy()` | **0x36b9b4** | packed MOVZ **`0x0e0d`** → **0x0D0E** | len **7** type **SET 3** | empty. Caller **`EnterDormancy`** — not a disable-dormancy TX. No `IpcTxGetDormancy`. |
| `GPRS_MS_CLASS` | str **0x1268ef** | **0x0D07** | GET empty | **no IpcTx** / **no MOVZ `0x070d`**. Sibling names **`GPRS_SHOW_PDP_ADDR` 0x0D06**, **`GPRS_3G_QUAL_SRVC_PROFILE` 0x0D08**. |
| `IpcTxGetLceInfo()` | **0x36fc5c** | packed **`0x230d`** → **0x0D23** | len **7** type **GET 2** | empty. Twin SET **`IpcTxSetLceMode(int,int)`** @ **0x36fbe0** len **12** type **SET 3**. |
| `IpcTxEnablePortBlackList()` | **0x36b600** | packed **`0x110d`** → **0x0D11** | len **22** type **EXEC 4** | alloc **0x32b** (811). String **`GPRS_PORT_LIST`**. |

**Do not SET MODE_SEL 0x0a/0x2f.** FD/dormancy were not expected to clear HLR GMM#7.

### Live ipc1 (SIM2 25501)

Static `/tmp/ps-p55` (`os/build/e4-ps-p55.c`, Zig musl **1120080**). mseq from **0xE0**. CP already **`ONLINE`** (same boot as probes 50–54). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** only (no STK) |
| GET `PHONE_STATE` **0xE0** | **0xE0** | **0x02** |
| GET `MODE_SEL` **0xE1** | **0xE1** | **0x0b** — **no SET** |
| GET `0x0808` **0xE2** | **0xE2** | body **`02`** (enum **2** CS_PS) — skip SET |
| GET `0x0816` **0xE3** | **0xE3** | body **`01 00`** (slot=**1** cause=0) — skip SET |
| GET CS / PS / GPRS_PS | **0xE4** / **0xE5** / **0xE6** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| GET `0x0D07` MS_CLASS **0xE7** | **0xE7** | type-2 RESP plen=1 body **`00`** (no GEN — implemented) |
| GET `0x0D06` SHOW_PDP_ADDR **0xE8** | **0xE8** | **GEN 0x0D06 0x8001** |
| GET `0x0D08` 3G_QUAL **0xE9** | **0xE9** | **GEN 0x0D08 0x8001** |
| GET `0x0D17` FD_INFO **0xEA** | **0xEA** | **GEN 0x0D17 0x8001** — **skip SET** |
| GET `0x0D0E` DATA_DORMANT **0xEB** | **0xEB** | **GEN 0x0D0E 0x8001** — **skip SET** (and EnterDormancy is the wrong direction) |

Both **0x0D17** and **0x0D0E** are **unimplemented on this CP** (GET **8001**), same class as **0x0844** / **0x0D14** / GET **0x0D26**. Continued unused-cmd GET-only map (no SET 0x0D03 / 0x0D14 / 0x0D1B / 0x0D01 / 0x0D04 / 0x0D1D / 0x0D22):

| GET | aseq | GEN / body |
|--|--|--|
| `0x0D02` QOS **0xEC** | **0xEC** | **8001** |
| `0x0D05` ENTER_DATA **0xED** | **0xED** | **8001** |
| `0x0D0B` TFT **0xEE** | **0xEE** | **8001** |
| `0x0D0C` HSDPA **0xEF** | **0xEF** | **8001** |
| `0x0D0D` SESSION_COUNTER **0xF0** | **0xF0** | **8001** |
| `0x0D11` PORT_LIST **0xF1** | **0xF1** | type-2 RESP FMT len **0x032b** (811) — all zeros (no GEN) |
| `0x0D23` LCE_INFO **0xF2** | **0xF2** | type-2 RESP plen=6 body **`00 00 00 00 00 02`** (no GEN) |
| `0x0D25` SYNC_PROFILE **0xF3** | **0xF3** | **GEN 0x0D25 0x8003** (empty GET; `IpcTxGetProfileInfo` wants 3 args) |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **`02`**. DDS left slot **1**. **No PsAttach.** **No FD SET.** **No dormancy SET.** PS/rmnet did **not** move.

### Raw NET_REGIST (after GET sweep)

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=R`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 56 — `IpcTxSetLceMode` 0x0D23 start/PUSH (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 55 left **0x0808 `02`**, **0x0816 slot=1**, PS **NONE GMM#7**, LCE GET implemented body **`00 00 00 00 00 02`**. This pass: unused **`IpcTxSetLceMode` / `GPRS_LCE_INFO`**. Host `e4-p56-decode.c` / `e4-p56-lce.c` on pulled `libsec-ril.so` **4541576**. **No** MODE_SEL SET. **No** PsAttach **0x0D03**. **No** PDP. **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running (did **not** STOP/kill). Killed leftover **sleep** PID **486** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8865** `/tmp/ps-p56` **1111784**.

### Binary

| Symbol | VA | cmd | FMT | payload |
|--|--|--|--|--|
| `IpcTxSetLceMode(int,int)` | **0x36fbe0** | packed MOVZ **`0x230d`** → **0x0D23** | len **12** type **SET 3** | `[7]`=w2 as mode byte; `[8..11]`=w1 LE32 interval. |
| `IpcTxGetLceInfo()` | **0x36fc5c** | packed **`0x230d`** → **0x0D23** | len **7** type **GET 2** | empty. |
| `IpcModemImplData::SetLceMode(int,int,Message*)` | **0x35009c** | AOSP **PULL=1** skips TX; **PUSH≠1** calls `IpcTxSetLceMode(interval, **1**)` | — | wire mode **1** = start. |
| `IpcModemImplData::StopLce` | **0x3502a4** | `IpcTxSetLceMode(0, **2**)` | — | wire mode **2** = stop. **Not sent.** |
| `DataCallManager::DoStartLce` | **0x23ea34** | logs `interval(%d), mode(%d)` | — | clamps interval to **min 1000** ms (`0x3e8`). |
| `IpcRxLceInfo` | **0x36fda8** | parses 6 B GET/NOTI | — | `mLastHopCapacityKbps`, `mConfidenceLevel`, `mLceSuspended` (AOSP `RIL_LceDataInfo`). GET **`00 00 00 00 00 02`** = cap 0 / conf 0 / suspended **2**. |
| Vendor data-on SET | — | **0x0D23** | **`01 e8 03 00 00`** | start/PUSH + 1000 ms. |
| `IpcTxSetProcTypeInfo(bool)` | **0x36fcd0** | JSON check **0x0D2B** | FMT path **returns -1** | JSON twin @ **0x3c29cc** sends key **`status`** + 519 B. **No** `IpcTxGetProcTypeInfo`. **SET skipped** (no well-formed FMT). |
| `IpcTxEnablePortBlackList` | **0x36b600** | **0x0D11** EXEC type 4 | — | **not sent** (GET already 811 B zeros; EXEC more likely to block). |

**Do not SET MODE_SEL 0x0a/0x2f.** LCE was not expected to clear HLR GMM#7.

### Live ipc1 (SIM2)

Static `/tmp/ps-p56` (`os/build/e4-ps-p56.c`, Zig musl **1111784**). mseq from **0x10**. CP already **`ONLINE`** (same boot as probes 50–55). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** only (no STK) |
| GET `PHONE_STATE` **0x10** | **0x10** | **0x02** |
| GET `MODE_SEL` **0x11** | **0x11** | **0x0b** — **no SET** |
| GET `0x0808` **0x12** | **0x12** | body **`02`** (enum **2** CS_PS) — skip SET |
| GET `0x0816` **0x13** | **0x13** | body **`01 00`** (slot=**1** cause=0) — skip SET |
| GET CS / PS / GPRS_PS | **0x14** / **0x15** / **0x16** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| GET `0x0D23` LCE **0x17** | **0x17** | type-2 RESP plen=6 body **`00 00 00 00 00 02`** (no GEN) |
| SET `0x0D23` start/PUSH **`01 e8 03 00 00`** **0x18** | **0x18** | **GEN_PHONE_RES 0x0D23 0x8000 SUCCESS** |
| GET `0x0D23` after SET **0x19** | **0x19** | still **`00 00 00 00 00 02`** |

After SET **8000**, CP started type-3 **0x0D23** NOTIs every **~1.2 s** (matches 1000 ms interval) with the same 6 B body. LCE reporting is on; capacity stays 0 / suspended **2** (no PS bearer). **No ProcTypeInfo SET.** **No 0x0D11 EXEC.**

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **`02`**. DDS left slot **1**. **No PsAttach.** PS/rmnet did **not** move. GMM **#7** sticky as expected.

### Raw NET_REGIST (after LCE SET)

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=S`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 57 — `IpcTxGetProfileInfo` 0x0D25 3-arg GET (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 56 left **0x0808 `02`**, **0x0816 slot=1**, PS **NONE GMM#7**, leftover **0x0D23** type-3 NOTIs ~1.2 s. This pass: unused **`IpcTxGetProfileInfo` / `GPRS_SYNC_PROFILE_INFO`**. Host `e4-p57-decode.c` / `e4-p57-args.c` on pulled `libsec-ril.so` **4541576**. **No** MODE_SEL SET. **No** PsAttach **0x0D03**. **No** PDP. **No** LCE SET/stop (ignored **662** leftover 0x0D23 NOTIs). **No** `IpcTxSyncProfileInfo` SET (0x0D25 SET len **0x147** APN — would repeat 0x0D1B). **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running (did **not** STOP/kill). Killed leftover **sleep** PID **524** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8866** `/tmp/ps-p57` **1119776**.

### Binary

| Symbol | VA | cmd | FMT | payload |
|--|--|--|--|--|
| `IpcProtocol41Data::IpcTxGetProfileInfo(int,int,int)` | **0x36a508** sz **140** | packed MOVZ **`0x250d`** → **0x0D25** | len **11** type **GET 2** | `[7]`=w2 (arg1) u8. `cmp w2,#2; b.ne`: if arg1==**2** then `[8]`=w3 (arg2) u8, `[9..10]`=w1 (arg0) LE16. Else last 3 B stay 0 (`stur wzr`). Always TX **11** B. |
| `IpcProtocol41JsonData::IpcTxGetProfileInfo` | **0x3c1f1c** | log stub | — | returns 0; no FMT. |
| `IpcTxSetProfileInfo` | — | **none** | — | no symbol. |
| `IpcTxSyncProfileInfo(...)` | **0x36a594** sz **1068** | packed **`0x250d`** + `0x103` → type **SET 3** | len **0x147** (327) | APN/profile sibling. VZW/USC path. **SET not sent.** |
| `IpcRxGetProfileInfo` | **0x36c984** sz **628** | parses GET/NOTI | — | loops records stride **0x13e** from `+0xdb`. Live single-record RESP was **320** B (= SET payload). |
| `IpcTxSetLinkCapacityReportingCriteria` | — | **no IpcTx** | — | only `ModemProxy` / `DataCallManager::DoSet…`. **Not sent.** |

**3 args:** arg0 = profile index / cid (LE16 when arg1==2); arg1 = query type (must be nonzero — **0** → GEN **8003**); arg2 = extra / proto (only on wire when arg1==2).

Vendor packs sent vs `.so`:

| pack | arg0,arg1,arg2 | TX payload | `.so` writes |
|--|--|--|--|
| idx0 type2 proto0 | 0, **2**, 0 | **`02 00 00 00`** | all 3 fields (arg1==2) |
| idx1 type2 proto0 | 1, **2**, 0 | **`02 00 01 00`** | all 3 fields |
| idx1 type2 proto2 | 1, **2**, 2 | **`02 02 01 00`** | all 3 fields |
| arg1=0 zeros | 0, 0, 0 | **`00 00 00 00`** | only `[7]`=0 |
| arg1=1 first-byte | 0, 1, 0 | **`01 00 00 00`** | only `[7]`=1 |

**Do not SET MODE_SEL 0x0a/0x2f.** Profile GET does not clear HLR GMM#7.

### Live ipc1 (SIM2)

Static `/tmp/ps-p57` (`os/build/e4-ps-p57.c`, Zig musl **1119776**). mseq from **0x10**. CP already **`ONLINE`** (same boot as probes 50–56). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | **0x0D23** type-3 LCE every ~1.2 s (ignored). DISP **0x0706** also seen. No STK ACK. |
| GET `PHONE_STATE` **0x10** | **0x10** | **0x02** |
| GET `MODE_SEL` **0x11** | **0x11** | **0x0b** — **no SET** |
| GET `0x0808` **0x12** | **0x12** | body **`02`** (enum **2** CS_PS) — skip SET |
| GET `0x0816` **0x13** | **0x13** | body **`01 00`** (slot=**1** cause=0) — skip SET |
| GET CS / PS / GPRS_PS | **0x14** / **0x15** / **0x16** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| GET `0x0D25` **`02 00 00 00`** **0x17** | **0x17** | type-2 RESP FMT len **9** plen=**2** body **`00 00`** (empty slot 0; no GEN) |
| GET `0x0D25` **`02 00 01 00`** **0x18** | **0x18** | type-2 RESP FMT len **327** plen=**320** (no GEN) |
| GET `0x0D25` **`02 02 01 00`** **0x19** | **0x19** | same **320** B (no GEN) |
| GET `0x0D25` **`00 00 00 00`** **0x1a** | **0x1a** | **GEN 0x0D25 0x8003** (empty-type still invalid) |
| GET `0x0D25` **`01 00 00 00`** **0x1b** | **0x1b** | same **320** B as index 1 (no GEN) |

### Profile body (320 B, index 1)

Only nonzero bytes: `[0]=01` `[2]=01` `[5..23]="www.vodafone.net.ua"` `[110]=02`. Rest zeros. No IPv4.

| off | bytes | vs prior SET |
|--|--|--|
| 0–1 | **`01 00`** | cid / profile index **1** (0x0D01/0x0D04 CID **1**) |
| 2–3 | **`01 00`** | mapped profile / type **1** |
| 4 | **`00`** | pad / auth 0 |
| 5–105 | APN[101] **`www.vodafone.net.ua`** | last Probe 53 SET (vodafone overwrote **`internet`**). **Not** `lte_internet` profile-name echo (0x0D1B name field absent). |
| 110 | **`02`** | DataProtocol **2** (same default as `IpcTxDefinePdpContext` 0x0D01) |

Index **0** is empty (`00 00`). 0x0D04 GET remains the 3-byte status **`01 18 00`** (Probe 34) — different cmd, not this APN blob. **No 0x0D09.** **No SyncProfile SET.**

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **`02`**. DDS left slot **1**. **No PsAttach.** PS/rmnet did **not** move. GMM **#7** sticky.

### Raw NET_REGIST (after 0x0D25 GET)

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=R`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 58 — GET-only `IpcTxGetBarringInfo` 0x083B + `IpcTxGetNetworkQuality` 0x0839 (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 57 left **0x0808 `02`**, **0x0816 slot=1**, PS **NONE GMM#7**, leftover **0x0D23** type-3 NOTIs ~1.2 s. This pass: unused **`IpcTxGetBarringInfo` / `NET_BARRING_INFO`** and **`IpcTxGetNetworkQuality` / `NET_NETWORK_QUALITY`**, plus sibling **`IpcTxGetNetworkQualityInfo` / `MISC_NETWORK_QUALITY_INFO`**. Host `e4-p58-decode.c` / `e4-p58-args.c` on pulled `libsec-ril.so` **4541576**. **No** MODE_SEL SET. **No** PsAttach **0x0D03**. **No** PDP. **No** LCE SET/stop (ignored **556** leftover 0x0D23 NOTIs). **No** SS `IpcTxSsSetCallBarring` (password). **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running (did **not** STOP/kill). Killed leftover **sleep** PID **548** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8867** `/tmp/ps-p58` **1123968**.

### Binary

| Symbol | VA | cmd | FMT | payload |
|--|--|--|--|--|
| `IpcProtocol41Net::IpcTxGetBarringInfo()` | **0x38c554** sz **176** | packed MOVZ **`0x3b08`** → **0x083B** | len **7** type **GET 2** | empty. String **`NET_BARRING_INFO`**. |
| `IpcProtocol41Net::IpcTxGetNetworkQuality()` | **0x38c604** sz **176** | packed MOVZ **`0x3908`** → **0x0839** | len **7** type **GET 2** | empty. String **`NET_NETWORK_QUALITY`**. |
| `IpcProtocol41Misc::IpcTxGetNetworkQualityInfo(int)` | **0x38379c** sz **180** | packed **`0x580a`** → **0x0A58** | len **8** type **GET 2** | `[7]`=w1 u8. String **`MISC_NETWORK_QUALITY_INFO`**. |
| `IpcTxSetBarringInfo` | — | **none** | — | no symbol. |
| `IpcTxSetNetworkQuality` | — | **none** | — | no symbol. |
| `IpcTxSsSetCallBarring` | **0x39fcd4** | SS | — | needs facility + password. **SET not sent.** |
| `IpcRxBarringInfo` | **0x392ae8** sz **1136** | parses GET/NOTI | — | `[7]`=cellInfoType; count then 17 B records (svc LE32, type LE32, factor, time, isBarred). AOSP cell/SSAC barring, not HLR SS BAOC/BAIC. |
| `IpcRxGetNetworkQualityInfoResp` | **0x383bfc** sz **168** | type-2 | — | ldrh `[8]`, data from `[10]`, 32 B alloc. |

**Do not SET MODE_SEL 0x0a/0x2f.** Barring GET **8001** cannot clear HLR GMM#7.

### Live ipc1 (SIM2)

Static `/tmp/ps-p58` (`os/build/e4-ps-p58.c`, Zig musl **1123968**). mseq from **0x10**. CP already **`ONLINE`** (same boot as probes 50–57). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | **0x0D23** type-3 LCE every ~1.2 s (ignored). DISP **0x0706** also seen. No STK ACK. |
| GET `PHONE_STATE` **0x10** | **0x10** | **0x02** |
| GET `MODE_SEL` **0x11** | **0x11** | **0x0b** — **no SET** |
| GET `0x0808` **0x12** | **0x12** | body **`02`** (enum **2** CS_PS) — skip SET |
| GET `0x0816` **0x13** | **0x13** | body **`01 00`** (slot=**1** cause=0) — skip SET |
| GET CS / PS / GPRS_PS | **0x14** / **0x15** / **0x16** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| GET `0x083B` empty **0x17** | **0x17** | **GEN 0x083B 0x8001** (no body) |
| GET `0x0839` empty **0x18** | **0x18** | **GEN 0x0839 0x8001** (no body) |
| GET `0x0A58` **`00`** **0x19** | **0x19** | **GEN 0x0A58 0x8001** |
| GET `0x0A58` **`01`** **0x1a** | **0x1a** | **GEN 0x0A58 0x8001** |
| GET `0x0A58` **`02`** **0x1b** | **0x1b** | **GEN 0x0A58 0x8001** |

### GEN table

| cmd | code |
|--|--|
| **0x083B** `NET_BARRING_INFO` | **8001** unimplemented |
| **0x0839** `NET_NETWORK_QUALITY` | **8001** unimplemented |
| **0x0A58** `MISC_NETWORK_QUALITY_INFO` arg 0/1/2 | **8001** unimplemented |

No type-2 RESP body. **No SET** (GET **8001**, no `IpcTxSetBarringInfo`, no vendor disable-barring pack). Cannot tell CS vs PS / BAIC vs BAOC / access-class barring from this CP — query is not implemented. GMM **#7** is **not** shown as operator/SS or AC barring here; it remains the HLR/subscription reject from NET_REGIST.

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **`02`**. DDS left slot **1**. **No PsAttach.** PS/rmnet did **not** move. GMM **#7** sticky.

### Raw NET_REGIST (after barring/quality GET)

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=R`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 59 — GET `IpcTxGetPlmnBarringTimer` 0x0A55 + unused `IpcTxGet*` sweep (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 58 left **0x0808 `02`**, **0x0816 slot=1**, PS **NONE GMM#7**, leftover **0x0D23** type-3 NOTIs ~1.2 s. This pass: named **`IpcTxGetPlmnBarringTimer` / MISC 0x0A55**, then (because GET **8001**) every remaining packable unused **`IpcTxGet*`**. Host `e4-p59-decode.c` on pulled `libsec-ril.so` **4541576** (10869 syms). **No** MODE_SEL SET. **No** PsAttach **0x0D03**. **No** PDP. **No** LCE SET/stop (ignored **736** leftover 0x0D23 NOTIs). **No** SS `IpcTxSsGetCallBarring` / Set (password). **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running (did **not** STOP/kill). Killed leftover **sleep** PID **570** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8868** `/tmp/ps-p59` **1120864**.

### Binary

| Symbol | VA | cmd | FMT | payload |
|--|--|--|--|--|
| `IpcProtocol41Misc::IpcTxGetPlmnBarringTimer()` | **0x37f058** sz **164** | packed MOVK **`0x550a`** → **0x0A55** | len **8** type **GET 2** | `[7]`=**`01`** (const). 64-bit pack `0x0102550A00000008`. |
| `IpcProtocol41Misc::IpcTxSetPlmnBarringTimer(uint)` | **0x37efa0** sz **184** | packed **`0x550a`** → **0x0A55** | len **12** type **SET 3** | `[7]`=**`01`**; `[8..11]`=w1 LE32 timer. **SET not sent** (GET **8001**). |
| `IpcRxMiscPlmnBarringTimerResp` | **0x383728** sz **116** | type check | — | no live body. |

**Do not SET MODE_SEL 0x0a/0x2f.** Barring-timer GET **8001** cannot clear HLR GMM#7.

### Live ipc1 (SIM2)

Static `/tmp/ps-p59` (`os/build/e4-ps-p59.c`, Zig musl **1120864**). mseq from **0x10**. CP already **`ONLINE`** (same boot as probes 50–58). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**.

| TX | aseq | result |
|--|--|--|
| leftover NOTI | — | **0x0D23** type-3 LCE every ~1.2 s (ignored). No STK ACK. |
| GET `PHONE_STATE` | — | **0x02** |
| GET `MODE_SEL` | — | **0x0b** — **no SET** |
| GET `0x0808` / `0x0816` | — | **`02`** CS_PS; slot=**1** cause=0 — skip SET |
| GET CS / PS / GPRS_PS | — | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| GET `0x0A55` **`01`** | — | **GEN 0x0A55 0x8001** (no body) — **no SET** |
| unused `IpcTxGet*` sweep | — | 62 GETs (table below) |

### GET 0x0A55

| | |
|--|--|
| GEN | **8001** unimplemented |
| type-2 body | none |
| SET | **not sent** |

### Unused `IpcTxGet*` sweep

Already-probed / forbidden omitted: **0x0808 0x0816 0x080A 0x0805 0x0D03 0x0107 0x0D14 0x0D1B 0x0D01 0x0D04 0x0D1D 0x0D22 0x0D26 0x0844 0x0D17 0x0D0E 0x0D07 0x0D23 0x0D25 0x0D11 0x083B 0x0839 0x0A58 0x0802 0x0A02** (IMSI) **0x0E03/0x0E01** (STK) **0x0A43** (GNSS) **0x1206** (SAP power) **0x0F09 0x1204 0x0601**. `IpcTxSsGetCallBarring` is not `IpcTxGet*` (needs facility + password) — skipped.

Counts: **23** type-2 body, **23** GEN **8001**, **16** GEN **8003**. **No SET** from the sweep (no timer / barred-PLMN / PS-forbid flag we never SET).

| symbol | cmd | sent | GEN | type-2 |
|--|--|--|--|--|
| `IpcTxGetEndcStatus` | 0x0835 | 0 | **8001** | — |
| `IpcTxGetScgBearerAllocationStatus` | 0x0836 | 0 | **8001** | — |
| `IpcTxGetDrxSetting` | 0x0829 | 0 | — | **00** (DRX 0) |
| `IpcTxGetNrDualConnectivityState` | 0x0840 | 0 | **8001** | — |
| `IpcTxGetLteRoamingEnabled` | 0x0821 | 0 | **8003** | — |
| `IpcTxGetDisable2g` | 0x0825 | 0 | **8003** | — |
| `IpcTxGetBandPriority` | 0x0820 | 0 | **8003** | — |
| `IpcTxGetCaEnabled` | 0x0822 | 0 | — | **01** (CA on) |
| `IpcTxGetAutonomousGap` | 0x0823 | 0 | **8001** | — |
| `IpcTxGetNr5gVoiceSupport` | 0x083F | 0 | **8001** | — |
| `IpcTxGetSystemSelectionChannels` | 0x0841 | 0 | **8001** | — |
| `IpcTxGetExtendedBandMode` | 0x0830 | 0 | no RESP 4s | — |
| `IpcTxGetNrModeConfig` | 0x083A | 0 | **8001** | — |
| `IpcTxGetVolteTestmode` | 0x0828 | 0 | **8001** | — |
| `IpcTxGetCdmaHybridMode` | 0x080E | 0 | **8001** | — |
| `IpcTxGetCaConfig` | 0x0807 | `02` | — | `02` + 8×`ff` (no band mask) |
| `IpcTxGetBandEnabled` | 0x0807 | `01` | — | same 9 B as CaConfig |
| `IpcTxGetCpInfoForAtc` | 0x0A51 | 0 | no RESP 4s | — |
| `IpcTxGetCpSpdStatus` | 0x0A48 | 0 | no RESP 4s | — |
| `IpcTxGetDeviceCapa` | 0x0A2D | 0 | — | `03 00` |
| `IpcTxGetTimeInfo` | 0x0A05 | 0 | **8001** | — |
| `IpcTxGetRfMipiInfo` | 0x0A53 | 0 | — | 48 B RF MIPI dump |
| `IpcTxGetCdmaSubscriptionSource` | 0x0A56 | 0 | **8001** | — |
| `IpcTxGetCaProperty` | 0x0A2F | 0 | **8003** | — |
| `IpcTxGetActivityInfo` | 0x0A50 | 0 | — | 36 B activity counters |
| `IpcTxGetSimCheckMessage` | 0x0A63 | `00` | **8001** | — |
| `IpcTxGetGripSensorInfo` | 0x0A42 | `00` | **8001** | — |
| `IpcTxGetCBConfig` | 0x040E | 0 | — | `02 02 32 00` |
| `IpcTxGetSmscAddress` | 0x040A | 0 | — | 12 B SMSC BCD (not transcribed) |
| `IpcTxGetStoredMsgCount` | 0x0409 | `00` | 0004 | no body |
| `IpcTxGetPinStatus` | 0x0501 | 0 | — | `00 00` |
| `IpcTxGetPhoneLock` | 0x0502 | `00` | — | `00 aa` |
| `IpcTxGetTfLockStatus` | 0x0514 | 0 | — | `0f 0f 0f` |
| `IpcTxGetAtr` | 0x050A | 0 | — | 24 B SIM ATR |
| `IpcTxGetSimAppsInfo` | 0x050F | 0 | — | 25 B USIM app info |
| `IpcTxGetUsimPhoneBookCapaEntriesInfo` | 0x0605 | `01` | — | 44 B PB capa |
| `IpcTxGetSapStatus` | 0x1202 | 0 | — | `03` |
| `IpcTxGetCardReaderStatus` | 0x1207 | 0 | — | `00 f0` |
| `IpcTxGetSapTransferAtr` | 0x1203 | 0 | — | 26 B ATR wrapper |
| `IpcTxGetMmsItem` | 0x1801 | 0 | **8003** | — |
| `IpcTxGetModemStatus` | 0x0109 | 0 | — | `01` |
| `IpcTxGetWorkingMode` | 0x0306 | 0 | **8003** | — |
| `IpcTxGetDdtmModeConfig` | 0x0305 | 0 | **8003** | — |
| `IpcTxGetEvdoRevisionConfig` | 0x030F | 0 | **8003** | — |
| `IpcTxGetActivatedDate` | 0x0F14 | 0 | — | 14 B OEM date |
| `IpcTxGetSsdData` | 0x0F35 | 0 | **8001** | — |
| `IpcTxGetRcData` | 0x0F0E | 0 | **8001** | — |
| `IpcTxGetReconditionedDate` | 0x0F11 | 0 | — | 8 B OEM date |
| `IpcTxGetEmbmsSignalStrength` | 0x1706 | 0 | no RESP 4s | — |
| `IpcTxGetEmbmsCcpGlobalCellId` | 0x170B | 0 | **8001** | — |
| `IpcTxGetEmbmsCcpMbmsSai` | 0x170A | 0 | **8001** | — |
| `IpcTxGetEmbmsSessionList` | 0x1705 | `00` | — | `00 00 00` |
| `IpcTxGetBandProvisioned` | 0x2214 | 0 | **8003** | — |
| `IpcTxGetBand41Enabled` | 0x2212 | 0 | **8003** | — |
| `IpcTxGetBand26Enabled` | 0x2211 | 0 | **8003** | — |
| `IpcTxGetEvdoAuthValue` | 0x2210 | 0 | **8003** | — |
| `IpcTxGetBand25Priority` | 0x2213 | 0 | **8003** | — |
| `IpcTxGetBand41TxSwitchingDiversity` | 0x2217 | 0 | **8003** | — |
| `IpcTxGetBand25Enabled` | 0x2215 | 0 | **8003** | — |
| `IpcTxGetLifeByte` | 0x2219 | 0 | **8003** | — |
| `IpcTxGetSIMBlobInfo` | 0x3005 | 0 | **8001** | — |
| `IpcTxGetDrkCpinfo` | 0x2601 | 0 | — | 62 B OEM ident (**redacted**) |

Implemented type-2 bodies are radio-feature reads (DRX/CA), RF/activity dumps, SIM/SMS/PB/SAP, and OEM dates. **None is a stuck PS/PLMN barring timer or a vendor PS-forbid flag we never SET.** **No remaining implemented unused `IpcTxGet*` that could move PS.** Do not invent another 8001-only probe as “next”.

CS stayed **HOME** → restore to 0x01 did **not** run. Domain left at **`02`**. DDS left slot **1**. **No PsAttach.** PS/rmnet did **not** move. GMM **#7** sticky.

### Raw NET_REGIST (after 0x0A55 + sweep)

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=R`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** Do not pack v032.

## Probe 60 — SET MODE_SEL `0x0a` only, GET-only wait (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 59 left GMM **#7** under GET-only MODE_SEL **`0x0b`**. This boot had **never SET MODE_SEL**. Hypothesis: UMTS-camped GMM#7 is not a hard HLR “no data”; vendor **`0x0a` LTE_WCDMA** (not AOSP 10 / **`0x2f`**) is required for EPS. **No** `0x2f`. **No** PsAttach **0x0D03**. **No** vendor PDP. **No** MODE_SEL **0x04/0x07**. **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running. Killed leftover **sleep** PID **610** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8869** `/tmp/ps-p60` **1129984**.

### Binary

Vendor emit `IpcTxNetSetPreferredNetType` table[12] **LTE_WCDMA** = **`0x0a`** (UMTS\|LTE). AOSP 10 emits **`0x2f`**. FMT SET: `08 00 XX ff 08 0a 03 0a`. Restore only **`0x0b`** if CS leaves HOME.

### Live ipc1 (SIM2)

Static `/tmp/ps-p60` (`os/build/e4-ps-p60.c`, Zig musl **1129984**). mseq from **0x20**. CP already **`ONLINE`** (same boot as probes 50–59). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**.

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | **0x0D23** type-3 LCE every ~1.2 s (ignored **526**). No STK ACK. |
| GET `PHONE_STATE` | **0x20** | **0x02** |
| GET `MODE_SEL` | **0x21** | **0x0b** |
| GET `0x0808` / `0x0816` | **0x22** / **0x23** | **`02`** CS_PS; slot=**1** cause=0 — **no 0x0808 SET** |
| GET CS / PS / GPRS_PS | **0x24–0x26** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `MODE_SEL` **0x0a** | **0x27** | **GEN 0x080A 0x8000 SUCCESS**. FMT `08 00 27 ff 08 0a 03 0a` |
| GET `MODE_SEL` after SET | **0x28** | **0x0a** (SET stuck; did **not** fold) |
| GET CS / PS after SET | **0x29–0x2b** | CS **HOME UMTS** fail=0 (LAC/CID zero — retune); PS **UMTS st=0x07** fail=**0** |
| GET-only polls | **0x2c–0x34** | t=0 / 3.2 / 6.4 s: MODE_SEL **0x0a**, CS HOME, PS st=**7** fail=0, att=0 |
| CS drop NOTI | — | LTE **0x21** CS **NONE** fail=0 at t≈6.4 s |
| SET `MODE_SEL` restore **0x0b** | **0x35** | **GEN 0x080A 0x8000 SUCCESS**. FMT `08 00 35 ff 08 0a 03 0b` |
| GET after restore | **0x36–0x39** | MODE_SEL **0x0b**; CS recovered **HOME UMTS** |
| final GET | **0x3a–0x3f** | MODE_SEL **0x0b**; **0x0808 `01`** (CP drifted; we did **not** SET domain); **0x0816** slot=**1**; CS **HOME UMTS**; PS **NONE fail=0**; att=0 |

**No SET 0x2f. No 0x0D03. No 0x0D1B/0x0D01/0x0D04.**

### After SET 0x0a (fail=0 window, ~6 s)

GMM **#7 cleared**. Immediate LTE NOTI, then UMTS GET with unknown regist **st=0x07** (not HOME/ROAMING/NONE). CS stayed HOME until a later LTE retune dropped CS to NONE.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |
| NOTI after SET 0x0a | **LTE 0x21** | **0x07 ?** | **0x00** | `21 03 07 00 00 00 48 37 42 06 00 c3 8d 02 01 00 7e 01 00 00` |
| GET after SET 0x0a CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| GET after SET 0x0a PS | **UMTS 0x04** | **0x07 ?** | **0x00** | `04 03 07 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| CS drop NOTI | **LTE 0x21** | **NONE 0x01** (dom CS) | **0x00** | `21 01 01 00 00 00 48 37 42 06 00 c3 8d 02 02 00 7e 01 00 00` |
| poll#3 CS GET | **LTE 0x21** | **NONE 0x01** | **0x00** | `21 02 01 00 00 00 48 37 42 06 00 c3 8d 02 02 00 ff ff 00 00` |
| poll#3 PS GET | **LTE 0x21** | **NONE 0x01** | **0x00** | `21 03 01 00 00 00 48 37 42 06 00 c3 8d 02 02 00 ff ff 00 00` |
| final CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| final PS | **UMTS 0x04** | **NONE 0x01** | **0x00** | `04 03 01 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |

FAIL HIST n=10: baseline GMM#7 → LTE/UMTS fail=0 st=7 while CS HOME → LTE CS NONE → after restore UMTS PS NONE fail=0.

`saw_fail0=1`. `saw_ps_home=0`. Wait aborted at **t≈6.4 s** (not the full 110 s). **Did not SET 0x2f.**

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / during / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=R`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** SET **0x0a** ACKs, sticks, clears GMM#7, produces LTE, then **CS drops** on LTE retune. Restore **0x0b** recovered CS HOME. Domain drifted **`02`→`01`**. Final PS is **NONE fail=0**, not GMM#7. **Do not SET MODE_SEL 0x2f** (same session; crash last boot was 0x0a+0x2f+PDP). **Do not SET 0x0D03.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 61 — hold MODE_SEL `0x0a` ~120 s, no restore on CS drop (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 60 aborted the fail=0 window at **t≈6.4 s** because CS left UMTS HOME on LTE — that abort was wrong (CS leaving UMTS HOME on LTE is expected). This pass: **SET MODE_SEL `0x0a` again**, GET-only **120 s**, **keep `0x0a`**. Restore **`0x0b` only** if the wait ends with no bearer (voice HOME back). **No** `0x2f`. **No** PsAttach **0x0D03**. **No** vendor PDP (PS never HOME). **No** MODE_SEL **0x04/0x07**. **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running. Killed leftover **sleep** PID **655** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8870** `/tmp/ps-p61` **1161360**.

### Binary

Same FMT as p60: `IpcTxNetSetPreferredNetType` table[12] **LTE_WCDMA** = **`0x0a`**. SET: `08 00 4c ff 08 0a 03 0a`. Helper **does not** restore `0x0b` when CS becomes LTE NONE. Restore `0x0b` only after a no-bearer wait. If `0x0808` drifted to `01` after restore, re-SET **`02`** and GET immediately.

### Live ipc1 (SIM2)

Static `/tmp/ps-p61` (`os/build/e4-ps-p61.c`, Zig musl **1161360**). mseq from **0x40**. CP already **`ONLINE`** (same boot as probes 50–60). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p61.txt` **1027162**.

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | **0x0D23** type-3 LCE every ~1.2 s (ignored **638**). No STK ACK. |
| GET `PHONE_STATE` | **0x40** | **0x02** |
| GET `MODE_SEL` | — | **0x0b** |
| GET `0x0808` / `0x0816` | — | **`01`** CS-only (p60 drift); slot=**1** cause=0 |
| GET CS / PS / GPRS_PS | — | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `0x0808` **02** | — | **GEN 0x0808 0x8000**. GET **`02`**. CS stayed **HOME** — continued |
| SET `0x0816` | — | skipped (slot already 1) |
| SET `MODE_SEL` **0x0a** | **0x4c** | **GEN 0x080A 0x8000 SUCCESS**. FMT `08 00 4c ff 08 0a 03 0a` |
| GET `MODE_SEL` after SET | — | **0x0a** (SET stuck; did **not** fold) |
| GET CS / PS after SET | — | CS **HOME UMTS** fail=0 (LAC/CID zero); PS **UMTS st=0x07** fail=**0** |
| GET-only hold **120 s** | 30 polls / 4 s | MODE_SEL **stayed `0x0a`**. See timeline. **No restore on CS drop.** |
| GET final-hold | — | MODE_SEL **0x0a**; **0x0808 `01`** (drifted); CS **HOME UMTS**; PS **NONE GMM#7**; att=0 |
| SET `MODE_SEL` restore **0x0b** | **0xcf** | **GEN 0x080A 0x8000**. GET **`0x0b`**. CS **HOME UMTS**. PS **st=0x07** fail=0 |
| Re-SET `0x0808` **02** | — | drifted to **`01`** after 0x0b. GEN **8000**. GET **`02`**. CS stayed **HOME** |

**No SET 0x2f. No 0x0D03. No 0x0D1B/0x0D01/0x0D04** (PS never HOME).

### Hold timeline (MODE_SEL `0x0a` the whole 120 s)

fail=0 did **not** hold for 110 s. Window was **~0–10.7 s**, then GMM **#7** returned and stuck for the rest of the hold. CS briefly left HOME on LTE at **t≈8.0 s** (same class as p60’s abort); helper **kept `0x0a`**. CS recovered HOME UMTS by **t≈8.4 s** and stayed HOME for the remaining ~112 s.

| t (s) | MODE | CS | PS | fail |
|--|--|--|--|--|
| baseline | **0x0b** | HOME UMTS | NONE UMTS | **7** |
| after SET 0x0a | **0x0a** | HOME UMTS | st=**0x07** UMTS | **0** |
| 0 / 4 / 8.0 | **0x0a** | HOME UMTS | st=**0x07** UMTS | **0** |
| **8.0** (NOTI) | **0x0a** | **LTE NONE** | **LTE NONE** | **0** — keep 0x0a |
| 8.4 | **0x0a** | HOME UMTS | NONE UMTS | **0** |
| **10.7** | **0x0a** | HOME UMTS | NONE UMTS | **7** (GMM#7 back) |
| 12 … 116 | **0x0a** | HOME UMTS | NONE UMTS | **7** (30 polls) |
| final-hold ~130 | **0x0a** | HOME UMTS | NONE UMTS | **7**; **0x0808 `01`** |
| after restore 0x0b | **0x0b** | HOME UMTS | st=**0x07** UMTS | **0**; **0x0808 `01`** |
| after re-SET 02 | **0x0b** | HOME UMTS | NONE UMTS | **0**; **0x0808 `02`** |

### Raw NET_REGIST

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |
| after SET 0x0a CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| after SET 0x0a PS | **UMTS 0x04** | **0x07 ?** | **0x00** | `04 03 07 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| LTE NOTI after SET | **LTE 0x21** | **0x07 ?** (dom PS) | **0x00** | `21 03 07 00 00 00 48 37 42 06 00 c3 8d 02 01 00 7e 01 00 00` |
| CS drop NOTI t≈8 s | **LTE 0x21** | **NONE 0x01** (dom CS) | **0x00** | `21 01 01 00 00 00 48 37 42 06 00 c3 8d 02 02 00 7e 01 00 00` |
| LTE CS GET t≈8 s | **LTE 0x21** | **NONE 0x01** | **0x00** | `21 02 01 00 00 00 48 37 42 06 00 c3 8d 02 02 00 ff ff 00 00` |
| LTE PS GET t≈8 s | **LTE 0x21** | **NONE 0x01** | **0x00** | `21 03 01 00 00 00 48 37 42 06 00 c3 8d 02 02 00 ff ff 00 00` |
| hold after t≈11 s CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| hold after t≈11 s PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

`saw_fail0=1`. `fail0_held=0` (GMM#7 from t≈10.7 s). `saw_ps_home=0`. `did_pdp=0`. Wait ran the full **120 s**. **Did not SET 0x2f.**

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / during / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=R` then `S`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** Holding **`0x0a`** for 120 s does **not** keep fail=0 and does **not** make PS HOME. GMM **#7** returns ~11 s after SET and stays while MODE_SEL is still **`0x0a`**. CS LTE NONE at ~8 s is transient; CS returns HOME UMTS without restore. After restore **`0x0b`**, PS is **NONE fail=0** again (not GMM#7). Domain drifted **`02`→`01`** during the hold; re-SET **`02`** stuck. **Do not SET MODE_SEL 0x2f** (same session; crash last boot was 0x0a+0x2f+PDP). **Do not SET 0x0D03.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 62 — SET 0x0a then 0x2f, GET-only, no PDP (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Recreate the old **~110 s fail=0** window (Probe 37/43: **`0x0a` then `0x2f`**) **without** vendor PDP (last-boot CRASH_EXIT was 0x0a+0x2f+PDP). Probe 61: **`0x0a` alone** is only ~11 s fail=0. This pass: cheap GET-only if leftover fail=0 under **`0x0b`**, then SET **`0x0a`** immediately SET **`0x2f`**, GET-only **120 s**. **No** restore on LTE CS NONE. **No** PsAttach **0x0D03**. **No** vendor PDP unless PS HOME. **No** MODE_SEL **0x04/0x07**. **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running. Killed leftover **sleep** PID **721** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8871** `/tmp/ps-p62` **1160720**.

### Binary

Vendor emit `IpcTxNetSetPreferredNetType` table[12] **LTE_WCDMA** = **`0x0a`**. table[10] **LTE_CDMA_EVDO_GSM_WCDMA** (AOSP-10 global) = **`0x2f`**. FMT SET 0x0a: `08 00 77 ff 08 0a 03 0a`. FMT SET 0x2f: `08 00 7c ff 08 0a 03 2f`. Helper does **not** restore `0x0b` when CS becomes LTE NONE. Restore `0x0b` only after a no-bearer wait. If `0x0808` drifted to `01` after restore, re-SET **`02`** and GET immediately.

### Live ipc1 (SIM2)

Static `/tmp/ps-p62` (`os/build/e4-ps-p62.c`, Zig musl **1160720**). mseq from **0x50**. CP already **`ONLINE`** (same boot as probes 50–61). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p62.txt` **1230508**.

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | **0x0D23** type-3 LCE every ~1.2 s (ignored **617**). No STK ACK. |
| GET `PHONE_STATE` | **0x50** | **0x02** |
| GET `MODE_SEL` | — | **0x0b** |
| GET `0x0808` / `0x0816` | — | **`02`** CS_PS; slot=**1** cause=0 — **no SET** |
| GET CS / PS / GPRS_PS | — | CS **HOME UMTS** fail=0; PS **NONE fail=0**; **attached=0** |
| cheap GET-only **30 s** | 8 polls / 4 s | MODE_SEL **`0x0b`**, CS HOME UMTS, PS **NONE fail=0** the whole 30 s. No HOME. No rmnet. **Continued.** |
| SET `0x0808` / `0x0816` | — | skipped (already **02** / slot 1) |
| SET `MODE_SEL` **0x0a** | **0x77** | **GEN 0x080A 0x8000 SUCCESS**. FMT `08 00 77 ff 08 0a 03 0a` |
| GET `MODE_SEL` after 0x0a | — | **0x0a** (SET stuck; did **not** fold) |
| GET CS / PS after 0x0a | — | CS **HOME UMTS** fail=0 (LAC/CID zero); PS **UMTS st=0x07** fail=**0** |
| SET `MODE_SEL` **0x2f** | **0x7c** | **GEN 0x080A 0x8000 SUCCESS**. FMT `08 00 7c ff 08 0a 03 2f`. GET-before was **0x0a** |
| GET `MODE_SEL` after 0x2f | — | **0x0b** (CP folded global → GSM\|UMTS\|LTE) |
| GET CS / PS after 0x2f | — | CS **HOME UMTS** fail=0 (LAC/CID still zero); PS **UMTS st=0x07** fail=**0**. `fail_after_2f=0` |
| GET-only hold **120 s** | 30 polls / 4 s | MODE_SEL stayed folded **`0x0b`**. See timeline. **No restore on CS drop.** |
| GET final-hold | — | MODE_SEL **0x0b**; **0x0808 `01`** (drifted); CS **HOME UMTS**; PS **NONE GMM#7**; att=0 |
| SET `MODE_SEL` restore **0x0b** | **0xff** | **GEN 0x080A 0x8000**. GET **`0x0b`**. CS **HOME UMTS**. PS **NONE GMM#7** |
| Re-SET `0x0808` **02** | — | drifted to **`01`** after 0x0b. GEN **8000**. GET **`02`**. CS stayed **HOME** |

**No 0x0D03. No 0x0D1B/0x0D01/0x0D04** (PS never HOME).

### Cheap 30 s (leftover fail=0 under `0x0b`)

p61 restore left PS **NONE fail=0**. That leftover window **held the whole 30 s** — GMM **#7 did not return**. PS stayed NONE. No rmnet. Helper continued to SET **0x0a** then **0x2f**.

### Hold timeline (after 0x0a+0x2f; MODE_SEL folded `0x0b`)

fail=0 did **not** hold for 110 s. After 0x2f the new window was **~0–8.5 s**, then GMM **#7** stuck for the rest of the 120 s. CS GET stayed **HOME UMTS** every poll (brief LTE NOTIs after SET 0x0a; helper **did not restore**).

| t (s) | MODE | CS | PS | fail |
|--|--|--|--|--|
| baseline | **0x0b** | HOME UMTS | NONE UMTS | **0** |
| cheap 0 … 30 | **0x0b** | HOME UMTS | NONE UMTS | **0** (held) |
| after SET 0x0a | **0x0a** | HOME UMTS | st=**0x07** UMTS | **0** |
| LTE NOTI after 0x0a | **0x0a** | LTE NONE (NOTI) | LTE st=**0x07** | **0** — keep, no restore |
| after SET 0x2f | **0x0b** | HOME UMTS | st=**0x07** UMTS | **0** |
| hold 0 / 4 | **0x0b** | HOME UMTS | st=**0x07** UMTS | **0** |
| 6.2 / 8.0 | **0x0b** | HOME UMTS | NONE UMTS | **0** |
| **8.5** | **0x0b** | HOME UMTS | NONE UMTS | **7** (GMM#7 back) |
| 12 … 116 | **0x0b** | HOME UMTS | NONE UMTS | **7** (30 polls) |
| final-hold ~130 | **0x0b** | HOME UMTS | NONE UMTS | **7**; **0x0808 `01`** |
| after restore 0x0b | **0x0b** | HOME UMTS | NONE UMTS | **7**; **0x0808 `01`** |
| after re-SET 02 | **0x0b** | HOME UMTS | NONE UMTS | **7**; **0x0808 `02`** |

### Raw NET_REGIST

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline / cheap PS | **UMTS 0x04** | **NONE 0x01** | **0x00** | `04 03 01 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after SET 0x0a CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| after SET 0x0a PS | **UMTS 0x04** | **0x07 ?** | **0x00** | `04 03 07 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| LTE NOTI after 0x0a | **LTE 0x21** | **0x07 ?** (dom PS) | **0x00** | (NOTI; fail **00**) |
| LTE CS NOTI | **LTE 0x21** | **NONE 0x01** (dom CS) | **0x00** | (NOTI; CS GET stayed HOME UMTS next poll) |
| after SET 0x2f CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| after SET 0x2f PS | **UMTS 0x04** | **0x07 ?** | **0x00** | `04 03 07 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| hold after t≈8.5 s CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| hold after t≈8.5 s PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

`saw_fail0=1`. `fail0_held=0` (GMM#7 from t≈8.5 s after 0x2f). `saw_ps_home=0`. `did_pdp=0`. Cheap ran **30 s** (`cheap_stop=0`). Wait ran the full **120 s**.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / cheap / hold / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. CP stayed **`ONLINE`**. Holder **338** alive (`state=R`, ipc0+rfs0). ipc1-holder **326** untouched. **Data-plane goal not complete.** SET **`0x0a`** ACKs and sticks; SET **`0x2f`** ACKs and folds to **`0x0b`**. On this boot that path does **not** recreate Probe 43’s 110 s fail=0 window — new fail=0 after 0x2f is **~8.5 s**, same class as p61’s **`0x0a`-alone ~11 s**. Leftover fail=0 under folded **`0x0b`** (from p61) **did** hold 30 s until the new SETs retuned (LAC/CID zero) and GMM **#7** returned. After restore **`0x0b`**, PS is **NONE GMM#7** (not fail=0). Domain drifted **`02`→`01`**; re-SET **`02`** stuck. **No crash.** **Do not SET 0x0D03.** **Do not vendor PDP** while PS is not HOME. **Do not SET 0x0808 0x03.** **Do not ACK STK.** Do not pack v032.

## Probe 63 — SET `0x0a` then immediate vendor PDP, no `0x2f` (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 62: leftover fail=0 under **`0x0b`** held 30 s with no attach; **`0x0a` then `0x2f`** did **not** recreate Probe 43’s 110 s window (new fail=0 only **~8.5 s**) and killed leftover fail=0. Remaining shot: vendor PDP **immediately** in the short **`0x0a`** fail=0 window — **no `0x2f`**, **no PsAttach**, **no 30 s wait**. **No** MODE_SEL **0x04/0x07/0x2f**. **No** `POWER_OFF`. **No** `loadnv`. Holder **338** STOP/CONT only (did **not** kill). ipc1-holder **326** left running. Killed leftover **sleep** PID **785** (held ipc1; not 326). RNDIS host **192.168.42.16**. Telnet `192.168.42.1:23`. wget **8872** `/tmp/ps-p63` **1160360**.

### Binary

Reuse p53/p45 PDP pack (`0x0D1B` len **0xCB** `lte_internet`+**`www.vodafone.net.ua`**, `0x0D01` len **0x95**, `0x0D04` len **0xF8**) and p60 MODE_SEL SET **`0x0a`** FMT `08 00 .. ff 08 0a 03 0a`. Helper **refuses** SET **`0x2f`**. If baseline fail=0 under **`0x0b`**: PDP immediately, no MODE_SEL. If GMM#7: ensure **0x0808=02** + DDS slot 1, SET **`0x0a`**, fire PDP on the same fd as soon as GEN ACK (max 2 s), do **not** poll 8–11 s first. Drain NOTIs **30 s** (hope **0x0D09**). Restore **`0x0b`** only if no bearer after drain; re-SET **0x0808=02** if drifted. **No** `0x0D03`.

### Live ipc1 (SIM2)

Static `/tmp/ps-p63` (`os/build/e4-ps-p63.c`, Zig musl **1160360**). mseq from **0x60**. CP already **`ONLINE`** at start (same boot as probes 50–62). Pre-telnet: holder **338** `radio-boot` fds ipc0+rfs0; **326** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p63.txt` **206739**.

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | **0x0D23** type-3 LCE (ignored **611**). No STK ACK. |
| GET `PHONE_STATE` | **0x60** | **0x02** |
| GET `MODE_SEL` | **0x61** | **0x0b** |
| GET `0x0808` / `0x0816` | **0x62** / **0x63** | **`02`** CS_PS; slot=**1** — **no SET** |
| GET CS / PS / GPRS_PS | **0x64–0x66** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `MODE_SEL` **0x0a** | **0x67** | **GEN 0x080A 0x8000 SUCCESS**. FMT `08 00 67 ff 08 0a 03 0a`. ACK drain **~1.5 s**. fail still **#7** |
| SET `0x0D1B` 0xCB vodafone | **0x68** | **GEN 0x0D1B 0x8000**. **fail0_at_pdp=0** (still GMM#7). APN **`www.vodafone.net.ua`** |
| SET `0x0D01` 0x95 | **0x69** | **GEN 0x0D01 0x8000**. **fail_now=0** (window opened) |
| SET `0x0D04` 0xF8 | **0x6a** | **GEN 0x0D04 0x8000**. **0x0D10** cid=**1** st=**0x03**. **No 0x0D09** |
| PDP drain **30 s** | **0x6b–0x82** | fail=0 until **~24 s**, then GMM **#7**. CS HOME UMTS. **No restore 0x0b** during drain |
| GET after drain | **0x83–0x88** | MODE_SEL **`0x0a`** (SET stuck); **0x0808 `01`** (drifted); PS **NONE GMM#7**; att=0 |
| SET `MODE_SEL` restore **0x0b** | **0x89** | **GEN 0x080A 0x8000**. GET **`0x0b`**. CS **HOME UMTS**. PS st=**0x07** fail=**0** |
| Re-SET `0x0808` **02** | **0x8f** | drifted to **`01`**. GEN **8000**. GET **`02`**. CS stayed **HOME** |
| GET final | **0x94–0x99** | MODE_SEL **0x0b**; **0x0808 `02`**; CS **HOME UMTS**; PS **NONE fail=0**; att=0 |

**No 0x0D03. No SET 0x2f.**

### fail=0 vs PDP TX

Baseline was **GMM#7** under **`0x0b`** — no leftover fail=0 window to fire PDP without MODE_SEL.

| when | fail | note |
|--|--|--|
| SET `0x0a` ACK (~1.5 s) | **7** | window not open yet |
| TX `0x0D1B` | **7** | **fail0_at_pdp=0** |
| GEN `0x0D01` (~250 ms later) | **0** | window opened between 0x0D1B and 0x0D01 |
| TX / GEN `0x0D04` | **0** | **0x0D10** already cid=1 st=`0x03` |
| drain t=0 … ~24 s | **0** | LTE then UMTS st=`0x07`, then NONE fail=0 |
| drain t≈24.1 s | **7** | GMM#7 back |
| helper exit | **0** | after restore `0x0b` leftover fail=0 (like p60) |

fail=0 during drain **~24 s** — longer than p61’s **`0x0a`-alone ~11 s** / p62’s after-0x2f **~8.5 s**. Still not Probe 43’s 110 s. **0x0D01 and 0x0D04 went out inside the window.** Still **0x0D10 st=0x03**, no **0x0D09**, no rmnet.

### Raw NET_REGIST

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |
| drain t=0 PS NOTI | **LTE 0x21** | **0x07 ?** | **0x00** | `21 03 07 00 00 00 48 37 42 06 00 c3 8d 02 01 00 7e 01 00 00` |
| drain fail=0 PS GET | **UMTS 0x04** | **0x07 ?** | **0x00** | `04 03 07 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| after PDP CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after PDP PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

`saw_fail0=1`. `fail0_held=0` (GMM#7 from drain t≈24 s). `saw_ps_home=0`. `did_pdp=1`. `pdp_under_0b=0`.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / drain / helper exit | ONLINE | rx=tx=**0** (all eight) |
| post-check | **CRASH_EXIT** | rx=tx=**0** |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 338** after probe. Helper `after modem_state=ONLINE` last_tx=**GPRS_PS GET mseq=0x99**. Last SET was re-SET **0x0808=02** mseq **0x8f**. Holder **338** alive (ipc0+rfs0). ipc1-holder **326** untouched.

### CRASH_EXIT (after helper)

Helper completed **ONLINE**, win=0. Later sysfs **`CRASH_EXIT`**. dmesg ring wrapped: earliest remaining **t=9479.64** already holder **338** `ipc_poll: umts_ipc0/rfs0: s318ap.state == CRASH_EXIT` (~50 s after CONT). **8237** `CRASH_EXIT` poll lines. **0** `CP_CRASH`. **0** `INIT_END`. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`. Same class as last-boot crash (**0x0a** + vendor PDP; this pass had **no 0x2f**).

**Data-plane goal not complete.** SET **`0x0a`** ACKs and sticks. Immediate PDP GENs **8000**. **0x0D10 st=0x03** even when **0x0D01/0x0D04** fired under fail=0. **Do not SET 0x0D03.** **Do not SET 0x2f.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 64 — CRASH_EXIT AP reboot + GET-only baseline (v031, 2026-09-02)

Goal: restore **`ONLINE`** after probe 63 delayed `CRASH_EXIT`, then GET-only ipc1 (MODE_SEL / 0x0808 / 0x0816 / NET_REGIST CS+PS hex / GPRS_PS / rmnet). **No SET** this pass (no MODE_SEL **0x04/0x07/0x0a/0x2f**, no PDP, no PsAttach, no MO, no SMS). **No** original efs. **No** `POWER_OFF`. **No** leftover `loadnv` on `CRASH_EXIT`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **316** (held ipc1; not 314). RNDIS host **192.168.42.20** (Ethernet 6, `ipconfig`; was **192.168.42.16** before reboot). Telnet `192.168.42.1:23`. wget **8873** `/tmp/ps-p64` **1108808**.

### Before (leftover probe 63 boot)

| | |
|--|--|
| uptime | **9818.37** s (same v031 boot as probes 50–63) |
| `modem_state` | **`CRASH_EXIT`** |
| RNDIS | **192.168.42.16** |
| holder | **338** `radio-boot` fds **5=`umts_ipc0`** **6=`umts_rfs0`** |
| ipc1-holder | **326** fd **3=`umts_ipc1`** |
| userdata | still mounted p38; bind copy on `/mnt/vendor/efs` |
| `rmnet0` | rx=tx=**0** |

**Did not** `loadnv`. **Did not** `IOCTL_POWER_OFF`. **Did not** IPC SET.

### AP reboot

Plain telnet `reboot` returned a prompt. Uptime kept climbing (**9974.84**, still **`CRASH_EXIT`**). No `/sbin/reboot` / `/bin/reboot` file; `type reboot` → `reboot is reboot`; PID 1 is `/init`. **`busybox reboot -f`** (force, skip init) dropped the session. Telnet back ~1–2 min. **Not** `IOCTL_POWER_OFF`. **Not** sysrq-b this pass.

### After reboot (first telnet)

| | |
|--|--|
| uptime | **129.46** s (fresh v031) |
| `modem_state` | **`OFFLINE`** |
| GNSS | **`OFFLINE`** |
| RNDIS | **192.168.42.20** |
| userdata | **not** mounted (`/mnt/userdata` missing) |
| `/tmp/sipc-holder.pid` | **absent** |

Re-read sysfs: `mmcblk0p38` `PARTNAME=userdata` **259:30**. `mknod` `/dev/block/mmcblk0p38` b 259 30 + p22 **259:14**. `mount -t ext2 /dev/block/mmcblk0p38 /mnt/userdata`. Still **`OFFLINE`**. `/mnt/userdata/cp-boot.sh` **4226**, `radio-boot` **1314320**, NV copy present. Original p1/p2/p4 **not** mounted.

### One `cp-boot.sh` (OFFLINE only)

`sh /mnt/userdata/cp-boot.sh` at uptime **178.08**. Start line **`modem_state=OFFLINE`**. Bind `saaios-efs-copy` → `/mnt/vendor/efs` (p38 only). ipc1-holder **314**. Then **one** `$RB` loadnv (radio-boot **325** → holder **326**):

| | |
|--|--|
| `IOCTL_POWER_ON` | rc=0; dmesg **`First init`** |
| `POWER_RESET` | rc=0; `already offline` |
| LOAD BOOT+MAIN+**VSS**+NV (userdata copy) | OK |
| `IOCTL_START_CP_BOOTLOADER` | rc=0 → **`BOOTING`** |
| UDL A00D/AF00 | **ACK** |
| `COMPLETE_NORMAL_BOOTUP` | rc=0 → **`ONLINE`** |
| dmesg | `rild_ready` ipc0+rfs0; **`INIT_END -> s318ap`** (179.18) |
| `GET_CP_STATUS` | **4** |

**Did not** run `loadnv` a second time. **Did not** `POWER_OFF`.

### Live (GET-only)

| | |
|--|--|
| uptime | **204.27** s (pre-helper); **288.14** s (post-check, still **`ONLINE`**) |
| `modem_state` | **`ONLINE`** |
| `/tmp/sipc-holder.pid` | **326** (`radio-boot loadnv`, fds **5=`umts_ipc0`** **6=`umts_rfs0`** **7=`/tmp/rfs.log`**) |
| ipc1-holder | **314** (`sh /mnt/userdata/cp-boot.sh`, fd **3=`umts_ipc1`**) |

Static `/tmp/ps-p64` (`os/build/e4-ps-p64.c`, Zig musl **1108808**). mseq from **0xB0**. `send_small` refuses non-GET. Leftover drain **5 s** saw LTE NOTI (act=**0x21** PS fail=0 st=`0x07`) then UMTS CS **HOME** + PS **NONE** fail=0, then GMM **#7** on NOTI before GETs.

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | LTE then UMTS CS **HOME**; PS fail=0 then **GMM#7**. DISP **0x0706**. **No STK ACK.** |
| GET `PHONE_STATE` | **0xB0** | **0x02** |
| GET `MODE_SEL` | **0xB1** | **0x0b** — **no SET** |
| GET `0x0808` | **0xB2** | **`01`** CS-only — **no SET** |
| GET `0x0816` | **0xB3** | **`00 00`** (slot=0 cause=0) |
| GET CS / PS / GPRS_PS | **0xB4** / **0xB5** / **0xB6** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |

`sets=0`. **CONT 326** after probe. CP stayed **`ONLINE`**. Holder **326** alive (`state=R`, ipc0+rfs0). ipc1-holder **314** untouched.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present** (`fail_missing=0`).

| domain | act | st | fail | body (20 B) |
|--|--|--|--|--|
| CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

0x0808 GET **`01`** (CS-only) on this fresh boot — same as Probe 50. GMM **#7** sticky on PS; CS still **HOME**.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **Data-plane goal not complete.** **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 65 — CS_PS then DDS (Probe 52 order), no wait if domain reverts (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 64 GET-only on this **new AP boot**: MODE_SEL **`0x0b`**, **0x0808 `01`**, **0x0816 `00 00`**, CS **HOME UMTS**, PS **NONE GMM#7**. This pass: Probe 52 order — SET **0x0808 `02`**, **GET immediately** (must **`02`** before any other SET), then SET **0x0816 `01 01`**, GET **0x0808** (must still **`02`**) + **0x0816**, then GET-only wait ~110 s for PS HOME. Vendor PDP **`www.vodafone.net.ua`** only if PS HOME. If still GMM#7: no PDP / no PsAttach / no MODE_SEL. **No** MODE_SEL **0x04/0x07/0x0a/0x2f**. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **356** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8874** `/tmp/ps-p65` **1135096**.

### Binary

Reuse p52 SET order + p53 wait/PDP pack. Helper **refuses** SET MODE_SEL / **0x0D03** / **0x0808 `03`**. PDP only if PS HOME (not under GMM#7). If GET after DDS is not **`02`**: abort wait and PDP.

### Live ipc1 (SIM2)

Static `/tmp/ps-p65` (`os/build/e4-ps-p65.c`, Zig musl **1135096**). mseq from **0xE0**. CP already **`ONLINE`** (same boot as probe 64). Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p65.txt` **15356**.

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** only. **No STK ACK.** |
| GET `PHONE_STATE` | **0xE0** | **0x02** |
| GET `MODE_SEL` | **0xE1** | **0x0b** — **no SET** |
| GET `0x0808` / `0x0816` | **0xE2** / **0xE3** | **`01`** CS-only; slot=**0** cause=0 |
| GET CS / PS / GPRS_PS | **0xE4–0xE6** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `0x0808` **`02`** | **0xE7** | **GEN 0x0808 0x8000 SUCCESS** |
| GET `0x0808` (before 0x0816) | **0xE8** | body **`02`** (enum **2** CS_PS) — **sticks** |
| SET `0x0816` **`01 01`** | **0xE9** | **GEN 0x0816 0x8000 SUCCESS** |
| GET `0x0808` after DDS | **0xEA** | body **`01`** — **first DDS this boot reverts CS_PS** |
| GET `0x0816` after DDS | **0xEB** | body **`01 00`** (slot=**1** cause=0) |
| GET CS / PS / GPRS_PS | **0xEC–0xEE** | CS **HOME UMTS**; PS **NONE GMM#7**; att=0 |

CS stayed **HOME** → **restore to 0x01 did not run**. GET after DDS was **`01`** → **no wait. No PDP. No 0x0D03. No MODE_SEL SET.**

### GEN

| cmd | GEN |
|--|--|
| SET `0x0808` `02` | **0x8000** SUCCESS |
| SET `0x0816` `01 01` | **0x8000** SUCCESS |
| `0x0D1B` / `0x0D01` / `0x0D04` | **not sent** |

### 0x0808 / 0x0816

| when | 0x0808 | 0x0816 |
|--|--|--|
| baseline | **`01`** CS | **`00 00`** |
| after SET-only GET (before DDS) | **`02`** CS_PS | (unchanged) |
| after DDS GET | **`01`** CS | **`01 00`** slot=1 |

Probe 52 same-fd GET after **0x0808 SET** was **`02`**, and DDS did **not** revert — but that boot already had **0x0816 slot=1** from probe 51. This clean boot (slot **0** → first **`01 01`**) **does** revert **0x0808 `02`→`01`**, matching Probe 51’s GET-after-both. Wait **did not run** (`wait_polls=0` `saw_home=0` `saw_fail0=0`). `did_pdp=0`. `skipped_pdp=1`.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline / after DDS CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline / after DDS PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. CP stayed **`ONLINE`** (uptime **695** s post-check). Holder **326** alive (`state=S`/`R`, ipc0+rfs0). ipc1-holder **314** untouched. **Data-plane goal not complete.** First DDS this boot reverts CS_PS; GET-only wait under **`02`** was not reached. **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 66 — CS_PS AFTER DDS already slot 1 (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 65: SET **0x0808 `02`** sticks on immediate GET; first **0x0816 `01 01`** this boot (slot was **`00 00`**) reverts it to **`01`**. Wait aborted. Hypothesis: with DDS already slot **1**, SET **0x0808 `02`** will **stick** (Probe 52). This pass: if slot != 1, SET **`01 01` first** then domain; if slot already 1, SET domain only. Immediate GET must **`02`**; GET after ~2 s must still **`02`**; then GET-only wait ~110 s. Vendor PDP **`www.vodafone.net.ua`** only if PS HOME. If still GMM#7: no PDP / no PsAttach / no MODE_SEL. **No** MODE_SEL **0x04/0x07/0x0a/0x2f**. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **384** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8875** `/tmp/ps-p66` **1136952**.

### Binary

Reuse p52 SET pack + p53 wait/PDP pack. Helper **refuses** SET MODE_SEL / **0x0D03** / **0x0808 `03`**. PDP only if PS HOME (not under GMM#7). If slot != 1: SET DDS **before** domain. If immediate or 2 s GET is not **`02`**: abort wait and PDP.

### Live ipc1 (SIM2)

Static `/tmp/ps-p66` (`os/build/e4-ps-p66.c`, Zig musl **1136952**). mseq from **0xF0**. CP already **`ONLINE`** (same boot as probes 64–65). Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p66.txt` **54641**.

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** + **0x0701**. **No STK ACK.** |
| GET `PHONE_STATE` | **0xF0** | **0x02** |
| GET `MODE_SEL` | **0xF1** | **0x0b** — **no SET** |
| GET `0x0808` / `0x0816` | **0xF2** / **0xF3** | **`01`** CS-only; slot=**1** cause=0 (from probe 65) |
| GET CS / PS / GPRS_PS | **0xF4–0xF6** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `0x0816` | — | **skipped** (`slot_was_1=1`) |
| SET `0x0808` **`02`** | **0xF7** | **GEN 0x0808 0x8000 SUCCESS** |
| GET `0x0808` immediate | **0xF8** | body **`02`** (enum **2** CS_PS) — **sticks** |
| GET `0x0808` after ~2 s | **0xF9** | body **`02`** — **still sticks** with slot already 1 |
| GET-only wait **110 s** (28 polls, CS+PS) | **0xFD+** | PS stayed **NONE GMM#7**; **saw_home=0** **saw_fail0=0**. **No 0x0D03** |
| GET final | **0x51–0x56** | MODE_SEL **`0x0b`**; **0x0808 `02`**; **0x0816 `01 00`**; CS **HOME**; PS **NONE GMM#7**; att=0 |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **`02`**. DDS left slot **1**. **No PDP. No PsAttach. No MODE_SEL SET.**

### GEN

| cmd | GEN |
|--|--|
| SET `0x0808` `02` | **0x8000** SUCCESS |
| SET `0x0816` | **not sent** (`did_dds=0`) |
| `0x0D1B` / `0x0D01` / `0x0D04` | **not sent** |

### 0x0808 / 0x0816

| when | 0x0808 | 0x0816 |
|--|--|--|
| baseline | **`01`** CS | **`01 00`** slot=1 |
| after SET immediate GET | **`02`** CS_PS | (unchanged) |
| after 2 s GET | **`02`** CS_PS | (unchanged) |
| after wait / final | **`02`** CS_PS | **`01 00`** slot=1 |

Hypothesis **confirmed**: with slot already **1**, SET **0x0808 `02`** sticks (immediate + 2 s + after wait). Probe 52 class. First-boot DDS revert (Probe 65 / 51) is a one-shot this boot. Wait **did run** (`wait_polls=28` `saw_home=0` `saw_fail0=0`). `did_pdp=0`. `skipped_pdp=1`. `sticky_gmm7=1`. Same wait outcome as Probe 53 under leftover **`02`**.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline / wait / final CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline / wait / final PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. CP stayed **`ONLINE`** (uptime **1203** s post-check). Holder **326** alive (`state=S`/`R`, ipc0+rfs0). ipc1-holder **314** untouched. **Data-plane goal not complete.** CS_PS **sticks** after DDS slot=1, but GET-only wait never sees PS HOME / fail=0. **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 67 — vendor PDP under CS_PS, NO MODE_SEL 0x0a (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Last boot crashed ~50 s after **`0x0a` + PDP**. Probe 66 left **0x0808 `02`** stuck + DDS slot **1**, MODE_SEL **`0x0b`**, PS **NONE GMM#7**, no PDP. This pass: vendor PDP **only** — MODE_SEL stays **`0x0b`** (no SET **0x0a/0x2f/0x04/0x07**). Same 0x0D1B/0x0D01/0x0D04 pack as Probe 53, APN **`www.vodafone.net.ua`**, on this clean boot with CS_PS already stuck. **No** PsAttach **0x0D03**. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **420** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8876** `/tmp/ps-p67` **1137384**.

### Binary

Reuse p53 PDP frames (`0x0D1B` len **0xCB** `lte_internet`+**`www.vodafone.net.ua`**, `0x0D01` len **0x95**, `0x0D04` len **0xF8**). Helper **refuses** SET MODE_SEL / **0x0D03** / **0x0808 `03`**. If **0x0808 != 02**: SET **02**, GET immediately; CS drop → restore **01**. If slot is **0**: SET **0x0816 `01 01` before** domain. Drain wall-clock **24 s** (6×4 s) for **0x0D09** vs **0x0D10**; sample CS/PS between chunks.

### Live ipc1 (SIM2)

Static `/tmp/ps-p67` (`os/build/e4-ps-p67.c`, Zig musl **1137384**). mseq from **0xA0**. CP already **`ONLINE`** (same boot as probes 64–66). Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p67.txt` **27601**.

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** only. **No STK ACK.** |
| GET `PHONE_STATE` | **0xA0** | **0x02** |
| GET `MODE_SEL` | **0xA1** | **0x0b** — **no SET** |
| GET `0x0808` / `0x0816` | **0xA2** / **0xA3** | **`02`** CS_PS; slot=**1** cause=0 — **both SETs skipped** |
| GET CS / PS / GPRS_PS | **0xA4–0xA6** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `0x0D1B` 0xCB vodafone | **0xA7** | **GEN 0x0D1B 0x8000 SUCCESS** |
| SET `0x0D01` 0x95 | **0xA8** | **GEN 0x0D01 0x8000 SUCCESS** |
| SET `0x0D04` 0xF8 | **0xA9** | **GEN 0x0D04 0x8000 SUCCESS**. NOTI **0x0D10** cid=**1** st=**0x03**. **No 0x0D09** |
| PDP watch **24 s** (6 samples, CS+PS) | **0xAA+** | **0x0D10** stayed cid=1 st=`0x03` n10=**1**. **n09=0**. CS HOME / PS NONE GMM#7 every sample. MODE_SEL **`0x0b`**. **0x0808 `02`** |
| GET final | **0xBC–0xC1** | MODE_SEL **`0x0b`**; **0x0808 `02`**; **0x0816 `01 00`**; CS **HOME**; PS **NONE GMM#7**; att=0 |

CS stayed **HOME** → **restore to 0x01 did not run**. Domain left at **`02`**. DDS left slot **1**. MODE_SEL stayed **`0x0b`**. **No PsAttach. No MODE_SEL SET.** `sets=0`. `did_pdp=1`. `skipped_0808=1`. `skipped_0816=1`. `slot_was_1=1`.

### GEN

| cmd | GEN |
|--|--|
| SET `0x0808` / `0x0816` | **not sent** (already 02 / slot 1) |
| `0x0D1B` | **0x8000** SUCCESS |
| `0x0D01` | **0x8000** SUCCESS |
| `0x0D04` | **0x8000** SUCCESS |

### 0x0D09 vs 0x0D10

| when | 0x0D10 | 0x0D09 |
|--|--|--|
| after SET `0x0D04` (~chunk 1) | cid=**1** st=**0x03** n=1 | **none** |
| samples 1–6 (t=4 … 36.5 s) | still 1/`0x03` n10=**1** | **n09=0** |
| after / final | same | **none** |

Same class as Probe 53: all three PDP GENs **8000**, immediate **0x0D10 st=0x03**, never **0x0D09**. Watch wall **36510** ms (`pdp_samples=6`). No extra 0x0D10 after the first NOTI.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**. Unchanged baseline / drain / final.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline / drain / final CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline / drain / final PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / drain / helper exit / +90 s | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. Helper `after modem_state=ONLINE` last_tx=**GPRS_PS GET mseq=0xc1**. Delayed check **~90 s** after CONT: still **`ONLINE`** (uptime **1717** s). **No CRASH_EXIT** (unlike last-boot **0x0a** + PDP). Holder **326** alive (`state=R`, ipc0+rfs0). ipc1-holder **314** untouched. GNSS **OFFLINE**. **Data-plane goal not complete.** Vendor PDP under leftover **CS_PS=02** + slot=1 + MODE_SEL **`0x0b`** on this clean boot matches Probe 53: **0x0D10 st=0x03**, no bearer. **Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 68 — decode GET 0x0D04 `01 18 00` / NOTI 0x0D10 st=`0x03` (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 67 vendor PDP GENs **8000** then **0x0D10** cid=1 st=**`0x03`**, no **0x0D09**. This pass: host-decode **`0x18`** / st=**`0x03`**, GET live **0x0D04**, SET only if the `.so` names a missing vendor IPC that is not MODE_SEL **0x0a**, not **0x0D03**, not another APN copy. **No** MODE_SEL SET. **No** PsAttach **0x0D03**. **No** PDP SET. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **454** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8877** `/tmp/ps-p68` **1108904**.

### Binary: `IpcRxPdpContext` / GET 0x0D04 (no `IpcRxSetPdpContext`)

Host `e4-p68-decode.c` (`e4-ril-disasm.c` / `e4-ip-cfg-decode.c` helpers). `libsec-ril.so` **4541576**.

No symbol **`IpcRxSetPdpContext`**. GET/NOTI parse is **`IpcRxPdpContext`** @ **0x36dc08** (size 592). **`IpcTxGetPdpContext`** @ **0x36a1f4** empty GET packed **`0x040d`**. **`IpcTxSetPdpContext`** @ **0x369ec8** is the 0xF8 SET (already sent p26–p67).

Type `[6]==SET 0x03` skips parse. GET: **`[7]` = count**. Records **2 B** from `[8]`: loop status at record+1 compared to **0x01 / 0x0A / 0x0B** only (map **4 / 5 / 6**); else mapped **0** (inactive). Record+0 is **stored raw** — **no `cmp #0x18`** in this RX.

| body | IpcRx field |
|--|--|
| `01` | count **1** |
| `18` | record[0] raw (stored; **not** the 1 / 0x0A / 0x0B status map) |
| `00` | record[1] status → **not** {1, 0x0A, 0x0B} → mapped **0** (inactive) |

### Binary: map `0x18`

No **`RIL_LastDataCallActivateFailCause`**. No **`SM cause` / `ESM cause` / `GMM cause` / `PDP_FAIL` / `not attached`** strings. No **`IpcRxPdpContextNoti`**. **`IpcRxPdpContext` does not switch on `0x18`.**

Data-plane **`cmp #0x18`**: only **`DataCallManager::IsSkipPartialRetryFailCause`** @ **0x24d91c** (AOSP `DataFailCause` **24**). 3GPP SM/ESM **#24** is **reserved** (gap #8–#25). If the byte were a NAS/GMM cause it would be **#24 Not authorized for this CSG** — this RX does **not** run that mapper. AOSP `RIL_DataCallFailCause` has no **0x18** (`0x19`=LLC_SNDCP, `0x1A`=NO_RESOURCES). Operator `CustomizeFailCause` strings convert many codes to **unspecified**.

**`0x18` = unnamed CP raw reason on one inactive PDP record.** Combined with live **GMM#7** + **attached=0** + **0x0D10 st=0x03** + end-reason **0x00**: **not-attached / unspecified reject**. **No new vendor IPC** named as a 0x0D04 prerequisite.

### Binary: `IpcRxGprsCallStatus` / 0x0D10 st=`0x03`

No **`IpcTxGetGprsCallStatus`** — **0x0D10 is NOTI-only** (no GET). Handler **`IpcRxGprsCallStatus`** @ **0x36d29c**: `[7]` CID, `[8]` status, `[9]` end-reason (`"Data call end reason(/%d)"` @ **0x36d5c0** when not connected), `[0xa]`/`[0xb]` extras, `[0xc]` throttle.

| `[8]` | map | flags | log |
|--|--|--|--|
| **0x01** | 4 | connected | `CDMA Data call(%d)` |
| **0x0A** | 5 | — | (no disconnected) |
| **0x0B** | 6 | — | (no disconnected) |
| **else (incl. 0x03)** | 0 | disconnected | **`CDMA Data call(%d) disconnected`** |

**st=`0x03` = disconnected** (default/else). Probe 67 NOTI full hex (still on device `/tmp/p67.txt`):

```text
IPC RX (17): 11 00 dc 00 0d 10 03 01 03 00 00 00 00 00 00 00 00
```

FMT len=**17** type=**3** plen=**10**. Body **`01 03 00 00 00 00 00 00 00 00`**: cid=**1**, st=**`0x03`**, end-reason **`0x00`**, extras/throttle **0**. No named fail beyond disconnected.

### Live ipc1 (SIM2) — GET-only

Static `/tmp/ps-p68` (`os/build/e4-ps-p68.c`, Zig musl **1108904**). mseq from **0xD0**. CP already **`ONLINE`**. Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p68.txt` **17524**. Leftover NOTI: DISP **0x0706** / **0x0701** only. **No STK ACK.**

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701**. **No STK ACK.** |
| GET `PHONE_STATE` | **0xD0** | **0x02** |
| GET `MODE_SEL` | **0xD1** | **`0x0b`** — **no SET** |
| GET `0x0808` / `0x0816` | **0xD2** / **0xD3** | **`02`** CS_PS; slot=**1** cause=0 — **no SET** |
| GET CS / PS / GPRS_PS | **0xD4–0xD6** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| GET `0x0D04` empty | **0xD7** | type-2 RESP FMT len **10** plen=**3** body **`01 18 00`**. **`0x18` still.** No GEN. |
| GET `0x0D10` | — | **not sent** (no `IpcTxGet*`) |
| GET final | **0xD8–0xDD** | MODE_SEL **`0x0b`**; **0x0808 `02`**; **0x0816 `01 00`**; CS **HOME**; PS **NONE GMM#7**; att=0 |

**No SET.** `sets=0`. `0x0D10` n=**0** this pass (no PDP SET → no NOTI).

### GEN

| cmd | GEN |
|--|--|
| any SET | **not sent** |

### Raw NET_REGIST / 0x0D04 / 0x0D10

FMT **n=27** plen=**20**, fail at byte **[17]** **present**. Unchanged baseline / final.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline / final CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline / final PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

GET **0x0D04** full: `0a 00 18 d7 0d 04 02 01 18 00` → body **`01 18 00`**.

p67 **0x0D10** full (not re-emitted): `11 00 dc 00 0d 10 03 01 03 00 00 00 00 00 00 00 00`.

### rmnet

| | modem | rmnet0–7 |
|--|--|--|
| pre / post | ONLINE | rx=tx=**0** (all eight) |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. CP stayed **`ONLINE`** (uptime **2401** s post-check). Holder **326** alive (`radio-boot`, ipc0+rfs0). ipc1-holder **314** untouched. GNSS **OFFLINE**. **Data-plane goal not complete.**

**`0x18` is not a named SM/ESM/GMM enum in this `.so`.** It is the raw reason on one **inactive** context. **st=`0x03` is disconnected.** Live PS is **NONE GMM#7** / **attached=0**. That is the not-attached / unspecified-reject class. **Do not PsAttach. Do not SET MODE_SEL 0x0a. Do not repeat PDP.** **There is no remaining safe SET on this boot** that historically moved PS.

**Do not SET MODE_SEL 0x04/0x07/0x0a/0x2f.** **Do not SET 0x0808 0x03.** **Do not SET 0x0D03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 69 — `0x0a` + PDP, **do not restore `0x0b`** (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 63 crashed ~50 s after CONT following **`0x0a` + PDP** then **restore `0x0b` + re-SET `0x0808=02`**. Probe 67 PDP under **`0x0b`** did **not** crash. Probe 60–61 **`0x0a` GET-only** did **not** crash. Hypothesis: crash was **restore/`0x0808` churn**, not PDP itself. This pass: SET MODE_SEL **`0x0a` only**, immediate vendor PDP, **leave `0x0a`**, **do not re-SET `0x0808`**. **No** `0x2f`. **No** PsAttach **0x0D03**. **No** MODE_SEL **0x04/0x07**. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **510** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8878** `/tmp/ps-p69` **1143784**.

### Binary

Reuse p63 MODE_SEL SET FMT `08 00 .. ff 08 0a 03 0a` and p67/p53 PDP pack (`0x0D1B` len **0xCB** `lte_internet`+**`www.vodafone.net.ua`**, `0x0D01` len **0x95**, `0x0D04` len **0xF8**). Helper **refuses** SET MODE_SEL except **`0x0a`** (no **`0x0b` restore**), **refuses** SET **0x0808** / **0x0816** / **0x0D03** / **0x2f**. Drain NOTIs **30 s**. Leave domain whatever GET says.

### Live ipc1 (SIM2)

Static `/tmp/ps-p69` (`os/build/e4-ps-p69.c`, Zig musl **1143784**). mseq from **0x10**. CP already **`ONLINE`** (same boot as probes 64–68). Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p69.txt` **78551**. Leftover NOTI: DISP **0x0706** / **0x0701**. **No STK ACK.** **No LCE 0x0D23** this pass.

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701**. **No STK ACK.** |
| GET `PHONE_STATE` | **0x10** | **0x02** |
| GET `MODE_SEL` | **0x11** | **`0x0b`** |
| GET `0x0808` / `0x0816` | **0x12** / **0x13** | **`02`** CS_PS; slot=**1** cause=0 — **no SET** |
| GET CS / PS / GPRS_PS | **0x14–0x16** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `MODE_SEL` **0x0a** | **0x17** | FMT `08 00 17 ff 08 0a 03 0a`. GEN **0x080A 0x8000** (ACK after GET below) |
| GET `MODE_SEL` after SET | **0x18** | **`0x0b`** (not stuck yet). fail_now=**0** st=**0x07** |
| SET `0x0D1B` 0xCB vodafone | **0x19** | **GEN 0x0D1B 0x8000**. **fail0_at_pdp=1** |
| SET `0x0D01` 0x95 | **0x1a** | **GEN 0x0D01 0x8000**. fail_now=**0** |
| SET `0x0D04` 0xF8 | **0x1b** | **GEN 0x0D04 0x8000**. **0x0D10** cid=**1** st=**0x03**. **No 0x0D09** |
| PDP drain **30 s** | **0x1c–0x33** | fail=0 until **~22 s**, then GMM **#7**. CS HOME UMTS (brief LTE NONE NOTI; **kept `0x0a`**). **No restore. No re-SET 0x0808.** |
| GET after drain | **0x34–0x39** | MODE_SEL **`0x0a`** (SET stuck). **0x0808 `01`** (drifted; **left**). PS **NONE GMM#7**; att=0 |

**No 0x0D03. No SET 0x2f. No restore 0x0b. No re-SET 0x0808. No SET 0x0816.**

### GEN

| cmd | GEN |
|--|--|
| SET `MODE_SEL` `0x0a` | **0x8000** SUCCESS |
| SET `0x0808` / `0x0816` | **not sent** |
| `0x0D1B` | **0x8000** SUCCESS |
| `0x0D01` | **0x8000** SUCCESS |
| `0x0D04` | **0x8000** SUCCESS |

### fail=0 vs PDP TX

Baseline was **GMM#7** under **`0x0b`**. The GET-MODE_SEL drain after SET (~3 s) opened the fail=0 / st=`0x07` window **before** `0x0D1B` (p63 fired `0x0D1B` while still **#7**).

| when | fail | note |
|--|--|--|
| baseline | **7** | MODE **`0x0b`** |
| GET MODE_SEL after SET `0x0a` | **0** | GET still **`0x0b`**; st=`0x07` |
| TX `0x0D1B` / `0x0D01` / `0x0D04` | **0** | **fail0_at_pdp=1** |
| drain t=0 … ~20 s | **0** | UMTS st=`0x07`, then NONE fail=0 |
| drain t≈22–24 s | **7** | GMM#7 back |
| helper exit | **7** | MODE **`0x0a`**; domain **`01`** |

fail=0 during drain **~22 s** — same class as p63’s **~24 s**. Still not Probe 43’s 110 s. **All three PDP SETs went out inside the window.** Still **0x0D10 st=0x03**, no **0x0D09**, no rmnet.

### 0x0D09 vs 0x0D10

| when | 0x0D10 | 0x0D09 |
|--|--|--|
| after SET `0x0D04` | cid=**1** st=**`0x03`** n=1 | **none** |
| samples 1–8 (t=0 … 28 s) | still 1/`0x03` n10=**1** | **n09=0** |
| after / final | same | **none** |

Full **0x0D10** (same class as p67):

```text
IPC RX (17): 11 00 44 00 0d 10 03 01 03 00 00 00 00 00 00 00 00
```

FMT len=**17** type=**3** plen=**10**. Body **`01 03` + 8 zeros**: cid=**1**, st=**`0x03`**, end-reason **`0x00`**.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |
| LTE NOTI after SET | **LTE 0x21** | **0x07 ?** (dom PS) | **0x00** | `21 03 07 00 00 00 48 37 42 06 00 c3 8d 02 01 00 7e 01 00 00` |
| drain fail=0 CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| drain fail=0 PS | **UMTS 0x04** | **0x07 ?** | **0x00** | `04 03 07 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| LTE CS NONE NOTI | **LTE 0x21** | **NONE 0x01** (dom CS) | **0x00** | `21 01 01 00 00 00 48 37 42 06 00 c3 8d 02 02 00 7e 01 00 00` — **kept 0x0a** |
| after PDP / final CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after PDP / final PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

`saw_fail0=1`. `fail0_held=0` (GMM#7 from drain t≈22 s). `saw_ps_home=0`. `did_pdp=1`. `restore0b=0`.

### rmnet + 90 s watch (no IPC SET)

| | modem | rmnet0–7 |
|--|--|--|
| pre / drain / helper exit | **ONLINE** | rx=tx=**0** (all eight) |
| watch t=0 (uptime **2837.60**) | **ONLINE** | rx=tx=**0** |
| watch t≈47 s (**2884.94**) | **ONLINE** | rx=tx=**0** |
| watch t≈92 s (**2929.68**) | **ONLINE** | rx=tx=**0** |
| watch t≈121 s (**2958.26**) | **ONLINE** | rx=tx=**0** |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. Helper `after modem_state=ONLINE` last_tx=**GPRS_PS GET mseq=0x39**. Last SET was **`0x0D04`** mseq **0x1b** (not restore / not `0x0808`). Holder **326** alive (`state=R`, fds **5=`umts_ipc0`** **6=`umts_rfs0`**). ipc1-holder **314** untouched (`sh`, fd **3=`umts_ipc1`**). GNSS **OFFLINE**. dmesg after watch: **0** `CRASH_EXIT`. **0** `CP_CRASH`. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

### Crash vs restore

p63: helper exited **ONLINE**, then **restore `0x0b` + re-SET `0x0808=02`**, CONT, **CRASH_EXIT ~50 s later**. This pass: **same `0x0a`+PDP**, **no restore**, **no re-SET `0x0808`**. Watch **>90 s** (past the p63 50 s window) stayed **`ONLINE`**. Hypothesis **supported**: crash was restore/`0x0808` churn after `0x0a`+PDP, not PDP itself.

**Data-plane goal not complete.** SET **`0x0a`** ACKs and eventually sticks (immediate GET still **`0x0b`**; GET after drain **`0x0a`**). Immediate PDP GENs **8000** inside fail=0. **0x0D10 st=0x03** again. Domain drifted **`02`→`01`** and was **left**. **Do not restore `0x0b` this boot.** **Do not re-SET `0x0808`.** **Do not SET 0x0D03.** **Do not SET 0x2f.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 70 — domain `02` then `0x0a` then PDP, no restore (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Probe 69: SET **`0x0a`** + vodafone PDP, **no restore `0x0b`**, **no re-SET `0x0808`**. CP stayed **ONLINE**. **fail0_at_pdp=1**. **0x0D10 st=`0x03`**. **No 0x0D09**. **0x0808** drifted **`02`→`01`** and was left — PDP likely ran as **CS-only**. Hypothesis: p69 failed because **CS_PS reverted before/during PDP**. This pass: SET **`0x0808` `02`** (p66 pack), GET immediately (must **`02`**), SET MODE_SEL **`0x0a`**, **no 3 s GET-wait** (p69 spent ~3 s and domain drifted), optional short GET **0x0808** (re-SET **`02`** only if not stuck), **immediate** vendor PDP. **No** restore **`0x0b`**. **No** re-SET **`0x0808` after PDP**. **No** `0x2f`. **No** PsAttach **0x0D03**. **No** MODE_SEL **0x04/0x07**. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **542** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8879** `/tmp/ps-p70` **1153752**.

### Binary

Reuse p66 SET **`0x0808` `02`** (never **`0x03`**) and p69 MODE_SEL SET FMT `08 00 .. ff 08 0a 03 0a` + p67/p53 PDP pack (`0x0D1B` len **0xCB** `lte_internet`+**`www.vodafone.net.ua`**, `0x0D01` len **0x95**, `0x0D04` len **0xF8**). If DDS slot != 1: SET **`0x0816` `01 01`** **before** domain. Helper **refuses** SET MODE_SEL except **`0x0a`** (no **`0x0b` restore**), **refuses** SET **0x0808** except **`02`**, **refuses** SET **0x0808 after PDP**, **refuses** **0x0D03** / **0x2f** / **0x03**. Drain NOTIs **30 s**. GET **0x0808** during/after drain. Leave domain whatever GET says after PDP.

### Live ipc1 (SIM2)

Static `/tmp/ps-p70` (`os/build/e4-ps-p70.c`, Zig musl **1153752**). mseq from **0x70**. CP already **`ONLINE`** (same boot as probes 64–69). Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p70.txt` **77436**. Leftover NOTI: DISP **0x0706** / **0x0701**. **No STK ACK.** **No LCE 0x0D23** this pass.

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701**. **No STK ACK.** |
| GET `PHONE_STATE` | **0x70** | **0x02** |
| GET `MODE_SEL` | **0x71** | **`0x0a`** (left from p69) |
| GET `0x0808` / `0x0816` | **0x72** / **0x73** | **`01`** CS-only; slot=**1** cause=0 — **skip DDS SET** |
| GET CS / PS / GPRS_PS | **0x74–0x76** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `0x0808` **`02`** | **0x77** | FMT `08 00 77 ff 08 08 03 02`. GEN **0x0808 0x8000** |
| GET `0x0808` immediately | **0x78** | **`02`** CS_PS — **stuck**. CS still HOME. **No MODE_SEL restore.** |
| SET `MODE_SEL` **0x0a** | **0x79** | FMT `08 00 79 ff 08 0a 03 0a`. GEN **0x080A 0x8000**. **No 3 s GET-wait.** |
| GET `0x0808` short | **0x7a** | **`02`** — **no retry SET** |
| SET `0x0D1B` 0xCB vodafone | **0x7b** | **GEN 0x0D1B 0x8000**. **0x0808=02**. **fail0_at_pdp=0** |
| SET `0x0D01` 0x95 | **0x7c** | **GEN 0x0D01 0x8000**. **0x0808=02**. fail_now=**7** |
| SET `0x0D04` 0xF8 | **0x7d** | **GEN 0x0D04 0x8000**. **0x0D10** cid=**1** st=**0x03**. **No 0x0D09**. **0x0808=02** |
| PDP drain **30 s** | **0x7e–0x9d** | **GMM#7** entire drain. **0x0808 `02`** all 8 samples. CS HOME UMTS. **No restore. No re-SET 0x0808.** |
| GET after drain | **0x9e–0xa3** | MODE_SEL **`0x0a`**. **0x0808 `02`** (**still stuck; left**). PS **NONE GMM#7**; att=0 |

**No 0x0D03. No SET 0x2f. No restore 0x0b. No re-SET 0x0808 after PDP. No SET 0x0816.**

### GEN

| cmd | GEN |
|--|--|
| SET `0x0808` `02` | **0x8000** SUCCESS |
| SET `MODE_SEL` `0x0a` | **0x8000** SUCCESS |
| SET `0x0816` | **not sent** (slot already 1) |
| `0x0D1B` | **0x8000** SUCCESS |
| `0x0D01` | **0x8000** SUCCESS |
| `0x0D04` | **0x8000** SUCCESS |

### 0x0808 at PDP TX vs after

| when | 0x0808 |
|--|--|
| baseline | **`01`** CS-only (p69 drift, left) |
| immediate GET after SET `02` | **`02`** |
| short GET before PDP | **`02`** (no retry) |
| TX `0x0D1B` / `0x0D01` / `0x0D04` | **`02`** |
| drain samples 1–8 | **`02`** |
| after drain / final | **`02`** (**left**; no re-SET) |

Domain **did not drift**. p69’s CS-only theory is addressed: PDP ran under **CS_PS=02**. Still **0x0D10 st=0x03**, no **0x0D09**, no rmnet.

### fail=0 vs PDP TX

Skipping the ~3 s GET-MODE_SEL wait meant **no fail=0 window**. Baseline and every drain sample stayed **GMM#7**.

| when | fail | 0x0808 | note |
|--|--|--|--|
| baseline | **7** | **`01`** | MODE **`0x0a`** |
| SET `02` + immediate GET | **7** | **`02`** | CS HOME |
| short GET / TX PDP | **7** | **`02`** | **fail0_at_pdp=0** |
| drain t=0 … 30 s | **7** | **`02`** | no fail=0 at all |
| helper exit | **7** | **`02`** | MODE **`0x0a`** |

`saw_fail0=0`. `fail0_held=0`. `saw_ps_home=0`. `did_pdp=1`. `set0808_retry=0`. `restore0b=0`.

### 0x0D09 vs 0x0D10

| when | 0x0D10 | 0x0D09 |
|--|--|--|
| after SET `0x0D04` | cid=**1** st=**`0x03`** n=1 | **none** |
| samples 1–8 (t=0 … 28 s) | still 1/`0x03` n10=**1** | **n09=0** |
| after / final | same | **none** |

Full **0x0D10** (same class as p67/p69):

```text
IPC RX (17): 11 00 aa 00 0d 10 03 01 03 00 00 00 00 00 00 00 00
```

FMT len=**17** type=**3** plen=**10**. Body **`01 03` + 8 zeros**: cid=**1**, st=**`0x03`**, end-reason **`0x00`**.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**. Unchanged from baseline through drain (no LTE NOTI / no fail=0 body this pass).

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline / drain / final CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline / drain / final PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet + 60 s watch (no IPC SET)

| | modem | rmnet0–7 |
|--|--|--|
| pre / drain / helper exit | **ONLINE** | rx=tx=**0** (all eight) |
| watch t=0 (uptime **3447.52**) | **ONLINE** | rx=tx=**0** |
| watch t≈20 s (**3467.54**) | **ONLINE** | rx=tx=**0** |
| watch t≈40 s (**3487.56**) | **ONLINE** | — |
| watch t≈60 s (**3507.57**) | **ONLINE** | rx=tx=**0** |
| post-check (**3524.09**) | **ONLINE** | rx=tx=**0** |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. Helper `after modem_state=ONLINE` last_tx=**GPRS_PS GET mseq=0xa3**. Last SET was **`0x0D04`** mseq **0x7d** (not restore / not re-SET `0x0808`). Holder **326** alive (`state=R`, fds **5=`umts_ipc0`** **6=`umts_rfs0`**). ipc1-holder **314** untouched (`sh`, fd **3=`umts_ipc1`**). GNSS **OFFLINE**. dmesg after watch: **0** `CRASH_EXIT`. **0** `CP_CRASH`. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

### Domain-at-PDP vs fail=0

p69: **fail0_at_pdp=1**, domain drifted to **`01`** during the ~3 s GET-wait, **0x0D10 st=0x03**. This pass: **0x0808 `02` stuck** at TX and after, **fail0_at_pdp=0**, same **0x0D10 st=0x03**. **CS_PS at PDP is not sufficient.** Skipping the GET-wait closed the fail=0 window that p69 opened.

**Data-plane goal not complete.** SET **`02`** ACKs and **sticks** through PDP + 30 s drain when there is no 3 s wait. SET **`0x0a`** already stuck from p69; re-SET ACK **8000**. Immediate PDP GENs **8000** under GMM#7. **0x0D10 st=0x03** again. Domain **left `02`**. MODE_SEL **left `0x0a`**. **Do not restore `0x0b` this boot.** **Do not re-SET `0x0808` after PDP.** **Do not SET 0x0D03.** **Do not SET 0x2f.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 71 — wait fail=0, keep/re-SET `02`, then PDP (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Split so far: p69 = **fail=0** + domain **`01`**. p70 = domain **`02`** + **GMM#7**. Never both at PDP TX. This pass: poll CS/PS + GET **0x0808** every ~0.7 s for **~15 s**; on **fail=0** (or st=`0x07` fail=0) SET **`02`** if needed and PDP **same breath**. If 15 s no fail=0: SET **`0x0a`** once more, poll **~12 s**. **Prefer abort** if never fail=0 (do not waste another GMM#7 PDP like p70). **No** restore **`0x0b`**. **No** re-SET **`0x0808` after PDP**. **No** `0x2f`. **No** PsAttach **0x0D03**. **No** MODE_SEL **0x04/0x07**. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **633** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8880** `/tmp/ps-p71` **1159008**.

### Binary

Copy p69 wait + p70 keep-`02` / vendor PDP pack. Helper **refuses** SET MODE_SEL except **`0x0a`** (no **`0x0b` restore**), **refuses** SET **0x0808** except **`02`**, **refuses** SET **0x0808 after PDP**, **refuses** **0x0D03** / **0x2f** / **0x03**. If MODE already **`0x0a`** / domain already **`02`**: skip those SETs at baseline. Poll does **GET-only** of **0x0808** until fail=0.

### Live ipc1 (SIM2)

Static `/tmp/ps-p71` (`os/build/e4-ps-p71.c`, Zig musl **1159008**). mseq from **0x71**. CP already **`ONLINE`** (same boot as probes 64–70). Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p71.txt` **123155**. Leftover NOTI: DISP **0x0706** / **0x0701**. **No STK ACK.** **No LCE 0x0D23.** **No PDP.**

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701**. **No STK ACK.** |
| GET `PHONE_STATE` | **0x71** | **0x02** |
| GET `MODE_SEL` | **0x72** | **`0x0a`** (left from p69/p70) — **skip SET** |
| GET `0x0808` / `0x0816` | **0x73** / **0x74** | **`02`** CS_PS; slot=**1** cause=0 — **skip DDS SET**; **skip SET `02`** |
| GET CS / PS / GPRS_PS | **0x75–0x77** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| fail0 poll wait1 **15 s** | **0x78–0xcf** | **40** samples across both waits. Wait1 **22** samples **GMM#7**, **0x0808 `02`**, CS HOME. **saw_fail0=0** |
| SET `MODE_SEL` **0x0a** retry | **0xd0** | FMT `08 00 d0 ff 08 0a 03 0a`. GEN **0x080A 0x8000**. **No 0x2f.** |
| fail0 poll wait2 **12 s** | **0xd1–0x18** | **18** samples **GMM#7**, **0x0808 `02`**. **saw_fail0=0** |
| **ABORT PDP** | — | never fail=0 after 15+12 s. **pdp=0**. **fail0_at_pdp=0**. **0x0808_at_pdp_tx=n/a** |
| GET after abort | **0x19–0x1e** | MODE_SEL **`0x0a`**. **0x0808 `02`** (**left**). PS **NONE GMM#7**; att=0 |

**No 0x0D1B / 0x0D01 / 0x0D04. No 0x0D03. No SET 0x2f. No restore 0x0b. No SET 0x0808 this pass. No SET 0x0816.**

### GEN

| cmd | GEN |
|--|--|
| SET `0x0808` `02` | **not sent** (already **`02`**) |
| SET `MODE_SEL` `0x0a` retry | **0x8000** SUCCESS |
| SET `0x0816` | **not sent** (slot already 1) |
| `0x0D1B` / `0x0D01` / `0x0D04` | **not sent** (aborted) |

### 0x0808 at PDP TX vs after

| when | 0x0808 |
|--|--|
| baseline | **`02`** (p70 stuck; left) |
| wait1 / wait2 samples | **`02`** every GET |
| abort / after abort / final | **`02`** (**left**; no re-SET) |
| TX `0x0D1B` | **n/a** (no PDP) |

Domain **did not drift**. Re-SET **`0x0a`** from already-**`0x0a`** did **not** reopen the p61 ~11 s / p69 ~3 s fail=0 window.

### fail=0 vs PDP TX

| when | fail | 0x0808 | note |
|--|--|--|--|
| baseline | **7** | **`02`** | MODE **`0x0a`** — skip both SETs |
| wait1 t=0 … 15.4 s | **7** | **`02`** | 22 samples |
| SET `0x0a` retry | **7** | **`02`** | GEN **8000** |
| wait2 t=0 … 12.6 s | **7** | **`02`** | 18 samples |
| abort / helper exit | **7** | **`02`** | **fail0_at_pdp=0** **aborted=1** **pdp=0** |

`saw_fail0=0`. `fail0_held=0`. `saw_ps_home=0`. `did_pdp=0`. `set0a=1` `set0a_retry=1`. `set0808=0`. `restore0b=0`. `poll_n=40`. `fail0_t=-1`.

### 0x0D09 vs 0x0D10

| when | 0x0D10 | 0x0D09 |
|--|--|--|
| whole probe | **none** (n10=0) | **none** (n09=0) |

No vendor PDP, so no call-status / IP-config NOTIs.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**. Unchanged GMM#7 the whole poll (no LTE NOTI / no fail=0 body). CS LAC nibble moved vs p70 (`01 13` → `fe 12`); still HOME UMTS fail=0.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline / poll / abort CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d fe 12 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline / poll / abort PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d fe 12 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet + 60 s watch (no IPC SET)

| | modem | rmnet0–7 |
|--|--|--|
| pre / poll / helper exit | **ONLINE** | rx=tx=**0** (all eight) |
| watch t=0 (uptime **4068.71**) | **ONLINE** | rx=tx=**0** |
| watch t≈20 s (**4088.73**) | **ONLINE** | — |
| watch t≈40 s (**4108.75**) | **ONLINE** | — |
| watch t≈60 s (**4128.76**) | **ONLINE** | rx=tx=**0** |
| post-check (**4158.19**) | **ONLINE** | rx=tx=**0** |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. Helper `after modem_state=ONLINE` last_tx=**GPRS_PS GET mseq=0x1e**. Last SET was **`MODE_SEL 0x0a`** mseq **0xd0** (not restore / not `0x0808`). Holder **326** alive (`state=S`, fds **5=`umts_ipc0`** **6=`umts_rfs0`**). ipc1-holder **314** untouched (`sh`, fd **3=`umts_ipc1`**). GNSS **OFFLINE**. dmesg after watch: **0** `CRASH_EXIT`. **0** `CP_CRASH`. **Did not** `POWER_OFF`. **Did not** leftover `loadnv` (326 is the original holder).

### fail=0 + `02` still never together at PDP

p69: **fail0_at_pdp=1**, domain **`01`**, **0x0D10 st=0x03**. p70: **0x0808 `02`**, **fail0_at_pdp=0**, same **0x0D10 st=0x03**. This pass **waited** for fail=0 while **keeping `02`**, then **aborted** rather than fire another GMM#7 PDP. Re-SET **`0x0a`** from already-stuck **`0x0a`** did **not** recreate the fail=0 window (same class as p42 re-SET **0x2f** from already-folded **0x0b**).

**Data-plane goal not complete.** **0x0808 `02`** stuck through both polls. SET **`0x0a`** retry ACK **8000**. **No PDP.** Domain **left `02`**. MODE_SEL **left `0x0a`**. **Do not restore `0x0b` this boot.** **Do not re-SET `0x0808` after PDP.** **Do not SET 0x0D03.** **Do not SET 0x2f.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 72 — SET `0x0b` first, watch 60 s, then `0x0a` + fail=0 PDP (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. p71: re-SET **`0x0a`** from already-**`0x0a`** never opened fail=0. Fact: fail=0 only after SET **`0x0a`** from GET **`0x0b`** (p60/p69). This pass **must** SET **`0x0b`** first to reopen that window. p63 CRASH_EXIT ~50 s after restore **`0x0b`** + re-SET **`0x0808`** following **`0x0a`+PDP** — crash risk; watch **~60 s after `0x0b` before PDP**. If still ONLINE: SET **`0x0808` `02`** if needed, SET **`0x0a`**, poll fail=0 ~20 s, PDP immediately on fail=0 (or st=`0x07` fail=0). **No** restore **`0x0b` after PDP**. **No** re-SET **`0x0808` after PDP**. **No** `0x2f`. **No** `0x04`/`0x07`. **No** PsAttach **0x0D03**. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **721** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8881** `/tmp/ps-p72` **1172024**.

### Binary

Copy p71 wait + vendor PDP pack. Helper **allows** SET MODE_SEL **`0x0b` only as reopen** (before **`0x0a`/PDP**), **refuses** restore **`0x0b` after `0x0a`/PDP**, **refuses** SET **0x0808** except **`02`**, **refuses** SET **0x0808 after PDP**, **refuses** **0x0D03** / **0x2f** / **0x04** / **0x07** / **0x03**. After SET **`0x0b`** + GET: **CONT 326**, watch `modem_state` **~60 s**. If CRASH: stop, no further SET. If ONLINE: STOP 326 again, SET **`02`** if GET is not **`02`**, SET **`0x0a`**, poll ~20 s, PDP on fail=0.

### Live ipc1 (SIM2)

Static `/tmp/ps-p72` (`os/build/e4-ps-p72.c`, Zig musl **1172024**). mseq from **0x72**. CP already **`ONLINE`** (same boot as probes 64–71). Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p72.txt` **107747**. Leftover NOTI: DISP **0x0706** / **0x0701**. **No STK ACK.** **No LCE 0x0D23.**

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701**. **No STK ACK.** |
| GET `PHONE_STATE` | **0x72** | **0x02** |
| GET `MODE_SEL` | **0x73** | **`0x0a`** (left from p69–p71) |
| GET `0x0808` / `0x0816` | **0x74** / **0x75** | **`02`** CS_PS; slot=**1** cause=0 — **skip DDS SET** |
| GET CS / PS / GPRS_PS | **0x76–0x78** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `MODE_SEL` **0x0b** | **0x79** | FMT `08 00 79 ff 08 0a 03 0b`. Immediate GET **still `0x0a`**. GEN **0x080A 8000** arrived **late** (>1.5 s; NOTI flood). `GEN_080a_0b=0xffff` in helper slot. **fail_now=0** |
| CONT **326** + watch **60 s** | — | **ONLINE** all 13 samples. **crash_after_0b=0**. fail=0 / ST07 until **~20 s**, GMM **#7** from **~23 s** |
| GET after watch | **0x7e–0x83** | MODE_SEL **`0x0b`** (SET **stuck late**). **0x0808 `01`** (drifted). PS **NONE GMM#7** |
| SET `0x0808` **02** | **0x84** | **GEN 0x0808 0x8000**. Immediate GET **`02`** |
| SET `MODE_SEL` **0x0a** | **0x86** | FMT `08 00 86 ff 08 0a 03 0a`. **GEN 0x080A 0x8000**. GET **`0x0a`**. **fail=0** immediately (LTE ST07) — **no poll** |
| SET `0x0D1B` / `0x0D01` / `0x0D04` | **0x88–0x8a** | all GEN **8000**. **fail0_at_pdp=1**. **0x0808_at_pdp_tx=`02`**. **0x0D10** cid=**1** st=**0x03**. **No 0x0D09** |
| PDP drain **30 s** | **0x8b–0xaa** | fail=0 until **~20 s**, GMM **#7** from **~22 s**. Domain **`02`→`01`** by t≈4 s. **No restore. No re-SET 0x0808.** |
| GET after drain | **0xab–0xb0** | MODE_SEL **`0x0a`**. **0x0808 `01`** (**left**). PS **NONE GMM#7**; att=0 |

**No 0x0D03. No SET 0x2f. No restore 0x0b after PDP. No SET 0x0816.**

### Did `0x0b` crash?

**No.** Helper watch after SET **`0x0b`**: **ONLINE** t=0…60 s (13 samples). CONT **326** during watch (`state=R`). Then STOP again for **`0x0a`+PDP**. Post-PDP host watch **~60 s** (uptime **4840.42→4900.47**) still **ONLINE**. dmesg: **0** `CRASH_EXIT`. **0** `CP_CRASH`.

SET **`0x0b`** from already-**`0x0a`** ACKs **8000** but GET is **slow to stick**: immediate GET **`0x0a`**, after 60 s GET **`0x0b`**. Domain drifted **`02`→`01`** during that watch (same class as p60 restore **`0x0b`**).

### GEN

| cmd | GEN |
|--|--|
| SET `MODE_SEL` `0x0b` | **0x8000** SUCCESS (late; helper slot **ffff**) |
| SET `0x0808` `02` | **0x8000** SUCCESS |
| SET `MODE_SEL` `0x0a` | **0x8000** SUCCESS |
| `0x0D1B` / `0x0D01` / `0x0D04` | **0x8000** / **0x8000** / **0x8000** |
| SET `0x0816` | **not sent** (slot already 1) |

### 0x0808 at PDP TX vs after

| when | 0x0808 |
|--|--|
| baseline | **`02`** |
| after SET `0x0b` (immediate) | **`02`** |
| after 60 s `0x0b` watch | **`01`** (drifted) |
| SET `02` + GET | **`02`** (stuck) |
| TX `0x0D1B` | **`02`** (**0x0808_at_pdp_tx**) |
| GEN `0x0D04` | **`02`** |
| PDP drain t≈4 s … end | **`01`** (drifted; **left**) |
| after drain / final | **`01`** (**left**; no re-SET) |

### fail=0 vs PDP TX

| when | fail | 0x0808 | note |
|--|--|--|--|
| baseline | **7** | **`02`** | MODE **`0x0a`** |
| after SET `0x0b` | **0** | **`02`** | window opened; GET MODE still **`0x0a`** |
| watch t=0 … ~20 s | **0** | **`02`** (stale) | ST07 then NONE |
| watch t≈23 … 60 s | **7** | — | GMM#7 back |
| after watch GET | **7** | **`01`** | MODE now **`0x0b`** |
| SET `02` + SET `0x0a` | **0** | **`02`** | LTE ST07 — **PDP same breath** |
| TX `0x0D1B` | **0** | **`02`** | **fail0_at_pdp=1** **both** |
| drain t=0 … ~20 s | **0** | **`01`** from t≈4 s | ST07 then NONE |
| drain t≈22 s / helper exit | **7** | **`01`** | GMM#7 back |

`saw_fail0=1`. `fail0_held=0`. `saw_ps_home=0`. `did_pdp=1`. `aborted=0`. `set0b=1` `crash_after_0b=0` `set0a=1` `set0808=1`. `poll_n=0` `fail0_t=-1` (no 20 s poll; fail=0 on SET **`0x0a`** GET).

**First pass with both fail=0 and `02` at `0x0D1B`.** Still **0x0D10 st=0x03**, no **0x0D09**, no rmnet.

### 0x0D09 vs 0x0D10

| when | 0x0D10 | 0x0D09 |
|--|--|--|
| GEN `0x0D04` / drain / final | cid=**1** st=**0x03** n10=**1** | **none** (n09=0) |

Same disconnected call-status as p63/p67/p69/p70. No IP-config NOTI.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d fe 12 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d fe 12 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |
| after SET 0x0b PS NOTI | **LTE 0x21** | **0x07 ?** | **0x00** | `21 03 07 00 00 00 48 37 42 06 00 c3 8d 02 01 00 7e 01 00 00` |
| watch fail=0 PS GET | **UMTS 0x04** | **0x07 ?** | **0x00** | `04 03 07 00 00 00 fe 12 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| after watch / after PDP CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d fe 12 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after watch / after PDP PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d fe 12 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet + watches (no IPC SET after helper)

| | modem | rmnet0–7 |
|--|--|--|
| pre / 0x0b watch / PDP drain / helper exit | **ONLINE** | rx=tx=**0** (all eight) |
| 0x0b helper watch t=0…60 s | **ONLINE** | rx=tx=**0** |
| post-PDP watch t=0 (uptime **4840.42**) | **ONLINE** | rx=tx=**0** |
| post-PDP t≈20 s (**4860.44**) | **ONLINE** | — |
| post-PDP t≈40 s (**4880.45**) | **ONLINE** | — |
| post-PDP t≈60 s (**4900.47**) | **ONLINE** | rx=tx=**0** |
| post-check (**4939.28**) | **ONLINE** | rx=tx=**0** |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. Helper `after modem_state=ONLINE` last_tx=**GPRS_PS GET mseq=0xb0**. Last SET was **`0x0D04`** mseq **0x8a** (not restore / not re-SET `0x0808`). Holder **326** alive (`state=S`, fds **5=`umts_ipc0`** **6=`umts_rfs0`**). ipc1-holder **314** untouched (`sh`, fd **3=`umts_ipc1`**). GNSS **OFFLINE**. dmesg after watch: **0** `CRASH_EXIT`. **0** `CP_CRASH`. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

### fail=0 + `02` together at PDP — still no bearer

p69: **fail0_at_pdp=1**, domain **`01`**, **0x0D10 st=0x03**. p70: **0x0808 `02`**, **fail0_at_pdp=0**, same **0x0D10 st=0x03**. p71: never fail=0, aborted. This pass: **both** at TX **`0x0D1B`**. Same **0x0D10 st=0x03**, no **0x0D09**. Domain drifted **`02`→`01`** within ~4 s of PDP (p69 class) — **left**. SET **`0x0b`** from **`0x0a`** did **not** crash this boot.

**Data-plane goal not complete.** SET **`0x0b`** ACK **8000**, sticks late, opens a ~20 s fail=0 window, then GMM#7. SET **`0x0a`** from that **`0x0b`** reopens fail=0 immediately. Vendor PDP GENs **8000** under **fail=0 + `02`**. **0x0D10 st=0x03** again. MODE_SEL **left `0x0a`**. **0x0808 left `01`**. **Do not restore `0x0b` after this PDP.** **Do not re-SET `0x0808` after PDP.** **Do not SET 0x0D03.** **Do not SET 0x2f.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 73 — recreate window, `0x0D03` then PDP (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. p72: fail=0 + CS_PS at vendor PDP still **0x0D10 st=0x03** / no **0x0D09**. Historical **PsAttach `0x0D03` `{01 00 00}`** ACK’d **8000** then **attached=0** and **reintroduced GMM#7** — that was **not** in the **`0x0b`→`0x0a`** fail=0 window with domain **`02`** at TX. This pass recreates that window (SET **`0x0b`**, SET **`02`**, SET **`0x0a`**), then **`IpcTxPsAttach`** on the same fd, then PDP if still attached=1 or fail=0. Watch after **`0x0b`** **~20 s** (p72 60 s was clean). **No** restore **`0x0b` after attach/PDP**. **No** re-SET **`0x0808` after attach/PDP**. **No** `0x2f`. **No** `0x04`/`0x07`. **No** `0x0808` **`0x03`**. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **825** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8882** `/tmp/ps-p73` **1182984**.

### Binary

Copy p72 window + vendor PDP pack. Helper **allows** SET **0x0D03** only **`01 00 00`** (FMT len 10, `IpcTxPsAttach`). **Refuses** restore **`0x0b` after `0x0a`/attach/PDP**, **refuses** SET **0x0808** except **`02`**, **refuses** SET **0x0808 after attach/PDP**, **refuses** **0x2f** / **0x04** / **0x07** / **0x03**. After SET **`0x0b`** + GET: **CONT 326**, watch `modem_state` **~20 s**. If CRASH: stop, no further SET. If ONLINE: STOP 326, SET **`02`** if needed, SET **`0x0a`**, poll fail=0 ~15 s (re-SET **`02`** if drifted), then attach, then PDP.

### Live ipc1 (SIM2)

Static `/tmp/ps-p73` (`os/build/e4-ps-p73.c`, Zig musl **1182984**). mseq from **0x73**. CP already **`ONLINE`** (same boot as probes 64–72). Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p73.txt` **114923**. Leftover NOTI: DISP **0x0706** / **0x0701**. **No STK ACK.** **No LCE 0x0D23.**

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701**. **No STK ACK.** |
| GET `PHONE_STATE` | **0x73** | **0x02** |
| GET `MODE_SEL` | **0x74** | **`0x0a`** (left from p69–p72) |
| GET `0x0808` / `0x0816` | **0x75** / **0x76** | **`01`** CS; slot=**1** cause=0 — **skip DDS SET** |
| GET CS / PS / GPRS_PS | **0x77–0x79** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `MODE_SEL` **0x0b** | **0x7a** | FMT `08 00 7a ff 08 0a 03 0b`. Immediate GET **still `0x0a`**. GEN **0x080A 8000** arrived **late** (`GEN_080a_0b=0xffff`). **fail_now=0** |
| CONT **326** + watch **20 s** | — | **ONLINE** all 5 samples. **crash_after_0b=0**. **fail=0** the whole watch (ST07) |
| GET after watch | **0x7f–0x84** | MODE_SEL **`0x0b`** (SET **stuck late**). **0x0808 `01`**. PS **ST07 fail=0** (window still open; p72 60 s watch had lost it) |
| SET `0x0808` **02** | **0x85** | **GEN 0x0808 0x8000**. Immediate GET **`02`**. CS briefly **NONE** then **HOME** (`cs_left_home=1`; no restore) |
| SET `MODE_SEL` **0x0a** | **0x87** | FMT `08 00 87 ff 08 0a 03 0a`. **GEN 0x080A 0x8000**. GET **still `0x0b`**. **fail=0** immediately (LTE ST07) — **no poll** |
| SET `GPRS_PS` **`01 00 00`** | **0x89** | FMT `0a 00 89 ff 0d 03 03 01 00 00`. **GEN 0x0D03 0x8000**. **fail0_at_attach=1**. **0x0808_at_attach_tx=`02`**. GET **cid=0 attached=0**. fail **still 0**. Domain **`02`→`01`** |
| SET `0x0D1B` / `0x0D01` / `0x0D04` | **0x8f–0x91** | all GEN **8000**. **fail0_at_pdp=1**. **0x0808_at_pdp_tx=`01`** (**not** re-SET). **0x0D10** cid=**1** st=**0x03**. **No 0x0D09** |
| PDP drain **30 s** | **0x92–0xb1** | fail=0 until **~16 s**, GMM **#7** from **~18 s**. Domain **`01`** (left). **No restore. No re-SET 0x0808.** |
| GET after drain | **0xb2–0xb7** | MODE_SEL **`0x0a`**. **0x0808 `01`** (**left**). PS **NONE GMM#7**; att=0 |

**No SET 0x2f. No restore 0x0b after attach/PDP. No SET 0x0816. No re-SET 0x0808 after attach.**

### Did `0x0b` crash?

**No.** Helper watch after SET **`0x0b`**: **ONLINE** t=0…20 s (5 samples). CONT **326** during watch (`state=R`). Then STOP again for **`0x0a`+attach+PDP**. Post-probe host watch **~60 s** (uptime **5845.36→5905.41**) still **ONLINE**. dmesg: **0** `CRASH_EXIT`. **0** `CP_CRASH`.

SET **`0x0b`** from already-**`0x0a`** ACKs **8000** but GET is **slow to stick**: immediate GET **`0x0a`**, after 20 s GET **`0x0b`**. Shorter watch kept **fail=0** (p72’s 60 s watch saw GMM#7 from ~23 s).

### Did attach immediately reintroduce GMM#7?

**No** (unlike Probe 38). **GEN 0x0D03 8000**. GET **attached=0**. fail stayed **0** through attach GET and ~16 s of PDP drain. Domain dropped **`02`→`01`** on the attach GET (left; no re-SET). Historical attach that reintroduced GMM#7 was **not** in this **`0x0b`→`0x0a`** window.

### GEN

| cmd | GEN |
|--|--|
| SET `MODE_SEL` `0x0b` | **0x8000** SUCCESS (late; helper slot **ffff**) |
| SET `0x0808` `02` | **0x8000** SUCCESS |
| SET `MODE_SEL` `0x0a` | **0x8000** SUCCESS |
| SET `0x0D03` `01 00 00` | **0x8000** SUCCESS |
| `0x0D1B` / `0x0D01` / `0x0D04` | **0x8000** / **0x8000** / **0x8000** |
| SET `0x0816` | **not sent** (slot already 1) |

### 0x0808 at attach TX vs PDP TX vs after

| when | 0x0808 |
|--|--|
| baseline | **`01`** |
| after SET `0x0b` (immediate / 20 s watch) | **`01`** |
| SET `02` + GET | **`02`** (stuck) |
| TX `0x0D03` | **`02`** (**0x0808_at_attach_tx**) |
| after attach GET | **`01`** (drifted; **left**) |
| TX `0x0D1B` | **`01`** (**0x0808_at_pdp_tx**) |
| PDP drain / after drain / final | **`01`** (**left**; no re-SET) |

### fail=0 vs attach TX vs PDP TX

| when | fail | 0x0808 | note |
|--|--|--|--|
| baseline | **7** | **`01`** | MODE **`0x0a`** |
| after SET `0x0b` | **0** | **`01`** | window opened; GET MODE still **`0x0a`** |
| watch t=0 … 20 s | **0** | **`01`** | ST07; **held** (p72 lost it by ~23 s) |
| after watch GET | **0** | **`01`** | MODE now **`0x0b`** |
| SET `02` + SET `0x0a` | **0** | **`02`** | LTE ST07 — **attach same breath** |
| TX `0x0D03` | **0** | **`02`** | **fail0_at_attach=1** **both** |
| after attach GET | **0** | **`01`** | **attached=0**; attach did **not** kill fail=0 |
| TX `0x0D1B` | **0** | **`01`** | **fail0_at_pdp=1** |
| drain t=0 … ~16 s | **0** | **`01`** | ST07 then NONE |
| drain t≈18 s / helper exit | **7** | **`01`** | GMM#7 back |

`saw_fail0=1`. `fail0_held=0`. `saw_ps_home=0`. `did_attach=1`. `attach_killed=0`. `did_pdp=1`. `aborted=0`. `set0b=1` `crash_after_0b=0` `set0a=1` `set0808=1`. `poll_n=0` `fail0_t=-1` (no 15 s poll; fail=0 on SET **`0x0a`**).

**First pass with fail=0 + `02` at attach TX.** Attach ACK **8000**, **attached=0**, fail stayed **0**. PDP then ran under fail=0 + drifted **`01`**. Still **0x0D10 st=0x03**, no **0x0D09**, no rmnet.

### 0x0D09 vs 0x0D10

| when | 0x0D10 | 0x0D09 |
|--|--|--|
| GEN `0x0D04` / drain / final | cid=**1** st=**0x03** n10=**1** | **none** (n09=0) |

Same disconnected call-status as p63/p67/p69/p70/p72. No IP-config NOTI. Leftover GPRS **0x0D19** NOTI after attach (`00 0a 00 00`); no ACK.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |
| attach TX PS | **LTE 0x21** | **0x07 ?** | **0x00** | `21 03 07 00 00 00 48 37 42 06 00 c3 8d 02 01 00 7e 01 00 00` |
| after attach PS GET | **UMTS 0x04** | **0x07 ?** | **0x00** | `04 03 07 00 00 00 fe 12 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| after PDP CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d fe 12 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after PDP PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d fe 12 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |

### rmnet + watches (no IPC SET after helper)

| | modem | rmnet0–7 |
|--|--|--|
| pre / 0x0b watch / attach / PDP drain / helper exit | **ONLINE** | rx=tx=**0** (all eight) |
| 0x0b helper watch t=0…20 s | **ONLINE** | rx=tx=**0** |
| post-probe watch t=0 (uptime **5845.36**) | **ONLINE** | rx=tx=**0** |
| post-probe t≈20 s (**5865.38**) | **ONLINE** | — |
| post-probe t≈40 s (**5885.40**) | **ONLINE** | — |
| post-probe t≈60 s (**5905.41**) | **ONLINE** | rx=tx=**0** |
| post-check (**5935.21**) | **ONLINE** | rx=tx=**0** |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. Helper `after modem_state=ONLINE` last_tx=**GPRS_PS GET mseq=0xb7**. Last SET was **`0x0D04`** mseq **0x91** (not restore / not re-SET `0x0808`). Holder **326** alive (`state=S`, fds **5=`umts_ipc0`** **6=`umts_rfs0`**). ipc1-holder **314** untouched (`sh`, fd **3=`umts_ipc1`**). GNSS **OFFLINE**. dmesg after watch: **0** `CRASH_EXIT`. **0** `CP_CRASH`. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

### Attach in the fail=0 + `02` window — still no bearer

p38 attach (fail=0, **not** this `0x0b`→`0x0a` window) ACK **8000**, **attached=0**, **immediately GMM#7**. This pass: **fail0_at_attach=1** and **`02`** at TX. ACK **8000**, **attached=0**, fail **stayed 0**. Domain drifted **`02`→`01`** on the attach GET — **left**. PDP then ran under **fail=0 + `01`**. Same **0x0D10 st=0x03**, no **0x0D09**. SET **`0x0b`** from **`0x0a`** did **not** crash this boot.

**Data-plane goal not complete.** SET **`0x0b`** ACK **8000**, sticks late, 20 s watch stayed fail=0. SET **`0x0a`** from that **`0x0b`** keeps fail=0. **`0x0D03` `01 00 00`** GEN **8000** under **fail=0 + `02`**. **attached=0**. Vendor PDP GENs **8000**. **0x0D10 st=0x03** again. MODE_SEL **left `0x0a`**. **0x0808 left `01`**. **Do not restore `0x0b` after this attach/PDP.** **Do not re-SET `0x0808` after attach/PDP.** **Do not SET 0x2f.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 74 — re-SET `02` after attach, then PDP (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. p73: **fail0_at_attach=1** and **`02` at attach TX**, GEN **0x0D03 8000**, **attached=0**, fail stayed **0**, but domain dropped **`02`→`01` on the attach GET**. PDP then ran fail0=**1** with **`01`**. Still **0x0D10 st=0x03** / no **0x0D09**. Hole: never had **attach + PDP with `0x0808=02` at PDP TX**. p73 forbade re-SET **`02`** after attach (crash association was restore **`0x0b` plus `0x0808`**, not **`02` alone** while MODE stays **`0x0a`**). This pass copies the p73 window+attach, then **immediately SET `0x0808` `02` again** after attach GET, GET immediately (must be **`02`**), then vendor PDP. **No** restore **`0x0b` after attach/PDP**. **No** SET **`0x0808` after PDP**. **No** `0x2f`. **No** `0x04`/`0x07`. **No** `0x0808` **`0x03`**. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **925** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8883** `/tmp/ps-p74` **1187920**.

### Binary

Copy p73 window + attach + vendor PDP pack. Helper **allows** one SET **0x0808 `02` after attach GET**, **refuses** SET **0x0808 after PDP**, **refuses** restore **`0x0b` after `0x0a`/attach/PDP**, **refuses** SET **0x0808** except **`02`**, **refuses** **0x2f** / **0x04** / **0x07** / **0x03**. After SET **`0x0b`** + GET: **CONT 326**, watch `modem_state` **~20 s**. If CRASH: stop, no further SET. If ONLINE: STOP 326, SET **`02`** if needed, SET **`0x0a`**, poll fail=0 ~15 s, attach, **re-SET `02`**, then PDP if fail=0 or attached=1 (or fail already #7 if **`02` stuck**).

### Live ipc1 (SIM2)

Static `/tmp/ps-p74` (`os/build/e4-ps-p74.c`, Zig musl **1187920**). mseq from **0x74**. CP already **`ONLINE`** (same boot as probes 64–73). Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p74.txt` **118152**. Leftover NOTI: DISP **0x0706** / **0x0701**. **No STK ACK.** **No LCE 0x0D23.**

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701**. **No STK ACK.** |
| GET `PHONE_STATE` | **0x74** | **0x02** |
| GET `MODE_SEL` | **0x75** | **`0x0a`** (left from p69–p73) |
| GET `0x0808` / `0x0816` | **0x76** / **0x77** | **`01`** CS; slot=**1** cause=0 — **skip DDS SET** |
| GET CS / PS / GPRS_PS | **0x78–0x7a** | CS **HOME UMTS** fail=0; PS **NONE GMM#7**; **attached=0** |
| SET `MODE_SEL` **0x0b** | **0x7b** | FMT `08 00 7b ff 08 0a 03 0b`. **GEN 0x080A 8000**. GET **`0x0b`** (stuck this pass). **fail_now=0** |
| CONT **326** + watch **20 s** | — | **ONLINE** all 5 samples. **crash_after_0b=0**. **fail=0** the whole watch (ST07) |
| GET after watch | **0x80–** | MODE_SEL **`0x0b`**. **0x0808 `01`**. PS **ST07 fail=0** |
| SET `0x0808` **02** | **0x86** | **GEN 0x0808 0x8000**. Immediate GET **`02`**. CS briefly left HOME (`cs_left_home=1`; no restore) |
| SET `MODE_SEL` **0x0a** | **0x88** | FMT `08 00 88 ff 08 0a 03 0a`. GEN late (helper slot **ffff**; final **GEN_080a=8000**). GET **still `0x0b`**. **fail=0** immediately — **no poll** |
| SET `GPRS_PS` **`01 00 00`** | **0x8a** | FMT `0a 00 8a ff 0d 03 03 01 00 00`. **GEN 0x0D03 0x8000**. **fail0_at_attach=1**. **0x0808_at_attach_tx=`02`**. GET **cid=0 attached=0**. fail **still 0**. Domain **`02`→`01`** |
| SET `0x0808` **02** again | **0x90** | **GEN 0x0808 0x8000**. Immediate GET **`02`**. **fail0_after_reset02=1**. **att=0** |
| SET `0x0D1B` / `0x0D01` / `0x0D04` | **0x95–0x97** | all GEN **8000**. **fail0_at_pdp=1**. **0x0808_at_pdp_tx=`02`**. **0x0D10** cid=**1** st=**0x03**. **No 0x0D09** |
| PDP drain **30 s** | **0x98–0xb7** | fail=**0** the whole drain. Domain **`02`** (held). **No restore. No re-SET 0x0808 after PDP.** |
| GET after drain | **0xb8–0xbd** | MODE_SEL **`0x0a`**. **0x0808 `02`** (**left**). PS **NONE fail=0**; att=0 |

**No SET 0x2f. No restore 0x0b after attach/PDP. No SET 0x0816. No SET 0x0808 after PDP.**

### Did `0x0b` crash?

**No.** Helper watch after SET **`0x0b`**: **ONLINE** t=0…20 s (5 samples). CONT **326** during watch (`state=R`). Then STOP again for **`0x0a`+attach+re-SET+PDP**. Post-probe host watch **~60 s** (uptime **6463.24→6523.29**) still **ONLINE**. dmesg: **0** `CRASH_EXIT`. **0** `CP_CRASH`.

SET **`0x0b`** from already-**`0x0a`** ACKs **8000** and GET **stuck immediately** this pass (p73 immediate GET was still **`0x0a`**). SET **`0x0a`** GEN arrived late; GET after SET still **`0x0b`**, after drain GET **`0x0a`**.

### Did attach immediately reintroduce GMM#7?

**No.** **GEN 0x0D03 8000**. GET **attached=0**. fail stayed **0** through attach GET, re-SET **02**, whole 30 s PDP drain, and helper exit (`fail0_held=1`). Domain dropped **`02`→`01`** on the attach GET — **re-SET `02` stuck** (GET **`02`**, GEN **8000**).

### GEN

| cmd | GEN |
|--|--|
| SET `MODE_SEL` `0x0b` | **0x8000** SUCCESS |
| SET `0x0808` `02` (after 0x0b watch) | **0x8000** SUCCESS |
| SET `MODE_SEL` `0x0a` | **0x8000** SUCCESS (late; helper slot **ffff**) |
| SET `0x0D03` `01 00 00` | **0x8000** SUCCESS |
| SET `0x0808` `02` (after attach) | **0x8000** SUCCESS |
| `0x0D1B` / `0x0D01` / `0x0D04` | **0x8000** / **0x8000** / **0x8000** |
| SET `0x0816` | **not sent** (slot already 1) |

### fail0 / 0x0808 snapshots

| when | fail | 0x0808 |
|--|--|--|
| attach TX | **0** (**fail0_at_attach=1**) | **`02`** |
| after re-SET 02 GET | **0** (**fail0_after_reset02=1**) | **`02`** |
| PDP TX (`0x0D1B`) | **0** (**fail0_at_pdp=1**) | **`02`** |

**attached GET=0.** First pass with **fail=0 + `02` at attach TX and at PDP TX**.

### 0x0808 timeline

| when | 0x0808 |
|--|--|
| baseline | **`01`** |
| after SET `0x0b` / 20 s watch | **`01`** |
| SET `02` + GET (before attach) | **`02`** (stuck) |
| TX `0x0D03` | **`02`** (**0x0808_at_attach_tx**) |
| after attach GET | **`01`** (drifted) |
| re-SET `02` + GET | **`02`** (**0x0808_after_reset02**) |
| TX `0x0D1B` | **`02`** (**0x0808_at_pdp_tx**) |
| PDP drain / after drain / final | **`02`** (**left**; no re-SET after PDP) |

### fail=0 vs attach TX vs PDP TX

| when | fail | 0x0808 | note |
|--|--|--|--|
| baseline | **7** | **`01`** | MODE **`0x0a`** |
| after SET `0x0b` | **0** | **`01`** | window opened; GET MODE **`0x0b`** |
| watch t=0 … 20 s | **0** | **`01`** | ST07; **held** |
| SET `02` + SET `0x0a` | **0** | **`02`** | GET MODE still **`0x0b`** — **attach same breath** |
| TX `0x0D03` | **0** | **`02`** | **fail0_at_attach=1** **both** |
| after attach GET | **0** | **`01`** | **attached=0**; attach did **not** kill fail=0 |
| re-SET `02` + GET | **0** | **`02`** | **fail0_after_reset02=1** |
| TX `0x0D1B` | **0** | **`02`** | **fail0_at_pdp=1** **both** (p73 hole) |
| drain t=0 … 30 s / helper exit | **0** | **`02`** | **held** (p73 lost fail=0 ~18 s and domain **`01`**) |

`saw_fail0=1`. `fail0_held=1`. `saw_ps_home=0`. `did_attach=1`. `attach_killed=0`. `did_pdp=1`. `aborted=0`. `set0b=1` `crash_after_0b=0` `set0a=1` `set0808=2` `set0808_after_attach=1`. `poll_n=0` `fail0_t=-1` (no 15 s poll; fail=0 on SET **`0x0a`**).

**First pass with fail=0 + `02` at attach TX and at PDP TX.** Attach ACK **8000**, **attached=0**, fail stayed **0**. Re-SET **`02`** stuck. PDP GENs **8000** under **fail=0 + `02`**. Still **0x0D10 st=0x03**, no **0x0D09**, no rmnet. Domain **left `02`**. fail **left 0** at helper exit.

### 0x0D09 vs 0x0D10

| when | 0x0D10 | 0x0D09 |
|--|--|--|
| GEN `0x0D04` / drain / final | cid=**1** st=**0x03** n10=**1** | **none** (n09=0) |

Same disconnected call-status as p63/p67/p69/p70/p72/p73. No IP-config NOTI. Leftover GPRS **0x0D19** NOTI after attach (`00 0a 00 00`); no ACK.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| baseline CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| baseline PS | **UMTS 0x04** | **NONE 0x01** | **0x07** | `04 03 01 b5 c3 8d 01 13 1d 05 07 c3 8d 02 02 01 ff ff 00 00` |
| attach TX PS | **LTE 0x21** | **0x07 ?** | **0x00** | `21 03 07 00 00 00 48 37 42 06 00 c3 8d 02 01 00 7e 01 00 00` |
| after attach PS GET | **UMTS 0x04** | **0x07 ?** | **0x00** | `04 03 07 00 00 00 01 13 1d 05 00 c3 8d 02 01 01 7e 01 00 00` |
| after re-SET 02 / PDP TX PS | **UMTS 0x04** | **NONE 0x01** | **0x00** | `04 03 01 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after PDP CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after PDP PS | **UMTS 0x04** | **NONE 0x01** | **0x00** | `04 03 01 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |

### rmnet + watches (no IPC SET after helper)

| | modem | rmnet0–7 |
|--|--|--|
| pre / 0x0b watch / attach / re-SET / PDP drain / helper exit | **ONLINE** | rx=tx=**0** (all eight) |
| 0x0b helper watch t=0…20 s | **ONLINE** | rx=tx=**0** |
| post-probe watch t=0 (uptime **6463.24**) | **ONLINE** | rx=tx=**0** |
| post-probe t≈20 s (**6483.26**) | **ONLINE** | — |
| post-probe t≈40 s (**6503.28**) | **ONLINE** | — |
| post-probe t≈60 s (**6523.29**) | **ONLINE** | rx=tx=**0** |
| post-check (**6573.36**) | **ONLINE** | rx=tx=**0** |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. Helper `after modem_state=ONLINE` last_tx=**GPRS_PS GET mseq=0xbd**. Last SET was **`0x0D04`** mseq **0x97** (not restore / not re-SET `0x0808` after PDP). Holder **326** alive (`state=S`, fds **5=`umts_ipc0`** **6=`umts_rfs0`**). ipc1-holder **314** untouched (`sh`, fd **3=`umts_ipc1`**). GNSS **OFFLINE**. dmesg after watch: **0** `CRASH_EXIT`. **0** `CP_CRASH`. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

### Attach + re-SET `02` + PDP — still no bearer

p73: **fail0_at_pdp=1**, domain **`01`** at PDP TX, **0x0D10 st=0x03**. This pass closed that hole: **fail0_at_pdp=1** and **`02` at PDP TX**. Same **0x0D10 st=0x03**, no **0x0D09**. Re-SET **`02` after attach** did **not** crash (MODE stayed **`0x0b`/`0x0a`**, no restore **`0x0b`**). Domain **held `02`** through drain (p73 class drift after attach was repaired and **left**). fail **held 0** through drain (`fail0_held=1`). SET **`0x0b`** from **`0x0a`** did **not** crash this boot.

**Data-plane goal not complete.** SET **`0x0b`** ACK **8000**, 20 s watch stayed fail=0. SET **`0x0a`** from that **`0x0b`** keeps fail=0. **`0x0D03` `01 00 00`** GEN **8000** under **fail=0 + `02`**. **attached=0**. Re-SET **`02`** GEN **8000**, GET **`02`**. Vendor PDP GENs **8000** under **fail=0 + `02`**. **0x0D10 st=0x03** again. MODE_SEL **left `0x0a`**. **0x0808 left `02`**. **Do not restore `0x0b` after this attach/PDP.** **Do not SET `0x0808` after PDP.** **Do not SET 0x2f.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** Do not pack v032.

## Probe 75 — vendor 0x0D04 IPv4v6 `[0xF7]=3` (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. p74 closed the IPC “all green” combo: attach + re-SET `02` + vodafone **0x0D1B/0x0D01/0x0D04** all **8000** under fail=0 + `02`, still **immediate 0x0D10 st=0x03 end=0x00**, no **0x0D09**. That combo is **closed**. Host `e4-p75-decode.c` on pulled `libsec-ril.so` **4541576**. **No** restore **`0x0b`**. **No** SET **`0x0a`/`0x2f`/`0x04`/`0x07`**. **No** SET **`0x0808`**. **No** **0x0D03**. **No** **0x0D1B/0x0D01**. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running (did **not** STOP/kill). Killed leftover **sleep** PID **1014** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8885** `/tmp/ps-p75` **1132504** (8884 is a Windows excluded port).

### Binary

No unused named **`IpcTxSetIp*` / `SetDataAllowed` / `SetGlobalData` / `SetLink` / `SetPkt` / `Register*` data-path**. **No** `IpcTx` for **rmnet / svnet / IPA / SIT**. Strings **`rmnet%d`**, **`svnet0`**, **`svnet uevent`** live in **`OnSvnetUevent`** / **`TrafficControl`** / **`ContextActivationDca::SetIpv6Config`** — userspace **after** a real **0x0D09**, not a pre-PDP FMT SET. Direct BL to **`IpcTxSetPdpContext`** / **`DefinePdpContext`** is **0** (vtable). **`IpcTxIpv6Configuration`** @ **0x36b300** is unused **0x0D09 SET** len **24** but the payload is a caller buffer (no const pack) — **not SET**. **`IpcTxSetPdpContextLegacy`** is the same cmd at len **0x70** — **not SET** this pass. **`IpcTxSetSipParam`** is IMS **0x0313** — skipped.

`DataCallSetup::ToDataProtocol` @ **0x2e5190**: strcmp **`IPV4V6`** → **`movz w20, #3`**. **`IP`** / default → **1**. DefinePdp default proto **2** is **IPV6**, not IPv4v6. p26–p74 **0x0D04** 0xF8 APN-copy left **`[0xF7]=0`** (unknown). Vendor **0x0D04** from **`IpcTxSetPdpContext`**: FMT len **0xF8**, **`[7]=1`** APN-copy, CID @ **8**, present @ **9**, APN[101] @ **13**, auth @ **0xF5**, **proto @ `0xF7`**. This pass packs **`[0xF7]=3`** (IPV4V6) cid=**1**. APN **`www.vodafone.net.ua`** is the live **0x0D25** profile (not guessed).

**rild not started.** Decode was not empty. **326** still holds ipc0+rfs0 — do **not** kill it to give rild ipc0. rmnet/svnet in this `.so` need a **0x0D09** first; we still have none.

### Live ipc1 (SIM2)

Static `/tmp/ps-p75` (`os/build/e4-ps-p75.c`, Zig musl **1132504**). mseq from **0x75**. CP already **`ONLINE`** (same boot as probes 64–74). Pre-telnet: holder **326** `radio-boot` fds ipc0+rfs0; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p75.txt` **31476**. Leftover NOTI: DISP **0x0706** / **0x0701**; leftover CS/PS fail=0. **No STK ACK.** **No LCE SET.**

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701**. CS **HOME UMTS** fail=0; PS **NONE fail=0**. **No STK ACK.** |
| GET `PHONE_STATE` | **0x75** | **0x02** |
| GET `MODE_SEL` | **0x76** | **`0x0a`** — **no SET** |
| GET `0x0808` / `0x0816` | — | **`02`** CS_PS; slot=**1** cause=0 — **no SET** |
| GET CS / PS / GPRS_PS | — | CS **HOME UMTS** fail=0; PS **NONE fail=0**; **attached=0** |
| SET `0x0D04` IPV4V6 | **0x7c** | FMT len **0xF8**. `[7]=01` cid=**1** present=**1** APN@13 proto`[0xf7]=3`. **GEN 0x0D04 0x8000**. Immediate **0x0D10** cid=**1** st=**0x03** end=**0x00**. **No 0x0D09** |
| GET `0x0D04` / drain **30 s** | **0x7d–0x82** | GET body **`01 18 00`**. Domain **`02`**. att=**0**. fail=**0**. **n09=0** **n10=1** |
| GET after drain | **0x83–0x88** | MODE_SEL **`0x0a`**. **0x0808 `02`**. slot **1**. CS **HOME**; PS **NONE fail=0**; att=0 |

**No SET 0x0b. No SET 0x0a. No SET 0x0808. No 0x0D03. No 0x0D1B. No 0x0D01. No 0x2f/0x04/0x07.**

### GEN

| cmd | GEN |
|--|--|
| SET `0x0D04` IPV4V6 `[0xF7]=3` | **0x8000** SUCCESS |
| SET `MODE_SEL` / `0x0808` / `0x0D03` / `0x0D1B` / `0x0D01` | **not sent** |

### 0x0D09 vs 0x0D10

| when | 0x0D10 | 0x0D09 |
|--|--|--|
| GEN `0x0D04` / drain / final | cid=**1** st=**0x03** end=**0x00** n10=**1** body `01 03 00 00 00 00 00 00 00 00` | **none** (n09=0) |

Same immediate disconnected call-status as p63/p67/p69/p70/p72/p73/p74. GET **0x0D04** still **`01 18 00`**. No IP-config NOTI.

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| leftover / baseline CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 00 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| leftover / baseline PS | **UMTS 0x04** | **NONE 0x01** | **0x00** | `04 03 01 b5 c3 8d 00 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after SET / drain CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after SET / drain PS | **UMTS 0x04** | **NONE 0x01** | **0x00** | `04 03 01 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |

### rmnet + watches (no IPC SET after helper)

| | modem | rmnet0–7 |
|--|--|--|
| pre / SET / 30 s drain / helper exit | **ONLINE** | rx=tx=**0** (all eight) |
| helper watch t=0…25 s (6 samples) | **ONLINE** | rx=tx=**0** |
| post-check | **ONLINE** | — |

No IPv4 on rmnet. IPv4 only on **rndis0**. **CONT 326** after probe. Helper `after modem_state=ONLINE` last_tx=**GPRS_PS GET mseq=0x88**. Last SET was **`0x0D04`** mseq **0x7c**. Holder **326** alive (`state=S`, fds **5=`umts_ipc0`** **6=`umts_rfs0`**). ipc1-holder **314** untouched (`sh`, fd **3=`umts_ipc1`**). GNSS **OFFLINE**. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

### IPv4v6 pack — still no bearer

p74: vodafone **0x0D1B/0x0D01/0x0D04** with **`[0xF7]=0`**. This pass sent **only** vendor **0x0D04** with **`[0xF7]=3`**. ACK **8000**, same **0x0D10 st=0x03 end=0x00**, no **0x0D09**, no rmnet. Filling the missing IP-type byte did **not** start a PDN. Decode: AP data-path (svnet/rmnet) is **post-0x0D09** userspace, not a missing FMT SET we can send on ipc1.

**Data-plane goal not complete.** **Do not restore `0x0b`.** **Do not SET `0x0808`.** **Do not SET 0x2f.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** **Do not kill 326/314.** Do not pack v032. Do not repeat p74 combo.

## Probe 76 — data path without 0x0D09 / without killing sipc holder (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 or IPv4 on rmnet. Chicken-egg: CP never sends **0x0D09**; RIL configures rmnet only after **0x0D09**; rild wants ipc0 which **326** holds. This pass tests **ipc0 multi-open**, documented **svnet `vnet_open`**, **GET 0x0D09**, and a **minimal 0x0D09 RX** in `e4-radio-boot.c` — **without** repeating p74/p75 **0x0D04** and **without** killing **326/314**. Host `e4-p76-decode.c` on pulled `libsec-ril.so` **4541576**. Kernel ioctl from tree `maazm7d/kernel_samsung_a12` `net_io_device.c` (`vnet_open` = `SIOCSIFFLAGS` IFF_UP). **No** restore **`0x0b`**. **No** SET **`0x0a`/`0x2f`/`0x04`/`0x07`**. **No** SET **`0x0808`**. **No** **0x0D03** SET. **No** **0x0D04**. **No** **0x0109** SET. **No** `POWER_OFF`. **No** `loadnv`. Holder **326** STOP/CONT only (did **not** kill). ipc1-holder **314** left running. Killed leftover **sleep** PID **1089** (held ipc1; not 314). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8887** `/tmp/ps-p76` **1141744** (8885 was p75; 8886 served a stale rebuild).

### Binary / kernel

**`IpcTxIpv6Configuration`** @ **0x36b300** sz=132: FMT len **24**, cmd **0x0D09**, type SET. Copies **16+1 B from caller** (`ld1`/`ldrb`). Pack recover `[7]=0x18` is the **length movz**, not a const payload. **No `IpcTxGetIpv*`**. BL count to 0x36b300 = **0**. **GET 0x0D09 only; no SET.**

**`IpcTxEnableModem(bool)`** @ **0x395bf4**: complete pack len **8**, cmd **0x0109**, `[7]=bool&1`. **SET skipped** (PHONE already ONLINE).

**`IpcTxSetMobileDataSetting`** already sent (p24). **`IpcTxSetDataCallEstablish`** is CDMA **0x0302** — skip. No `SetDataAllowed` / `IpcTxInit` / `IpcTxSetRadio`. No ioctl / `SIOCSIFFLAGS` strings in the `.so`.

Kernel: rmnet is `IODEV_NET` via `vnet_setup` at probe. Documented start is **`vnet_open`** = **`SIOCSIFFLAGS` IFF_UP** (`atomic_inc(&iod->opened)`, `ld->init_comm`, `netif_start_queue`). **No** custom Samsung register/link/start ioctl. **No** invented `SIOCDEVPRIVATE`. **No** `IOCTL_POWER_OFF` on `umts_boot0`.

**radio-boot source** (`os/build/e4-radio-boot.c`): sipcinit holder now walks FMT and on **0x0D09** NOTI brings `rmnet{cid-1}` UP + `SIOCSIFADDR` if a plausible IPv4 is in the body. **Running 326 is the old binary** — did **not** kill 326 to replace it. Gap: the live holder will not apply 0x0D09 until the next `sipcinit`. Probe helper applied the same RX on ipc1 if a NOTI arrived (none did).

### Live

Static `/tmp/ps-p76` (`os/build/e4-ps-p76.c`, Zig musl **1141744**). mseq from **0x76**. CP already **`ONLINE`** (same boot as probes 64–75, uptime ~8493 s). Pre-telnet: holder **326** `radio-boot` fds **5=`umts_ipc0`** **6=`umts_rfs0`**; **314** `sh` fd **3=`umts_ipc1`**. GNSS left **OFFLINE**. Log `/tmp/p76.txt` **33267**. Leftover NOTI: DISP **0x0706** / **0x0701**; leftover CS/PS fail=0. **No STK ACK.**

| test | result |
|--|--|
| ipc0 `open()` while 326 holds | **OK fd=3 errno=0** (multi-open allowed). **Did not kill 326.** **No TX on ipc0** (no unused complete rild-boot IpcTx; SIM2 GET on ipc1) |
| `SIOCSIFFLAGS` IFF_UP rmnet0–7 | **8/8 ok** (`sioc_up=8`). flags **`0x1090`→`0x1091`**. operstate **unknown**. IPv6 LLA **`fe80::200:ff:fe00:0/64`** on all eight |
| unused IpcTx SET | **none sent** (`IpcTxEnableModem` skipped; `IpcTxIpv6Configuration` no const pack) |

| TX | aseq / mseq | result |
|--|--|--|
| leftover NOTI | — | DISP **0x0706** / **0x0701**. CS **HOME UMTS** fail=0; PS **NONE fail=0**. **No STK ACK.** **No 0x0D09.** |
| GET `PHONE_STATE` | **0x76** | **0x02** |
| GET `MODE_SEL` | **0x77** | **`0x0a`** — **no SET** |
| GET `0x0808` / `0x0816` | — | **`02`** CS_PS; slot=**1** cause=0 — **no SET** |
| GET CS / PS / GPRS_PS | — | CS **HOME UMTS** fail=0; PS **NONE fail=0**; **attached=0** |
| GET `0x0D09` ipc1 | **0x7d** | **GEN 0x0D09 `0x8001`**. **No body. n09=0** |
| drain **30 s** | — | DISP only. **n09=0 n10=0** |
| GET `0x0D09` after drain | **0x84** | **GEN `0x8001`** again. MODE_SEL **`0x0a`**. **0x0808 `02`**. CS **HOME**; PS **NONE fail=0**; att=0 |

**No SET 0x0b. No SET 0x0a. No SET 0x0808. No 0x0D03 SET. No 0x0D04. No 0x0D09 SET. No 0x0109 SET. No 0x2f/0x04/0x07.**

### GEN

| cmd | GEN |
|--|--|
| GET `0x0D09` (baseline + after drain) | **`0x8001`** (not SUCCESS; no IP-config) |
| SET `0x0D09` / `0x0D04` / `0x0109` / `MODE_SEL` / `0x0808` / `0x0D03` | **not sent** |

### 0x0D09 vs 0x0D10

| when | 0x0D10 | 0x0D09 |
|--|--|--|
| leftover / GET / drain / final | **none** (n10=0) | **none** (n09=0). GET GEN **`0x8001`** |

### Raw NET_REGIST

FMT **n=27** plen=**20**, fail at byte **[17]** **present**.

| when | act | st | fail | body (20 B) |
|--|--|--|--|--|
| leftover / baseline CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d fe 12 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| leftover / baseline PS | **UMTS 0x04** | **NONE 0x01** | **0x00** | `04 03 01 b5 c3 8d fe 12 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after drain CS | **UMTS 0x04** | **HOME 0x02** | **0x00** | `04 02 02 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |
| after drain PS | **UMTS 0x04** | **NONE 0x01** | **0x00** | `04 03 01 b5 c3 8d 01 13 1d 05 00 c3 8d 02 02 01 ff ff 00 00` |

### rmnet + watches (no IPC SET)

| | modem | rmnet0–7 |
|--|--|--|
| pre IFF_UP | **ONLINE** | rx=tx=**0**, flags **`0x1090`**, operstate **down** |
| after IFF_UP | **ONLINE** | rx=**0** tx=**1** (48 B each) |
| helper exit / 30 s watch | **ONLINE** | rx=**0** tx=**4…5**. IPv6 LLA only. **No IPv4** |
| post-check | **ONLINE** | rmnet0 flags **`0x1091`**, operstate **unknown**, rx=**0** tx=**6** |

`rmnet_nz=1` is **AP-side** IPv6 ND after `vnet_open` (same as probe 12). **rx stayed 0.** IPv4 only on **rndis0**. **Not a cellular PDN.**

**CONT 326** after probe (`state=R` then `S`). Helper `after modem_state=ONLINE` last_tx=**GPRS_PS GET mseq=0x89**. Holder **326** alive (fds **5=`umts_ipc0`** **6=`umts_rfs0`**). ipc1-holder **314** untouched (`sh`, fd **3=`umts_ipc1`**). GNSS **OFFLINE**. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

### Chicken-egg after this pass

ipc0 is **not exclusive** — a second `open()` succeeds while 326 holds. That removes “cannot talk to ipc0 without killing 326.” There is still **no unused complete IpcTx** that starts a bearer, **no** in-tree ioctl beyond IFF_UP, and **GET 0x0D09** is **`0x8001`** with no NOTI. AP IFF_UP does not create a PDN.

A **clean vendor rild** data client would still need **326 not draining ipc0** (STOP or kill) so two readers do not race, then start rild — **not done**. **Do not kill 326/314.**

**Data-plane goal not complete.** **Do not restore `0x0b`.** **Do not SET `0x0808`.** **Do not SET 0x2f.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** **Do not kill 326/314.** Do not pack v032. Do not repeat p74/p75 **0x0D04**.

## Probe 77 — vendor rild with 326 STOPPED (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 from CP or IPv4 on rmnet. p76: ipc0 multi-open OK while 326 holds; remaining clean vendor path was **STOP 326 (not kill) so radio-boot stops draining ipc0, then start rild**. This pass did that. **No** SET **`0x0a`/`0x2f`/`0x04`/`0x07`**. **No** SET **`0x0808`**. **No** p74/p75 **0x0D04**. **No** `POWER_OFF`. **No** `loadnv` (already ONLINE). **Did not kill 326/314.** **Did not skip `pthread_cond_timedwait`.** HIDL join symbol is **`_ZN7android8hardware17joinRpcThreadpoolEv` (`17`)** — not used this pass (`selinux-allow.so` is **SELinux-only**). **Did not** start / kill `hidl-radio` (killing it previously killed rild). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8888** `/tmp/ps-p77` **1055304**; **8889** `/tmp/minird` **1058344**; **8890** `/tmp/selinux-allow.so` **4456**.

### How this image starts rild

`/mnt/a-vendor/etc/init/vendor.sem.rilchip.rc`:

```text
service ril-daemon /vendor/bin/hw/rild
    class main
    ...
    onrestart restart cpboot-daemon
```

No `-l`. `rild` itself set `rilLibPath = /vendor/lib64/libsec-ril.so`. `init.baseband.rc` is `cbd` only (`/vendor/bin/cbd -d -tss310 -bm -mm -P by-name/radio`) — **not run**. `selinux-allow.so` is SELinux ACL allow for `hwservicemanager` (`selinux_check_access` → 0); it does **not** skip cond waits.

### SUPER RO loops (same offsets as probe 22)

`losetup -r -o 3675258880 /dev/loop10` vendor **503668736**. `losetup -r -o 1048576 /dev/loop11` system (first extent **3674210304**). Bind `bin`/`lib64`/`etc/selinux`/`etc/vintf`. Runtime + i18n APEX on loop12/13. binderfs + selinuxfs. Original efs **not** mounted (`/mnt/vendor/efs` = userdata **p38** copy).

### ipc1 multi-open (314 still holds)

Static `/tmp/ps-p77` (`os/build/e4-ps-p77.c`, Zig musl **1055304**). **GET-only**. **Did not STOP/kill 326/314** in the helper.

| test | result |
|--|--|
| ipc1 `open()` while 314 holds | **OK fd=3 errno=0** (same class as p76 ipc0). **Did not kill 314.** |
| GET PHONE / MODE / 0x0808 / 0x0816 / GPRS_PS / 0x0D09 | PHONE **0x02**. MODE_SEL **`0x0a`**. **0x0808 `02`**. **0x0816 `1 0`**. att=**0**. GEN 0x0D09 **`0x0201`**. CS/PS RESP not in 8 s drain. **No SET.** |

### STOP 326 + rild

`kill -STOP 326` (SIGSTOP, **not** SIGKILL). `/proc/326/stat` **`T`**. fds **5=`umts_ipc0` 6=`umts_rfs0`** still open. **314** left `S`.

`/tmp/minird` **1298** (`ro.property_service.version=2` + stub logdw). `hwservicemanager` **1306** with `LD_PRELOAD=/tmp/selinux-allow.so`. Then vendor command:

```text
/vendor/bin/hw/rild
```

(`LD_PRELOAD=/tmp/selinux-allow.so` on rild too — SELinux-only.)

| | |
|--|--|
| rild PID | **1317** stayed up (wchan `binder_ioctl_write_read`) |
| fds | **38=`umts_ipc0`** **34=`umts_ipc1`** **42=`umts_rfs0`** **47=`drb`** **7=`hwbinder`** |
| RIL | `RIL_Init` / `RIL_register` v15 / `RIL_register_socket` completed |
| HIDL | `IRadio/slot1`+`slot2` registered; `ril.halservice.registered.slot1=true` |
| SIM / data | `ril.hasisim=0,0` `ICC_TYPE0/1=0` `phone.connected.slot1=false`. **`OnDataCallStateChanged - data call is nullptr`** |
| RADIO_NOT_AVAILABLE | **not logged**. rild opened ipc and finished init — talks to CP |
| hidl-radio | **not started** (rild already had ipc0; IRadio already registered) |

### 50 s watch (no GET after rild — would fight ipc1)

| t | uptime | modem | rild | rmnet0 rx/tx |
|--|--|--|--|--|
| W0 | 9309.92 | **ONLINE** | 1317 | **0** / 8 |
| W1 | 9319.97 | **ONLINE** | 1317 | **0** / 8 |
| W2 | 9330.00 | **ONLINE** | 1317 | **0** / 8 |
| W3 | 9340.05 | **ONLINE** | 1317 | **0** / 8 |
| W4 | 9350.10 | **ONLINE** | 1317 | **0** / 8 |
| W5 | 9360.14 | **ONLINE** | 1317 | **0** / 8 |

All eight rmnet **rx=0**. IPv4 only **rndis0** `192.168.42.1/24`. tx=8 is leftover AP IPv6 LLA from p76 IFF_UP — **not** a PDN. GNSS **OFFLINE**. **326** still **`T`** (alive). **314** still **`S`**. Final uptime **9398** still **ONLINE**. **Did not CONT 326** (would drain ipc0 against rild). **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

**Data-plane goal not complete.** Exclusive ipc0 for rild (326 STOPPED) is **not** enough for a bearer: `hasisim=0,0`, data call nullptr, rmnet rx=0. **Do not restore `0x0b`.** **Do not SET `0x0808`.** **Do not SET 0x2f.** **Do not SET 0x0808 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** **Do not kill 326/314.** **Do not skip `pthread_cond_timedwait`.** Do not pack v032. Do not repeat p74/p75 **0x0D04**.

## Probe 78 — start hidl-radio client; rild already serving IRadio (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 from CP or IPv4 on rmnet. p77: rild **1317** registered IRadio slot1+slot2 but nothing called `setupDataCall`. Hypothesis: HIDL client down. **No** SET **`0x0a`/`0x2f`/`0x04`/`0x07`**. **No** SET **`0x0808`**. **No** p74/p75 **0x0D04**. **No** `POWER_OFF`. **No** `loadnv` (already ONLINE). **Did not kill 326/314/rild.** **Did not CONT 326.** **Did not kill hidl-radio.** **Did not skip `pthread_cond_timedwait`.** `selinux-allow.so` not used on this exec (static musl). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8891** `/tmp/hidl-radio` **1209976**; **8892** `/tmp/ps-p78` **1139032**.

### Vendor HIDL radio binary

`ls /vendor/bin/hw`: **no** `android.hardware.radio@*-service`. Radio HIDL **is** `/vendor/bin/hw/rild` (already **1317** from p77). Join `_ZN7android8hardware17joinRpcThreadpoolEv` (**17**) is in **`/vendor/lib64/libril_sem.so`** (count **1**), not in the rild ELF. Companion client is static `/tmp/hidl-radio` (`os/build/android-ril/hidl-radio.c`). Watch helper `os/build/e4-ps-p78.c` — **no IPC** (rild owns ipc0/ipc1).

### hidl start command

```text
/tmp/hidl-radio >/tmp/hidl-radio.log 2>&1 &
```

PID **1477**. Did **not** `LD_PRELOAD` hidl-pool (that stubs `WaitForOnline` / skips cond wait). Did **not** kill hidl. IRadio **slot1** handle=**1**. ISehRadio@2.2 **slot1** handle=**2**, **slot2** handle=**3**. `connected=1` after ISeh `setResponseFunction` + `FW_READY`. AOSP `setResponseFunctions` on slot1 completed (`V1_5: setResponseFunctions` in minird). `setRadioPower_1_5 serial=41 on=1`. `setupDataCall serial=34 rat=3 apn=internet proto=IPV4V6`. ISeh `SET_DATA_ALLOWED` / `GET_ICC` slot1+slot2. **GET IRadio slot2 failed** (LIST still shows `IRadio/slot1`+`slot2`; ISeh slot2 OK). `HIDL_RADIO_DONE connected=1`. **1477** stayed in `pause()`.

minird after client: `SehRadioImpl::setResponseFunctions` **twice**; `SET ril.phone.connected.slot1=true` (was **false** in p77). hwservicemanager: `Cannot find entry ... IRadioResponse/saai-rsp` / `IRadioIndication/saai-ind` in VINTF (expected dummy names). **No** `setupDataCall` string in minird. **No** `0x0D09`. hasisim SET stayed **`0,0`**.

### 50 s watch (no IPC — would fight rild)

| t | uptime | modem | rild | hidl | 326 | rmnet0 rx/tx | hasisim | connected |
|--|--|--|--|--|--|--|--|--|
| W0 | 9996.27 | **ONLINE** | 1317 | 1477 | **T** | **0** / 8 | **0,0** | slot1=**true** slot2=false |
| W1 | 10006.63 | **ONLINE** | 1317 | 1477 | **T** | **0** / 8 | **0,0** | slot1=**true** slot2=false |
| W2 | 10017.01 | **ONLINE** | 1317 | 1477 | **T** | **0** / 8 | **0,0** | slot1=**true** slot2=false |
| W3 | 10027.38 | **ONLINE** | 1317 | 1477 | **T** | **0** / 8 | **0,0** | slot1=**true** slot2=false |
| W4 | 10037.75 | **ONLINE** | 1317 | 1477 | **T** | **0** / 8 | **0,0** | slot1=**true** slot2=false |
| W5 | 10048.13 | **ONLINE** | 1317 | 1477 | **T** | **0** / 8 | **0,0** | slot1=**true** slot2=false |

All eight rmnet **rx=0**. **No IPv4** on rmnet. tx=8 leftover AP IPv6 LLA from p76 — **not** a PDN. `ICC_TYPE0/1=0`. GNSS **OFFLINE**. **326** still **`T`** (alive, ipc0+rfs0). **314** still **`S`**. **rild 1317** alive. **hidl 1477** alive. **Did not CONT 326**. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`. **Did not** GET SIM on ipc1.

**Data-plane goal not complete.** HIDL client is up (`connected=1`, slot1 `phone.connected=true`, `setupDataCall` sent) but **`hasisim` stayed `0,0`**, rild did not log a PDN, rmnet rx=0. **Do not restore `0x0b`.** **Do not SET `0x0808`.** **Do not SET 0x2f.** **Do not SET `0x0808` 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** **Do not kill 326/314/rild.** **Do not CONT 326** while rild holds ipc0. **Do not skip `pthread_cond_timedwait`.** **Do not kill hidl-radio.** Do not pack v032. Do not repeat p74/p75 **0x0D04**.

## Probe 79 — getIccCardStatus + DDS + data on slot2 (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 from CP or IPv4 on rmnet. p78: HIDL client up, `setupDataCall` on slot1, **`hasisim=0,0`**. This pass: decode what sets `hasisim`, call **getIccCardStatus** both slots, **setPreferredDataModem** DDS **1** (0-based SIM2), **setupDataCall** on slot2. **No** SET **`0x0a`/`0x2f`/`0x04`/`0x07`**. **No** SET **`0x0808`**. **No** p74/p75 **0x0D04**. **No** `POWER_OFF`. **No** `loadnv` (already ONLINE). **Did not kill 326/314/rild.** **Did not CONT 326.** **Did not steal ipc1.** **Did not skip `pthread_cond_timedwait`.** Restarted **hidl-radio only** (client; 1477 then stuck 1585). RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8893** `/tmp/hidl-radio` **1248056** (first push **1253376**, then nonblock rebuild); `/tmp/ps-p79` **1140128**.

### What sets `hasisim` / `ICC_TYPE` (libsec-ril.so 4541576, 10869 syms)

Host `os/build/e4-p79-decode.c` on pulled `libsec-ril.so`. **Did not GET ipc1.**

| | |
|--|--|
| `ril.hasisim` | `SimManager::ConvertToSimState(SimStatus*)` @ **0x20fef4** (string next to `ConvertToSimStateOnInternalSimIoDone`) |
| `ril.ICC_TYPE0/1` | `IpcRxSecSimCardType` @ **0x39ab38** / **`SEC_SIM_ICC_TYPE`** |
| HIDL `IRadio.getIccCardStatus` | **`RIL_REQUEST_GET_SIM_STATUS`** |
| HIDL `ISehRadio.getIccCardStatus` | **`RIL_REQUEST_SEC_GET_SIM_STATUS`** |
| IPC rild should send | GET **`0x0501`** `IpcTxGetPinStatus` (len 7); GET **`0x050F`** `IpcTxGetSimAppsInfo` (already 25 B USIM in p59). SET **`0x0510`** `IpcTxSetSimOnOff` / **`0x050D`** `IpcTxSetSimPower` are ISeh `setSimOnOff` — already sent in p78. |
| `setDataAllowed` / `setAllowedCarriers` | **not** on the `hasisim` path — **not** used to “fix” SIM |

SIMs are PIN READY on the unit; `hasisim=0` is rild state, not absent cards. Next HIDL to trigger the GETs is **getIccCardStatus** (AOSP + ISeh) — rild owns ipc.

### hidl start

Replaced client **1477** (ETXTBSY on overwrite) then stuck **1585** (two-way `setResponseFunctions` blocked in `binder_ioctl_write_read`). Rebuilt with **O_NONBLOCK** + wr mutex + poll loopers. New PID **1675**.

```text
/tmp/hidl-radio >/tmp/hidl-radio.log 2>&1 &
```

IRadio **slot1** handle=**1**. ISeh@2.2 **slot1** handle=**2**, **slot2** handle=**3**. `connected=1` after ISeh `setResponseFunction` + `FW_READY`. AOSP two-way `setResponseFunctions` **timeout** (`timeout waiting for reply code=1`) — then continued. `setRadioPower_1_5` + slot1 `setupDataCall` serial=**34**. ISeh `getIccCardStatus` slot1 serial=**200** / slot2 **201**; AOSP slot1 **202**. ISeh `setMobileDataSetting` slot2 serial=**230** on=1 roam=1. **GET IRadioConfig default failed** (LIST still shows `@1.0`+`@1.1` `/default`) — **`setPreferredDataModem` not sent**. **GET IRadio slot2 failed** on **1.5/1.4/1.2/1.0** (LIST still shows slot2; later GETs **`BR dead/failed reply`**). Fallback: `setupDataCall` **1.0** serial=**222** + **`setupDataCall_1_5`** serial=**220** APN `www.vodafone.net.ua` access=**2** (UTRAN) and serial=**221** APN `internet` access=**3** on **ISeh slot2** handle=**3**. `HIDL_RADIO_DONE connected=1`. **1675** stayed in `pause()`.

### Card status (both slots)

**No `CARD-RSP` / `CARD-APP` incoming.** Only incoming txs were hwservicemanager `IBase.interfaceChain` (pid **1306**), not rild **1317**. minird: **no** `getIcc` / `CardStatus` / `SIM_STATUS` / `setupDataCall` string. hasisim SET in minird stayed the p77 **`0,0`** (no new SET).

| slot | HIDL getIcc | callback | parsed cardState |
|--|--|--|--|
| slot1 IRadio | sent serial **30** / **202** | **none** | unknown (rild never returned) |
| slot1 ISeh | sent serial **54** / **200** | **none** | unknown |
| slot2 ISeh | sent serial **55** / **201** | **none** | unknown |
| slot2 IRadio | **GET handle failed** | — | — |

**Not proven ABSENT vs PRESENT.** rild **never queried** in a way that produced a HIDL card-status response or a `hasisim` SET. **Did not kill 326 to “fix” SIM.**

### 60 s watch (no IPC — would fight rild) + post HIDL_RADIO_DONE

First watch ran against hidl **1585** (stuck before card/PDN). After **1675** finished:

| | |
|--|--|
| modem | **ONLINE** (uptime **11262**) |
| hasisim | **`0,0`** |
| ICC_TYPE0/1 | **0** |
| connected | slot1=**true** slot2=**false** |
| rmnet0–7 | rx=**0** tx=**8** (leftover AP IPv6 LLA — **not** a PDN) |
| rmnet IPv4 | **none** |
| GNSS | **OFFLINE** |
| 326 | **`T`** alive (ipc0+rfs0) |
| 314 | **`S`** |
| rild | **1317** |
| hidl | **1675** |

**Did not CONT 326**. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`. **Did not** GET SIM on ipc1.

**Data-plane goal not complete.** getIcc + slot2 PDN were **sent** but rild did not store AOSP callbacks (two-way `setResponseFunctions` timeout / later **BR_DEAD_REPLY**), did not return card status, **`hasisim` stayed `0,0`**, rmnet rx=0. **Do not restore `0x0b`.** **Do not SET `0x0808`.** **Do not SET 0x2f.** **Do not SET `0x0808` 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** **Do not kill 326/314/rild.** **Do not CONT 326** while rild holds ipc0. **Do not skip `pthread_cond_timedwait`.** **Do not kill hidl-radio.** Do not pack v032. Do not repeat p74/p75 **0x0D04**.

## Probe 80 — HIDL replies / CARD-RSP (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 from CP or IPv4 on rmnet. p79: requests fired, **no CARD-RSP**, two-way `setResponseFunctions` timed out, IRadio slot2 **BR_DEAD_REPLY**. This pass: joinRpc-style binder loopers (no ioctl mutex, no `O_NONBLOCK`), real `getIccCardStatusResponse` dump, wait for two-way SRF, slot2 ISeh fallback, IRadioConfig. **No** SET **`0x0a`/`0x2f`/`0x04`/`0x07`**. **No** SET **`0x0808`**. **No** p74/p75 **0x0D04**. **No** `POWER_OFF`. **No** `loadnv`. **Did not kill 326/314.** **Did not CONT 326.** **Did not skip `pthread_cond_timedwait`.** Restarted **rild+hidl** after p79 HIDL thread wedged (`futex_wait_queue_me`). Started AOSP **servicemanager** on `/dev/binder`. RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8894** `/tmp/hidl-radio` **1280976**; `/tmp/ps-p80` **1140656**.

### Client fix (`os/build/android-ril/hidl-radio.c`)

- Removed process-wide `g_wr_mu` (deadlocked two-way SRF vs incoming `interfaceChain`).
- Blocking `/dev/hwbinder`; **3** looper threads (`BC_ENTER_LOOPER` + `BC_REGISTER_LOOPER`) = joinRpcThreadpool-style.
- Two-way send is write-only then poll for **BR_REPLY** on the sending thread (60 s for SRF).
- **BR_DEAD_REPLY** no longer kills loopers.
- `dump_card_status` on `IRadioResponse.getIccCardStatusResponse` (code **1**) prints **CARD-RSP**.
- Slot2 uses separate local binders (`saai-rsp2`/`saai-ind2`). IRadio slot2 GET fail → **ISeh only**.

### hidl start

```text
/tmp/hidl-radio >/tmp/hidl-radio.log 2>&1 &
```

Final PIDs: **rild 2130**, **hidl 2169**, **servicemanager 1988**. IRadio slot1 handle=**1**. ISeh@2.2 slot1=**2** slot2=**3**. `connected=1` after ISeh `setResponseFunction`. AOSP two-way `setResponseFunctions` **timeout** (`srf_ok=0`). minird: **`V1_5: setResponseFunctions`** then **`ServiceManager: Waiting 1s on context object on /dev/binder`**, later **`android.system.suspend.ISystemSuspend/default`**. **GET IRadioConfig failed** (`BR_DEAD_REPLY`). **GET IRadio slot2 failed** 1.5/1.4/1.2/1.0 (`BR_DEAD_REPLY`) → ISeh slot2 only. ISeh `getIcc` serial **200**/**201**. Fallback `setupDataCall` on ISeh slot2 APN `www.vodafone.net.ua`. `HIDL_RADIO_DONE connected=1`.

rild **did** call our local binders (`incoming` pid=**2130** `IBase.interfaceChain`). Loopers reply. HIDL **SRF reply still never arrives** — rild’s V1_5 thread stays in framework binder (`/dev/binder` SM, then ISystemSuspend), so **`mRadioResponse` is not stored**.

### Card status (both slots)

**No incoming CARD-RSP.** minird: **no** `GET_SIM_STATUS` / `CardStatus` / `getIcc`. hasisim SET stayed **`0,0`**.

| slot | HIDL getIcc | callback | parsed cardState |
|--|--|--|--|
| slot1 IRadio | sent serial **202** | **none** | unknown (SRF not completed) |
| slot1 ISeh | sent serial **200** | **none** | unknown |
| slot2 ISeh | sent serial **201** | **none** | unknown |
| slot2 IRadio | **GET handle BR_DEAD_REPLY** | — | — |

**Not proven ABSENT vs PRESENT.** **Did not kill 326 to “fix” SIM.**

### 60 s watch (no IPC)

| | |
|--|--|
| modem | **ONLINE** (uptime **12810–12873**) |
| hasisim | **`0,0`** |
| ICC_TYPE0/1 | **0** |
| connected | slot1=**true** slot2=**false** |
| registered | slot1=**true** slot2=**true** |
| rmnet0–7 | rx=**0** tx=**8** (leftover AP IPv6 LLA — **not** a PDN) |
| rmnet IPv4 | **none** |
| GNSS | **OFFLINE** |
| 326 | **`T`** alive (ipc0+rfs0) |
| 314 | **`S`** |
| rild | **2130** |
| hidl | **2169** |

**Did not CONT 326**. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

**Data-plane goal not complete.** Client can take rild `interfaceChain`, but V1_5 `setResponseFunctions` does not return a HIDL reply (blocks on `/dev/binder` + **ISystemSuspend**), **no CARD-RSP**, **`hasisim` stayed `0,0`**, rmnet rx=0. **Do not restore `0x0b`.** **Do not SET `0x0808`.** **Do not SET 0x2f.** **Do not SET `0x0808` 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** **Do not kill 326/314.** **Do not CONT 326** while rild holds ipc0. **Do not skip `pthread_cond_timedwait`.** Do not pack v032. Do not repeat p74/p75 **0x0D04**.

## Probe 81 — ISystemSuspend stub so V1_5 SRF completes (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 from CP or IPv4 on rmnet. p80: joinRpc loopers work, AOSP two-way `setResponseFunctions` **timeout** (`srf_ok=0`) because rild V1_5 blocks on `/dev/binder` **`android.system.suspend.ISystemSuspend/default`**. This pass: stub + stock suspend on binder so SRF stores `mRadioResponse`. **No** SET **`0x0a`/`0x2f`/`0x04`/`0x07`**. **No** SET **`0x0808`**. **No** p74/p75 **0x0D04**. **No** `POWER_OFF`. **No** `loadnv`. **Did not kill 326/314.** **Did not CONT 326.** **Did not skip `pthread_cond_timedwait`.** Restarted **rild+hidl** after stub/stock suspend were up. RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8895** `/tmp/suspend-stub` **1131072** then **1135688**; `/tmp/ps-p81` **1139080**; hidl-radio left **1280976**.

### Decode (`libril_sem.so` / rild / suspend libs)

`libril_sem.so` has **no** `ISystemSuspend` (only `android::releaseWakeLock` / `s_wakelock_count`). rild **3506** maps **`android.system.suspend-V1-ndk.so`** + `libbinder_ndk.so` + `libbinder.so` (not `libpower.so`).

| iface | methods rild can call |
|--|--|
| AIDL `android.system.suspend.ISystemSuspend` | **`acquireWakeLock(WakeLockType, string) → IWakeLock`**, `getInterfaceVersion`, `getInterfaceHash` |
| AIDL `android.system.suspend.IWakeLock` | **`release`**, `getInterfaceVersion`, `getInterfaceHash` |
| HIDL `android.system.suspend@1.0::ISystemSuspend` | **`acquireWakeLock`** + IBase **`interfaceChain`** / ping / hashChain |
| HIDL `IWakeLock` | **`release`** (oneway) + IBase |
| Stock `WakeupList` / `ISuspendControlService` **notifyWakeup** | in `@1.0-service` only — **not** mapped by rild |

Name on `/dev/binder` is AIDL **`android.system.suspend.ISystemSuspend/default`**. HIDL name is `android.system.suspend@1.0::ISystemSuspend/default` on hwbinder.

### Stub + stock service

`os/build/android-ril/isystem-suspend.c` → `/tmp/suspend-stub` **3422**. Loopers on `/dev/binder` + `/dev/hwbinder`. HIDL `interfaceChain` + AIDL `acquireWakeLock` → dummy `IWakeLock` + wakeup/control no-ops.

Raw `SVC_MGR_ADD_SERVICE` to servicemanager **1988** returned **`TF_STATUS_CODE 0x80000001`** (parcel rejected — AIDL/classic/NDK variants). HIDL add on hwbinder **did** work (hwservicemanager **1306** `interfaceChain`).

Stock **`/mnt/a-system/system/bin/hw/android.system.suspend@1.0-service`** **3490** (`AServiceManager_addService` + HIDL `registerAsService`) **did** register AIDL. minird: `Found android.system.suspend.ISystemSuspend/default in framework VINTF manifest.` / `Adding 'kernel' service (...:3490)`. Then HIDL overwrite: stock **3490** registered over stub **3422** on hwbinder.

### hidl start (after rild restart)

Old rild **2130** already gone. New **rild 3506** + **hidl-radio 3547**. CP **ONLINE**. **326** still **T**.

```text
/tmp/hidl-radio >/tmp/hidl-radio.log 2>&1 &
```

IRadio slot1 handle=**1**. ISeh@2.2 slot1=**2** slot2=**3**. IRadioConfig handle=**4**. **IRadio slot2 handle=5** (p80 was **BR_DEAD_REPLY**). AOSP two-way `setResponseFunctions` **COMPLETED** (`srf_ok=1`) on slot1 **and** slot2. `setPreferredDataModem serial=211 modemId=1`. ISeh `getIcc` **200**/**201**. Fallback `setupDataCall` on ISeh slot2 APN `www.vodafone.net.ua`. `HIDL_RADIO_DONE connected=1`.

### Card status (both slots)

Incoming callbacks now arrive (rild **3506** / oneway pid=**0**). Printed **CARD-RSP** lines are **mis-parses** of other responses (inline HIDL token + PTR, real `RadioResponseInfo` is in extra buffers):

| incoming | real info (SG) | cardState |
|--|--|--|
| IRadioResponse code **113** | serial=**32** (setDataAllowed) error=**1 RADIO_NOT_AVAILABLE** | not a card rsp |
| IRadioConfigResponse code **1** | serial=**210** (getSimSlotsStatus) error=**6 REQUEST_NOT_SUPPORTED** | not a card rsp |
| ISehRadioResponse code **38** | serial=**230** error=**1 RADIO_NOT_AVAILABLE** | not a card rsp |
| IRadioIndication code **33** (both slots) | type=UNSOLICITED only | — |

**No** `IRadioResponse.getIccCardStatusResponse` (code **1**). minird: **no** `GET_SIM_STATUS` / `CardStatus` / `getIcc`. hasisim SET stayed **`0,0`**. **Not proven ABSENT vs PRESENT.**

### 60 s watch (no IPC)

| | |
|--|--|
| modem | **ONLINE** (uptime **13994–14056**) |
| hasisim | **`0,0`** |
| ICC_TYPE0/1 | **0** |
| connected | slot1=**true** slot2=**false** |
| registered | slot1=**true** slot2=**true** |
| rmnet0–7 | rx=**0** tx=**8** (leftover AP IPv6 LLA — **not** a PDN) |
| rmnet IPv4 | **none** |
| GNSS | **OFFLINE** |
| 326 | **`T`** alive (ipc0+rfs0) |
| 314 | **`S`** |
| rild | **3506** |
| hidl | **3547** |
| servicemanager | **1988** |
| suspend-stub | **3422** |
| stock suspend | **3490** |

**Did not CONT 326**. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

**Data-plane goal not complete.** ISystemSuspend **is** registered (stock **3490** AIDL; stub HIDL on hwbinder). **`srf_ok=1`**. Slot2 IRadio + IRadioConfig GETs work. Responses return **`RADIO_NOT_AVAILABLE`** / **`REQUEST_NOT_SUPPORTED`**; **no real getIcc CARD-RSP**, **`hasisim` stayed `0,0`**, rmnet rx=0. **Do not restore `0x0b`.** **Do not SET `0x0808`.** **Do not SET 0x2f.** **Do not SET `0x0808` 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** **Do not kill 326/314.** **Do not CONT 326** while rild holds ipc0. **Do not skip `pthread_cond_timedwait`.** Do not pack v032. Do not repeat p74/p75 **0x0D04**.

## Probe 82 — why RADIO_NOT_AVAILABLE after SRF (v031, 2026-09-02)

Goal: `rmnet*` rx/tx ≠ 0 from CP or IPv4 on rmnet. p81: ISystemSuspend registered, **`srf_ok=1`**, HIDL responses **`RADIO_NOT_AVAILABLE`**. This pass: decode when HIDL returns RNA, drive **RIL `setRadioPower` ON** (not `IOCTL_POWER_OFF` on `umts_boot0`), parse real **RadioError** from extra buffers, resync GETs (`getCurrentCalls` / `getSignalStrength` / `getIccCardStatus`). **No** SET **`0x0a`/`0x2f`/`0x04`/`0x07`**. **No** SET **`0x0808`**. **No** p74/p75 **0x0D04**. **No** `POWER_OFF`. **No** `loadnv`. **Did not kill 326/314.** **Did not CONT 326.** **Did not skip `pthread_cond_timedwait`.** RNDIS host **192.168.42.20**. Telnet `192.168.42.1:23`. wget **8897** `/tmp/hidl-radio` **1284152**; `/tmp/ps-p82` **1139496**. Pulled `/vendor/lib64/libril_sem.so` **840880** via TCP **8896** (size match).

### Decode (`libril_sem.so` 840880 / `libsec-ril.so` 4541576)

Host `os/build/e4-p82-decode.c`. **No HIDL `getRadioState`.** Radio state is unsol only.

| | |
|--|--|
| HIDL RNA | `libril_sem` `E_RADIO_NOT_AVAILABLE` is **`failCauseToString`** (RIL_Errno **1**). Not a separate “rilc down” string. |
| RadioState names | `radioStateToString` @ **0x51a98**: **0=OFF**, **1=RADIO_UNAVAILABLE**, **10=ON**. |
| `setRadioPower` | `RadioImpl::setRadioPower` @ **0x54cb8** logs then **`CALL_ONREQUEST` `RIL_REQUEST_RADIO_POWER` (0x17=23)** — **does not** itself reject UNAVAILABLE. `setRadioPower_1_5` @ **0x836b8**. |
| Unsol | `radioStateChangedInd` @ **0x67958**; string **`radioStateChangedInd: radioState %d`**. **`rilConnectedInd`**. Slot with no callback: **`radioService[%d]->mRadioIndication == NULL`**. |
| `rilc_*` | `rilc_thread_pool` / `rilc_configure_thread_pool` only — **no** “rilc not connected” gate in this ELF. |
| `WaitForOnline` | **`libsec-ril` `_Z13WaitForOnlinei` @ 0x335a9c** sz **1260** (ioctl CP ONLINE + **`pthread_cond`**). **Not** skipped. SRF completing means this returned — CP ioctl is ONLINE. |
| Vendor power | `PowerManager::SetRadioState` / `ConvertToRadioState` / `IpcTxRadioPower` / **`RIL_REQUEST_RADIO_POWER`**. String **`radioState(%d) so do not update simStatus`**. |

**RNA after SRF is HIDL RadioState UNAVAILABLE (1), not missing suspend and not “rild never opened ipc.”** `WaitForOnline` can succeed (CP ONLINE) while `PowerManager` still has RadioState **1**. Requests other than power complete immediately with **RIL_E_RADIO_NOT_AVAILABLE**. `RADIO_POWER` is dispatched; if CP never ACKs, **no HIDL `setRadioPowerResponse`**.

### Live (same AP boot; 326 stayed T)

First hidl replace **SIGTERM 3547** took down previous **rild 3506** (`RIL Restart for Phone Died`). **Restarted rild only** (PIDs died). **Did not CONT 326.** **Did not** kill **314**. minird **1298**, hwservicemanager **1306**, servicemanager **1988**, stock suspend **3490**, stub **3422** left up.

```text
export LD_PRELOAD=/tmp/selinux-allow.so
/vendor/bin/hw/rild >/tmp/rild.stdout 2>/tmp/rild.stderr &
/tmp/hidl-radio >/tmp/hidl-radio.log 2>&1 &
```

| | |
|--|--|
| rild | **3780** |
| hidl-radio | **3816** (`pause()` after `HIDL_RADIO_DONE`) |
| IRadio | slot1 handle=**1**, slot2 handle=**5** |
| ISeh @2.2 | slot1=**2** slot2=**3** |
| IRadioConfig | handle=**4** |
| AOSP SRF | **`srf_ok=1`** (V1_5 `setResponseFunctions`) |
| `ril.phone.connected.slot1` | **true** |

### RadioError / state (real extra-buffer parse; not p81 CARD-RSP misread)

There is **no** HIDL `getRadioState`. Client listens for `IRadioIndication.radioStateChanged` (code **1**). That callback **did not arrive** on our binders (only **code 33 `rilConnected`**). minird **did** log the HAL call:

```text
RILC: rilConnectedInd
RILC: radioStateChangedInd: radioState 1
RILC: setRadioPower: serial 31 on 1
RILC: setRadioPower: serial 204 on 1
```

**`radioState 1` = UNAVAILABLE** (`radioStateToString`). **Not OFF (0). Not ON (10).**

| HIDL | serial | RadioError |
|--|--|--|
| `setRadioPower` / `_1_5` ON (slot1 **31/41**, slot2 **204/214**) | rild **logged** the request | **no `setRadioPowerResponse`** (`pwr_err` / `pwr15_err` stayed **-1**) |
| `getCurrentCalls` slot1 | **50** | **1 RADIO_NOT_AVAILABLE** (IRadioResponse@1.2 code **140**) |
| `getCurrentCalls` slot2 | **223** | **1 RADIO_NOT_AVAILABLE** |
| `getSignalStrength` / `getIccCardStatus` | sent | **no IRadioResponse** (RNA at dispatch; **no** minird `GET_SIM_STATUS` / `getIcc` / `CardStatus`) |
| IRadioConfig `getSimSlotsStatus` | **210** | **6 REQUEST_NOT_SUPPORTED** (not a card rsp; parser also tagged this as CARD-RSP err=6 — **not** a real `cardState`) |

**No real `getIccCardStatusResponse` CARD-RSP.** **Not proven ABSENT vs PRESENT.** hasisim SET stayed **`0,0`**. **Did not** brute PIN. **Did not** steal `umts_ipc` from rild.

Resync GETs after radio-on **cannot** clear UNAVAILABLE: they RNA without reaching `libsec-ril` SIM/IPC. `setRadioPower` **does** enter `RadioImpl` but **never completes** — consistent with power request stuck waiting on CP while state stays **1**.

### 60 s watch (no IPC)

| | |
|--|--|
| modem | **ONLINE** (uptime **14931–14994**) |
| hasisim | **`0,0`** |
| ICC_TYPE0/1 | **0** |
| connected | slot1=**true** slot2=**false** |
| registered | slot1=**true** slot2=**true** |
| rmnet0–7 | rx=**0** tx=**8** (leftover AP IPv6 LLA — **not** a PDN) |
| rmnet IPv4 | **none** |
| GNSS | **OFFLINE** |
| 326 | **`T`** alive (ipc0+rfs0) |
| 314 | **`S`** |
| rild | **3780** |
| hidl | **3816** |

**Did not CONT 326**. **Did not** `POWER_OFF`. **Did not** leftover `loadnv`.

### Does RNA require CONT 326?

**Yes, that is the remaining hypothesis — and this pass did not CONT.** CP is **ONLINE** (ioctl / sysfs). `WaitForOnline` returned. rild holds **ipc0**. HIDL RadioState is **UNAVAILABLE (1)**, not OFF. Boot UNSOLs (`INIT_END` / phone-state) were already on the sipc path while **326** was the keeper; **326 is STOPPED (`T`)** so rild never saw them, and HIDL GETs cannot resync that. **CONT 326 while rild holds ipc0** would race two readers on ipc0 (historically 326 is the sipc keeper that also keeps CP ONLINE). **Do not CONT** until rild is stopped cleanly **and** there is a plan to keep CP ONLINE.

**Data-plane goal not complete.** RNA is **RadioState UNAVAILABLE**, not missing ISystemSuspend. **`srf_ok=1`**. `setRadioPower` ON **sent**, **no power response**. `getCurrentCalls` **RADIO_NOT_AVAILABLE**. **No GET_SIM_STATUS**. **`hasisim` `0,0`**. rmnet rx=0. **Do not restore `0x0b`.** **Do not SET `0x0808`.** **Do not SET 0x2f.** **Do not SET `0x0808` 0x03.** **Do not ACK STK.** **Do not POWER_OFF.** **Do not leftover loadnv.** **Do not kill 326/314.** **Do not CONT 326** while rild holds ipc0. **Do not skip `pthread_cond_timedwait`.** Do not pack v032. Do not repeat p74/p75 **0x0D04**.

## Next

- CP **`ONLINE`**. **326** alive **STOPPED** (`T`, ipc0+rfs0 still open). ipc1-holder **314**. **rild 3780** alive. **hidl-radio 3816** alive. **servicemanager 1988**. stock **ISystemSuspend 3490**. MODE_SEL left **`0x0a`**. rmnet **rx=0**, **no IPv4**. Goal **not complete**.
- Probe 82: RNA is HIDL **RadioState UNAVAILABLE (1)**, not missing suspend. **`srf_ok=1`**. `setRadioPower` ON **sent**, **no power rsp**. `getCurrentCalls` **RADIO_NOT_AVAILABLE**. **No GET_SIM_STATUS**. **No real cardState**. **Did not CONT 326** (would race ipc0). Watch **60 s ONLINE**. **rmnet rx=0**.
- Probe 81: ISystemSuspend **registered** (stock AIDL **3490**; stub HIDL). **`srf_ok=1`**. IRadio slot2 + IRadioConfig **up**. Responses **RADIO_NOT_AVAILABLE**. **No GET_SIM_STATUS**. **No real cardState**. Watch **60 s ONLINE**. **rmnet rx=0**.
- Probe 80: joinRpc loopers work (rild **2130** `interfaceChain`). AOSP SRF **timeout**. **No CARD-RSP**. **No GET_SIM_STATUS** in minird. IRadio slot2 + IRadioConfig **BR_DEAD_REPLY**. Blocker was **rild V1_5** waiting on **`/dev/binder`** / **ISystemSuspend/default**. Watch **60 s ONLINE**. **rmnet rx=0**.
- Probe 79: `hasisim` via **`ConvertToSimState`** / HIDL **GET_SIM_STATUS** → IPC **0x0501**/**0x050F** (rild). getIcc **sent**, **no CARD-RSP**. GET IRadio **slot2** + IRadioConfig **failed** (`BR_DEAD_REPLY`). ISeh slot2 `setupDataCall_1_5` sent. Watch **ONLINE**. **rmnet rx=0**.
- Probe 78: vendor radio HIDL **is rild** (join **17** in `libril_sem.so`). Client **`/tmp/hidl-radio`** **1477**. `hasisim=0,0`. `phone.connected.slot1=true`. `setupDataCall` sent. **GET IRadio slot2 failed**. Watch **50 s ONLINE**. **rmnet rx=0**.
- Probe 77: **ipc1 multi-open OK** errno=**0**. **STOP 326** (`T`, not killed). Vendor **`/vendor/bin/hw/rild`** → **1317** opened ipc0+ipc1+rfs0, IRadio registered, **no RADIO_NOT_AVAILABLE**. **hasisim=0,0**. Watch **50 s ONLINE**. **rmnet rx=0**.
- Probe 76: **ipc0 multi-open OK** errno=**0**. **GET 0x0D09 GEN `0x8001`**. **SIOCSIFFLAGS** rmnet0–7. **No 0x0D09 NOTI**. **No rild**. Helper **30 s** watch **ONLINE**.
- Probe 75: **one vendor 0x0D04 IPV4V6 `[0xF7]=3`**. **GEN 8000**. **0x0D10 st=0x03 end=0x00**. **No 0x0D09**. **No rild** (decode not empty; 326 holds ipc0). Helper **30 s** watch **ONLINE**.
- Probe 74: **p73 window+attach, then re-SET `02`, then PDP**. **0x0b crash=no**. **fail0_at_attach=1**. **0x0808_at_attach_tx=`02`**. **GEN 0x0D03 8000**. **attached=0**. **fail0_after_reset02=1**. **0x0808_after_reset02=`02`**. **fail0_at_pdp=1**. **0x0808_at_pdp_tx=`02`**. **0x0D10 st=0x03**. **No 0x0D09**. fail **held 0** through drain. Domain **held `02`**. Helper `0x0b` watch + **~60 s** post watch **no CRASH_EXIT**.
- Probe 73: **`0x0b`→`02`→`0x0a` then `0x0D03` then PDP (no re-SET)**. **fail0_at_pdp=1**. **0x0808_at_pdp_tx=`01`**. Same **0x0D10 st=0x03**.
- Probe 72: **SET `0x0b` first, watch 60 s, then `0x0a` + fail=0 PDP**. **No PsAttach.** **fail0_at_pdp=1**. **0x0808_at_pdp_tx=`02`**. Same **0x0D10 st=0x03**.
- Probe 71: **wait fail=0 + keep `02`, prefer abort**. **fail0_at_pdp=0**. **aborted=1**.
- Probe 70: **domain `02` + `0x0a` + PDP, no restore**. **0x0808_at_pdp_tx=`02`**. **fail0_at_pdp=0**. **0x0D10 st=0x03**.
- Probe 69: **`0x0a` + PDP, no restore**. **fail0_at_pdp=1**. Domain drifted **`02`→`01`**. Same **0x0D10 st=0x03**.
- **Do not restore MODE_SEL `0x0b` after attach/PDP.** **Do not SET `0x0808` after PDP.** **Do not** SET **0x2f** / **0x04** / **0x07**.
- **Do not POWER_OFF.** **Do not leftover loadnv** while ONLINE.
- **Do not SET 0x0808 0x03.**
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
