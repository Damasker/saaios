# SM-A127F — TD4150 bring-up (canonical)

Human flashes Odin **AP** only. **Do not flash from make.** **Do not commit** unless asked. **Do not unbind** `synaptics_tcm_spi`. **Never pulse `gpio_lcd_rst`** (`reset-gpio` is `-1`). **Do not overwrite** `/srv/media/saaios-boot-v019-since54.tar` … `...-since75.tar`. **Do not flash since66.** **Do not flash since73** (only 0x05; superseded). **Do not flash since75** if the auto-0x05 image was packed — superseded by since76 menu. **Do not flash since75.**

**Opcode experiments on the maze are STOPPED.** Maze is retired as a flash target. **since56 LIVE (settled):** 0x40-only IDENTIFY is **byte-identical** to post-0x45 IDENTIFY from since49/54 (`TD4150-12.0.12` / `38 df 2b 7d` / packrat **2100027192** / mode **0x02** / leftover `0x1b` `10 00` / silence). **since57 LIVE (settled):** clean RomBoot **0x04** stock 0x45 (no 0x40, SPI actual=97297) still yields the same IDENTIFY; `write_message` treats that IDENTIFY as reset/CMD_ERROR (`-5`) so stock `switch_mode(BOOTLOADER)` never runs. **since59 LIVE (settled):** delayed 0x45 after both panel callbacks + rails + 400 ms; still RomBoot 0x04 and known-good APP_CODE immediately before 0x45; after full SPI, same `TD4150-12.0.12` / mode 0x02. Panel delay is **falsified**. **since60 LIVE (discriminator missed):** banner since60; IDENTIFY before 0x45 still RomBoot 0x04; after same `TD4150-12.0.12` / 2100027192 — **but** `syna_corrupt_app=0`, RAM APP_CODE unmodified, sha256 **match=1**. boot.img header has `syna_corrupt_app=1`; Samsung BL did not pass it. **since61 LIVE (settled):** compile-time FORCE one-byte XOR APP_CODE[4] `0x00→0x01`; sha256 **match=0**, crc32 **`0x67cba83c`**, TX `APP_CODE[4]=0x01`; after 0x45 still `TD4150-12.0.12` / mode **0x02** / packrat **2100027192**. Offset 4 may be unused; **not** enough to conclude ROM ignores the payload. **since62 LIVE (settled):** in-place APP_CODE[0] magic `0x55→0x54` (`54 aa 01 00`, match=0, crc32 **`0xe3b77bae`**, TX `APP_CODE[0]=0x54`); still same IDENTIFY. Magic is **not** a launch gate. **since63 LIVE (settled):** full-zero APP_CODE really went on the wire (SPI 97297, nonzero=0) and IDENTIFY was still `TD4150-12.0.12` / 2100027192 / 0x02 / `-5`. Payload content is **not** what produces that IDENTIFY **under oneshot 97k**. New hypothesis: oneshot SPI 97297 exceeds IDENTIFY `max_write=1024`, and the 16-bit TCM length overflows (`payload 97294=0x17c0e`, header `0e 7c`=31758). `spi_sync actual=97297` only means the host clocked that many bits. Do **not** treat IDENTIFY as STATUS_OK. **since64 LIVE (partial):** chunked 0x45 produced leftover `0x1b` `10 00` while **mode stayed 0x04** (not on-chip HDL 0x02). 1030 identical REPORT_STATUS in 68 ms (~15k/s) flushed probe/0x45/wr_chunk from dmesg. `host_downloading=0` `cmd=0x00`. **since65 LIVE (settled, reinterpreted):** chunked 0x45 from a generic TCM writer view was **wrong**. APP_CODE split correctly (96 packets: 0x45 + 95× CONTINUE_WRITE), but after the last packet: no Identify, no app launch, IC stayed `td4150_rom / 0x04`, ATTN held asserted (gpio=0), one IRQ entry `irq_n=2` with growing `loop`, same `0x1b 10 00` forever, emergency hold after 32 leftover reads, 0x45 timed out `-ETIME`. CONTINUE_WRITE only honors the 16-bit TCM length (`0x7c0e`) and breaks the extended 0x45 transaction (`payload=0x17c0e`, `reserved[0]=0x01`). `HDL_WR_CHUNK_SIZE=0` is the **0x45 one-shot** mode. **since66** went the opposite way (keep chunking, treat leftover 0x1b as waiter STATUS_OK) — **do not flash it.** **since67 LIVE (settled):** oneshot 0x45 + IDENTIFY-as-STATUS_OK worked (`write_message retval=0`, SPI 97297, sha256 match=1). After 0x45 the IC is already HDL **0x02**, so stock `switch_mode(BOOTLOADER)` sent **0x1f** (not 0x42) and timed out `-ETIME`. ATTN after oneshot is healthy: one leftover `0x1b` then idle (unlike chunked since65). IDENTIFY already announced configs. **since68 LIVE (settled):** skip `switch_mode` when already HDL **0x02** worked. No `Command = 0x1f`, no `-62`. Leftover `0x1b` `10 00` queued stock REINIT (`need_*=0`); we **blocked** `HELP_SEND_REINIT` / 0x25. irq_cnt=2 rx_cnt=3 gpio=1 through t=33s, `tc:0`, no event6. Silence was the skipped leftover REINIT, not a failed skip-0x1f. Old 0x25 timeout was since55 probe-after-0x40, not this post-0x45 HDL-idle state. **since69 LIVE (settled):** leftover REINIT after skip switch_mode **worked**. Gate `IS_FW_MODE` (0x01 or **0x02**) AND `host_downloading==0` is correct. `identify(false)` / 0x20 retval=0 `app_status=OK`; 0x25 retval=0 128 bytes; `sec_touchscreen` **input8 / Handlers=event6** (`/dev/input/event6` exists; event8 was the wrong path). `touch_init retval=0 init_touch_ok=1`. **No** 0x05 / 0x26. Finger + power-button off/on: irq 244 **stuck at 4** through t=1500s. Power key on BusyBox initramfs does **not** hit `syna_tcm_resume` (empty after `AFTER_RESUME`). **since70 LIVE (settled, falsified):** late `touch_resume()` ran (`lp_state=0 boot_resume=0 mode=0x02 fb_ready=2 init_touch_ok=1 host_downloading=0 irq_cnt=4`), `mod_cb->resume n=0`, `touch_resume retval=0`, **no** extra Command after 0x25 (no 0x05, no 0x26). After touches: irq 244 **still 4**, no `REPORT_TOUCH`, `od event6` empty. Late resume sent **no SPI**. Keep it as a known no-op after 0x05. **since71 LIVE (missed 0x26):** banner since71, no `Command = 0x26`, no `0x26 start`, irq 244 still **4** before and after touch. Helper ran after `UNLOCK_BUFFER(tcm_hcd->config)`; 0x25 failure / reporting skip had no `0x26 skip` log. **since72 LIVE (falsified):** kmemdup-before-unlock **sent** `Command = 0x26` (`0x26 start len=128 first=1a 08 10 08… wr_chunk=512`); `0x26 retval=-62 timeout=1 response=0xff resp_len=0`; IRQ 244 stayed **4** through the 1s wait **and** after touches. ATTN never asserted — HDL silent on SET (not STATUS_ERROR). Device banner still said `since71` but logs match the since72 fix. Do **not** retry 0x26. **since73** packed `0x05 ENABLE_REPORT` only — **do not flash it.** **since74 LIVE (settled):** 0x23 TX `23 01 00 01` spi_len=4 was **correct**; `retval=-62` response=`0xff` resp_len=0; IRQ 244 stayed **4**; 0x05 skipped (`0x23_failed`). Hang is **not** a TX/framing bug. That does **not** prove all payloads are broken — 0x23/0x26 may simply be ignored in `MODE_HOSTDOWNLOAD` 0x02. Native payload command for this mode is **0x30 DOWNLOAD_CONFIG** (next image if 0x05 times out and 0x20 still works). **since75** sends **0x05 ENABLE_REPORT** `0x11` after 0x25 then live `identify(false)` / 0x20. No 0x23. Skip log is **`HDL firmware running`** (mode 0x02 is `MODE_HOSTDOWNLOAD_FIRMWARE`; 0x01 is application). That IDENTIFY is **not** proof downloaded APP_CODE launched (since56). Do not revive maze PAGE_SIZE chunking. Do not add `0x1f` / `0x42-from-0x02` / helper IDENTIFY / retry-0x26 / `lcd_rst` tests.

Tars: `/srv/media/saaios-boot-v019-sinceN.tar` (`mike:media` 664). Banner: `SaaiOS v019 sinceN` (`os/init/init.c`, `SUBVERSION_V019` in `os/Makefile`). Debug prefix: `SAaiOS_TOUCH_DBG`. Console: `SaaiOS v019 sinceN`.

Init pings Exynos WDT ~2×/s; dmesg wraps after ~16 min. Capture greps in the first 10 s. Pattern `0x30` also hits USB phy / battery / muic — keep `SAaiOS_TOUCH_DBG` in the grep.

---

## Hardware / goal

Samsung **SM-A127F** (a12s / Exynos 850), SaaiOS. Unit PDA **A127FXXSDDXJ2**. Panel **`td4150_boe_a12s_default`**.

| | |
|--|--|
| Controller | Synaptics **TD4150** over SPI **`spi1.2`**, 7 MHz mode 3 |
| IRQ | **244** (`exynos7_wkup_irq_chip`, **LEVEL_LOW** + **ONESHOT**, `synaptics_tcm`) |
| ATTN GPIO | **gpio=0 active**, **gpio=1 idle** (LEVEL_LOW). After leftover `0x1b`, gpio=1 is idle, not an ATTN request |
| Input after firmware start | **`sec_touchscreen`** sysfs `input8`, evdev **`/dev/input/event6`** (Handlers=event6). Same SPI also has input9→event7. Stock/v011 used event3. |
| Firmware | `tsp_synaptics/td4150_a12s_boe.bin` (**147456**), kernel-builtin `=y` |
| Driver in use today | OSS td4150 overlay in `os/third_party/kernel_samsung_a12/.../td4150/` (synced with `td4150_oss_dxj6/`) |
| Clean Samsung sources | `os/third_party/td4150_oss_dxj6/` (DXJ6 extract + since76 lab extras) |
| Build (maze Image) | `make -f os/Makefile kernel boot-v019` from `/home/mike/projects/saaios` |
| lcdtype | AP `get_lcd_info("id")` = **`0x1AF240`** matches **neither** DT overlay. GPIO idx 5 would pick td4160; **force idx 4** `td4150_a12s_boe.bin` (`0x3A6220`). Same blob stock v011 used. Do not switch to td4160. |
| Defer | Touch probe is `device_initcall`; DECON is `late_initcall`. **Defer 0x45 until first panel up** |

---

## Protocol

Modes: `MODE_APPLICATION_FIRMWARE=0x01`, `MODE_HOSTDOWNLOAD_FIRMWARE=0x02`, `MODE_ROMBOOTLOADER=0x04`. `IS_FW_MODE` is **0x01 and 0x02**.

Cold-boot IDENTIFY: `td4150_rom-10.0`, mode **`0x04`**, packrat **2893283** (`e3 25 2c 00`), **max_write=1024**.

Bin `tsp_synaptics/td4150_a12s_boe.bin` **147456**:

| Area | Size | Notes |
|------|------|-------|
| `ROMBOOT_APP_CODE` | 97280 | crc **`0x5a555e91`**, sha256 **`034f1e842d1a01f318ec0cda18c26ad5a6a72946cba50e30c485066222374165`**, first=`55 aa 01 00`, file_off=**64** |
| `APP_CONFIG` | 4096 | packrat **2100042344** / `0x7d2c1a68`, img_version **01040010** |
| `DISPLAY` | 2048 | |
| `BOOT_CONFIG` | 256 | |

Stock `CMD_ROMBOOT_DOWNLOAD` **0x45**: **14 reserved** (`[0]=size>>16=0x01`, rest 0) + APP_CODE. TCM payload=**97294**; remaining_length=**97296**. Header u16 is `(unsigned char)length` + `(unsigned char)(length >> 8)` so **97294=0x17c0e wraps** to `0e 7c`=**31758**. Cold IDENTIFY **max_write=1024**. **`HDL_WR_CHUNK_SIZE=0` is the 0x45 one-shot mode** (unlimited / one continuous transfer). Generic `CMD_CONTINUE_WRITE` (`0x01`) only honors the 16-bit TCM length (`0x7c0e`) and **breaks** the extended 0x45 (`payload=0x17c0e`, `reserved[0]=0x01`) — since65 proved this: 96 correctly split packets, IC stayed RomBoot 0x04, ATTN stuck, timeout `-ETIME`. Other TCM commands still use `MIN(max_write, WR_CHUNK_SIZE)` = **512**. Force `wr_chunk=0` only around `write_message(0x45)`, then restore. Do **not** treat IDENTIFY after chunk 0 of a *chunked* 0x45 as firmware-started (maze PAGE_SIZE dead end). Oneshot 0x45 historically produces IDENTIFY `TD4150-12.0.12` / mode **0x02**; since67 treats that as waiter STATUS_OK so stock post-0x45 can run. since67 then sent **0x1f** because dispatch already stored mode 0x02. since68 skips `switch_mode` (no 0x1f, no forced APP_CONFIG). since69 runs leftover `HELP_SEND_REINIT` when mode is 0x01 or 0x02 and `host_downloading=0`. That IDENTIFY is **not** proof downloaded APP_CODE launched (since56: 0x40 alone yields the same string).

After 0x45 (maze since49/54) this IC emitted IDENTIFY **`TD4150-12.0.12`**, mode **`0x02`**, packrat **2100027192** (`38 df 2b 7d` = `0x7d2bdf38`). Delta vs img packrat: **`0x3b30`**. **since56 LIVE:** the same IDENTIFY is **byte-identical** after **0x40 without 0x45**. `TD4150-12.0.12` / packrat **2100027192** (absent from the blob; image APP_CONFIG packrat is **2100042344**) is **on-chip / built-in HDL fallback**, not proof of downloaded APP_CODE. `0x45` is **not** confirmed as starting downloaded APP_CODE. Investigate **acceptance/launch of 0x45 in RomBoot**, not post-HDL opcodes.

Stock Samsung after `write_message(0x45)` from **RomBoot 0x04** is `switch_mode(BOOTLOADER)` → **0x42**. From **mode 0x02**, `switch_mode(BOOTLOADER)` sends **0x1f**, not 0x42. Dispatch stores `id_info.mode` from the 0x45 IDENTIFY, so later `switch_mode()` can never emit 0x42.

`firmware_status` packed u16 (`invalid_static:1 need_disp:1 need_app:1 hdl_version:4 need_osh:1 reserved:8`): leftover 0x1b payload `10 00` = **hdlv=2, all need_*=0**. Stock 0x1b is a **report**, not a command response. 0x1b must **not** complete 0x42/0x40 waiters. Oneshot 0x45 success is IDENTIFY mode **0x02**, then typically one leftover `0x1b`; do **not** remap that leftover 0x1b as 0x45 STATUS_OK (since66). When `host_downloading==0`, skip leftover `download_config`.

Stock never sends `CMD_ENABLE_REPORT` 0x05 in probe/touch/resume (only `synaptics_tcm_testing.c` factory report collection). Stock `#define USE_DEFAULT_TOUCH_REPORT_CONFIG` makes `touch_set_report_config()` a no-op, so **0x26 SET_TOUCH_REPORT_CONFIG is compiled out**. Reporting setup is GET 0x25 of the firmware default, then `input_register_device`. Resume/`mod_resume` does **not** enable reports: `enable_irq` + `wait_hdl` (returns immediately if `host_downloading==0` after a 100 ms sleep) + optional factory `DC_ENABLE_EDGE_REJECT` (`CONFIG_SEC_FACTORY` is **not** set). `DC_NO_DOZE` / `DC_START_STOP_TOUCH_WORK` are unused. `CMD_SET_SCAN_START_STOP` 0xb0 is suspend-only when `prox_lp_scan_cnt>0`. `touch_resume()` is inside `#if 0`. No `do_polling`.

Stock unpatched HDL after 0x45: `switch_mode(FW_MODE_BOOTLOADER)` then IRQ 0x1b → `zeroflash_download_config()`; if `need_*=0` queue `HELP_SEND_REINIT` (`identify(true)` = CMD_IDENTIFY **0x02**, then `touch_reinit`). **since54 skipped that on purpose.**

---

## Blob identity

The Project-Xed blob **is** the stock DXJ2 blob. Exact byte match of all four `tsp_synaptics/*.bin` inside stock `A127FXXSDDXJ2` `boot.img` kernel (`CONFIG_EXTRA_FIRMWARE="./firmware"`).

| file | size | file sha256 | APP_CODE sha256 | stock kernel off | PX kernel off |
|------|------|-------------|-----------------|------------------|---------------|
| `td4150_a12s_boe.bin` | 147456 | `0ea7f387…261c4` | `034f1e84…74165` | 24486240 | 21193648 |
| `td4150_a21s.bin` | 151562 | `39765d37…a6a65` | `4f261e29…529a3` | 24334672 | 21042080 |
| `td4375_a12s_boe.bin` | 155578 | `e02fe69c…3b6f0` | `2c3e9d30…38f2` | 24633696 | 21341104 |
| `td4160_a12s_boe.bin` | 164236 | `709f7b89…e7cbe` | `a3b343f1…fad4` | 24789280 | 21496688 |

Stock **vendor.img / odm.img / ramdisk** have **no** `td4150*.bin` / `IMAGE_FILE_MAGIC`. TSP is **kernel-builtin**, not `/vendor/firmware`. Do **not** hunt vendor firmware.

`TD4150-12.0.12` and packrat **2100027192** are **absent** from the blobs and from the stock kernel — they are **IC IDENTIFY strings**.

**Missing (do not invent):** stock Android `dmesg` for **this** unit is **not on the host**. The human must boot **stock** (same DTBO; we do not flash DTBO) and capture continuous synaptics from probe to first touch.

```sh
dmesg
dmesg | grep -iE 'synaptics|td4150|tsp|touch'
grep synaptics /proc/interrupts
```

Look for whether stock sends any command after 0x45 / 0x1b, whether IRQ 244 grows on tap, and whether `sec_touchscreen` exists. `logcat` is not needed.

---

## What since54 proved

**since54 LIVE** (device 2026-08-21, user flash) is the control image. Stuck-here is **passively hung HDL**, not a missing opcode.

After one-shot `0x45`: RX#1 IDENTIFY `0x02`, RX#2 STATUS `0x1b` `10 00`, then **silence**. IRQ stays **3**, gpio stays **1** (idle). Host sent **no** post-HDL commands. Firmware **does** enter HDL `0x02`. Touch **does not**. The IC **never re-asserts ATTN**.

Control tar: `/srv/media/saaios-boot-v019-since54.tar` sha256 **`eb96d2becff9a94d4403d505dc1deaf2d4f3d3fab6cc37a6bc6599716fb6f619`**. Banner `SaaiOS v019 since54`. **Do not overwrite.**

Logs may print `HDL_READY` / `hdl_reinit_done=1` while `touch_hcd=NULL init_touch_ok=0 report_touch=0 sec_touchscreen=0`. That is an **observe-complete misnomer**, not a working touch stack. Future code must use `HDL_STATUS_COMPLETE` / `HDL_OBSERVE`. Do **not** rebuild since54 to rename it.

Missing event6 on since54 is **expected** (`touch_init` skipped). Hang proof is no new IRQ/RX, not missing evdev. Panel rails were on: `vdd_ldo28=1`, `gpio_lcd_rst=1`, `gpio_lcd_bl_en=1`.

Closed by earlier sinceN (do not reopen):

- 0x45 **was thought** to start downloaded APP_CODE (since49 before/after IDENTIFY). **since56 reopened this:** the same IDENTIFY is produced by 0x40 with no 0x45. That IDENTIFY is on-chip HDL fallback.
- HDL race: IRQ must own 0x1b decode before helper (since51). since50 called `download_config` on stale `fw_status 00 00`.
- Missing-0x42 is **CLOSED** (since53: `42 00 00` actually TX’d, timeout `-62`). 0x42 is RomBoot-only; the IC already left 0x04 during 0x45.
- From mode 0x02, `switch_mode(BOOTLOADER)` selects **0x1f** (since52: TX `1f 00 00`, `-62`).

### Live experiment chain (compact)

| Tag | What | Result |
|-----|------|--------|
| since47 | APP_CODE identity dump | Transfer correct (`file_off=64` size=97280 crc=`0x5a555e91` SPI=97297). Running packrat ≠ img. |
| since48 | 0x42 from 0x02 | Packed then postponed; superseded. **Not flashed.** |
| since49 | IDENTIFY before vs after 0x45 | before=`td4150_rom-10.0` 0x04 packrat **2893283**; after=`TD4150-12.0.12` 0x02 packrat **2100027192**. 0x45 starts downloaded APP_CODE. |
| since50 | Keep Samsung HDL after 0x02; trace 0x1b | Race: init called `download_config` on `fw_status 00 00` before IRQ copied `10 00`. |
| since51 | Serialize: IRQ owns 0x1b decode; delayed helper | Ordering OK. Helper IDENTIFY still `-62` ~1s. gpio=1, IRQ=3. IC deaf. |
| since52 | `switch_mode(BOOTLOADER)` after 0x1b | From 0x02 selected **0x1f**. TX `1f 00 00`, `-62`. |
| since53 | Freeze `routing_mode=0x04`; 0x42 immediately after `write_message(0x45)` | `42 00 00` actually sent; leftover 0x1b did not complete waiter; timeout `-62`. Missing-0x42 branch closed. 0x42 is RomBoot-only; IC already left 0x04 during 0x45. |
| since54 | 30s passive baseline: no post-HDL TX, no helper, no `mod_resume` | After 0x45: RX#1 IDENTIFY 0x02, RX#2 0x1b `10 00`, then silence. IRQ stays 3, gpio stays 1 (idle). |
| since55 | OSS+3 (fw-sel, defer 0x45, logs) | 0x40 → IDENTIFY 0x02 with **no 0x45**. Probe **-5** (0x25 timeout). Deferred 0x45 never ran. |
| since56 | 0x40-only IDENTIFY dump; no 0x45; stay bound | Before 0x40: `td4150_rom-10.0` 0x04 packrat **2893283**. After 0x40: `TD4150-12.0.12` 0x02 packrat **2100027192** `38 df 2b 7d` — **byte-identical** to post-0x45. Leftover 0x1b; IRQ=3; gpio=1 idle. On-chip HDL, not downloaded APP_CODE. |
| since57 | Clean OSS stock 0x45 from RomBoot 0x04; no 0x40; wait panel | Cold+pre-0x45: `td4150_rom-10.0` 0x04 packrat **2893283**. SPI actual=**97297** remaining=0. After: same on-chip IDENTIFY `TD4150-12.0.12` / **2100027192** / 0x02 / 0x1b `10 00`. IDENTIFY abort waiter 0x45 → `write_message` **-5** → return **before** stock `switch_mode`. `fb_ready=0` during 0x45 (ran inside `panel_enabled`). |
| since58 | Delayed stock 0x45 after `panel_enabled` returns + rail wait | Packed (`5d6c6b3a841fd6bf4951f2c54d0ad0b2d9afe33840e18b51091897db4740a15d`). **Not flashed.** Superseded by since59. |
| since59 | Delayed 0x45 after panel callbacks + rails + 400 ms; IDENTIFY-before-0x45 gate | **LIVE (settled).** Pre-0x45 still RomBoot 0x04 + sha256 match=1. After 0x45: `TD4150-12.0.12` / 2100027192 / `-5`. Panel delay **falsified**. |
| since60 | Same delayed 0x45; intended one RAM APP_CODE byte flip (offset 4) | **LIVE (invalid / discriminator missed).** Banner since60. boot.img header has `syna_corrupt_app=1`; `/proc/cmdline` does not (Samsung BL). Driver `syna_corrupt_app=0`, sha256 match=1, no offset 4 log. After 0x45: same `TD4150-12.0.12` / 2100027192 / `-5` as since59. **Not a scientific result.** |
| since61 | Same delayed 0x45; `#define SAAIOS_FORCE_CORRUPT_APP 1`; in-place XOR APP_CODE[4] `^= 0x01` on 0x45 payload | **LIVE (settled).** Compile-time force took. `flip offset=4 old=0x00 new=0x01`; sha256 **match=0** (`f839bf14…1dee`); crc32 **`0x67cba83c`**; TX `APP_CODE[4]=0x01`. After 0x45: same `TD4150-12.0.12` / 2100027192 / mode 0x02 / `-5`. Offset 4 may be unused; Identify metadata unchanged. **Not** enough to conclude ROM ignores payload. |
| since62 | Same delayed 0x45; in-place APP_CODE[0] `0x55→0x54` (magic only) | **LIVE (settled).** `54 aa 01 00`, match=0, crc **`0xe3b77bae`**, TX `APP_CODE[0]=0x54`, SPI=97297. After: same `TD4150-12.0.12` / 2100027192 / 0x02 / `-5`. Magic is **not** a launch gate. |
| since63 | Same delayed 0x45; `memset(app_code, 0, 97280)` + SPI TX dump before `spi_sync` | **LIVE (settled).** Full-zero APP_CODE really on the wire. TX `45 0e 7c 01 00…`, reserved kept, APP_CODE nonzero=0, SPI=97297, CRC32 SPI=`0xe64db3ca` APP_CODE=`0x3504b34e`. After: same `TD4150-12.0.12` / 2100027192 / 0x02 / `-5`. Payload content is not what produces that IDENTIFY **under oneshot 97k**. irq_cnt stayed 1 through 400 ms wait. |
| since64 | Delayed 0x45; **unmodified** APP_CODE; stock `write_message` CONTINUE_WRITE; `wr_chunk=1024` from leftover IDENTIFY max_write; no 0x40 | **LIVE (partial).** Leftover `0x1b` `10 00` storm; **mode stayed 0x04** (new vs oneshot 0x02). 1030 identical REPORT_STATUS / 68 ms. Probe/0x45/wr_chunk flushed. `host_downloading=0` `cmd=0x00`. wr_chunk not verified from that file. |
| since65 | Same chunked 0x45 as since64; **do not** call `download_config` on leftover 0x1b when `host_downloading=0`; one-shot 0x1b per 0x45 cycle; first-8 IRQ_LOOP ATTN logs; hold IRQ after 32 leftover / 32-in-one-entry | **LIVE (settled, reinterpreted).** Chunked 0x45 stayed `td4150_rom / 0x04`. ATTN stuck, same `0x1b` `10 00`. CONTINUE_WRITE broke extended 0x45. Hold + `-ETIME` were correct leftovers of a broken transaction, not a stolen STATUS_OK. |
| since66 | Deliver leftover `0x1b` `10 00` to stock `write_message` waiter as STATUS_OK; keep chunking | **Built, do not flash.** Opposite of the since65 reread. |
| since67 | Oneshot 0x45 (`HDL_WR_CHUNK_SIZE=0` only around `write_message(0x45)`); `wr_chunk=512` otherwise; IDENTIFY 0x02 completes waiter as STATUS_OK; leftover 0x1b is leftover (skip `download_config` if `host_downloading=0`); keep since65 storm guard | **LIVE (settled).** Oneshot+waiter OK (`retval=0`). Stock then sent **0x1f** because already mode 0x02; timeout `-62`. ATTN cleared after one leftover 0x1b. Next is configs, not bootloader switch. |
| since68 | Same oneshot 0x45 + waiter; **skip `switch_mode`** when already `MODE_HOSTDOWNLOAD_FIRMWARE` (0x02); no 0x1f; no forced APP_CONFIG | **LIVE (settled).** Skip 0x1f OK. Leftover queued REINIT; we skipped 0x25. irq silent to t=33s (`tc:0`, no event6). |
| since69 | Same skip switch_mode; leftover `HELP_SEND_REINIT` when `IS_FW_MODE` (0x01 or **0x02**) and `host_downloading=0`; `identify(false)` / 0x20 + 0x25; no 0x1f; no forced 0x30 | **LIVE (settled).** 0x20+0x25+`input8`/`event6` OK. IRQ 244 frozen at **4**. Power key does not hit touch resume. |
| since70 | One-shot late `touch_resume()` after successful REINIT; gate `IS_FW_MODE` 0x01\|**0x02**; no full `syna_tcm_resume`; no 0x26 | **LIVE (settled, falsified).** Ran, sent no SPI, IRQ 244 still **4**. |
| since71 | After successful 0x25: `kmemdup` of `tcm_hcd->config.buf` **after** `UNLOCK_BUFFER(config)`; wr_chunk=512; then late resume; no 0x05 | **LIVE (missed).** First-boot: no `Command = 0x26`, irq 244 = **4**, `irq_cnt=4 rx_cnt=4`. Later empty `Command = 0x` is wrap, not missing 0x45. |
| since72 | Same 0x26 echo: `kmemdup` **before** unlock in `touch_get_input_params()`; log `0x26 skip reason=` on every skip including 0x25 failure | **LIVE (falsified).** 0x26 SENT; `retval=-62 timeout=1`; irq 244 stayed **4**; HDL silent on SET. |
| since73 | After successful 0x25: stock `write_message(CMD_ENABLE_REPORT, {0x11}, 1)` only; no 0x26; wr_chunk=512; then late resume | **Packed, do not flash.** Only 0x05; superseded by since74. |
| since74 | After successful 0x25: `CMD_GET_DYNAMIC_CONFIG` 0x23 `{DC_NO_DOZE}` then if OK `CMD_ENABLE_REPORT` 0x05 `{REPORT_TOUCH}`; SPI TX dump; no 0x26 | **LIVE (settled).** 0x23 TX `23 01 00 01` spi_len=4 correct; `-ETIME`; irq stayed 4; 0x05 skipped. Not a framing bug. Does **not** prove all payloads broken. |
| since75 | After successful 0x25: `CMD_ENABLE_REPORT` 0x05 `{REPORT_TOUCH}` then live `identify(false)` / 0x20; spi_sync + actual_length; no 0x23 / 0x26 / 0x30 | **Superseded by since76.** Auto-0x05 after REINIT — do not flash if a menu image exists. |
| since76 | Same HDL boot as since69. Skip 0x25 after 0x20 OK. **Late post-boot 0x05/0x24/0x30 all LIVE -62** (SPI TX OK, no ATTN). Auto **live20 ladder** 0/10/100/500 ms after REINIT (empty GET only). `retval<0` → `state=dead` `response=ff`. Menu: live20 / run_app / enable_report / no_doze (**no app_config**). | **Current flash target.** Grep auto live20 delays; do not send 0x05/0x30 first. |

Control capture (already flashed; do not re-flash for opcodes):

```sh
dmesg | grep 'SaaiOS v019'
dmesg | grep -E 'since54 probe|IDENTIFY after 0x45|0x1b|SPI TX|SPI RX|observe|gpio=|irq_cnt|event6|mod_resume|no post-HDL'
grep synaptics /proc/interrupts
```

---

## OSS vs maze

Zip: `/srv/media/SM-A127F_SWA_13_Opensource.zip` — **242026746** bytes (231M), mtime **2026-08-20 17:51**. Documented as **A127FXXSDDXJ6** (same binary **D** as this unit’s DXJ2). `CONFIG_LOCALVERSION=""`.

Extract: `os/third_party/td4150_oss_dxj6/` — **17 source files** only (+ `THREE_PATCH_PLAN.txt` notes). Clean Samsung, **no** KernelSU / SUSFS / Project-X.

The maze (`os/third_party/kernel_samsung_a12/.../td4150/`) is **not** stock. 12 files identical. Maze extras: `.o`, `.orig`, `built-in.a`. Five sources changed:

| file | OSS | maze | maze Δ |
|------|-----|------|--------|
| `synaptics_tcm_zeroflash.c` | 1410 | 2905 | +1495 |
| `synaptics_tcm_core.c` | 3592 | 5831 | +2239 |
| `synaptics_tcm_core.h` | 819 | 906 | +87 |
| `synaptics_tcm_touch.c` | 1460 | 1580 | +120 |
| `synaptics_tcm_spi.c` | 867 | 927 | +60 |

Source-only maze **+4173 / −172** vs OSS.

**Do not port the 50-patch maze.** Allowed extras on OSS td4150 only:

1. fw-sel `td4150_a12s_boe.bin` on lcdtype mismatch (idx 4 / `0x3A6220`).
2. Defer 0x45 until first panel up, then run OSS `zeroflash_do_romboot_firmware_download()` unchanged (`0x45` + `switch_mode(BOOTLOADER)`).
3. Diagnostic logs (`fw_name`, lcdtype, 0x45 start/end, `switch_mode` retval, 0x1b `need_*` / `hdl_version`, `HELP_SEND_REINIT`). No opcode changes.

Then **leave OSS HDL alone.** Plan notes: `os/third_party/td4150_oss_dxj6/THREE_PATCH_PLAN.txt`. **since55 applied those three on OSS** and overlays `kernel_samsung_a12/.../td4150/` only (maze sources in that gitignored dir are replaced by OSS+3; do not re-apply maze `KERNEL_PATCHES`).

**since55 LIVE** (2026-08-21): fw-sel + SPI + `defer 0x45` log worked. Probe then continued `identify(false)` → `syna_tcm_get_romboot_info` **0x40**. IDENTIFY arrived **mode=0x02** (no 0x45, no panel_enabled). Dispatch treated it as “Device has been reset” (IDENTIFY is **not** a 0x40 response). Helper then **0x25** (`CMD_GET_TOUCH_REPORT_CONFIG`) timed out ~1 s. **`synaptics_tcm.0` probe -5** unbound the core device (`spi1.2` SPI driver stayed). Deferred 0x45 never ran. `/bin/ash: /: Permission denied` is a typed second prompt, not a driver bug.

---

## since56 LIVE (settled): 0x40 IDENTIFY is on-chip HDL

Banner `SaaiOS v019 since56`. Probe stayed bound. No 0x45, no 0x25.

**Before 0x40 (cold-boot):**
`payload_len=24 ver=0x01 mode=0x04 part='td4150_rom-10.0' build=e3 25 2c 00 packrat=2893283 max_write=1024`

**After 0x40 (IDENTIFY on IRQ, not STATUS_OK for 0x40):**
`payload_len=24 ver=0x01 mode=0x02 part='TD4150-12.0.12' build=38 df 2b 7d packrat=2100027192 max_write=1024`

This is **byte-identical** to post-0x45 IDENTIFY from since49/54:

| field | after 0x40 only | after old 0x45 |
|-------|-----------------|----------------|
| part | TD4150-12.0.12 | same |
| build | 38 df 2b 7d | same |
| packrat | 2100027192 | same |
| mode | 0x02 | same |
| max_write | 1024 | same |
| next frame | 0x1b `10 00` | same |
| after | silence | silence |

Therefore:

- `0x45` is **not** confirmed as starting downloaded APP_CODE.
- `TD4150-12.0.12` / packrat **2100027192** (absent from blob; image APP_CONFIG packrat is **2100042344**) belongs to **on-chip / built-in HDL**.
- `0x40` (`CMD_GET_ROMBOOT_INFO`) from RomBoot 0x04 is sufficient to enter that 0x02 IDENTIFY.
- Leftover `0x1b` also appears after 0x40 without 0x45 (rx_cnt 2→3). At t=0 after path: irq_cnt=3 gpio=0; at 5s: irq_cnt=3 rx_cnt=3 gpio=1 idle. Same hang shape as since54 after 0x45.
- Panel came up later; since56 correctly did not send 0x45.
- No event6 (expected). IRQ 244 stays at 3, driver still bound (`synaptics_tcm`).

Control tar `/srv/media/saaios-boot-v019-since56.tar` sha256 **`4b081d6bca7fa7f96e635fb3da4de721090faa634d640009ad462515b7718f28`**. **Do not overwrite.**

The failure point is **inside CMD_ROMBOOT_DOWNLOAD 0x45**: ROM accepts the SPI transaction but does not run downloaded APP_CODE and takes the standard fallback. Investigate **acceptance/launch criteria of 0x45 in RomBoot**, not post-HDL opcodes. Do not “fix” in this next image; log enough to see which:

- wrong ROMBOOT header semantics despite correct SPI length
- APP_CODE signature/internal header
- ROM wants a different image revision
- panel/reset/power state before download
- `write_message()` diverged from original OSS
- ROM does not see all ~97KB even if SPI says actual=97297

---

## since57 LIVE (settled): clean stock 0x45 still on-chip fallback

Banner `SaaiOS v019 since57`. Tar `/srv/media/saaios-boot-v019-since57.tar` sha256 **`e04b43789bdf6df62f75f6f6141ae353c16272fe78d3b9161110c5a1b30aaaf2`**. **Do not overwrite.** `syna_corrupt_app=0`. No 0x40.

Cold and pre-0x45 IDENTIFY: `td4150_rom-10.0` mode **0x04** packrat **2893283** max_write **1024**. Panel-up then stock `zeroflash_do_romboot_firmware_download`.

0x45 sizes: payload=97294 reserved=14 app=97280 remaining_length=97296 chunks=1 HDL_WR_CHUNK_SIZE=0 spi_first=97297. **SPI done actual=97297 remaining_length=0**.

IRQ immediately: IDENTIFY `TD4150-12.0.12` build `38 df 2b 7d` packrat **2100027192** mode **0x02** — **same on-chip HDL as 0x40-only**. Driver: `IDENTIFY abort waiter command=0x45 (not STATUS_OK)` → `reason=IDENTIFY_abort_CMD_ERROR` → `write_message retval=-5` → **`0x45 path return before stock switch_mode`**. Leftover `0x1b` `10 00` need_*=0 hdlv=2. irq_cnt=3 rx_cnt=3 gpio=1. No event6. HELP_SEND_REINIT skipped by design.

Stock also logged “Switched to TCM mode and going to download the configs” while init already failed 0x45 — race, not success. Treating that IDENTIFY as STATUS_OK was masking 0x45 failure (do not “fix” that abort).

**Timing:** 0x45 ran **synchronously inside `panel_enabled` while `fb_ready=0`**. ~178 ms SPI blocked the panel callback; `fb_ready=1` only logged **after** 0x45 finished. Download may have been mid panel sequencing. Do **not** conclude ROM rejects the blob until 0x45 is tried after `panel_enabled` has fully completed (that is since58).

**SHA256 log was a print/API bug, not a bad APP_CODE.** since57 printed `sha256=0000…0000` + last 16 hex chars of known_good, `header[0]=0x01`, `match=0`. Cause: `SHASH_DESC_ON_STACK(desc, tfm)` ran with **uninitialized** `tfm`, so `crypto_shash_descsize(tfm)` was garbage; the VLA overflowed into `digest[]` and zeroed the first 16 bytes. Last 16 hex chars matching known_good plus expected APP_CODE start `55 aa 01 00` means the RAM image was good. `header[0]=0x01` was **reserved[0] = size>>16**, not APP_CODE[0]. Known good remains CRC32 **`0x5a555e91`** SHA256 **`034f1e842d1a01f318ec0cda18c26ad5a6a72946cba50e30c485066222374165`**. since58 logs APP_CODE[0..3] and reserved[0..13] separately and hashes after a valid `tfm`.

Conclusion: even a clean RomBoot 0x04 stock 0x45 with full SPI still yields the same IDENTIFY; `write_message` treats that IDENTIFY as reset/CMD_ERROR so stock `switch_mode(BOOTLOADER)` never runs. since59 showed the same after a finished panel sequence. Two readings remain: ROM reject/fallback, or **success mis-labeled `-EIO`**. since60 was meant to test that; the RAM flip never ran (BL dropped cmdline).

---

## since58 (packed, not flashed)

Banner `SaaiOS v019 since58`. Tar `/srv/media/saaios-boot-v019-since58.tar` sha256 **`5d6c6b3a841fd6bf4951f2c54d0ad0b2d9afe33840e18b51091897db4740a15d`**. Probe: `since58 delayed 0x45 after panel callback, no 0x40`. **Do not overwrite.** Human has not flashed this image; superseded by **since59**.

---

## since59 LIVE (settled): panel delay falsified

Banner `SaaiOS v019 since59`. Tar `/srv/media/saaios-boot-v019-since59.tar` sha256 **`e34d391ab7cb2dc0372a8fe3523758576a8c646b44638642f7f5c72709854cec`**. Probe: `since59 delayed 0x45 after panel, callback-count, no 0x40`. **Do not overwrite.**

Delayed 0x45 after `panel_enabled` returns (not synchronous inside the callback). No 0x40.

Live (device 2026-08-21):

- Cold: `td4150_rom-10.0` mode **0x04** packrat **2893283**. No 0x40.
- `panel callback enter n=1` `fb_ready=0` → **exit before 0x45** (queued delayed work). `n=2` at `fb_ready=1` already scheduled (`FB_EARLY_EVENT_BLANK` + `FB_EVENT_BLANK`); only the first queues 0x45.
- Rails: `fb_ready=1` `vdd_ldo28=1` `gpio_lcd_rst=1` `gpio_lcd_bl_en=1` attn idle, still 0x04. Extra 400 ms. `elapsed since panel exit = 429 ms`.
- **IDENTIFY immediately before delayed 0x45:** still `td4150_rom-10.0` mode **0x04** packrat **2893283**. **Not contaminated.**
- `APP_CODE[0..3]=55 aa 01 00` **sha256 match=1** (`034f1e842d1a01f318ec0cda18c26ad5a6a72946cba50e30c485066222374165`). since57 SHA zeros were a print bug.
- Then stock 0x45 SPI actual=**97297** remaining=0 from mode 0x04 `fb_ready=2`.
- **Only after 0x45:** IDENTIFY `TD4150-12.0.12` / `38 df 2b 7d` / packrat **2100027192** / mode **0x02**. IDENTIFY abort waiter, `write_message` **-5**, no stock `switch_mode`. leftover `0x1b` `10 00`. irq 1→3.

Panel sequencing is **not** why 0x45 ends at this IDENTIFY. Pre-send buffer damage is **excluded** (known-good APP_CODE in RAM).

**Competing interpretations:**

| | Reading | What since63 tested / since64 tests |
|--|---------|-------------------|
| **Primary (human)** | Driver turns a **successful app restart** into `CMD_ERROR` / `-EIO` because IDENTIFY after 0x45 aborts the waiter. Application may already be running (`TD4150-12.0.12`). The error is **mis-classification of success**, not ROM reject. | since61/62 flipped unused bytes. since63 zeros the **entire** APP_CODE **under oneshot 97k** and IDENTIFY was **again** `TD4150-12.0.12`. Payload content is not the producer of that IDENTIFY on that transport. Do **not** treat IDENTIFY as STATUS_OK. |
| Transport | Oneshot SPI 97297 exceeds IDENTIFY **max_write=1024**; TCM u16 wraps (`0e 7c`=31758). `HDL_WR_CHUNK_SIZE=0` should mean use max_write. | **since64 tests this** with stock `write_message` CONTINUE_WRITE, wr_chunk=1024, good APP_CODE, delayed panel path unchanged. |

`resp_len=1714697008` on failed 0x45 is **uninitialized garbage** on the IDENTIFY-abort path (`write_message` never stored `*resp_length`). Log noise; since60 zeros `resp_length` before `write_message()` (**confirmed LIVE:** `resp_len=0`).

**Caveat:** `IDENTIFY immediately before 0x45` logs **cached `id_info`**, not a live Identify command. It does **not** 100% prove the IC is still physically in RomBoot at TX time.

---

## since60 LIVE (discriminator missed): boot.img cmdline dropped

Banner `SaaiOS v019 since60`. Tar `/srv/media/saaios-boot-v019-since60.tar` sha256 **`1a7f99599b8437378e590be3056cc3772433d0f37610c60c460537ada3174046`**. Probe: `since60 delayed 0x45 corrupt APP_CODE one RAM byte, no 0x40`. **Do not overwrite.**

Packed boot.img header cmdline includes `syna_corrupt_app=1` (and `androidboot.selinux=permissive loop.max_part=7`). **`/proc/cmdline` does not** — Samsung bootloader supplies its own (`androidboot.selinux=enforcing`, no `syna_corrupt_app`, no `loop.max_part`). Driver `__setup("syna_corrupt_app=")` never ran.

Live (device 2026-08-21, telnet `127.0.0.1:2323`):

- Cold + immediately before delayed 0x45: `td4150_rom-10.0` mode **0x04** packrat **2893283**. No 0x40. Panel enter/exit n=1 then n=2; rails on; extra sleep; elapsed **406 ms**.
- File / RAM-before-flip sha256 **match=1** (`034f1e842d1a01f318ec0cda18c26ad5a6a72946cba50e30c485066222374165`).
- **`syna_corrupt_app=0 (RAM APP_CODE unmodified)`**. No `offset=4` old/new log.
- After “flip”: sha256 **match=1**, crc32 **`0x5a555e91`**. `APP_CODE[0..3]=55 aa 01 00`. reserved[0]=`0x01`.
- 0x45 SPI actual=**97297**. **IDENTIFY after:** `TD4150-12.0.12` / `38 df 2b 7d` / packrat **2100027192** / mode **0x02**. IDENTIFY abort waiter, `write_message` **-5**, `resp_len=0`, leftover `0x1b` `10 00`. irq 1→3.

This is a **since59 replay** (known-good APP_CODE). Same post-0x45 IDENTIFY does **not** decide the fork — the byte was never flipped.

**since60 is invalid as a corrupt control.** `__setup("syna_corrupt_app=")` / `module_param` never saw the token: builtin driver (`CONFIG_TOUCHSCREEN_SYNAPTICS_TCM_ZEROFLASH=y`) would also have needed `synaptics_tcm_zeroflash.syna_corrupt_app=1` for `module_param`, and Samsung BL stripped the unprefixed name from `/proc/cmdline`. Do **not** treat same IDENTIFY as a scientific result.

### IDENTIFY after corrupt 0x45 — decision tree (since61 did not distinguish)

| After corrupt 0x45 | Meaning |
|--------------------|---------|
| Stays RomBoot **or** download fails differently (not `TD4150-12.0.12` / 2100027192) | `TD4150-12.0.12` was launched **from transferred APP_CODE**. since59 `-EIO` was **mis-classified success**. |
| **Same** `TD4150-12.0.12` / packrat **2100027192** / leftover `0x1b` `10 00` | Flipped region not checked or not used (on-chip fallback, **or unused slice**). since61 and since62 landed here; next is entire-APP_CODE zeros (since63). |

**Do not overwrite** since54 (`eb96d2becff9a94d4403d505dc1deaf2d4f3d3fab6cc37a6bc6599716fb6f619`), since55 (`fd7eea685f2b3c236f9a389f9321267a17127c4ff2cb43ebde60f8aa13c9ca5c`), since56 (`4b081d6bca7fa7f96e635fb3da4de721090faa634d640009ad462515b7718f28`), since57 (`e04b43789bdf6df62f75f6f6141ae353c16272fe78d3b9161110c5a1b30aaaf2`), since58 (`5d6c6b3a841fd6bf4951f2c54d0ad0b2d9afe33840e18b51091897db4740a15d`), since59 (`e34d391ab7cb2dc0372a8fe3523758576a8c646b44638642f7f5c72709854cec`), since60 (`1a7f99599b8437378e590be3056cc3772433d0f37610c60c460537ada3174046`), since61 (`bb1028931a57834964ed1b6f2f807d13317e8c06d4da569a5dfcc998f7e5a174`), or since62 (`29d04f4b4855b72c275f85c1bdb213aa4e3022a4f8d35a85bd09d6abc273f5ae`).

---

## since61 LIVE (settled): one-byte XOR APP_CODE[4] — IDENTIFY unchanged

Banner `SaaiOS v019 since61`. Probe: `since61 FORCE RAM corrupt offset 4, no 0x40`. Tar `/srv/media/saaios-boot-v019-since61.tar` sha256 **`bb1028931a57834964ed1b6f2f807d13317e8c06d4da569a5dfcc998f7e5a174`**. **Do not overwrite.**

Compile-time `#define SAAIOS_FORCE_CORRUPT_APP 1` (not cmdline). Same delayed-0x45 path as since59 (no 0x40; RomBoot 0x04 immediately before send).

Live (device 2026-08-21):

```
syna_corrupt_app=FORCE flip offset=4 old=0x00 new=0x01
APP_CODE[0..3]=55 aa 01 00
sha256 after flip=f839bf145aba4a00e018a21ca7baf5e62c60fc328b5b402a55bec4a7f5811dee match=0
crc32=0x67cba83c known_good=0x5a555e91
0x45 TX payload APP_CODE[4]=0x01
SPI actual=97297
before 0x45: td4150_rom-10.0 mode 0x04 packrat 2893283
after 0x45: TD4150-12.0.12 / 38 df 2b 7d / packrat 2100027192 / mode 0x02 / 0x1b 10 00 / write_message -5
```

**Valid corrupt control** (unlike since60). Same IDENTIFY as good since59 and as 0x40-without-0x45. Offset 4 may be reserved/unused; Identify metadata unchanged. **Not** enough to conclude ROM ignores 0x45 payload content. Do **not** treat IDENTIFY as STATUS_OK.

---

## since62 LIVE (settled): APP_CODE[0] magic 0x55→0x54 — IDENTIFY unchanged

Banner `SaaiOS v019 since62`. Probe: `since62 APP_CODE[0] 0x55->0x54 magic, no 0x40`. Tar `/srv/media/saaios-boot-v019-since62.tar` sha256 **`29d04f4b4855b72c275f85c1bdb213aa4e3022a4f8d35a85bd09d6abc273f5ae`**. **Do not overwrite.**

Compile-time `#define SAAIOS_FORCE_CORRUPT_APP 1`. Same delayed-0x45 path. In-place `app_code[0] = 0x54` on the 0x45 payload after stock copy.

Live (device 2026-08-21):

```
APP_CODE[0..3]=54 aa 01 00
sha256 after=8e8d70d0cba05a9d5ec6bf62c3ddd88a0e698f0ab9c21b19a2da349a52160603 match=0
crc32=0xe3b77bae known_good=0x5a555e91
0x45 TX APP_CODE[0]=0x54
SPI actual=97297
before: td4150_rom-10.0 mode 0x04 packrat 2893283
after: TD4150-12.0.12 / 38 df 2b 7d / 2100027192 / mode 0x02 / 0x1b 10 00 / write_message -5
```

**Valid magic-break control.** Magic is **not** a launch gate for this ROM path. Same IDENTIFY as since59/61 and as 0x40-without-0x45. One/two bytes still do **not** prove the whole payload is ignored.

**Caveat:** `IDENTIFY immediately before 0x45` is **cached `id_info`**, not a live read. It does not 100% prove the IC is still physically in RomBoot at TX time.

---

## since63 LIVE (settled): full-zero APP_CODE on the wire, same IDENTIFY — oneshot 97k

Banner `SaaiOS v019 since63`. Probe: `since63 APP_CODE memset 0 all 97280, SPI dump, no 0x40`. Tar `/srv/media/saaios-boot-v019-since63.tar` sha256 **`55634ceb409ffe786e6c095f0848cebd502e6aa1759b31ff4bb64ea1e2586ebf`**. **Do not overwrite.**

Same delayed-0x45 path as since59. Compile-time FORCE memset APP_CODE 97280 zeros; reserved[14] kept; SPI dump before `spi_sync`.

```
tx[0..31]=45 0e 7c 01 00 …
cmd 0x45, payload_u16=31758=97294 (u16 wrap of 0x17c0e)
reserved tx[3..16]=01 00..00
APP_CODE start/mid/last all 00, nonzero=0, APP_CODE[0]=0 APP_CODE[97279]=0
CRC32 SPI buf=0xe64db3ca len=97297; CRC32 APP_CODE=0x3504b34e
SPI done actual=97297
irq_cnt stayed 1 through 400ms wait (mode 0x04 packrat 2893283) until after 0x45
after 0x45: TD4150-12.0.12 / 38 df 2b 7d / 2100027192 / 0x02 / 0x1b 10 00 / write_message -5
```

**Zero APP_CODE really went on the wire.** IDENTIFY unchanged vs good blob, magic-break, offset-4 flip, and 0x40-without-0x45. Payload content is **not** what produces `TD4150-12.0.12` **under oneshot 97k**.

Cached `id_info` before 0x45 is not a live Identify. irq_cnt=1 during wait suggests no extra IDENTIFY before TX.

New hypothesis (not tested in since63): illegal oneshot vs IDENTIFY **max_write=1024** + u16 length wrap. `spi_sync actual=97297` only means the host clocked that many bits. Samsung `HDL_WR_CHUNK_SIZE=0` means “use max_write”. Skipping 0x40 skipped a usable wr_chunk assignment; RomBoot then stored `wr_chunk_size = HDL_WR_CHUNK_SIZE` (0) and `write_message` oneshot `chunk_space = remaining_length`.

Do **not** treat IDENTIFY as STATUS_OK.

---

## since64 LIVE (partial): chunked 0x45, leftover 0x1b storm, mode stayed 0x04

Banner `SaaiOS v019 since64`. Probe: `since64 chunked 0x45 wr_chunk=1024 good APP_CODE, no 0x40`. Tar `/srv/media/saaios-boot-v019-since64.tar` sha256 **`82c7968c542833bde9575221767ef0ff0d4555e95d6ab7e177a5c3b38d8b690a`**. **Do not overwrite.**

Transport intent (unchanged in since65): delayed 0x45 after panel, no 0x40, no 0x25, stay bound, unmodified APP_CODE, `wr_chunk=1024` from leftover IDENTIFY max_write, stock `write_message` CONTINUE_WRITE. Expect chunk_space=1023 chunks=96 spi_first=1024 spi_last=112. IDENTIFY after chunk 0 is **not** STATUS_OK.

**Captured file (beginning flushed):** 1030 identical leftover `0x1b` REPORT_STATUS in 68 ms (~15k msg/s) around t=49.79–49.86. payload `10 00`, mode **0x04**, `host_downloading=0`, `cmd=0x00`. HDL_OBSERVE printed `stock OSS download_config follows` on every leftover even though download was not active. `download_config()` itself is a no-op when `host_downloading=0`, but leftover still entered that path every IRQ.

This is **new vs since63**: oneshot 97k went to on-chip HDL **0x02**; this leftover storm stayed **RomBoot 0x04**. wr_chunk / whether 0x45 actually completed **cannot** be verified from that file (dmesg wrap / IRQ flood). Two causes to distinguish: ATTN stays asserted and the IRQ re-reads the same packet; or REPORT_STATUS is regenerated (download_config / reset notification).

---

## since65 LIVE (settled): chunked 0x45 works; ATTN stuck; STATUS_OK stolen from waiter

Banner `SaaiOS v019 since65`. Probe: `since65 leftover 0x1b storm guard, chunked 0x45 wr_chunk=1024 good APP_CODE, no 0x40`. Tar `/srv/media/saaios-boot-v019-since65.tar` sha256 **`97f8e31d021b0093ea33a95f03b39db0166212af9921a064de9734e9380333fc`**. **Do not overwrite.**

**Chunked 0x45 ran correctly** (first proof): `wr_chunk=1024` `chunk_space=1023` `chunks=96` `spi_first=1024` `spi_last=112`. SPI first `45 0e 7c 01 00 00 00 00...` (stock 16-bit TCM length `0x7c0e`=31758; reserved[0]=`0x01` is size>>16). Second/last cmd=`0x01` CONTINUE_WRITE. SPI done actual=**97392** remaining=0 chunks=96. APP_CODE sha256 **match=1** crc32=`0x5a555e91`. Before 0x45: RomBoot `td4150_rom-10.0` mode=`0x04` packrat=**2893283**.

**Leftover storm cause:** ATTN stuck, same packet re-read. All 8 `IRQ_LOOP` lines are **irq_n=2** (one IRQ entry): `attn_before=0 attn_after=0 irq_on=0` `raw=a5 1b 02 00 10 00 5a 5a` `report=0x1b plen=2 payload=10 00`. ATTN never deasserts. Not regenerated REPORT_STATUS from new IRQs.

**Reinterpretation:** the leftover `0x1b` `10 00` storm is **not** a stolen STATUS_OK. CONTINUE_WRITE broke the extended 0x45 (`payload=0x17c0e` / `reserved[0]=0x01`); the IC stayed RomBoot 0x04, ATTN stayed asserted, and the same `0x1b 10 00` was re-read. Emergency hold after 32 leftover reads and 0x45 `-ETIME` were correct. `HDL_WR_CHUNK_SIZE=0` is oneshot, not “use max_write”.

At t=2.265: SPI done → leftover snapshot `sent_0x45=1 last_0x45_retval=2147483647` (`0x7fffffff`=never returned) `host_downloading=1 cmd=0x45 mode=0x04` → leftover n=1 payload=`10 00` → HDL_OBSERVE `need_app=0` → `host_downloading` became 0 → 32 leftover 0x1b → emergency hold disabled IRQ → timeout `-62` → return before stock `switch_mode`. After: still cached id_info mode=`0x04` `td4150_rom-10.0`.

**Oneshot vs chunked:** oneshot 97k (since57–63) jumped to on-chip HDL `TD4150-12.0.12` mode=`0x02` and IDENTIFY aborted the waiter as `-EIO`. Chunked 0x45 never left RomBoot. since67 restores oneshot and treats that IDENTIFY 0x02 as waiter STATUS_OK so stock post-0x45 can run. That IDENTIFY is still **not** proof downloaded APP_CODE launched (since56).

---

## since66 (built, do not flash)

Banner `SaaiOS v019 since66`. Probe: `since66 0x1b STATUS_OK completes 0x45 waiter, stock switch_mode, no hold during 0x45`. Tar `/srv/media/saaios-boot-v019-since66.tar` sha256 **`eb10d1eb321a3cfecb6b952fc4586328a5c414c204905387c37b1ce7e0f4ab13`**. **Do not overwrite. Do not flash.** Kept chunked 0x45 and remapped leftover `0x1b` `10 00` as waiter STATUS_OK — the opposite of the since65 reread.

---

## since67 LIVE (settled): oneshot+waiter OK; stock then sent 0x1f

Banner `SaaiOS v019 since67`. Probe: `since67 oneshot 0x45 HDL_WR_CHUNK_SIZE=0, IDENTIFY 0x02 completes waiter STATUS_OK, wr_chunk=512 otherwise, no 0x40`. Tar `/srv/media/saaios-boot-v019-since67.tar` sha256 **`16e2ad40ac7a228ad4592557a2608da49341969e0d027d33e8745dc667c5d0fa`**. **Do not overwrite.**

Oneshot 0x45 + IDENTIFY-as-STATUS_OK **worked**:

- `wr_chunk before 0x45=512`; forced `HDL_WR_CHUNK_SIZE=0`; restored 512
- `chunks=1` `spi_len=97297` SPI done actual=97297 remaining=0
- APP_CODE sha256 **match=1** crc32=`0x5a555e91`
- Before 0x45: `td4150_rom-10.0` mode=`0x04`
- After 0x45 IRQ: IDENTIFY `TD4150-12.0.12` mode=`0x02` packrat **2100027192**
- `IDENTIFY 0x02 completed 0x45 waiter as STATUS_OK`
- `0x45 return reason=STATUS_OK` / `write_message retval=0`

ATTN after oneshot is **healthy** (unlike chunked since65):

- IRQ_LOOP irq_n=2 loop=0: IDENTIFY raw=`a5 10 18 00 01 02 54 44` attn_before=0 attn_after=0 (ATTN still up — another frame pending)
- IRQ_LOOP irq_n=2 loop=1: leftover 0x1b raw=`a5 1b 02 00 10 00 5a 5a` attn_before=0 attn_after=1 (**ATTN CLEARED**)
- One leftover 0x1b, no storm

Stock post-0x45 path failed for a **new** reason:

- `stock switch_mode enter` while already mode=`0x02`
- Therefore stock sent `CMD_RUN_BOOTLOADER_FIRMWARE` **0x1f** (not 0x42; 0x42 is only from RomBoot 0x04)
- Leftover 0x1b happened in the same IRQ as IDENTIFY, **before/as** 0x1f was sent; ATTN then idle
- HDL_OBSERVE: need_app=0 need_disp=0 need_osh=0 hdl_version=2 — leftover one-shot `download_config` is a no-op
- 0x1f timed out `-ETIME` (-62): `Failed to write command CMD_RUN_BOOTLOADER_FIRMWARE` / `Failed to switch to bootloader`
- After: still `TD4150-12.0.12` / 0x02 / packrat 2100027192

IDENTIFY handler also logged stock: `Switched to TCM mode and going to download the configs` — that is the correct HDL follow-up, not 0x1f.

since56 still stands: `TD4150-12.0.12` / 2100027192 is on-chip HDL, not proof downloaded APP_CODE. since67 only proved oneshot 0x45 + waiter remap lets stock continue. The continuation chose the wrong opcode.

---

## since68 LIVE (settled): skip 0x1f OK; leftover REINIT was blocked

Banner `SaaiOS v019 since68`. Probe: `since68 skip switch_mode already HDL 0x02 after 0x45, APP already running, oneshot 0x45, IDENTIFY 0x02 STATUS_OK, wr_chunk=512 otherwise, no 0x40`. Tar `/srv/media/saaios-boot-v019-since68.tar` sha256 **`1e6631d9be328dcaa9bd3421ef60e11d263bb4ee540ce343b77ff476dfaa9eb9`**. **Do not overwrite.**

Proven:

- 0x45 oneshot OK: spi 97297, sha256 match, IDENTIFY 0x02 completes waiter STATUS_OK, wr_chunk restored 512
- `skip switch_mode: APP already running after 0x45 mode=0x02 part='TD4150-12.0.12' packrat=2100027192`
- **NO** `Command = 0x1f`, no -62 timeout
- leftover 0x1b payload=`10 00`, ATTN after=1 (cleared), no storm
- HDL_OBSERVE need_app=0 need_disp=0 need_osh=0 hdl_version=2
- Stock IDENTIFY: `Switched to TCM mode and going to download the configs`
- Then: `stock download_config skip (need_*=0) queue REINIT`
- Then: **`skip HELP_SEND_REINIT identify/0x25 (leftover need_*=0 from HDL 0x02; known timeout)`**
- At t=33s: irq_cnt=2 rx_cnt=3 gpio=1 irq_en=1 still mode=0x02 packrat=2100027192, `tc:0`, no event6 / no sec_touchscreen reports

Boot-path skip succeeded. Silence is because leftover HDL queued stock REINIT (identify + GET_TOUCH_REPORT_CONFIG 0x25) and we **blocked** it. The old 0x25 timeout was since55 probe-after-0x40, not this post-0x45 HDL-idle state. since56 still stands: `TD4150-12.0.12` / 2100027192 is on-chip HDL, not proof of downloaded APP_CODE.

---

## since69 LIVE (settled): REINIT/touch_init OK, IRQ frozen at 4

Banner `SaaiOS v019 since69`. Tar `/srv/media/saaios-boot-v019-since69.tar` sha256 **`9d8413eed7be9d02210c393680c13e64878e86bea18058f85f6d5eac03889ad4`**. **Do not overwrite.**

Proven (do not rewind 0x45 oneshot / skip 0x1f / post-HDL REINIT):

- 0x45 oneshot OK, skip 0x1f, mode=0x02 `TD4150-12.0.12` packrat=2100027192
- `HELP_SEND_REINIT` ran: 0x20 APPLICATION_INFO retval=0 `app_status=OK` 46 bytes
- 0x25 GET_TOUCH_REPORT_CONFIG retval=0 128 bytes `first=1a 08 10 08 0f 01 17 08`
- `sec_touchscreen` sysfs `input8`, **Handlers=event6**, `/dev/input/event6` exists (13,70). Same SPI `input9`→`event7`. `event8` was the wrong path.
- `touch_init retval=0 init_touch_ok=1`
- Finger + power-button off/on: irq 244 **exactly 4** through t=1500s (`irq_cnt=4 rx_cnt=5 gpio=1 irq_en=1`)
- Power key does **not** hit the touch driver: `dmesg | grep -iE 'syna_tcm_(early_)?(suspend|resume)|mod_(suspend|resume)'` empty after `AFTER_RESUME`. BusyBox/initramfs — no Android userspace, fb/DRM notifier does not fire on power key.

Stock 0x05/0x26 did **not** run after 0x25 (0x05 testing-only; 0x26 compiled out via `USE_DEFAULT_TOUCH_REPORT_CONFIG`). `touch_resume()` was `#if 0`. Early `syna_tcm_resume` at ~1.58s was RomBoot 0x04 and is not a second enable. Do **not** wait for suspend/resume on this ramdisk.

---

## since70 LIVE (settled, falsified): late touch_resume sent no SPI, IRQ still 4

Banner `SaaiOS v019 since70`. Probe: `since70 one-shot late touch_resume after REINIT (IS_FW_MODE 0x01|0x02, no 0x26)`. Tar `/srv/media/saaios-boot-v019-since70.tar` sha256 **`7459bd9095808c012a7434705452d6014777548cd770693d510617473959ee4a`**. **Do not overwrite.**

Proven:

- `late touch resume start: lp_state=0 boot_resume=0 mode=0x02 fb_ready=2 init_touch_ok=1 host_downloading=0 irq_cnt=4`
- `mod_cb->resume n=0`
- `touch_resume retval=0`
- **No** extra Command after 0x25 (no 0x05, no 0x26)
- After touches: irq 244 **still 4**, no `REPORT_TOUCH`, `od` event6 empty

Late resume is a **known no-op** (no SPI). Do not rewind boot path. Keep it after 0x05 so order is enable-report then resume.

---

## since71 LIVE (missed): 0x26 never sent

Banner `SaaiOS v019 since71`. Probe: `since71 0x26 SET_TOUCH_REPORT_CONFIG echo of 0x25`. Tar `/srv/media/saaios-boot-v019-since71.tar` sha256 **`238d15406e9900a942d9c943770ef702238e4e11a65afc2b8fceabc0bf68221d`**. **Do not overwrite.**

Proven on device (first grep, while banner `SaaiOS v019 since71` still in the buffer):

- `dmesg | grep -E 'Command = 0x26|0x26 retval|REPORT_TOUCH|0x26 start'` **empty**
- irq 244 still **4** before and after touch — same as since70 after 0x20+0x25 only

That is the proof `write_message(0x26)` did **not** run. Stock would have logged `Command = 0x26` and IRQ would be ≥5.

Later `dmesg | grep 'Command = 0x'` empty and `SAaiOS_TOUCH_DBG | tail -40` showing only `since59 observe print_info` at t=277..830 is **ring-buffer wrap**, not a missing 0x45. Early `Command = 0x45/0x20/0x25` and all early `SAaiOS_TOUCH_DBG` were evicted. `sent_0x45=1 mode=0x02 packrat=2100027192` still proves HDL came up. Steady state `irq_cnt=4 rx_cnt=4 gpio=1 irq_en=1` vs since69 after 0x20+0x25 (`irq_cnt=4 rx_cnt=5`): 0x26 never added an IRQ.

**Root cause:** `touch_get_input_params()` (`synaptics_tcm_touch.c`). 0x25 fills `tcm_hcd->config` via `write_message(..., &tcm_hcd->config.buf, ...)`, which aliases `tcm_hcd->resp` onto that same buffer. `syna_tcm_alloc_mem()` memsets it and sets `data_length=0`. since71 called `touch_echo_set_touch_report_config()` **after** `UNLOCK_BUFFER(tcm_hcd->config)`, so the helper re-read a buffer that could already be empty/reset. The 0x25 `retval < 0` path returned **before** the helper with **no** `0x26 skip` log; `touch_set_input_reporting()` skip (`IS_NOT_FW_MODE` / `app_status != APP_STATUS_OK`) also did not match grep `0x26 start`.

---

## since72 LIVE (falsified): 0x26 SENT, HDL silent

Banner on device still said `SaaiOS v019 since71` (ramdisk wrap / not bumped in that flash) but 0x26 logs match the kmemdup-before-unlock fix. Probe: `since72 0x26 SET_TOUCH_REPORT_CONFIG kmemdup of 0x25 before unlock`. Tar `/srv/media/saaios-boot-v019-since72.tar` sha256 **`9617fd333990505c0b721f412bbbeae727ead0338e667bcecb34aea53eef4a58`**. **Do not overwrite.**

Proven:

```
0x26 start len=128 first=1a 08 10 08 0f 01 17 08 12 10 16 04 04 01 06 04 mode=0x02 irq_cnt=4 wr_chunk=512
Command = 0x26
0x26 retval=-62 timeout=1 response=0xff resp_len=0 mode=0x02 irq_cnt=4
```

IRQ 244 stayed **4** through the 1s wait **and** after touches. ATTN never asserted — HDL did **not** respond to SET_TOUCH_REPORT_CONFIG at all (not STATUS_ERROR; silence). Do **not** retry 0x26 (wastes 1s, already falsified). Do not force APP_CONFIG. Do not rewind 0x45 / skip 0x1f / leftover REINIT 0x20+0x25.

---

## since73 packed, do not flash (0x05 only)

Banner `SaaiOS v019 since73`. Probe: `since73 0x05 ENABLE_REPORT payload 0x11 after 0x25`. Tar `/srv/media/saaios-boot-v019-since73.tar` sha256 **`94cdeb8ec8347cc1467c154b1231f2432095f7575bfa8b6ac8b33c9b0e38406d`**. **Do not overwrite. Do not flash.** Refined before first flash: since73 only sent 0x05. since74 inserts 0x23 first.

---

## since74 LIVE (settled): 0x23 TX correct, IC silent

Banner `SaaiOS v019 since74`. Probe: `since74 0x23 GET_DYNAMIC_CONFIG DC_NO_DOZE then 0x05 ENABLE_REPORT 0x11 after 0x25`. Tar `/srv/media/saaios-boot-v019-since74.tar` sha256 **`dd5db9d6d27fa528690eafaf2e35b31922c071abe00b416107cc5aeacf39bc7f`**. **Do not overwrite.**

After 0x25:

```
0x23 SPI TX immediately before spi_sync spi_len=4 first=23 01 00 01 (expect 23 01 00 01 spi_len=4)
Command = 0x23
0x23 id=0x01 retval=-62 response=0xff resp_len=0 ... irq_cnt=4
0x05 skip reason=0x23_failed
```

IRQ 244 stayed **4** through the ~1s wait and after touches. Framing of the 4-byte GET_DYNAMIC_CONFIG packet is **correct**. The hang is **not** a TX bug. That does **not** prove all payloads are broken. 0x20/0x25 (empty-payload GETs) still work. 0x23 (1-byte payload GET) and earlier 0x26 (128-byte SET) both `-ETIME` with no ATTN. ENABLE_REPORT 0x05 was never tried (skipped). 0x23/0x26 may simply be ignored in `MODE_HOSTDOWNLOAD` 0x02. Native payload command for this mode is **0x30 DOWNLOAD_CONFIG** — next image if 0x05 times out and 0x20 still works.

---

## since75 packed, superseded (auto 0x05)

Banner `SaaiOS v019 since75`. Probe: `since75 0x05 ENABLE_REPORT 0x11 after 0x25 then live identify(false)/0x20`. Tar `/srv/media/saaios-boot-v019-since75.tar` sha256 **`86f56e34580cdd43ccbd01a6302ce19f01777276bafc982fade7ee94185d5433`**. **Do not overwrite. Do not flash** if a since76 menu image exists — auto-0x05 after REINIT is superseded by the queued lab.

---

## Current flash target: since76 (live20 ladder + dead-on-timeout)

Banner `SaaiOS v019 since76`. Probe: `auto live20 ladder 0/10/100/500ms` + `retval<0→dead response=ff`. Sysfs `/sys/kernel/saaios_touch/{action,status}`. Tokens: `live20` `run_app` `enable_report` `no_doze` (`app_config` optional, **not** in menu). Tar `/srv/media/saaios-boot-v019-since76.tar` (overwrite OK). Do **not** overwrite since54–since75.

HDL unchanged: oneshot 0x45, skip 0x1f when HDL 0x02, skip 0x25 after REINIT 0x20 OK, fallback input, **no auto 0x05/0x23/0x26/0x30**.

### LIVE settled (late post-boot experiments)

Clean boot: `0x45 → 0x42 → 0x20`, skip 0x25, irq=3. From separate clean reboots:

- `0x05 ENABLE_REPORT(0x11)` 4B → `-62`, no ATTN, irq stayed 3
- `0x24 DC_NO_DOZE=1` 6B → `-62`, no ATTN, irq stayed 3
- oneshot `0x30` `spi_len=4101` → `-62`, then control `0x20` also `-62` (jam)

SPI TX works; HDL 0x02 ignores late experiment cmds. Not a menu/shell bug.

Policy: any experiment `retval<0` → `state=dead`, status shows `response=ff` (never stale `01`), queue `-EBUSY` until reboot. Auto after REINIT: **only** empty `live20` (0x20) at 0/10/100/500 ms; stop ladder on first failure.

### Flash + first grep

```sh
dmesg | grep 'SaaiOS v019'
dmesg | grep 'Command = 0x'
dmesg | grep SAaiOS_TOUCH_DBG | grep -E 'live20 ladder|saaios_reinit_ok|skip touch_reinit|state=dead|TOUCH_EXP'
cat /sys/kernel/saaios_touch/status
```

Expect: no auto `0x05`/`0x25`/`0x30`; `start live20 ladder`; four `live20 delay_ms=` steps or `state=dead` on first timeout.

### Why read-floor 256 (LIVE first since76 dump)

`PREDICTIVE_READING` is on. After successful 0x20 (`plen=46`, `total=51`) `read_length` becomes **51**. HDL sets `rd_chunk_size=HDL_RD_CHUNK_SIZE` (**0**), so predictive assigns `read_length=total_length` of the *last* message even when `RD_CHUNK_SIZE` would have capped at 256. Next command 0x25 first SPI read is only 51 bytes. Header is valid `a5 01 80 00` (`plen=128`, needs 4+128+1=133). Then `syna_tcm_continued_read` expects marker `0xA5` + `STATUS_CONTINUED_READ`; it got **marker 0x25**. Host continued-read protocol, not IC mute.

Fix: never shrink the next IRQ first-read below **`SAAIOS_RD_FLOOR` 256** (`RD_CHUNK_SIZE`). Clamp **after** the PREDICTIVE_READING assignment, **including when `rd_chunk_size==0`**. Probe initial `read_length` is 256 (not `MIN_READ_LENGTH=9`). Before `syna_tcm_read` in `syna_tcm_read_message`, `syna_tcm_realloc_mem` `in.buf` to at least `read_length+1`. Log once per 0x25: `SAaiOS_TOUCH_DBG: read-floor=%u first_read=%u plen=%u total=%u continued=%d`. Expect **`continued=0`** (first_read 256 ≥ total 133). Do **not** change oneshot 0x45 TX. Do **not** send 0x1f. Do **not** auto 0x05/0x23/0x26.

### LIVE (first since76 flash — dead lab)

HDL still good: oneshot 0x45 STATUS_OK, IDENTIFY mode **0x02** `TD4150-12.0.12` packrat **2100027192**, skip 0x1f. Leftover 0x1b `need_*=0` `hdl_version=2`, queue REINIT.

- **0x20 GET_APPLICATION_INFO OK:** retval=0 `app_status=OK (0x0000)` resp_len=46. IRQ `raw=a5 01 2e 00 … read_length=51 report=0x01 plen=46`.
- **0x25 GET_TOUCH_REPORT_CONFIG FAILED -5 EIO, not timeout.** Command 0x25 sent. IRQ first read: `raw=a5 01 80 00 1a 08 10 08 read_length=51 report=0x01 plen=128 read_retval=-5`. `Incorrect header marker (0x25)` / `Failed to do continued read`. IC **did respond** with a valid A5 header and claimed payload 128. First SPI read is ~51 bytes (same as the working 0x20). Continued read then sees marker 0x25 (likely MOSI still holding cmd 0x25). Host continued-read / RX framing failure, not “IC ignores 0x25”. Empty-payload GET that fits in the first ~51B works. Larger RX does not.
- `touch_reinit/0x25 retval=-5` → no `input_dev` / no `sec_touchscreen`. irq_cnt stayed **4**.
- `/sys/class/sec` exists but **no `tsp`**. `find /sys -name '*saaios*'` empty. `/dev/input` event0–5 only (`gpio_keys`=event1, `sec-pmic-key`=event2). Root cause: lab sysfs lived inside `sec_fn_init()`, which is **never called**. Do **not** start calling `sec_fn_init()` (`test_init` may send extra SPI). Do **not** depend on `/sys/class/sec/tsp`.

Second pack (`014427f1…`, 46151680 bytes): `kernel_kobj` sysfs. **Not flashed.** That image still had the 51-byte first-read; the next rebuild added read-floor + `/sbin/touchlab`.

### LIVE (two boots after read-floor + touchlab)

Settled HDL still: oneshot 0x45, skip 0x1f, leftover 0x1b `need_*=0` `hdl_version=2`, REINIT 0x20 OK (`saaios_reinit_ok=1`), mode **0x02** `TD4150-12.0.12` packrat **2100027192**.

**Read-floor worked:** `read-floor=256 first_read=256 plen=128 total=133 continued=0`. **0x25 still -5** (IRQ raw `a5 01 80 00 1a 08 10 08` `read_length=256`). Optional log `padding=0x%02x at total-1` confirms padding-after-overread. Separate from the 0x30 oneshot.

**touchlab parser bug (fixed this image):** kernel `cat status` was `seq=0 state=ready` while `touchlab status` printed fake `dead retval=-62 response=255`. Status is two lines `seq= state= action= retval= response=` / `live20= mode= attn= irq= rx= report_touch=`. `touchlab` must parse those keys and dump them; `status` must not invent dead; missing/zero `live20` is not dead. `run` writes sysfs even if last live20 was 0; kernel `state=dead` → write **EBUSY** (correct).

**Boot A:** `echo enable_report` → TX `05 01 00 11` spi_len=4 ok. `done 0x05 retval=-62 response=ff` irq stayed 5. Then **live 0x20 also -62** → `state=dead`. `echo app_config` → EBUSY (lockout worked).

**Boot B (clean, skipped 0x05):** `echo app_config` → stock `zeroflash_download_app_config` hdl_version=2 need_app=0 (no version guess). TX `30 02 10 02 01 53` **prepared_len=512 actual=512** then `done 0x30 retval=-62 response=01` irq=5. Then live 0x20 -62 → dead.

Interpretation: HDL 0x02 is silent on payload cmds **when they are chunked**. 0x30 APP_CONFIG is ~4096+2; first chunk 512 then waiter -ETIME; IC likely waiting for CONTINUE_WRITE. Same class as broken chunked 0x45 (since64/65). Empty-payload 0x20 at REINIT works; after an incomplete chunked TX, 0x20 dies (parser/SPI jammed). **Do not send 0x05 first** on the next flash test — it also jams.

### LIVE (oneshot 0x30 — chunking **falsified**)

Boot cmds: `0x45`, `0x42`, `0x20`, `0x25`. No auto 0x05.

`echo app_config`:
- `0x30 force wr_chunk saved=512 forced=0`
- `payload=4098 remaining=4100 chunks=1 wr_chunk=0 spi_first=4101`
- `TX first=30 02 10 02 01 53 prepared_len=4101`
- `spi_sync retval=0 actual_length=4101` ← **oneshot really on the wire**
- `done 0x30 retval=-62 response=01` irq stayed 5
- live 0x20 also -62 → `state=dead`

Chunking is **not** why 0x30 fails. IC silent on full DOWNLOAD_CONFIG. Same jam of later 0x20 as after 0x05.

**This pack's discriminator:** after successful REINIT 0x20, auto-run **only** empty `live20` (GET 0x20) at delays **0 / 10 / 100 / 500 ms**. Measure whether empty GET stays alive over time. **Do not** auto-send 0x05/0x24/0x30 (those jam). Any experiment `retval<0` → `state=dead`, status `response=ff` (never stale `01`), queue `-EBUSY` until reboot. Stop the ladder on first failure. Removed the old `skip live 0x20 after timeout → state=ready` path.

Keep oneshot 0x45. Skip 0x1f when HDL 0x02. Skip 0x25 after 0x20 OK. No auto 0x05/0x23/0x26/0x30. Optional manual `app_config` (oneshot 0x30) still accepted via sysfs but **not** in menu/run-all.

Post-HDL experiments are **queued**, not run from sysfs store:

| Path | Mode | Tokens / format |
|------|------|-----------------|
| `/sys/kernel/saaios_touch/action` | 0200 write | `live20` \| `run_app` \| `enable_report` \| `no_doze` (`app_config` optional, not menu) |
| `/sys/kernel/saaios_touch/status` | 0400 read | `seq=… state=ready\|busy\|dead action=0x.. retval=… response=..` then `live20=… mode=.. attn=.. irq=.. rx=.. report_touch=..` |

`/sbin/touchlab`:

```
touchlab status
touchlab run live20|run_app|enable_report|no_doze
touchlab run-all [--touch-window SEC] [--reboot-on-dead]
touchlab monitor
```

Default touch-window **10** s. **Do not reboot** unless `--reboot-on-dead`. Dead → exit **75**. `touchlab status` dumps the two kernel lines then JSON. Write token, poll status until `state!=busy`. `run-all` starts with `live20` (never auto 0x05/0x30). Touch window finds **`sec_touchscreen` by name** under `/sys/class/input/event*/device/name`. fb Vol/Power menu in `os/init/init.c` stays as emergency local UI.

Init prefers those kernel_kobj paths. Store **only queues** `saaios_touch_wq`. Returns `-EBUSY` if `experiment_busy` or `state=dead`. Manual action cancels remaining auto ladder.

Guards before sending: `sent_0x45` && `0x45_retval==0` && **`saaios_reinit_ok`** && `IS_FW_MODE` && `!host_downloading`. Pre-TX log: irq_cnt, attn gpio, irq_enabled, mode, host_downloading, lp_state, wr_chunk, delay_ms. On any timeout/error: `saaios_mark_dead` → `state=dead` `response=ff`.

REINIT no longer sends 0x25. Fallback `sec_touchscreen` uses 0x20 `app_info` dims or A12s **720x1640** (no SPI).

Sequences (workqueue):

- `live20` / 0x20: empty control GET only. Does **not** mark dead on success. On timeout → dead. Auto ladder uses this only.
- `run_app` / 0x14: empty RUN_APPLICATION_FIRMWARE.
- `enable_report` / 0x05: ENABLE_REPORT payload `11`. Manual only — LIVE -62 jams.
- `no_doze` / 0x24: `set_dynamic_config(DC_NO_DOZE, 1)`. Manual only — LIVE -62.
- `app_config` / 0x30: optional oneshot `zeroflash_download_app_config` (`wr_chunk=0`). **Not** in menu/run-all. LIVE -62.

SPI TX dumps for 0x05/0x24/0x30 are tagged `SAaiOS_TOUCH_DBG TOUCH_EXP[seq]:`.

On-screen menu (`fb0`): Vol+ next, Vol− previous, short Power (&lt;2s, fire on **release**) writes the action token, long Power **2s** reboots immediately (does not wait for release). Ignore `EV_KEY value=2`. Debounce ~150 ms. `ppoll` + `sizeof(struct input_event)`. Keys by name `gpio_keys` / `sec-pmic-key`, fallback event1/event2.

**Grep immediately** (first 10 s, while `SaaiOS v019 since76` is still in the buffer). Init pings WDT ~2×/s; dmesg wraps after ~16 min. A later empty `Command = 0x` grep is wrap, not a missing opcode.

```sh
dmesg | grep 'SaaiOS v019'
dmesg | grep 'Command = 0x'
dmesg | grep SAaiOS_TOUCH_DBG | grep -E 'live20 ladder|saaios_reinit_ok|skip touch_reinit|state=dead|TOUCH_EXP'
cat /sys/kernel/saaios_touch/status
```

Expect: no auto `0x05`/`0x25`/`0x30`; `start live20 ladder`; four `live20 delay_ms=` steps (0/10/100/500) or `state=dead` / `response=ff` on first timeout. If autos pass, optional manual `echo live20` / `run_app` / `enable_report` / `no_doze` — **do not** send 0x05/0x30 first if you still care about the ladder.

Never flash DTBO.

---

## Next (logical order — do not skip ahead)

1. Flash **since76** AP tar (boot+vbmeta only). **Do not flash since66, since73, or since75.** **Grep immediately** (before wrap). Banner must be `SaaiOS v019 since76`. Confirm HDL path (oneshot 0x45, skip 0x1f, REINIT 0x20, **no** 0x25) and **no** auto 0x05/0x23/0x26/0x30. Confirm **`start live20 ladder delays_ms=0,10,100,500`**.
2. Watch auto `live20` steps. On any timeout → `state=dead` `response=ff` (never stale `01`); further writes **EBUSY** until reboot.
3. If all four delays OK: optional manual experiments; long-press Power 2s to reboot when dead.
4. Do **not** flash since66, since73, or since75. Do **not** rewind 0x45 oneshot / skip 0x1f / leftover REINIT 0x20. Do **not** retry 0x26 / auto 0x05/0x25/0x30.
5. **Optional:** stock Android dmesg on this unit / same DTBO. Still **missing** on the host.

Constraints that stay in force:

- Do not flash from the agent. Do not commit unless asked.
- Do not overwrite `/srv/media/saaios-boot-v019-since54.tar`, `...-since55.tar`, `...-since56.tar`, `...-since57.tar`, `...-since58.tar`, `...-since59.tar`, `...-since60.tar`, `...-since61.tar`, `...-since62.tar`, `...-since63.tar`, `...-since64.tar`, `...-since65.tar`, `...-since66.tar`, `...-since67.tar`, `...-since68.tar`, `...-since69.tar`, `...-since70.tar`, `...-since71.tar`, `...-since72.tar`, `...-since73.tar`, `...-since74.tar`, or `...-since75.tar`.
- Do not add IDENTIFY / `0x1f` / `0x42-from-0x02` / retry-0x26 / constructed-0x26 / `lcd_rst` experiments on the maze.
- Never pulse `gpio_lcd_rst`. Never unbind `synaptics_tcm_spi`.
- Do not treat leftover `0x1b` as 0x45 STATUS_OK. IDENTIFY mode 0x02 after oneshot 0x45 is waiter success only — not proof APP_CODE launched.
- Do not port the maze.

---

## Dead ends — DO NOT REVIVE

SET_TOUCH_REPORT_CONFIG **0x26** echo of 0x25 (since72: SENT, `retval=-62 timeout=1`, irq stayed **4**, HDL silent — do **not** retry); maze/constructed SET_TOUCH_REPORT_CONFIG **0x26** (stock `#define USE_DEFAULT_TOUCH_REPORT_CONFIG` no-op); `do_polling`; pulse `lcd_rst`; **maze PAGE_SIZE chunked 0x45 that treated IDENTIFY after chunk 0 as firmware-started** (stock `write_message` CONTINUE_WRITE with wr_chunk=max_write is since64, not this); `touch_init` in 0x04; RUN_APPLICATION **0x14** after 0x45; GET_ROMBOOT_INFO **0x40** while leftover 0x1b; drain-until-idle before 0x42; **unmask IRQ during idle 0x1b in RomBoot** (since37 storm ~8k/s); **`syna_tcm_romboot_drain_attn()` after 0x02** (holds IRQ; since41); forced **0x30**; 300 ms then CMD_IDENTIFY (since46); **0x42 after successful HDL 0x02** (since53: TX `42 00 00`, timeout); **0x1f from switch_mode** (since52); helper IDENTIFY after 0x1b (since51, IC deaf); premature `download_config` on `fw_status 00 00` (since50 race).

**Stop opcode experiments.**

---

## Historical (do not treat as current)

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

### boot-v019 since19–46 (RomBoot 0x45 → HDL 0x02)

Builtin td4150 driver, deferred 0x45 until panel up (since38), then stock one-shot 0x45 (since39). since47–54 are in the compact table above.

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
| since42 | After 0x02: `touch_init`, **IRQ live**, no drain, no 0x30, no identify wait | `gpio=1 irq_en=1 held=0`. IRQ=3 stable. **No REPORT_TOUCH.** |
| since43 | 0x30 APP_CONFIG with IRQ live; 0x1b/IDENTIFY complete 0x30 waiter | `0x30 APP_CONFIG retval=-62` ~1 s, gpio=1, held=0, IRQ still 3. |
| since44 | **0x30 one-shot** (no CONTINUE_WRITE at 512); HDL wr_chunk=0 in mode 0x02 | Still `0x30` `-62`. Chunking is not the cause. |
| since45 | Skip 0x30 (`need_app=0`); stock identify + `touch_reinit` with IRQ live | Both `-62` ~1 s. IDENTIFY started <1 ms after leftover 0x1b. |
| since46 | Dump part; wait 300 ms; one `CMD_IDENTIFY` `02 00 00` | Delay: gpio=1 irq_cnt=3. Then `-62`, no SPI RX, IRQ still 3. Settling is not the cause. |

Maze patches for that chain live in `os/patches/syna-tcm-*.patch`. They are **history**. Do not apply more of them. `RESET_ON_RESUME` is commented out and only lives in `!in_hdl_mode`.

---

## Clone / DTB / toolchain

Reuse [maazm7d/kernel_samsung_a12](https://github.com/maazm7d/kernel_samsung_a12) (a12s / Exynos 850 / S5E3830, **not** Helio A125F) **only as the current maze Image**. It is **not** a clean DXJ2 dump.

| | |
|--|--|
| Path | `os/third_party/kernel_samsung_a12` (**gitignored**) |
| Defconfig | `arch/arm64/configs/exynos850-a12snsxx_defconfig` |
| SoC | `CONFIG_SOC_EXYNOS3830=y` |
| TSP | `CONFIG_TOUCHSCREEN_SYNAPTICS_TCM=y` + SPI; Makefile builds `synaptics/td4150/` |
| Version | `4.19.111` |
| Live LOCALVERSION | `4.19.111-Project-Xed-KernelSU-Next+SUSFS` (clang 9.0.1) |

`os/bootimg/bootimg.py pack --kernel` replaces only the Image. **DTB stays stock** `os/build/stock-boot/dtb` from **A127FXXSDDXJ2** `boot.img`. Image has **no** appended FDT. Project-Xed `exynos3830.dtb` is **not** in the boot image. TSP `synaptics,fw_name` / `synaptics,lcdtype` come from **DTBO** (stock `dtbo` partition — we do not flash DTBO).

```sh
make -f os/Makefile kernel boot-v019
```

`os/bootimg/bootimg.py pack --kernel` replaces only the Image. **DTB stays stock** `os/build/stock-boot/dtb` from **A127FXXSDDXJ2** `boot.img`. Image has **no** appended FDT. Project-Xed `exynos3830.dtb` is **not** in the boot image. TSP `synaptics,fw_name` / `synaptics,lcdtype` come from **DTBO** (stock `dtbo` partition — we do not flash DTBO).

```sh
make -f os/Makefile kernel boot-v019
```

clang-9 on `PATH`, `O=os/build/kernel-out`. Copies Image to `os/build/kernel-Image`. `CONFIG_RTL8188EU` stays off. `make flash` still **refuses**. Packing another sinceN **must not overwrite** the since54–since75 tars (`SUBVERSION_V019` would have to change, and even then do not copy over those tars). Only **since76** may be overwritten. Samsung BL does **not** pass boot.img extra cmdline (`syna_corrupt_app=1` packed in since60 never reached `/proc/cmdline`).

After USB net is up, Windows reverse-forwards telnet and SSH (pubkey only; keys in `os/init/ssh/authorized_keys`):

```powershell
ssh -i $env:USERPROFILE\.ssh\saaios-odin-win -N `
  -R 2323:192.168.42.1:23 -R 2222:192.168.42.1:22 `
  mike@192.168.168.110
```

From R620: `ssh -i ~/.ssh/saaios_phone -p 2222 -o StrictHostKeyChecking=no root@127.0.0.1`. Direct from Windows: `ssh -i $env:USERPROFILE\.ssh\saaios-odin-win root@192.168.42.1`. Host keys are generated on first dropbear start (`-R`); expect TOFU each boot.
