# SM-A127F — TD4150 resume / REPORT_TOUCH (v012 plan)

Phone stays on **boot-v011** (stock `4.19.111-27127798` + our ramdisk) until a human flashes an Odin AP tar. **Do not flash from make.** **No v012 Image** on disk yet.

## Clone

Reuse [maazm7d/kernel_samsung_a12](https://github.com/maazm7d/kernel_samsung_a12) (a12s / Exynos 850 / S5E3830, **not** Helio A125F).

| | |
|--|--|
| Path | `os/third_party/kernel_samsung_a12` (**gitignored**) |
| HEAD | `6d2c200` (shallow clone; sparse-checkout **disabled**, full worktree) |
| Defconfig | `arch/arm64/configs/exynos850-a12snsxx_defconfig` |
| SoC | `CONFIG_SOC_EXYNOS3830=y` |
| TSP | `CONFIG_TOUCHSCREEN_SYNAPTICS_TCM=y` + SPI; Makefile builds `synaptics/td4150/` |
| Version | `4.19.111` (same as unit) |

Caveat: this tree is **Project-Xed / KernelSU / SUSFS** (`CONFIG_LOCALVERSION="-Project-xed-Ksun-4.c11"`), not a clean OSS dump of **A127FXXSDDXJ2**. The TD4150 driver is still Samsung vendor code. Closest official zip on [opensource.samsung.com](https://opensource.samsung.com/uploadSearch?searchValue=SM-A127F) is **A127FXXSDDXJ6** → `SM-A127F_SWA_13_Opensource.zip` (binary D, Android 13). Prefer that zip if we want a stock-like Image; same patch applies.

Do not hit giant GitHub recursive APIs to re-clone.

## Patch

File: `os/patches/syna-tcm-resume-hdl-idle.patch`

Apply (already done on the local tree; kernel tree is not committed):

```text
patch -d os/third_party/kernel_samsung_a12 -p1 < os/patches/syna-tcm-resume-hdl-idle.patch
```

Two sites in `drivers/input/touchscreen/synaptics/td4150/synaptics_tcm_core.c` (not the unused generic `synaptics_tcm/` copy):

1. `syna_tcm_resume`: after `wait_hdl` succeeds, `goto do_reset` when `host_downloading==0`. Keep `mod_resume` only while a download is in flight.
2. `syna_tcm_reset_and_reinit`: do not skip identify + `mod_cb->reinit` when HDL is idle (`in_hdl_mode && host_downloading`).

`RESET_ON_RESUME` is **commented out** at `synaptics_tcm_core.c:43`. Uncommenting it alone **does not help this unit**: it only runs in the `!in_hdl_mode` branch.

No DT reset GPIO on this board (`reset_gpio < 0`); `do_reset` is **software** `CMD_RESET`, not a GPIO.

## Driver files

Built path (a12s Makefile): `drivers/input/touchscreen/synaptics/td4150/`

| File | Why |
|------|-----|
| `synaptics_tcm_core.c` | `syna_tcm_resume` / `syna_tcm_reset_and_reinit` / `syna_tcm_wait_hdl` |
| `synaptics_tcm_core.h` | `/* #define USE_FLASH */`, `in_hdl_mode`, `REPORT_TOUCH = 0x11` |
| `synaptics_tcm_touch.c` | `touch_report()` gated by `init_touch_ok` |
| `synaptics_tcm_zeroflash.c` | HDL; `CONFIG_TOUCHSCREEN_SYNAPTICS_TCM_ZEROFLASH=y` |

## Live proof (v011, 2026-08-19)

`syna_tcm_resume start(0) (1)` → `Interrupt already enabled` → **~100 ms** (`HOST_DOWNLOAD_WAIT_MS`) → `mod_resume` → `end`. **No** `do_reset` line.

That 100 ms is `syna_tcm_wait_hdl()`: always `msleep(100)`, then return if `host_downloading==0`. So **`in_hdl_mode` is true** and HDL is already idle. Resume then `goto mod_resume` and **skips** software reset / `CMD_REZERO`.

IRQ `244 synaptics_tcm` stays at **7**. Periodic kworker `tc:0 … irq:1 // v:000E` = driver thinks IRQ is enabled; IC is not scanning.

No ramdisk trick: no `tsp/enabled`, `incell_power_control` is `lcdoff_test`, `check_connection` / reading `sensitivity_mode` runs CMD `0x2a` and **times out**. **Do not unbind** `synaptics_tcm_spi` (kills `event3`). `/sys/power/state` = `freeze mem` only.

## Toolchain (R620)

This tree’s top-level `Makefile` hardcodes `CC=clang`. `AndroidKernel.mk` wants AOSP clang + `CROSS_COMPILE=aarch64-linux-android-` and `CLANG_TRIPLE=aarch64-linux-gnu-`. `build_kernel.sh` is:

```sh
export PLATFORM_VERSION=13
export ANDROID_MAJOR_VERSION=t
export ARCH=arm64
make ARCH=arm64 exynos850-a12snsxx_defconfig
make ARCH=arm64 -j64
```

Exact cmdline once clang-9 (or AOSP clang for 4.19) is on disk:

```sh
cd os/third_party/kernel_samsung_a12
export PLATFORM_VERSION=13 ANDROID_MAJOR_VERSION=t ARCH=arm64
make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- CLANG_TRIPLE=aarch64-linux-gnu- \
  CC=/path/to/clang-9 exynos850-a12snsxx_defconfig
make ARCH=arm64 CROSS_COMPILE=aarch64-linux-gnu- CLANG_TRIPLE=aarch64-linux-gnu- \
  CC=/path/to/clang-9 -j"$(nproc)"
```

Output Image: `os/third_party/kernel_samsung_a12/arch/arm64/boot/Image`.

Checked on **R620** this session:

| Tool | State |
|------|--------|
| `aarch64-linux-gnu-gcc` | Debian 14.2.0 (present; kernel still wants clang) |
| `os/third_party/clang-9` | **missing** |
| `clang-9` package | **missing** |
| `/usr/bin/clang-17` | present; **too new** for 4.19 — do not use |
| LLVM tarball | **do not download** in the pack session |

No kernel compile until clang-9 (or matching AOSP clang) is installed. Host gcc-14 is not a substitute.

## Pack boot-v012

`os/bootimg/bootimg.py pack --kernel` replaces the unpacked stock kernel; **DTB stays stock** (`os/build/stock-boot/dtb`).

```text
make -f os/Makefile boot-v012
```

- If `$(KERNEL_IMAGE)` exists: pack **new Image + v011 ramdisk + stock DTB**, `--seandroid --pad-to 46137344`, patched vbmeta (flags OR 3). Write `os/build/saaios-boot-v012.tar` and copy `/srv/media/saaios-boot-v012.tar`.
- If Image is **missing** (current): print that fact, do **not** pack, do **not** copy the media tar.

`make -f os/Makefile flash` still **refuses**. Default `make -f os/Makefile` still builds **v011**. Human flashes Odin AP only.

Until an Image exists: stay on v011. IRQ=7 deaf is expected.
