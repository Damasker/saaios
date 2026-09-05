# SaaiOS — OS track

Native bootable OS. Not Android. Not a Debian install. Platform Track (`crates/`, `services/`) stays a Linux userspace AI runtime (ADR-001, ADR-004).

```text
              SaaiOS OS
                 │
       ┌─────────┼─────────┬─────────┐
       ↓         ↓         ↓         ↓
 SM-A127F    Pixel 7      Pi 5      x86
 (first)    (native)    (later)    (dev)
```

On-device splash string: `SaaiOS`.

## Pixel 7 target

The native Pixel 7 (`panther`) target boots its own static PID 1 and brings
up display, touch, buttons, F2FS userdata, Wi-Fi, Bluetooth, speakers,
haptics, brightness, time sync and the SaaiOS runtime without Android
userspace. See the [live bring-up record](targets/panther/README.md) and
[reproducible source layout](../../os/targets/panther/README.md).

## Phase 0 gate

Phase 0 is green on this unit (stock firmware on disk, recovery understood). `make` still **refuses** to flash. Current TD4150 work: [kernel-touch.md](targets/sm-a127f/kernel-touch.md).

## Target dossier (A12 Nacho)

| File | What |
|------|------|
| [hardware.md](targets/sm-a127f/hardware.md) | SoC, GPU, PMIC, display, touch, BOM variance |
| [unit.md](targets/sm-a127f/unit.md) | This phone (PDA, input map, stock blobs) |
| [kernel-touch.md](targets/sm-a127f/kernel-touch.md) | **Canonical TD4150 bring-up** (protocol, blob, since54, OSS vs maze, next) |
| [boot-chain.md](targets/sm-a127f/boot-chain.md) | BootROM → sboot → kernel |
| [partitions.md](targets/sm-a127f/partitions.md) | eMMC GPT (reference dump + what we still need) |
| [recovery.md](targets/sm-a127f/recovery.md) | Download mode, stock restore, what is **not** BROM |
| [known-risks.md](targets/sm-a127f/known-risks.md) | Knox, AVB, binary fuse, DSN TWRP caveats |
| [bringup-plan.md](targets/sm-a127f/bringup-plan.md) | Phases 0–12, first milestone, rollback |
| [hardware-control-plan.md](targets/sm-a127f/hardware-control-plan.md) | Command the board (display / input / power); not Android, not AI |
| [wifi-usb.md](targets/sm-a127f/wifi-usb.md) | RTL8188EUS USB dongle — not onboard `wlan`; not TSP |
| [sources.md](targets/sm-a127f/sources.md) | Citations |

## First milestone (Phase 1)

```text
SaaiOS booting...
kernel: OK
rootfs: OK
console: OK
```

No GUI. Serial or USB gadget console is enough.

## Rules

See [ENGINEERING.md](ENGINEERING.md).
