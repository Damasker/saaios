# SM-A127F — hardware control (OS track)

Endpoint is **command of this board**, not “Android boots” and not Platform Track AI. Bring-up of a ramdisk ([bringup-plan.md](bringup-plan.md)) already happened. Touch resume patch detail: [kernel-touch.md](kernel-touch.md).

Worker: docs, kernel patch, pack `boot.img`. Human: Download mode + Odin AP. **Do not flash from `make`.** Do not unbind TSP. Do not race another worker cloning `kernel_samsung_a12` or packing boot-v012.

```text
our init / BusyBox  →  sysfs + evdev + fb0  →  board
```

---

## Goal

Userspace we own (PID 1 now; a small compositor later) can **command**:

1. **Display** — own `fb0` (DRM only if a custom kernel ever exposes it); brightness / backlight.
2. **Input** — real finger coordinates from TD4150 (`event3`), plus volume and power keys.
3. **Power** — suspend/resume, and a real power-off path **in the kernel**, not a ramdisk fake.
4. Then, **one subsystem at a time**: audio, Wi‑Fi / modem, USB roles.

## Non-goals

| Out | Why |
|-----|-----|
| GSI / Lineage / “Android boots again” | Wrong product |
| KernelSU / Magisk / root-as-product | Tree may ship it; we do not ship it |
| Flashing BL / TZ / EFS / sboot | Download mode lives in sboot; we keep it |
| Mainline 4.19 replacement as step 1 | Vendor kernel first |
| Inventing a TSP driver | Reuse Samsung `synaptics/td4150` |
| Platform Track (`crates/`, `services/`) | Separate track |

---

## Now (boot-v011, 2026-08-19)

Stock **DTB + kernel** in `boot.img` (`4.19.111-27127798`). Our ramdisk only. Vbmeta patched Magisk-style (flags OR 3). Pack: `SEANDROIDENFORCE` + pad **44 MiB**. Odin **AP**, human-only.

| Piece | State |
|-------|--------|
| PID 1 | our `/init` + BusyBox |
| Display | `fb0` text splash (unblank **once**) |
| USB | RNDIS `0525:A4A2`; telnet **23 / 2323**; httpd **8080** |
| Volume | `event1` `gpio_keys` — live |
| Power key | `event2` `sec-pmic-key` — live |
| Touch | `event3` `sec_touchscreen` TD4150 `spi1.2` — **node exists, silent** |
| TSP IRQ | **244** stuck at **7**; IC not scanning (`REPORT_TOUCH` never arrives) |
| Resume | `in_hdl_mode` + idle HDL → skip `do_reset` / `CMD_REZERO` |
| Poweroff | ramdisk `/sys/power/state` = **`freeze mem` only** |
| Unbind TSP | **destructive** until reboot (`event3` gone) |

Already visible in sysfs (read-only until a phase owns it): `fb0`, gpio volume, PMIC power key, battery, `/sys/class/sec/tsp/cmd`, USB gadget.

Kernel tree to **reuse** (a12s / Exynos 850, not Helio): [maazm7d/kernel_samsung_a12](https://github.com/maazm7d/kernel_samsung_a12). Do not invent a driver.

## Endpoint vs now

```text
now:      splash + RNDIS + keys; TSP deaf; no poweroff
          ↓  Phase A  (kernel Image + resume patch)
touch:    IRQ 244 moves; event3 packets
          ↓  Phase B
input:    init reads evdev (log, then on-screen cursor)
          ↓  Phase C
display:  backlight / brightness; blank once if needed
          ↓  Phase D
power:    long-press policy; kernel poweroff; suspend
          ↓  Phase E+
radios / audio / USB roles — listed, not scheduled
```

---

## Phases

Each phase is one ENGINEERING.md loop: Goal / Current / Change / Test / Acceptance / Rollback. Smallest verifiable step. Tagged `boot-vNNN.img` is the return point. Human flashes Odin AP; `make flash` **refuses**.

### Phase A — vendor Image + TSP `do_reset`

**Depends on:** v011 ramdisk kept as-is; a12s tree built once. Pack path: `bootimg.py pack --kernel` and `make -f os/Makefile boot-v012` (stock DTB; copies `/srv/media/saaios-boot-v012.tar` only if `arch/arm64/boot/Image` exists).

| | |
|--|--|
| Goal | TD4150 scans after resume; `event3` delivers ABS_MT |
| Current | Stock Image; `syna_tcm_resume` `goto mod_resume`; IRQ=7 |
| Change | Build `Image` from a12s tree. Two-site patch in `synaptics_tcm_core.c` only ([kernel-touch.md](kernel-touch.md)): HDL idle → `do_reset`; `reset_and_reinit` fall-through when `host_downloading==0`. Pack boot-v012 = **new Image + v011 ramdisk** + stock DTB, 44 MiB pad, patched vbmeta |
| Test | Human Odin AP. Telnet: `grep synaptics /proc/interrupts`; `hexdump -C /dev/input/event3` while touching |
| Acceptance | IRQ 244 **increments**; finger packets on `event3`; `do_reset` in dmesg. No unbind. No `check_connection` retry |
| Rollback | `saaios-boot-stock-restore.tar` or last good v011 tar |

Do **not** `#define RESET_ON_RESUME` alone (wrong branch). Do **not** write a new driver. Prefer Samsung OSS zip **A127FXXSDDXJ6** (binary D, same as this unit’s **DXJ2**) if Project-Xed `LOCALVERSION` misbehaves — same two-site patch.

### Phase B — input into our userspace

**Depends on:** Phase A acceptance (packets exist).

| | |
|--|--|
| Goal | PID 1 (or a tiny `inputd`) consumes real coordinates |
| Current | Keys work; `event3` unread / was empty |
| Change | Read evdev. First: log `ABS_MT_POSITION_*` over telnet. Then: a 1-pixel or box cursor on `fb0` |
| Test | Finger move → log lines and/or cursor tracks |
| Acceptance | Touch, vol±, power all visible to **our** userspace. UI still must not own raw evdev long-term (`inputd` later) |
| Rollback | Previous boot-vNNN (ramdisk-only if Image is good) |

### Phase C — display control

**Depends on:** Phase B not required, but do **not** couple a new blank path with a TSP experiment. v011 unblanks **once**.

| | |
|--|--|
| Goal | Command brightness / backlight; optional overlay |
| Current | We paint `fb0`; backlight sysfs unused |
| Change | Find and drive the panel backlight node (sysfs). Optional: one extra layer / status line. Blank **only** if a dedicated boot proves resume still delivers touch |
| Test | Brightness steps visible; screen still paints; IRQ 244 still moves after any blank |
| Acceptance | Userspace sets brightness without a second accidental `syna_tcm_resume`. No DRM required |
| Rollback | Previous boot-vNNN |

### Phase D — power policy

**Depends on:** keys (already true); kernel Image we can change (Phase A).

| | |
|--|--|
| Goal | Long-press power in init; real `poweroff`; then suspend/resume |
| Current | `/sys/power/state` = `freeze mem`; ramdisk cannot halt |
| Change | **Kernel:** enable a real power-off path (`poweroff` / `disk` or vendor equivalent — whatever this 4.19 actually implements; do not fake it in BusyBox). **Init:** long-press `KEY_POWER` → that path. Then `echo mem` suspend and a resume that still has touch (Phase A patch must survive) |
| Test | Long-press → device off. Power-on from cold. Later: suspend, wake on power key, `event3` still alive |
| Acceptance | Off is a kernel shutdown, not a hang. Resume does not re-deaf the TSP |
| Rollback | Previous Image; do not iterate power and TSP in one flash |

### Phase E+ — listed, not scheduled

One subsystem per tagged image. Order is preference, not a calendar:

| # | Subsystem | First probe (read-only) | Command later |
|---|-----------|-------------------------|---------------|
| E1 | Audio | `AUD3004X` / `sma1303` ALSA nodes | `aplay` tone |
| E2 | USB roles | gadget we already have (RNDIS) | host vs device if UDC allows |
| E3 | Wi‑Fi | `wlan` / firmware path | `wpa_supplicant` or equivalent |
| E4 | Modem | `RADIO` partition exists; do not write it | later; EFS is forever off-limits |

Do not start E+ until A–D have return-point images.

---

## Risks

| Risk | What to do |
|------|------------|
| **Project-Xed** `LOCALVERSION` vs stock **DXJ2** | Image may boot and still be the wrong personality. If modules, firmware paths, or TSP probe diverge, rebuild from Samsung **DXJ6** zip (binary D) + the same patch |
| Stock **DTB** + new **Image** | Keep stock DTB/DTBO until a boot proves otherwise. Do not ship a new DTB in the same experiment as the TSP patch |
| **AVB** | Patched stock vbmeta (flags OR 3) with every AP tar. Rollback = stock `boot` **and** stock `vbmeta` |
| **Unbind TSP** | Never. Rebind already killed `event3` until reboot |
| Second `FBIOBLANK` | Already broke resume once (v010). Unblank once until C is a dedicated boot |
| Factory `cmd` `0x2a` | `check_connection` / `sensitivity_mode` time out. Do not retry as a “wake” |
| `make flash` | **Refuses.** Human Odin AP only. Never BL/TZ/EFS |

---

## Flash rule (explicit)

```text
make flash          → refuse
make *-flash        → refuse
Odin AP             → human, BOOT + VBMETA only
Odin BL (full zip)  → never (sboot / TZ)
```

Pack stays: `--seandroid --pad-to 46137344`, Magisk-style vbmeta. Restore tar stays next to every experiment.
