# SM-A127F/DSN — known risks

Ordered by “this bricks the week”.

## 1. Wrong SoC assumption

`A12` in a shop listing can be **A125F (MT6765)** or **A127F (Exynos 850)**. Mixing TWRP, kernels, or “BROM unbrick” between them is how phones become archaeology.

## 2. Anti-rollback (binary U)

Bootloader fuse / SW REV. You can usually **not** flash an older `A127FXXU<n>` over a newer `n`. Custom images must match the running U. Kernel source zip must match that generation (U3/U4/U5/U7/SDxx).

## 3. AVB / vbmeta

Unsigned `boot.img` with stock vbmeta → bootloop. Community always patches or disables vbmeta. We only do this **after** stock vbmeta is saved.

## 4. OEM unlock is a wipe

Unlocking in Download mode **factory-resets**. OEM toggle can be hidden (date −14 days + check update) until Samsung KG/RMM is happy. Unlock **voids Knox** (`0x1` forever): Samsung Pay / Secure Folder / some banking gone. Irreversible.

## 5. Prenormal / KG / Vaultkeeper

After unlock or flash, device may refuse further custom images until it phones home. Symptom: OEM unlock grey, Download mode “KG: Prenormal”. Fix is wait/network, not more flashing.

## 6. EFS / IMEI

Writing `EFS`, `SEC_EFS`, `CPEFS` can lose cellular identity. Never in our flash scripts.

## 7. TWRP on DSN

XDA: TWRP install “may not work for a127f/DSN” on some older packages. We skip TWRP until console-on-boot works. Format Data + `multidisabler` destroys encryption; not needed for a custom ramdisk in `boot`.

## 8. SUPER / dynamic partitions

Replacing `system` without repacking SUPER bricks the Android userspace. Phase 1 must **not** touch SUPER. Our rootfs lives in ramdisk (or a file in userdata later).

## 9. Display / touch BOM

Novatek vs Synaptics; LCM id not unique. A “white screen” milestone can still have a dead digitizer. Don’t couple graphics and input in one experiment. This unit is **Synaptics TCM TD4150** (`tsp_synaptics/td4150_a12s_boe.bin` on `spi1.2`); `event3` = `sec_touchscreen`. PID 1 looping `FBIOBLANK` UNBLANK (~500ms) made `syna_tcm_early_resume` / `syna_tcm_resume` log **abnormal call** — v010 stopped that loop. v010 still unblanked twice at boot (ioctl `FBIOBLANK` + sysfs `fb0/blank`); the second resume did not finish and `event3` stayed silent. v011 unblanks once. `/sys/class/sec/tsp/input/enabled` store() is the same resume path — do not write it after a successful unblank, and do not send `probe_enable` on `cmd`.

**Unbind is destructive.** `echo synaptics_tcm.0 > …/synaptics_tcm_spi/unbind` then bind: probe `Incorrect header code (0x01)` / `-5`, `sec_touchscreen` **gone until reboot**. Do not unbind/bind to “wake” touch.

**Ramdisk cannot power off.** `/sys/power/state` is `freeze mem` only. No `poweroff`/`reboot` from this init.

**IRQ 244 stuck at 7** after a correct single unblank: IC not scanning (`REPORT_TOUCH` never arrives). Factory `check_connection` / `sensitivity_mode` poke CMD `0x2a` and time out — do not retry. Fix is a vendor-kernel resume patch ([kernel-touch.md](kernel-touch.md)), not init.

## 10. Mainline kernel

Exynos 850 mainline is early (A13-oriented). Booting mainline on A12s as step 1 is a trap. Vendor 4.19 first.

## 11. No UART by default

Without a gadget console, a failed boot looks like a black brick. First custom boot should enable USB serial or keep stock recovery intact so Download mode is always reachable (sboot independent of our ramdisk — **as long as we don’t flash BL**).

## 12. ISP / test points

Last-resort eMMC. Wrong voltage kills the board. Not a worker task.

## 13. Samsung boot.img tail (SEANDROID / AVB)

Stock `boot.img` is **44 MiB** (full BOOT partition). After the Android v2 payload Samsung appends `SEANDROIDENFORCE` (+ `SignerVer03`) and an AVB footer (`AVB0` / `AVBf`).

`boot-v001` packed only the kernel+ramdisk+dtb (~37 MiB), no `SEANDROIDENFORCE`, no pad. On this phone that matched: long Samsung splash, then reboot, never our cyan `fb0`. Odin may also leave the old AVB tail in the unused 6 MiB of the partition, so sboot hashes the new payload against the leftover footer.

v002 appends `SEANDROIDENFORCE` and zero-pads to 44 MiB (wipes leftover AVB). **Same splash-then-reboot on the unit.** Unlocked/orange did **not** skip partition vbmeta: stock `vbmeta.img` has `flags=0` and still hashes `boot`.

v003 empty 4096-byte vbmeta (`flags=3`, algorithm NONE) was **rejected by Odin** (`vbmeta.img` then `RQT_CLOSE`). `boot.img` in that tar did write. v004 patches **stock** vbmeta in place (same 9744 bytes, Magisk-style flags OR 3). Rollback must restore **both** stock `boot.img` and stock `vbmeta.img`.

## 14. glibc as PID 1

A glibc-static `/init` can hang before `main` (it wants `/proc`, TLS, rseq). v001 used that. v002 is freestanding syscalls only.

## 15. Splash can stay after a successful kernel

v004 Odin **PASS**, then hang on the Samsung logo (no reboot). That is different from v001/v002 (logo then watchdog reboot = AVB). Exynos DECON keeps the **boot splash** (`logo.jpg` from UP_PARAM) until a real display takeover; a one-shot `fb0` mmap may never become visible. That leftover is **not** the orange Power-key unlock warning (see §16). v005 retries blank/unblank, cycles colors, writes `/dev/pmsg0`, and brings up USB ACM.

## 16. PARAM / UP_PARAM (unlock-warning JPEGs)

sboot blits JPEGs from **UP_PARAM** before the kernel. The orange “press Power to continue” screen is `svb_orange.jpg` in that partition, **not** in `boot.img`. Community nologo tars replace those JPEGs with black frames of the same size.

A bad `up_param` can replace the cyan Download-mode splash (`download.jpg` / `warning.jpg`) with garbage or a black screen while sboot is otherwise fine — Download mode may still work, but you cannot see it. Never flash the full stock **BL** zip to “fix a logo”: that writes `sboot` / TZ / keystorage. Only flash a tar whose sole member is `up_param.bin.lz4`, and keep `saaios-up_param-stock-restore.tar` on the same share.

`make flash` / `make up_param-flash` refuse. Human flashes Odin **BL** slot only.

## Residual risk after following the gate

sboot stays stock ⇒ Download mode should survive a bad `boot.img`. That is the whole point of Phase 1 scope. A PARAM-only flash does not touch sboot, but a corrupt `up_param` can hide Download-mode art until the stock restore tar is applied.
