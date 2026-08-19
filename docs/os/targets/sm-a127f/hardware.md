# SM-A127F/DSN — hardware

Codename in trees: **a12s**. Marketing: Galaxy A12 **Nacho**.

> **Not** the original Galaxy A12 `SM-A125F` (MediaTek Helio P35 / PowerVR GE8320).
> `SM-A127F` is **Exynos 850**. MediaTek BROM / SP Flash Tool is the wrong emergency path.

Confidence: **C** = confirmed from public specs / dumps / XDA; **L** = likely / BOM-variant; **N** = needs a dump from *this* unit.

## Identity to confirm on the phone (human)

Settings → About → Software information, plus the silkscreen under the battery cover / box:

| Field | Expected |
|-------|----------|
| Model | `SM-A127F` or `SM-A127F/DSN` |
| Platform | `exynos850` |
| Kernel | `4.19.111-…` (vendor) |
| Binary / bootloader | `A127FXXU…` — **U-number matters** |

If the model is `SM-A125F`, stop. Different SoC, different this entire dossier.

## SoC

| | |
|--|--|
| SoC | Samsung **Exynos 850** (`S5E3830`) **C** |
| Process | 8 nm **C** |
| CPU | 8× Cortex-A55 @ up to 2.0 GHz **C** |
| ISA | AArch64 (ARMv8.2-A) **C** |
| GPU | ARM **Mali-G52 MP1** **C** |
| RAM | 3 / 4 / 6 GB LPDDR4X **C** (read `/proc/meminfo` on unit) |
| Storage | eMMC 5.1 **C** — vendor **BOM-split**: Samsung `DP6DAB` / `DX68MB` or Micron `G1J9R8` **L** |

## Power

| Part | Role | Conf |
|------|------|------|
| **s2mpu12** | PMIC | C (Device Info HW `OTHER`) |
| **sm5714** | charger + USB-PD companion | C |
| Battery | 5000 mAh Li-Ion/Li-Po, USB-C 15 W | C |

Do not poke charger/PMIC registers until `powerd` exists. Brick risk is real.

## Display

| | |
|--|--|
| Panel | 6.5″ PLS TFT / Infinity-V, **720×1600**, 60 Hz **C** |
| Controller | Samsung DECON / DPU in Exynos 850 **L** (vendor kernel) |
| LCM id seen in the wild | `ID_0xba6220` **L** (one Device Info HW dump) |
| Path for Phase 3 | **fb0** on this unit (stock 4.19: `/sys/class/drm` missing). DRM only if we turn it on in a custom kernel **C** |

Mainline DRM on this DPU is **not** a Phase 1 requirement. First graphics = fill framebuffer.

## Touch

**BOM variance — do not assume one driver.**

| Sample | Driver |
|--------|--------|
| Device Info HW #99921 | **NVT-ts** (Novatek) |
| Device Info HW #78967 | **synaptics_tcm** |

Phase 6 must detect `/dev/input/event*` and map ABS_MT_*, not hardcode an IC.

## Other I/O (for later services)

| | Typical | Conf |
|--|---------|------|
| USB | Type-C 2.0 | C |
| Wi-Fi / BT | onboard combo (need DTS from Samsung kernel) | N |
| Modem | Exynos baseband; partition `RADIO` | C |
| Audio | `aud3004x`, amp `sma1303` | L |
| IMU | ST `LIS2DLC12` | L |
| Fingerprint | Goodix or `gw36t1` | L |
| NFC | `pn547` on some SKUs | L |
| SAR | Semtech `SX9360` | L |

## UART / debug

No public, reliable “USB-C UART for free” recipe for a12s. Expect:

- Samsung USB gadget when a custom kernel enables it;
- last_kmsg / pstore if the vendor kernel left it;
- hardware UART / ISP only after a board photo (last-resort, see [recovery.md](recovery.md)).

Phase 1 console target: **USB serial gadget** or **stock Download mode + known-good boot**, not soldering.

## What this unit still must report

From the phone, no flashing:

```text
getprop ro.product.model
getprop ro.boot.hardware
getprop ro.bootloader
uname -a
cat /proc/cpuinfo | head
ls /dev/input
ls /sys/class/drm  /sys/class/graphics
```

Paste that into `targets/sm-a127f/unit.md` when captured.
