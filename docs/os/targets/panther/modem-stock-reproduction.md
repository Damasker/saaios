# Pixel 7 stock cellular — reproduction recipe

Captured **2026-10-05** on the same panther after the user flashed stock
Android (KernelSU root, `adb root`). Slot `_b`, product `panther`.
Read-only of NV contents. No IMEI / ICCID / PIN / addresses in this
file. Do not invent SIT opcodes.

This is the working control: radio + SIM + network succeed on stock.
SaaiOS native `cp-boot` UDL is **not** this path. Reproduce **cbd +
rfsd + rild_exynos + real EFS NV**, not a TOC splice.

Related history: [modem.md](modem.md). Adjacent silicon that can fail
the same way as a bad SIT sequence:
[hardware-risks.md](hardware-risks.md).

## Proven result (stock)

| Item | Value |
|---|---|
| Baseband | `g5300q-260317-260505-B-15346003` |
| Kernel | `6.1.157-android14-11-gbd23337e42e7-ab14791245` |
| Slot | `_b` → `vendor.cbd.partition=modem_b` (`sda29`) |
| SIM | phone0 `LOADED`, phone1 `NOT_READY` (`dsds`) |
| CS/PS | `IN_SERVICE` / `REG_HOME`, rat **LTE**, PLMN **25503** (Kyivstar) |
| Data | `SETUP_DATA_CALL` `cause=NONE`, `cid=2`, `ifname=rmnet1` UP |
| Attach | `mLteAttachResultType=2`, VoPS/EMC bearer 3, NR available, EN-DC false |
| Time to camp | ~1 s from first `OUT_OF_SERVICE` notify to LTE HOME |

`SETUP_DATA_CALL` ran **after** CS/PS HOME. Camp is NV + CP boot +
radio power, not the data-call opcode.

SaaiOS previously: empty UDL `NV_NORM` (crc 0), RFS quarantine on
handle 3 only, WCDMA `REG_DENIED` reject 0. That is the gap.

## Boot order (init)

`fstab.modem` (`mount_all` from `init.modem.rc` `on fs`):

| GPT | Device (this phone) | Mount | FS |
|---|---|---|---|
| `efs` | `sda5` | `/mnt/vendor/efs` | f2fs `noatime,sync` |
| `efs_backup` | `sda6` | `/mnt/vendor/efs_backup` | f2fs `noatime,sync` |
| `modem_userdata` | `sda7` | `/mnt/vendor/modem_userdata` | f2fs `noatime,sync` |
| `modem` slotselect | `sda29` (`modem_b`) | `/mnt/vendor/modem_img` | ext4 **ro** |

CP image: `/mnt/vendor/modem_img/images/default/modem.bin` (~98 265 168
bytes). Prop `vendor.modem.bin.path` uses a double slash before
`modem.bin`; that is what stock set.

`init.modem.rc` after mount: `restorecon_recursive` + `chown radio
system` on efs, efs_backup, modem_userdata.

Processes (class `main`, user `radio` except smp):

```text
/vendor/bin/cbd -d -t s5100sit -P by-name/modem_b -s 2
/vendor/bin/rfsd -d
/vendor/bin/hw/rild_exynos
/vendor/bin/shared_modem_platform -s
```

`cbd.rc` expands `-t ${ro.vendor.cbd.modem_type}` (`s5100sit`) and
`-P by-name/${vendor.cbd.partition}` after
`modem${ro.boot.slot_suffix}`. `-s 2` is dual SIM. Capabilities:
`BLOCK_SUSPEND NET_ADMIN NET_RAW SYSLOG`.

`rfsd.rc`: `shutdown critical`, `ioprio rt 0`.

`rild_exynos.rc`: `vendor.rild.libpath=libsitril.so`, Radio HAL
`ro.vendor.ril.use_radio_hal=2.2`.

`shared_modem_platform_config` file contents: `RPC`.

Kernel (this boot): `cpif_page.ko` then `cpif.ko`;
`create_modemctl_device: s5300 is created!!!`; PROTOCOL_SIT;
`IOCTL_START_CP_BOOTLOADER` on `umts_boot0` at ~4.5 s;
`init` starts `cpboot-daemon` at ~4.1 s (pid 1139).

## Character nodes and fds

Major 490 on this stock boot (SaaiOS probe dumps used 493 — different
CPIF bind; match **names**, not a hardcoded major).

| Node | Stock owners | Who holds it |
|---|---|---|
| `umts_boot0` | radio/system | **cbd** (and rild also has a fd) |
| `umts_rfs0` | radio/radio | **rfsd** |
| `umts_ipc0` | radio/radio | **rild_exynos** |
| `umts_ipc1` | radio/radio | **rild_exynos** (DSDS second socket) |
| `umts_router` | system/system | AT (`GOOGGETNV` / factory) |
| `umts_dm0` | system/system | unused for camp |

Shannon `com.shannon.imsservice` / `rcsservice` run after bearer.
Data call completed before the second `ENABLE_VONR`. IMS is not the
camp gate.

## NV / RFS (the actual gate)

Live `/mnt/vendor/efs` (radio:radio). Sizes only:

| File | Bytes | Role |
|---|---:|---|
| `nv_normal.bin` | 524288 | padded NV_NORM (TOC 512 KiB) |
| `nv_normal.bin.tmp` | **476784** | payload cbd/rfsd use |
| `nv_normal.bin.md5` | 32 | ascii md5 |
| `nv_protected.bin` | 524288 | padded NV_PROT |
| `nv_protected.bin.tmp` | **189452** | payload |
| `nv_protected.bin.md5` | 32 | ascii md5 |

Backup: `/mnt/vendor/efs_backup/nv_protected.bak` (+ `.md5`). First
boot of this flash: rfsd log
`File nv_protected.bin restored from backup` after open/unprotect
failed. Later boots: `nv_normal.bin : Verified`,
`nv_normal.bin file is ok`, `nv_protected.bin file is ok`, then
`Open temp file …/nv_normal.bin.tmp`.

Header of both normal files (first 16-ish bytes): magic **`ERIG`**
(`45 52 49 47`) after a small LE preamble. Not zeros. Injecting a
userdata copy into the **modem.bin TOC** produced
`invalid verified NV copy` on the SaaiOS probe. Stock does not splice
TOC; **cbd UDL-loads NV_NORM/NV_PROT from these EFS files** while rfsd
serves handles **1 (normal) and 3 (protected)** on `umts_rfs0`.

SaaiOS must:

1. Mount the **real** `efs` (and backup) the way `fstab.modem` does —
   not a handle-3-only quarantine.
2. Leave **both** NV files + md5 + `.tmp` as rfsd verifies them.
3. Let **stock `cbd`** inject them. Do not write zeros into `NV_NORM`.
4. Keep identity in protected NV on-device. Do not log it. Do not
   commit NV blobs to git.

## Props that stock set when camped

```text
vendor.cbd.boot_done=1
vendor.cbd.modem_bin_status=2
vendor.cbd.modem_bin_type=2
vendor.cbd.partition=modem_b
vendor.ril.cbd.rfs_check_done=1
vendor.ril.allow_data_0=1
vendor.radio.allowed_types_loaded0=1
persist.vendor.ril.camp_on_earlier=1
persist.vendor.radio.multisim.config=dsds
persist.vendor.ril.support_nr_ds=1
persist.vendor.ril.use_radio_hal=2.2
persist.radio.is_vonr_enabled_0=false
persist.vendor.radio.target_oper=400
ro.vendor.cbd.modem_type=s5100sit
```

## RIL sequence (names from `logcat -b radio`, PHONE0)

Radio buffer on this capture started after `RADIO_POWER` (already
rotated). Names below are **observed**, not guessed SIT.

After SIM I/O (`SIM_IO` sw `0x90`), still unregistered
(`NOT_REG_MT_NOT_SEARCHING_OP`), then LTE PCC unsol
(`UNSOL_PHYSICAL_CHANNEL_CONFIG`, `UNSOL_RESPONSE_NETWORK_STATE_CHANGED`),
then:

1. `SET_DATA_PROFILE`
2. `SET_INITIAL_ATTACH_APN` — APN name `internet`, numeric `25503`,
   protocol `IPV4V6`, types `supl | hipri | default`,
   `TrafficDescriptor.mDnn=internet`, `preferred=true`
3. `ENABLE_VONR` (ACK empty)
4. `SETUP_DATA_CALL` `reason=NORMAL` `accessNetworkType=EUTRAN`
   same profile → `rmnet1`, `cause=NONE`
5. `SET_ALLOWED_NETWORK_TYPES_BITMAP` (ACK empty)
6. Poll `OPERATOR` `{UA-KYIVSTAR, UA-KS, 25503}`,
   `VOICE_REGISTRATION_STATE` / `DATA_REGISTRATION_STATE` `REG_HOME`
   `rat: LTE` `reasonForDenial: NONE`
7. `IS_NR_DUAL_CONNECTIVITY_ENABLED` → `true`
8. `SET_UNSOLICITED_RESPONSE_FILTER 127`

Other verbs seen: `GET_CELL_INFO_LIST`, `GET_RADIO_CAPABILITY`
(`mLogicModemId=exynos_modem0`, raf `906119`), `GET_USAGE_SETTING {1}`,
`GSM_SET_BROADCAST_CONFIG`, `SET_DATA_THROTTLING`,
`SET_SIGNAL_STRENGTH_REPORTING_CRITERIA`, `SET_CLIR`,
`QUERY_NETWORK_SELECTION_MODE {0}`.

PHONE1 stays unregistered (empty SIM). Do not require it.

IA APN on another SIM must come from that SIM/carrier-config, not from
this 25503 example.

Unsols while camped: `UNSOL_DATA_CALL_LIST_CHANGED`,
`UNSOL_SIGNAL_STRENGTH`, `UNSOL_BARRING_INFO_CHANGED`,
`UNSOL_RESTRICTED_STATE_CHANGED`, `UNSOL_VOICE_RADIO_TECH_CHANGED`,
`UNSOL_EMERGENCY_NUMBER_LIST`, `UNSOL_LCEDATA_RECV`.

## What SaaiOS must match

| Stock | SaaiOS miss (prior) | Action |
|---|---|---|
| Real `/mnt/vendor/efs` + both NV | quarantine / zeros in UDL NV | mount originals or an **rfsd-verified** copy of both handles |
| `cbd -d -t s5100sit -P by-name/modem_<slot> -s 2` | custom `cp-boot` stall in MAIN UDL | run **vendor cbd** against stock `modem.bin` |
| `rfsd -d` on `umts_rfs0` for normal **and** protected | handle 3 only | stock rfsd |
| `rild_exynos` + `libsitril` on ipc0+ipc1 | host SIT probes | full rild after `vendor.cbd.boot_done=1` |
| IA APN + `SETUP_DATA_CALL` EUTRAN | INTPS/dual ACK, `ratbm` error 2 | use the **named** RIL requests above; no new opcodes |
| LTE HOME then rmnet | WCDMA `REG_DENIED` | treat REG_DENIED as NV/FLASH failure, not APN |

## Forbidden (unchanged)

- Log or commit IMEI, ICCID, IMSI, PIN, SMSC, bearer IPs.
- Invent SIT bytes / opcodes (`ratbm`, `scan734`, etc.).
- Splice NV into `modem.bin` TOC (stock rejects verified copy).
- `IOCTL_POWER_OFF`.
- Write original EFS unless the user explicitly grants a new write.

## Success on SaaiOS

Same as stock, not “ACK on a probe”:

- `vendor.cbd.boot_done=1` and rfsd `Verified` for **nv_normal**
- CS+PS `IN_SERVICE` LTE (or the SIM’s home RAT) `REG_HOME` reject 0
- `SETUP_DATA_CALL cause=NONE` and `rmnet*` UP with IPv4 (and/or IPv6)

Wi-Fi / IWLAN is not a substitute.
