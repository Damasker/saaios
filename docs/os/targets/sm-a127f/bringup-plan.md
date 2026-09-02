# SM-A127F/DSN — bring-up plan

A12 is the first **target**, not the OS. HAL backends later: `samsung-a127f` | `raspberry-pi` | `x86`.

Worker does software. Human does USB, Download mode, photos, logs. Hardware command (fb0, TD4150, poweroff) after this ramdisk track: [hardware-control-plan.md](hardware-control-plan.md).

**No flash until recovery gate is green** ([recovery.md](recovery.md)).

---

## Phase 0 — dossier (this commit)

| | |
|--|--|
| Goal | Written hardware/boot/partition/recovery facts for SM-A127F/DSN |
| Current | Empty OS track |
| Change | `docs/os/` + ADR-004 |
| Test | Docs review; model confirmed on the phone |
| Acceptance | Human agrees this is Exynos, not Helio P35; stock FW download started |
| Rollback | n/a (docs only) |

**Human next:** confirm model, photo of software info + Download mode, download stock 4-file + SHA256.

---

## Phase 1 — control of boot

```text
stock sboot
    ↓
our boot.img (vendor kernel + our ramdisk)
    ↓
PID 1
    ↓
console
```

Milestone:

```text
SaaiOS booting...
kernel: OK
rootfs: OK
console: OK
```

Worker prepares (no device write): unpack/repack `boot.img`, minimal init, `make boot.img` / `make restore` **scripts that refuse to run without `images/stock/`**.

First flash: **BOOT ± VBMETA only**.

| | |
|--|--|
| Goal | Custom ramdisk prints a line on console |
| Rollback | Stock `boot.img` + `vbmeta.img` |

---

## Phase 2 — tiny userspace

BusyBox + musl (or static BusyBox), `/dev` `/proc` `/sys` `/tmp` `/run`. No Debian.

```text
PID 1
 ├── (later) devd
 └── shelld
```

Target size: tens of MB. Boot to shell in a few seconds if the vendor kernel allows.

---

## Phase 3 — pixels without Android

Path: **`/dev/fb0`** on this unit (stock has no `/sys/class/drm`). DRM/KMS only later, if the vendor kernel can be built with it.

First image: solid color, then:

```text
SaaiOS
Kernel        OK
```

No Chromium, no animation.

---

## Phase 4–5 — retained-mode renderer, e-ink rules — **console: v032 LIVE**

Dirty-rect UI. No 60 fps loop. Update on `state changed → event → dirty → blit`.

Constraints: no animation unless requested, no transparency/blur/gradients by default, no polling loop.

**LIVE 2026-09-02 (`boot-v032`, human Odin AP, human confirmed):** `os/init/init.c` `draw_console()` converted from a full-panel clear+redraw on every call to a retained per-line field cache (`struct console_field` / `set_field()`) — banner, resolution, and the three help lines paint once; the five dynamic lines (bat/bl/usb/mem/aud) repaint only when their formatted text differs from what was last painted. Same layout, same call sites/frequency (periodic tick + Vol± key handler), just no more whole-panel flash on every redraw. Human: **"экран не мигает теперь."** This closes the console piece of Phase 4-5; a general reusable retained-mode/dirty-rect renderer for future non-console UI is still open.

---

## Phase 6 — input

`/dev/input/event*` → `inputd` → `TouchDown/Move/Up/Tap/LongPress/Swipe`. UI never sees evdev.

---

## Phase 7 — event bus

Same idea as Platform `event-bus`, OS-native:

`battery.changed`, `touch.tap`, `power.button`, `clock.minute`, …

---

## Phase 8–9 — services + HAL

`powerd` `networkd` `audiod` … talk to `PowerBackend` / `DisplayBackend` / …  
`platform/samsung-a127f` implements sysfs; UI never reads `/sys/class/power_supply` directly.

---

## Phase 10–11 — apps + permissions

Not APKs. Manifest + binary + capability list. Dangerous ops go through policy (same spirit as Platform `AskUser`).

---

## Phase 12 — AI layer

Platform Track (`saaios-runtime`) as a service on this OS:

```text
user text → tools → policy → powerd/process.list/…
```

Do **not** start this before a shell and one sensor (battery or CPU) work.

---

## Iteration template (copy per experiment)

```text
Goal:
Current state:
Change:
Test: (human: flash / photo / log)
Acceptance:
Rollback: images/stock/... or boot-vNNN.img
```

Example next after Phase 0 gate:

```text
Goal: Unpack stock boot.img on the build host; print header + ramdisk file list.
Acceptance: Documented header version and init binary path in unit.md.
Rollback: none (host-only).
```
