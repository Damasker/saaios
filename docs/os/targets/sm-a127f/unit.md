# SM-A127F — this unit

Captured 2026-08-18, lock state updated 2026-08-19 after OEM unlock. ADB from R620: `192.168.168.22:5555` (`RF8RB139S0B`).

## Identity

| Field | Value |
|-------|--------|
| Serial | `RF8RB139S0B` |
| Model | **SM-A127F** |
| Device | **a12s** |
| Product | `a12snsxx` |
| Hardware | **exynos850** |
| Bootloader / PDA | **A127FXXSDDXJ2** |
| Build id | `TP1A.220624.014.A127FXXSDDXJ2` |
| Binary / bit | **D (13)** |
| Android | **13** |
| `ro.csc.sales_code` | **XID** |
| `ril.official_cscver` | **A127FOLEDDXJ2** (OLE package — matches downloaded CSC) |
| Kernel | **4.19.111-27127798** aarch64 |
| RAM | **2760076 kB** (~2.63 GiB, 3 GB SKU) |
| LAN ADB | `192.168.168.22:5555` |

## Lock / AVB (after OEM unlock)

| Prop | Value |
|------|--------|
| `ro.boot.flash.locked` | **0** |
| `ro.boot.vbmeta.device_state` | **unlocked** |
| `ro.boot.verifiedbootstate` | **orange** |
| `sys.oem_unlock_allowed` | **1** |
| `ro.boot.warranty_bit` | **0** (may flip on first custom image) |
| `knox.kg.state` | **Checking** — keep Wi‑Fi on until it settles |

Custom `boot.img` is now possible. Still do not flash until a host-side ramdisk is packed and `make restore` has stock `boot.img` + `vbmeta.img`.

## Phase 1 — first custom boot (2026-08-19)

`boot-v005` + Magisk-style patched stock `vbmeta` (flags 3). Odin AP PASS.

On-device: vendor 4.19 kernel + our ramdisk `/init` (no Android). Samsung splash is replaced; PID 1 owns `fb0` (color cycle). Rollback: `saaios-boot-stock-restore.tar` (stock `boot.img` + stock `vbmeta.img`).

`boot-v006`: BusyBox ramdisk. Screen **yellow** (BGR vs our hardcoded cyan), **no new COM** — USB ACM gadget did not enumerate. Kernel likely has no ACM, or UDC bind failed.

`boot-v007`: pack pixels via `fb_var` RGB offsets (true cyan/navy + on-screen log: `fn rndis=/acm=/udc`). Prefer RNDIS + `telnet 192.168.42.1` (USB tethering is in stock kernels). Photo the text if COM still missing.

## boot-v020 — board console (TSP parked)

Ramdisk-only on **stock** DXJ2 kernel+DTB. Banner `SaaiOS v020`. Odin AP: `\\192.168.168.110\media\saaios-boot-v020.tar`. Rollback: `saaios-boot-stock-restore.tar`.

On-screen: battery `%` + status, backlight `cur/max`, USB, MemAvailable. Vol± writes sysfs brightness (never 0, never a second `FBIOBLANK`). Power 2s reboots. Telnet `:23`/`:2323` and dropbear `:22` unchanged. Init does not run the TSP lab.

**LIVE 2026-08-29:** human flashed AP; banner, brightness, battery, USB/telnet all accepted. Phase C return point. TSP still parked.

## boot-v021 — speaker beep (Phase E1)

Ramdisk-only on **stock** DXJ2 kernel+DTB. Banner `SaaiOS v021`. Odin AP: `\\192.168.168.110\media\saaios-boot-v021.tar`. Does not overwrite v020. Rollback: `saaios-boot-v020.tar` or `saaios-boot-stock-restore.tar`. Path: [audio.md](audio.md).

On-screen: same v020 lines plus `aud` (`/proc/asound/cards`). Short Power runs `/sbin/beep` (SMA1303 / `pcmC0D3p`). 2 s Power still reboots. ABOX firmware is in the ramdisk under `/vendor/firmware`. No maze Image. No auto-beep at boot.

**LIVE 2026-08-29 (kernel path):** human flashed AP. Telnet `:23`/`:2323`. Cards `Exynos3830-Madera` / `abox_vdma` / `abox_dump`. `pcmC0D3p` present. `/sbin/beep` opened RDMA3: Calliope `NFB0`, SMA1303 UNMUTE, UAIF1 48 kHz / 16-bit / 2ch, `abox_rdma_trigger[3](1)` then stop ~1 s later. **Inaudible:** idle mixer `Speaker Mode=Off`, mute On, `ABOX SPUS ASRC3=On`, volume 118/167. Tar SHA256 `30869d39f024af5e75a143fb55a6631889be3b2af48f9c44f1b869b57ae505da`.

## boot-v022 — audible speaker (Phase E1)

Same stock Image as v021. `/sbin/beep` sets volume **160** (TLV invert; stock 118 = hardware `0x31`), Power Up, Mono after PCM open, 800 ms louder square. Does **not** turn `ABOX SPUS ASRC3` Off. rcS dropbear `-R`. Odin AP: `\\192.168.168.110\media\saaios-boot-v022.tar`. Rollback: v021 tar. Path: [audio.md](audio.md).

**Flashed 2026-08-29, still silent.** Windows RNDIS appeared as Ethernet 5 but **Network cable unplugged** (gadget bound, no carrier). Cause: packed `rcS` had **CRLF**; BusyBox ash died at `do\r` so `ifconfig up` / telnetd never ran.

## boot-v023 — ABOX Sound Type SPEAKER

Jack does not mute SMA1303 (AUD3004X evdev only). Live v021 mixer: `ABOX Sound Type` = **VOICE**. v023 sets **SPEAKER**, keeps HP/EP off, logs `event7` hp/mic bits. Odin AP: `\\192.168.168.110\media\saaios-boot-v023.tar`. Rollback: v022. Path: [audio.md](audio.md).

Same CRLF `rcS` as v022 (not LIVE). Tar SHA256 `fcbbe633789aa02247effb81e21b866ed029e0763a848fcde6a124a343d34090`.

## boot-v024 — USB listeners + carrier

Same stock Image as v023. Banner `SaaiOS v024`. `rcS` is LF; telnetd `:23`/`:2323` and dropbear `-R` `:22` start **before** waiting on usb0. Background `ifconfig` + udhcpd when the iface appears (Windows carrier). Init retries empty configfs UDC. Keeps v023 `ABOX Sound Type=SPEAKER`. Odin AP: `\\192.168.168.110\media\saaios-boot-v024.tar`. Tar SHA256 `bbd04ddd02c7b284d1dd82b8959c1e9f6b7e1e9b978454d13ce87bf5179f834c`. Does not overwrite v021/v022/v023. Rollback: `saaios-boot-v021.tar` (last USB-good LIVE). Path: [audio.md](audio.md).

## boot-v025 — TONEGEN on RDMA3_A

GitHub: no one published a tinyalsa-without-HAL speaker recipe for SMA1303 / Exynos ABOX. Vendor `mixer_paths.xml` `media-speaker` always sets `ABOX RDMA3_A=BD_MIXER` (needs VPCM). Live idle is `None` — Calliope has no source on that slot. v025 keeps v024 USB + SPEAKER and routes `TONEGEN_1KHZ` into `RDMA3_A` (1 kHz from ABOX, not our 880 Hz PCM). Hold Power Up during write. Odin AP: `\\192.168.168.110\media\saaios-boot-v025.tar`. Tar SHA256 `4fffa8373264eb5196e68cffdb4293130ccc7dcf724af9c216f73cfdb227d2ec`. Does not overwrite v021–v024. Path: [audio.md](audio.md).

**LIVE 2026-08-29:** banner `SaaiOS v025`. `rndis0` 192.168.42.1 UP. Cards + `pcmC0D3p` present. First Power beep: DAPM `ABOX TONEGEN_1KHZ` / `RDMA3_A` / `RDMA3`, Calliope `NFB0`, SMA UNMUTE, `abox_rdma_trigger[3](1)`. Idle mixer: `RDMA3_A=TONEGEN_1KHZ`, `UAIF1=SIFS0`, `SPUS OUT3=SIFS0`, volume 160, Mode Mono. Telnet beep `pcm_writei` EIO. **Ear silent.** Mux-None hypothesis falsified — next is UAIF/SIFS1 or amp analog.

## boot-v031 — wpa_supplicant + wifi-join (Phase E3 closed)

Same stock Image as v030 (Maxwell fw + `/sbin/iw` + D1/SIFS1 play + gadget boot unchanged). Adds static aarch64 `/sbin/wpa_supplicant` 2.11 (nl80211, internal TLS, libnl-3.11), `/sbin/wpa_cli`, `/sbin/wifi-join` (args SSID + PSK, or `WIFI_SSID`/`WIFI_PSK`; conf only under `/tmp`; `udhcpc` + `/usr/share/udhcpc/default.script`). **Telnet-only — not at boot. No PSK in the image.** Does not run `usb-host`. Odin AP: `\\192.168.168.110\media\saaios-boot-v031.tar`. Tar SHA256 `dfd2fcafc898dced5ac4b90ea3e3ea6d14ebca63dd929bea277a67aa59dc819e`. Does not overwrite v021–v030. Rollback: `saaios-boot-v030.tar` or `saaios-boot-v029.tar`. Path: [hardware-control-plan.md](hardware-control-plan.md) E3. Next is E4.

After flash: `/sbin/wifi-join 'SSID' 'PSK'`. Phone can stay on v029 until then.

**LIVE 2026-08-29 (still on v029):** Wallbox WPA2 join via `/tmp/wpa_supplicant` only; `wlan0` **192.168.168.8/24**. RNDIS kept. PSK never packed.

## boot-v030 — static iw scan (Phase E3 scan)

Same stock Image as v029 (Maxwell fw + D1/SIFS1 play + gadget boot unchanged). Adds static aarch64 `/sbin/iw` 6.9 (nl80211 + libnl-3.11) and `/sbin/wifi-scan` (`wifi-up` then `iw dev wlan0 scan`). **Telnet-only — not at boot.** No join, no `wpa_supplicant`, no network config. Does not run `usb-host`. Odin AP: `\\192.168.168.110\media\saaios-boot-v030.tar`. Tar SHA256 `8eff38986a11fdc348fe1f6941f3aec3d8f2c9b7f108c2d3a70a9c13ab396bc3`. Does not overwrite v021–v029. Rollback: `saaios-boot-v029.tar`. Path: [hardware-control-plan.md](hardware-control-plan.md) E3.

**LIVE 2026-08-29 (still on v029):** banner `SaaiOS v029`; dropbear `:22` dead; `wget` of `/tmp/iw` from Windows RNDIS `192.168.42.15:8765`; `iw version 6.9`; `iw dev wlan0 scan` → 12 BSS / 11 named 2.4 GHz SSIDs. v030 packs that binary so the next flash does not need the HTTP push.

## boot-v029 — Maxwell firmware + wifi-up (Phase E3)

Same stock Image as v028 (D1/SIFS1 play + gadget boot unchanged). Packs vendor `/etc/wifi` production files into ramdisk `/vendor/etc/wifi` (`mx140.bin`, hcf, `hydra_config.sdb`, `slsi_reg_database.bin`, `platform.txt`). Skips `mx140_t.bin` / `mx140_t_*` / `mx140/debug`. `/etc/wifi` and `/system/etc/wifi` symlink to `/vendor/etc/wifi`. `/sbin/wifi-up` is **telnet-only** (`ifconfig wlan0 up` + operstate/MAC/dmesg tail) — **not at boot**. Does not run `usb-host`. Odin AP: `\\192.168.168.110\media\saaios-boot-v029.tar`. Tar SHA256 `a9e743caa02da38f6dcc5bc7adbf4ddd81dcb31b6d06eccc8deb49fe01bb2cf7`. Does not overwrite v021–v028. Rollback: `saaios-boot-v028.tar`. Path: [hardware-control-plan.md](hardware-control-plan.md) E3.

**LIVE 2026-08-29:** banner `SaaiOS v029`; kernel `4.19.111-27127798` stock DXJ2; `/sbin/wifi-up` → `wlan0` MAC `00:00:0f:08:0e:af`, UP, `operstate=down`. `mx140.bin` + hcf loaded. 2.4 GHz Y, 5 GHz N. MIB `NACHO_S612_A127F`. Scan is v030 / LIVE `iw` (above).

## boot-v028 — USB host/device scripts (Phase E2)

Same stock Image as v027 (D1/SIFS1 play unchanged). Adds `/sbin/usb-host` and `/sbin/usb-device`. Boot stays RNDIS device. `usb-host` unbinds gadget, writes typec `data_role=host` and dwc3 `id=0`, plants `/tmp/usb-role-host` so init does not rebind. **Never host at boot.** Way back: Power 2s reboot (or `usb-device` if a shell remains). Odin AP: `\\192.168.168.110\media\saaios-boot-v028.tar`. Does not overwrite v021–v027. Rollback: `saaios-boot-v027.tar`. Path: [hardware-control-plan.md](hardware-control-plan.md) E2.

**LIVE 2026-08-29 (device):** human flashed AP; banner `SaaiOS v028`; `rndis0` UP; telnet `:23`. UDC `configured`, `is_otg=1`. `/sbin/usb-host` and `/sbin/usb-device` present.

**LIVE 2026-08-29 (`usb-host` then reboot):** human ran `/sbin/usb-host`; RNDIS dropped. Photo after Power 2s: banner `SaaiOS v028`, `rndis0`, telnet `:23`/`:2323`, play `test.wav` still in log. Host/OTG stick not confirmed. Tar SHA256 `673c9b0ad6dd5be76ec4b4f5f9edc0f7218270b3389e5117b9612cc476fe28c8`.

## boot-v027 — WAV play on D1/SIFS1

Same stock Image + v026 speaker route (`pcmC0D1p` → SIFS1 → UAIF1 → SMA1303). `/sbin/play FILE.wav` (16-bit PCM). Packed `/usr/share/sounds/test.wav` (Ode to Joy). No auto-play; Power tap still beep. Odin AP: `\\192.168.168.110\media\saaios-boot-v027.tar`. Tar SHA256 `4fce1292e80bb7153c9113466a1f01ba34a8eef61c4de1e5dca9bbe6150b1101`. Does not overwrite v021–v026. Rollback: `saaios-boot-v026.tar`. Path: [audio.md](audio.md).

**LIVE 2026-08-29:** user said **работает**. `/sbin/play` WAV on D1/SIFS1. First heard via `/tmp` on still-running v026; then confirmed (likely after flashing v027, or still live `/tmp`). Replay: `/sbin/play /usr/share/sounds/test.wav`. Do not regress to `pcmC0D3p` / SIFS0.

## boot-v026 — pcmC0D1p / UAIF1=SIFS1

S10 GSI speaker used `pcmC0D1p` + `UAIF1 SPK=SIFS1`. This vendor XML has `route-rdma3-to-sifs1` (`SPUS OUT3=SIFS1`, `SIFS1=SPUS OUT3`) and `route-sifs1-to-uaif1`. v026 opens RDMA1, `SPUS OUT1=SIFS1`, `SIFS1=SPUS OUT1`, `UAIF1=SIFS1`, TONEGEN on `RDMA1_A`. Same LF `rcS` / USB. Stock kernel+DTB. Odin AP: `\\192.168.168.110\media\saaios-boot-v026.tar`. Tar SHA256 `094a8c06e4794f1094c36cbfb6ee0ee528449444f96f602048af289ba9c0baa1`. Does not overwrite v021–v025. Rollback: v025. Path: [audio.md](audio.md).

**LIVE 2026-08-29:** banner `SaaiOS v026`. Ear heard the tone. Telnet `:23`/`:2323`. Cards + `pcmC0D1p` present. Beep dmesg: DAPM `TONEGEN_1KHZ` / `RDMA1_A` / `SPUS OUT1` / `SIFS1` / `UAIF1 SPK`, `reset sifs1_cnt_val`, Calliope `NFB0`, SMA UNMUTE, `abox_rdma_trigger[1](1)`. Mixer after beep: `Sound Type=SPEAKER`, `UAIF1 SPK=SIFS1`, `SPUS OUT1=SIFS1`, `SIFS1=SPUS OUT1`, `RDMA1_A=TONEGEN_1KHZ`, `RDMA3_A=None`, `OUT3=RESERVED`, volume 160, Mode Mono, Power Up On. Working PCM is **D1 / SIFS1 / UAIF1**, not vendor `media-speaker` RDMA3/SIFS0 — that is why v021–v025 were silent (wrong FE/SIFS, not missing firmware). USB fix was CRLF `rcS` (v024). TONEGEN on RDMA3 proved the digital path. Dropbear `:22` empty until host keys written.

## Display / partitions (live)

- Graphics: **fb0** only, no `/sys/class/drm`
- `/dev/block/by-name`: `boot recovery dtb dtbo vbmeta vbmeta_system super` — **no `vendor_boot`**. Matches [partitions.md](partitions.md).

## Input map (boot-v009 ramdisk, telnet)

Live `/sys/class/input/event*/device/name` + hexdump. Volume/power confirmed; touch node exists but is silent while the screen is being touched.

| Node | Name | Role |
|------|------|------|
| event0 | `meta_event` | kernel meta, ignore |
| **event1** | **`gpio_keys`** | **volume** — `KEY_VOLUMEDOWN` `0x72`, `KEY_VOLUMEUP` `0x73` (hexdump **C**) |
| **event2** | **`sec-pmic-key`** | **power** — `KEY_POWER` `0x74` (hexdump **C**) |
| event3 | `sec_touchscreen` | Synaptics TCM TD4150 (`spi1.2`), fw `tsp_synaptics/td4150_a12s_boe.bin`. hexdump while touching = **no events** (v009–v011) |
| event4 | `sec_touchproximity` | ear/hover (same TSP IC, `support_ear_detect_mode`) |
| event5 | `grip_sensor_sub` | SAR grip |
| event6 | `grip_notifier` | grip helper |
| event7 | `AUD3004X Headset Input` | headset buttons |

### Why `sec_touchscreen` is silent (no Android init)

Not a missing evdev node. The vendor 4.19 kernel already registered `sec_touchscreen` (`/dev/input/event3`). Opening evdev is only needed to *read* events; the kernel still `input_report_abs` without userspace holding the fd.

This TD4150 driver (`drivers/input/touchscreen/synaptics/td4150/`) has **no** `/sys/class/sec/tsp/enabled`, **no** `tsp/input/`, and **no** `tsp/power/control`. Stock sysinput HAL nodes from other Samsungs are absent here. Factory interface is `/sys/class/sec/tsp/cmd` (+ `cmd_status` / `cmd_result` / `cmd_list`). Extra attrs: `sensitivity_mode`, `support_feature`, `prox_power_off`, `virtual_prox`.

**v011 live:** one `syna_tcm_resume` (`start(0) (1)` → `mod_resume` → `end`); no second resume; PID 1 logged `tsp skip (on or absent)`. `hexdump /dev/input/event3` still silent. `cmd_list` has `fw_update`, `get_fw_ver_{bin,ic}`, `check_connection`, `aot_enable`, `incell_power_control`, … — **no** `module_on_master`.

`incell_power_control,<0|1>` only sets `tcm_hcd->lcdoff_test` (factory LCD-off tests). It does **not** power the TSP. `aot_enable,<0|1>` is DT2W only. `check_connection` is a live production-test SPI poke (`TEST_CHECK_CONNECT`); OK/NG, not an enable. v012 packed; v013 is the fw-select fix; v014 skips boot factory `0x2a` and enables `REPORT_TOUCH`; v015 does not re-identify after skip-on-0x02.

After `mod_resume`, finger reports are gated in `touch_report()` by `init_touch_ok`, `lp_state != PWR_OFF`, and `lp_state != LP_MODE` (LP only delivers AOT). `PWR_ON` is 0. Resume with `in_hdl_mode` skips `do_reset` / `CMD_REZERO` (`USE_FLASH` is off); `wait_hdl` is a no-op if `host_downloading` is already 0. IRQ must deliver `REPORT_TOUCH`.

Stock vendor `etc/init/vendor.samsung.hardware.sysinput@1.3-service.rc` chowns TSP sysfs. On *other* Samsungs the HAL writes `tsp/input/enabled`; **this unit has no such node.**

DTBO `dtbo.img` (AP) has three SPI TSP drivers on one bus, selected by LCD-id GPIOs (BOM split, same as [hardware.md](hardware.md)):

| Driver (DT compatible) | Input name | a12s `synaptics,fw_name` / `novatek,fw_name` / `iliteck,fw_name` |
|------------------------|------------|------------------------------------------------------------------|
| `synaptics,tcm-spi` (`synaptics_tcm@2`) | `sec_touchscreen` | `tsp_synaptics/td4150_a12s_boe.bin`, `td4160_a12s_boe.bin`, `td4375_a12s_boe.bin` |
| `novatek,NVT-ts-spi` (`novatek@1`) | same | `tsp_novatek/nt36525_a12s_{csot,dtc,sharp}.bin` (+ `*_mp_*` factory) |
| `iliteck,ili9882x-spi` (`ilitek_ili9882x@0`) | same | `tsp_ilitek/ili7807s_a12s.hex`, `ili9882_a12s.hex` |

Kernel Image also has `syna_tcm_zeroflash` / “Failed to get firmware image”. DTB chosen cmdline includes `firmware_class.path=/vendor/firmware` (boot.img header cmdline does **not**). LCD overlays in this DTBO: `td4150_boe`, `td4160_boe`, `td4375_boe`, `nt36525b_{dtc,sharp,ctc}`, `ili7807s`, `ili9882q_boe`. **This unit (stock v011):** `parse_dt: fw name(tsp_synaptics/td4150_a12s_boe.bin)`. **v012 Project-Xed:** GPIO idx 5 picked **td4160** (`0x1AF240` vs `0xBA6220`); builtin 164236-byte blob, ROMBOOT failed. See [kernel-touch.md](kernel-touch.md).

**Those `tsp_*` files are not in stock SUPER.** Host unpacked AP `super.img.lz4` → vendor (503 MiB ext4) + scanned all SUPER raw extents (system/vendor/product/odm): **zero** `tsp_synaptics` / `tsp_novatek` / `tsp_ilitek` filenames. `/vendor/firmware/` has audio/camera/NFC/MFC only. They are also not builtin in the kernel Image (name string table only). Stock Android still has working touch ⇒ this SKU’s TDDI almost certainly runs **on-chip** firmware; host `request_firmware` is the update/zeroflash path (`/sdcard/Firmware/TSP/tsp_signed.bin`), not the reason event3 is empty.

Touch drivers also take `regulator_lcd_{vdd,reset,bl}` from the panel. `fb0` is already painting, so a regulator sysfs poke is the fallback, not the first step.

### Confirm on the phone now (telnet, v011 — do not re-unblank)

```sh
TSP=/sys/class/sec/tsp
printf 'check_connection' > "$TSP/cmd"; cat "$TSP/cmd_status"; cat "$TSP/cmd_result"
# expect cmd_status OK and cmd_result check_connection:OK (NG = not in app FW / SPI fail)
printf 'get_chip_name' > "$TSP/cmd"; cat "$TSP/cmd_status"; cat "$TSP/cmd_result"
printf 'get_fw_ver_ic' > "$TSP/cmd"; cat "$TSP/cmd_status"; cat "$TSP/cmd_result"
# get_fw_ver_ic:SY00000000 means app_info never filled
# Do not: incell_power_control (lcdoff_test), aot_enable (DT2W), enabled (absent), FB unblank
```

**v010:** 500 ms `FBIOBLANK` loop is gone. First `syna_tcm_resume` succeeded (`mod_resume` / `end`). ~100 ms later a second unblank (`ioctl FBIOBLANK` + sysfs `fb0/blank`) started `syna_tcm_early_resume start(0) (0) (0)` with no `end`; event3 stayed silent. A successful resume followed by a second resume likely leaves the TCM IC in a bad input state.

**v011:** unblank **exactly once** (ioctl only; no sysfs blank). Writes `input/enabled` only if it still reads `0` (on this unit the node is absent → `tsp skip`). Does **not** send `probe_enable` on `cmd`. Also checks `/sys/class/input/event3/device/enabled`. On-screen `SaaiOS v011`. Not flashed from make. **Do not pack `incell_power_control` into ramdisk** — it is `lcdoff_test`, not power-on.

This unit has no `enabled` sysfs. Blind `echo 1` to a missing node is a no-op; do not invent a second resume. If `check_connection` is NG, next is IC mode / on-chip FW, not another unblank.

**v011 live (tunnel, cable-on-glass + sysfs):**

- LCD TSP rails are already on: `gpio_lcd_rst`, `vdd_ldo28`, `gpio_lcd_bl_en` = enabled. No dedicated `reset-gpio` in DT (only `irq-gpio`, `cs-gpio`; `irq-on-state=0`).
- Cached factory reads: `get_chip_name:TD4150`, `get_fw_ver_ic:SY0104000E`, `get_x_num:15`, `get_y_num:34`, `get_threshold:120`. These do **not** increment IRQ.
- `check_connection` → `FAIL` / `NG`: `Timed out waiting for response (command 0x2a)` / `Failed to write command CMD_PRODUCTION_TEST`.
- IRQ `244 synaptics_tcm` stayed at **7** with a USB cable on the glass; kworker `tc:0 noise:0`; `hexdump event3` empty. IC is **deaf**, not “light touch”.
- **Do not** `unbind`/`bind` `synaptics_tcm_spi`. Live rebind (2026-08-19): `syna_tcm_remove` then probe `Incorrect header code (0x01)` / `Failed to detect the sensor` / `probe of synaptics_tcm.0 failed with error -5`. `sec_touchscreen` **disappears** until reboot. Re-probe also picked `td4160_a12s_boe.bin` (`lcd id(5)`, ap lcdtype `0x1AF240` vs dt `0xBA6220`) vs first-boot `td4150_a12s_boe.bin`.

Ramdisk cannot fix ATTN/`REPORT_TOUCH`. v011 resume log: `Interrupt already enabled` then **100 ms** then `mod_resume` (no `do_reset`) ⇒ `in_hdl_mode` + idle HDL, `USE_FLASH` off. `#define RESET_ON_RESUME` is in the `!in_hdl_mode` branch only — a no-op here. **Do not** unbind; **do not** `poweroff` (ramdisk `/sys/power/state` = `freeze mem`); **do not** re-run `check_connection` / cat `sensitivity_mode` (both CMD `0x2a`, timeout).

**v012 live:** Project-Xed Image + old ramdisk (`SaaiOS v011` on screen). `dmesg | grep synaptics_tcm` (not `TCM`). GPIO idx 5 → td4160 HDL → `do_reset` → 0x20 timeout; IRQ 244 = **6**. Packed DTB was stock. **v013:** force `td4150_a12s_boe.bin` on lcdtype mismatch; skip resume reset if already in FW mode. Live: idx(4), blob **147456**, `mod_resume`/`end`, first `0x2a` printed 15×34 (~975–1060), second `0x2a` timed out during resume (`cp short`); IRQ 244 stuck at **7**; `event3` empty; `0x45` failed then mode `0x02`. **v014:** skip boot factory rawdata; treat `0x02` after failed `0x45` as on-chip FW; `CMD_ENABLE_REPORT` `REPORT_TOUCH`. Live **regressed**: first `0x20` after `0x45` succeeded (config `0104000E`), probe sent `0x20` again, timed out, `probe failed -62`, IRQ and `event3` gone. **v015:** after skip-on-0x02, no second `0x20` / identify; continue to touch init. Live: probe ok, `event3` exists, `0x05` timed out, `0x25` padding `0xa0`, IRQ 244 stuck at **5**, hexdump empty. **v019 since43 live:** one-shot 0x45 starts HDL firmware (`mode=0x02`); `touch_init` ok; IRQ live (`gpio=1 held=0`); **0x30 APP_CONFIG `-62`**; IRQ 244 = 3; **event6** silent. Full sinceN log and since44 one-shot 0x30: [kernel-touch.md](kernel-touch.md). `make -f os/Makefile kernel boot-v019`. **Do not flash from make.**

Optional later: `mkdir -p /lib/firmware /vendor/firmware` and `ln -s /lib/firmware /vendor/firmware` so `firmware_class.path=/vendor/firmware` works for grip/Wi‑Fi. Packing the `tsp_*` names above is a no-op until we obtain the actual files from somewhere other than this SUPER.

Host SUPER unpack (already in gitignored `os/build/stock-super/`, do not flash): from AP zip member `super.img.lz4` (3.3 GiB) → `lz4 -d` → Android sparse → lpunpack `vendor` / `odm`. `dtbo.img.lz4` is in the same AP tar (before super).

## Stock firmware on R620 (`/srv/media`)

PDA **A127FXXSDDXJ2**, CSC **OLE** (`A127FOLEDDXJ2`). Same bit **D**.

| File | SHA256 |
|------|--------|
| `BL_A127FXXSDDXJ2_…zip` | `8b694cc4f3f21f0a585d35d21c12ca0bc92241b071c8d3de61401b956697b2b1` |
| `AP_A127FXXSDDXJ2_…zip` | `aa218cc8ad33b221cc195f9d480741abf787fe12a9026f6dbf5dbecae3d25a6e` |
| `CP_A127FXXSDDXJ2_…zip` | `808a53f01874173542cba197afc29972348ade214de0edf3d089eadbc148c3b8` |
| `CSC_OLE_A127FOLEDDXJ2_…zip` | `2e88e2844b09082ea6a667563cd62642f83bcac5e0dac8f06fa7114c4eb3202d` |

Extracted rollback images (gitignored): `os/images/stock/A127FXXSDDXJ2/`

| Image | Size | SHA256 |
|-------|------|--------|
| `boot.img` | 44 MiB | `c0bc77e6f22a5ee57d20e963506371d45653ea322a00fb585adfde3e1ca6705f` |
| `vbmeta.img` | 9.6 KiB | `327621a74fb4d7f70598862201f77fdf09acb35997479fcaf1bcf9278f9f8b64` |
| `up_param.bin` | 839680 | `41ab8f0d5735aa0b121714f31bf62723b24a613d13835cdc061439dc00c6274f` |
| `up_param.bin.lz4` | 712735 | `135959a8f4945bedb69eafd5fde9a8edc8eae1c4829deb9f0932049d502e7158` |

`boot.img` cmdline: `androidboot.hardware=exynos850 androidboot.selinux=enforce loop.max_part=7`. Page size 2048. AP tar also has `recovery.img.lz4`, `dtbo.img.lz4`, `vbmeta_system.img.lz4`. **No `dtb.img` in AP** (DTB partition may live in BL).

## Download mode (photo still useful)

OEM unlock done. Every boot still shows the sboot **orange unlock warning** (Power to continue) until `UP_PARAM` is replaced. That is not the DECON leftover, and not Download mode.

## Two Samsung pictures (do not mix them)

| What | Where | When | Removed by |
|------|--------|------|------------|
| **Unlock warning** (orange state, “PRESS POWER KEY TO CONTINUE”) | **UP_PARAM** JPEG `svb_orange.jpg` (+ `booting_warning.jpg`) | sboot, **before** kernel | PARAM nologo tar below. **Not** `boot.img`. |
| **Boot splash** (SAMSUNG / Galaxy A12 / Knox) | UP_PARAM `logo.jpg` / `letter.jpg`; then **left on DECON** until userspace paints `fb0` | sboot paints it; kernel/init already overwrite it (v005 color cycle, v006 yellow/BGR, v007 cyan) | Blanking those JPEGs makes the leftover black. Speeding up our `fb0` paint does **not** skip the Power-key warning. |

Stock BL `A127FXXSDDXJ2` contains `up_param.bin.lz4` (no `param.bin`). Inner file is a ustar of 27 JPEGs. Download-mode teal art (`download.jpg`, `warning.jpg`, `warning_svb.jpg`, `device_unlock.jpg`, …) is left stock on purpose.

Host artifacts (`make -f os/Makefile up_param`, **does not flash**):

| File | SHA256 | Odin |
|------|--------|------|
| `\\192.168.168.110\media\saaios-up_param-nologo.tar` | `9b73fa89afb07b74844136a5552777b044d46cddf31600f36eaa9460dbcda79c` | **BL** slot. Inner name `up_param.bin.lz4` only. |
| `\\192.168.168.110\media\saaios-up_param-stock-restore.tar` | `befb5de3c8cbecfd5f95e6fe463a1304a0619ea3deb798ed89633669bcb76879` | **BL** slot. Bit-identical stock lz4 from the BL zip. |

Nologo replaces `svb_orange.jpg`, `booting_warning.jpg`, `logo.jpg`, `letter.jpg` with same-size black 3-component JPEGs. sboot may still wait on Power; the picture goes black. Never put this tar in AP. Never flash the full stock BL zip (that writes sboot/TZ). Rollback: restore tar, same BL slot. Risk: [known-risks.md](known-risks.md) §16.
