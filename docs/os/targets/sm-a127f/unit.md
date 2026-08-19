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

`incell_power_control,<0|1>` only sets `tcm_hcd->lcdoff_test` (factory LCD-off tests). It does **not** power the TSP. `aot_enable,<0|1>` is DT2W only. `check_connection` is a live production-test SPI poke (`TEST_CHECK_CONNECT`); OK/NG, not an enable. No v012 packed.

After `mod_resume`, finger reports are gated in `touch_report()` by `init_touch_ok`, `lp_state != PWR_OFF`, and `lp_state != LP_MODE` (LP only delivers AOT). `PWR_ON` is 0. Resume with `in_hdl_mode` skips `do_reset` / `CMD_REZERO` (`USE_FLASH` is off); `wait_hdl` is a no-op if `host_downloading` is already 0. IRQ must deliver `REPORT_TOUCH`.

Stock vendor `etc/init/vendor.samsung.hardware.sysinput@1.3-service.rc` chowns TSP sysfs. On *other* Samsungs the HAL writes `tsp/input/enabled`; **this unit has no such node.**

DTBO `dtbo.img` (AP) has three SPI TSP drivers on one bus, selected by LCD-id GPIOs (BOM split, same as [hardware.md](hardware.md)):

| Driver (DT compatible) | Input name | a12s `synaptics,fw_name` / `novatek,fw_name` / `iliteck,fw_name` |
|------------------------|------------|------------------------------------------------------------------|
| `synaptics,tcm-spi` (`synaptics_tcm@2`) | `sec_touchscreen` | `tsp_synaptics/td4150_a12s_boe.bin`, `td4160_a12s_boe.bin`, `td4375_a12s_boe.bin` |
| `novatek,NVT-ts-spi` (`novatek@1`) | same | `tsp_novatek/nt36525_a12s_{csot,dtc,sharp}.bin` (+ `*_mp_*` factory) |
| `iliteck,ili9882x-spi` (`ilitek_ili9882x@0`) | same | `tsp_ilitek/ili7807s_a12s.hex`, `ili9882_a12s.hex` |

Kernel Image also has `syna_tcm_zeroflash` / “Failed to get firmware image”. DTB chosen cmdline includes `firmware_class.path=/vendor/firmware` (boot.img header cmdline does **not**). LCD overlays in this DTBO: `td4150_boe`, `td4160_boe`, `td4375_boe`, `nt36525b_{dtc,sharp,ctc}`, `ili7807s`, `ili9882q_boe`. **This unit:** live dmesg `parse_dt: fw name(tsp_synaptics/td4150_a12s_boe.bin)` — Synaptics TCM TD4150 BOE.

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
