# SM-A127F — TD4150 bring-up (v019 since44)

Human flashes Odin **AP** only. **Do not flash from make.** **Do not unbind** `synaptics_tcm_spi`. **Never pulse `gpio_lcd_rst`** (`reset-gpio` is `-1`).

Driver: `os/third_party/kernel_samsung_a12/drivers/input/touchscreen/synaptics/td4150/` (gitignored tree; every `.c` change is an Odin AP). Builtin `=y`. Pack: `make -f os/Makefile kernel boot-v019` → `/srv/media/saaios-boot-v019-sinceN.tar`. Banner: `SaaiOS v019 sinceN` (`os/init/init.c`, `SUBVERSION_V019` in `os/Makefile`). Debug prefix: `SAaiOS_TOUCH_DBG`.

Panel: `td4150_boe_a12s_default`, fw `tsp_synaptics/td4150_a12s_boe.bin` (**147456**). SPI `spi1.2` @ 7 MHz mode 3. IRQ **244** (`exynos7_wkup_irq_chip`, Level, `synaptics_tcm`). Input: **`/dev/input/event6`** (`sec_touchscreen`) — not event3.

Init pings Exynos WDT ~2×/s; dmesg wraps after ~16 min. Capture greps in the first 10 s. Pattern `0x30` also hits USB phy / battery / muic — keep `SAaiOS_TOUCH_DBG` in the grep.

---

## Stuck here (2026-08-20)

Firmware **does** start. Touch **does not**.

| What works | Evidence |
|------------|----------|
| Stock one-shot `CMD_ROMBOOT_DOWNLOAD` **0x45** (APP_CODE 97280 + 14-byte header, no CRC, one SPI message ~97296 bytes) | since39+ |
| IDENTIFY after 0x45 | `mode=0x02` `remaining=0 (firmware started)` |
| `touch_init` | `retval=0`, `sec_touchscreen` = **event6** |
| IRQ unmasked after 0x02 | since42: `gpio=1 irq_en=1 held=0` |

| What fails | Evidence |
|------------|----------|
| `CMD_DOWNLOAD_CONFIG` **0x30** APP_CONFIG (~4096 + HDL v2 header) | since43: `retval=-62` (~1 s), `mode=0x02`, `gpio=1`, `irq_en=1`, `held=0` |
| REPORT_TOUCH / input | IRQ 244 stuck at **3**; `dd if=/dev/input/event6` times out |

since43 sent 0x30 **with IRQ live**. That rules out the since41 bug (`syna_tcm_romboot_drain_attn()` `disable_irq` and never release). The IC simply never pulled ATTN.

Likely cause of the since43 timeout (packed as **since44**, not confirmed on device yet): IDENTIFY `0x02` applied `WR_CHUNK_SIZE=512`, so ~4098-byte 0x30 went out as **CONTINUE_WRITE** frames. That is the same split that kept 0x45 in RomBoot until since39 one-shot. Header comment: *chunk size will not apply in HDL sensors*; `HDL_WR_CHUNK_SIZE=0` (unlimited). s3c64xx already splits DMA at `PACKET_CNT_MAX` **with CS held**.

If since44 logs `0x30 one-shot chunks=1` and **still** `-62` with IRQ live: the IC is ignoring 0x30. Then the leftover `0x1b` (`10 00` = hdlv=2, **all `need_*=0`**) is the next lead — firmware may not want APP_CONFIG, and something else is blocking REPORT_TOUCH.

### Next flash

`/srv/media/saaios-boot-v019-since44.tar` (AP only).

```sh
dmesg | grep 'SaaiOS v019'
dmesg | grep -E 'since44 probe|firmware started|touch_init after IDENTIFY|irq live|0x30 one-shot|0x30 APP_CONFIG|0x1b complete waiter|leftover 0x1b'
grep synaptics /proc/interrupts
timeout 3 dd if=/dev/input/event6 bs=24 count=1 2>/dev/null | od -An -tx4
```

Success: `chunks=1`, `0x30 APP_CONFIG retval` not `-62`, mode `0x01` (or touch in `0x02`), IRQ 244 grows on tap, events on **event6**.

---

## Settled protocol

- Modes: `MODE_APPLICATION_FIRMWARE=0x01`, `MODE_HOSTDOWNLOAD_FIRMWARE=0x02`, `MODE_ROMBOOTLOADER=0x04`. `IS_FW_MODE` is 0x01 and 0x02.
- Cold-boot IDENTIFY-24: `td4150_rom-10.0`, mode `0x04`, **max_write=1024**.
- Bin 147456: `ROMBOOT_APP_CODE` 97280 crc `0x5a555e91` first=`55 aa 01 00`; `APP_CONFIG` 4096 flags=1; `DISPLAY` 2048; `BOOT_CONFIG` 256.
- Stock 0x45: **14 reserved** (`[0]=size>>16=0x01`, rest 0) + APP_CODE. TCM u16 wraps (`31758`); 24-bit length via `[0]`. **No CRC appended.**
- **`HDL_WR_CHUNK_SIZE` is 0 (unlimited).** Unpatched `write_message` for 0x45 uses `chunk_space = remaining_length` — one SPI message. PAGE_SIZE + `CONTINUE_WRITE` (~4080) was **wrong**; IC stayed RomBoot + `0x1b`.
- Stock after `write_message(0x45)==0` is `switch_mode(BOOTLOADER)` → **0x42** from RomBoot. **This IC’s 0x45 completes with IDENTIFY `0x02`**, so **do not send 0x42** after that.
- `firmware_status` packed u16: `invalid_static:1 need_disp:1 need_app:1 hdl_version:4 need_osh:1 reserved:8`. Payload `10 00` = **hdlv=2, all need_*=0**.
- Stock 0x1b is a **report**, not a command response. This IC uses 0x1b as HDL ack for **0x45**; 0x1b must **not** complete 0x42/0x40. since43 also lets 0x1b/IDENTIFY complete a **0x30** waiter.
- Touch probe is `device_initcall`; DECON is `late_initcall`. **Defer 0x45 until first panel up.**
- Stock never sends `CMD_ENABLE_REPORT` 0x05 in probe/touch/resume. No `do_polling`.

---

## Dead ends (do not revive)

- ENABLE_REPORT **0x05**; `do_polling`; pulse `lcd_rst`; 24 separate `write_message(0x45)` with per-chunk u16=4080; PAGE_SIZE CONTINUE_WRITE; treat IDENTIFY after chunk 0 as firmware-started; printk on every 0x1b; unbounded drain; `touch_init` in 0x04; 97k retry; RUN_APPLICATION **0x14** after 0x45; GET_ROMBOOT_INFO **0x40** while leftover 0x1b; drain-until-idle before 0x42; **unmask IRQ during idle 0x1b in RomBoot** (since37 storm ~8k/s); **`syna_tcm_romboot_drain_attn()` after 0x02** (since41: 0x30/identify `-62`, IRQ stuck at 3).

---

## Attempt log

### boot-v012 … v018 (wrong blob / on-chip 0x02 / ENABLE_REPORT)

These builds did **not** land a successful 0x45. Early logs that look like “mode 0x02 / config 0104000E” were **on-chip leftover**, not host-downloaded APP_CODE.

| Tag | Change | Live result |
|-----|--------|-------------|
| v011 | Stock kernel + our ramdisk | `event3` exists, silent. IRQ 244 = 7. Resume skips `do_reset` (`in_hdl_mode` idle). |
| v012 | Project-Xed Image; HDL-idle resume → `do_reset` | GPIO idx 5 → **td4160** 164236-byte blob. 0x45 fail. `0x20` timeout. IRQ = 6. Packed DTB was stock (not the bug). |
| v013 | lcdtype mismatch → **td4150** 147456; skip resume reset if already FW mode | idx 4, blob 147456. `0x45` still fail then mode 0x02. Factory `0x2a` raced resume. IRQ = 7. event3 empty. |
| v014 | skip boot `sec_run_rawdata`; treat 0x02 after failed 0x45 as on-chip FW; ENABLE_REPORT 0x05 | First `0x20` ok (`0104000E`); **second** identify `0x20` timed out; probe **-62**; IRQ and event3 gone. |
| v015 | no second 0x20 after skip-on-0x02 | Probe ok, event3 exists, **0x05 timed out**, 0x25 padding `0xa0`, IRQ = 5. |
| v016 | restore first 0x20; SPI drain leftover; 0x05 once | Probe **regressed**: drain at cold boot → `Failed to detect the sensor`. |
| v017 | no drain in `read_message` retry | Probe ok; 0x05 timeout then `SPI drain … 0xa5`; 0x25 timed out twice; event3 silent. |
| v018 | salvage 0x05 with full `read_message`; no early 0x05; default report config; dropbear | Still no REPORT_TOUCH. Path abandoned: stock never sends 0x05. |

v012 GPIO table (DTBO overlays; we do not flash DTBO):

| idx | lcdtype | fw_name |
|-----|---------|---------|
| 0 | `0x7A6220` | `td4375_a12s_boe.bin` |
| 4 | `0x3A6220` | `td4150_a12s_boe.bin` |
| 5 | `0xBA6220` | `td4160_a12s_boe.bin` |

`get_lcd_info("id")` = `0x1AF240` matches **neither**. GPIO `id3|id2|id1 = 1|0|1` → idx 5 unless we force td4150.

### boot-v019 sinceN (RomBoot 0x45 → HDL 0x02)

Builtin td4150 driver, deferred 0x45 until panel up (since38), then stock one-shot 0x45 (since39).

| Tag | Change | Live result |
|-----|--------|-------------|
| since19–22 | On-chip 0x1b `10 00`; force/skip/sync **0x30** APP_CONFIG | 0x30 `-62`, gpio=1. Packet was 4096+2, not 45568. IC deaf after leftover 0x1b. **Before** working 0x45. |
| since23–30 | Chunked 0x45 (PAGE_SIZE / per-chunk u16 / CONTINUE_WRITE) | Stayed **RomBoot 0x04**. IDENTIFY mid-chunk is not firmware-started. |
| since31–32 | Drain after send; stock post-0x45 **0x42** | Still RomBoot. 0x1b is not a 0x42 ack. |
| since33 | 0x1b completes 0x45 waiter | Needed so one-shot 0x45 can finish. Must not complete 0x42/0x40. |
| since34 | Wait IDENTIFY after 0x42 | 0x42 from RomBoot: this IC never leaves 0x04 that way. |
| since35–36 | 0x40 GET_ROMBOOT_INFO | Leftover 0x1b made 0x40 look OK with empty payload. Still RomBoot. |
| since37 | Defer 0x45 until panel; **unmask IRQ** on idle 0x1b in RomBoot | IRQ storm ~8k/s. |
| since38 | Probe: **no** 0x45. Full 0x45+0x42 on first `panel_enabled`. Hold IRQ on idle 0x1b | IRQ=5. Still RomBoot (chunked 0x45). |
| **since39** | **Stock one-shot 0x45** (`chunks=1 xfer0=97296 wr_chunk=0`) | **`0x45 complete IDENTIFY mode=0x02 (firmware started)`** |
| since40 | Bring-up after 0x02: `touch_init`; **event6** | `identify -62` (~1 s) with `gpio=0`; leftover 0x1b locked skip-0x30. IRQ=3. |
| since41 | Don’t lock skip-0x30 on leftover 0x1b in 0x02; forced 0x30 at gpio=1 | 0x30 and identify **`-62`**. Cause: **`drain_attn` held IRQ**. IRQ=3. |
| **since42** | After 0x02: `touch_init`, **IRQ live**, no drain, no 0x30, no identify wait | `gpio=1 irq_en=1 held=0`. IRQ=3 stable. **No REPORT_TOUCH.** |
| **since43** | 0x30 APP_CONFIG with IRQ live; 0x1b/IDENTIFY complete 0x30 waiter | `0x30 APP_CONFIG retval=-62` ~1 s, gpio=1, held=0, IRQ still 3. |
| since44 | **0x30 one-shot** (no CONTINUE_WRITE at 512); HDL wr_chunk=0 in mode 0x02 | Packed `/srv/media/saaios-boot-v019-since44.tar`. **Not confirmed on device.** |

---

## Patches

Apply (`make -f os/Makefile kernel-patch`). Marker grep decides “already applied”; kernel tree is gitignored so the live `.c` edits live only on the build host.

v012–v018: `syna-tcm-resume-hdl-idle` … `syna-tcm-enable-report-salvage` (see git history of this file).

v019 chain (names match `os/patches/syna-tcm-*.patch`): fw-sel td4150, onchip-fw-touch, skip-second-identify, first-appinfo-resync, drain-not-at-probe, enable-report-salvage, ATTN-before-enable-report, continued-read-*, enable-report-plen-clear, read-message-plen-zero, drained-status-ok-only, enable-report-empty-ack, probe-enter-log, skip-hdl-identify, stop-poll-dump, oneshot-drain, onchip-clear-hdl-reinit, hdl-status-config, sync-download-config, hdl-complete-reinit, hdl-send-0x30, romboot-chunk-0x45 through **romboot-0x30-oneshot** (since44).

`RESET_ON_RESUME` is commented out and only lives in `!in_hdl_mode`.

---

## Driver files

Built path: `drivers/input/touchscreen/synaptics/td4150/`

| File | Why |
|------|-----|
| `synaptics_tcm_spi.c` | `parse_dt` fw_name / lcdtype / GPIO idx |
| `synaptics_tcm_core.c` | `write_message` one-shot 0x45/0x30; IDENTIFY; IRQ hold/release |
| `synaptics_tcm_zeroflash.c` | ROMBOOT HDL; defer 0x45; APP_CONFIG 0x30 |
| `synaptics_tcm_touch.c` | `touch_report()` gated by `init_touch_ok` |

In-tree blobs: `firmware/tsp_synaptics/td4150_a12s_boe.bin` and `td4160_a12s_boe.bin`.

---

## Clone / DTB / toolchain

Reuse [maazm7d/kernel_samsung_a12](https://github.com/maazm7d/kernel_samsung_a12) (a12s / Exynos 850 / S5E3830, **not** Helio A125F).

| | |
|--|--|
| Path | `os/third_party/kernel_samsung_a12` (**gitignored**) |
| Defconfig | `arch/arm64/configs/exynos850-a12snsxx_defconfig` |
| SoC | `CONFIG_SOC_EXYNOS3830=y` |
| TSP | `CONFIG_TOUCHSCREEN_SYNAPTICS_TCM=y` + SPI; Makefile builds `synaptics/td4150/` |
| Version | `4.19.111` |
| Live LOCALVERSION | `4.19.111-Project-Xed-KernelSU-Next+SUSFS` (clang 9.0.1) |

Caveat: this tree is **Project-Xed / KernelSU / SUSFS**, not a clean OSS dump of **A127FXXSDDXJ2**. Closest official zip on [opensource.samsung.com](https://opensource.samsung.com/uploadSearch?searchValue=SM-A127F) is **A127FXXSDDXJ6**.

`os/bootimg/bootimg.py pack --kernel` replaces only the Image. **DTB stays stock** `os/build/stock-boot/dtb` from **A127FXXSDDXJ2** `boot.img`. Image has **no** appended FDT. Project-Xed `exynos3830.dtb` is **not** in the boot image. TSP `synaptics,fw_name` / `synaptics,lcdtype` come from **DTBO** (stock `dtbo` partition — we do not flash DTBO).

```sh
make -f os/Makefile kernel boot-v019
```

clang-9 on `PATH`, `O=os/build/kernel-out`. Copies Image to `os/build/kernel-Image`. `CONFIG_RTL8188EU` stays off. `make flash` still **refuses**.

After USB net is up, Windows reverse-forwards telnet and SSH (pubkey only; keys in `os/init/ssh/authorized_keys`):

```powershell
ssh -i $env:USERPROFILE\.ssh\saaios-odin-win -N `
  -R 2323:192.168.42.1:23 -R 2222:192.168.42.1:22 `
  mike@192.168.168.110
```

From R620: `ssh -i ~/.ssh/saaios_phone -p 2222 -o StrictHostKeyChecking=no root@127.0.0.1`. Direct from Windows: `ssh -i $env:USERPROFILE\.ssh\saaios-odin-win root@192.168.42.1`. Host keys are generated on first dropbear start (`-R`); expect TOFU each boot.
