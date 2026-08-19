# SaaiOS — OS track

Native bootable OS. Not Android. Not a Debian install. Platform Track (`crates/`, `services/`) stays a Linux userspace AI runtime (ADR-001, ADR-004).

```text
              SaaiOS OS
                 │
       ┌─────────┼─────────┐
       ↓         ↓         ↓
 SM-A127F      Pi 5       x86
 (first)     (later)    (dev)
```

On-device splash string: `SaaiOS`.

## Phase 0 gate (now)

Nothing destructive until:

1. This dossier exists for the **exact** model (`SM-A127F/DSN`).
2. Matching **stock firmware** is downloaded and checksummed (human).
3. Recovery procedure in [`targets/sm-a127f/recovery.md`](targets/sm-a127f/recovery.md) is understood.

**Do not flash yet.**

## Target dossier (A12 Nacho)

| File | What |
|------|------|
| [hardware.md](targets/sm-a127f/hardware.md) | SoC, GPU, PMIC, display, touch, BOM variance |
| [boot-chain.md](targets/sm-a127f/boot-chain.md) | BootROM → sboot → kernel |
| [partitions.md](targets/sm-a127f/partitions.md) | eMMC GPT (reference dump + what we still need) |
| [recovery.md](targets/sm-a127f/recovery.md) | Download mode, stock restore, what is **not** BROM |
| [known-risks.md](targets/sm-a127f/known-risks.md) | Knox, AVB, binary fuse, DSN TWRP caveats |
| [bringup-plan.md](targets/sm-a127f/bringup-plan.md) | Phases 0–12, first milestone, rollback |
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
