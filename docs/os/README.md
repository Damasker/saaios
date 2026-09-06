# SaaiOS — OS track

Native bootable OS. Not Android. Not a Debian install. Platform Track (`crates/`, `services/`) stays a Linux userspace AI runtime (ADR-001, ADR-004).

Product and delivery sources of truth:

- [Product principles](PRODUCT-PRINCIPLES.md)
- [System intelligence and self-management](architecture/system-intelligence.md)
- [Wayland application-platform architecture](architecture/application-platform.md)
- [Development process and Definition of Done](DEVELOPMENT_PROCESS.md)
- [Quality strategy](QUALITY.md)
- [Sprint roadmap](sprints/README.md)

```text
            SaaiOS OS
                │
       ┌────────┼────────┐
       ↓        ↓        ↓
   Pixel 7     Pi 5     x86
  (primary)   (later)   (dev)
```

On-device splash string: `SaaiOS`.

## Pixel 7 target

The native Pixel 7 (`panther`) target boots its own static PID 1 and brings
up display, touch, buttons, F2FS userdata, Wi-Fi, Bluetooth, speakers,
haptics, brightness, time sync and the SaaiOS runtime without Android
userspace. See the [live bring-up record](targets/panther/README.md) and
[reproducible source layout](../../os/targets/panther/README.md).

## Current milestone

Delivery is governed by the [sprint roadmap](sprints/README.md). The current
gate is S01 system identity on the installed Pixel 7 image; work does not
advance to the Wayland vertical slice until S01 Evidence is complete.

## Rules

See [ENGINEERING.md](ENGINEERING.md).
