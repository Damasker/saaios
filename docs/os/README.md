# SaaiOS — OS track

Native bootable OS. Not Android. Not a Debian install. Platform Track (`crates/`, `services/`) stays a Linux userspace AI runtime (ADR-001, ADR-004).

Product and delivery sources of truth:

- [Product principles](PRODUCT-PRINCIPLES.md)
- [System intelligence and self-management](architecture/system-intelligence.md)
- [Cognitive Core / Work Scheduler mapping (Sprint 0)](architecture/cognitive-core-work-scheduler-mapping.md)
- [Wayland application-platform architecture](architecture/application-platform.md)
- [Human Interface Architecture v2](architecture/human-interface-architecture-v2.md)
- [Visual Language v1](architecture/visual-language-v1.md)
- [Component Library v1 inventory and base specifications](ui/component-library-v1.md)
- [Product visual target v1 (concept boards)](ui/product-visual-target-v1.md)
- [Development process and Definition of Done](DEVELOPMENT_PROCESS.md)
- [Quality strategy](QUALITY.md)
- [Sprint roadmap](sprints/README.md)
- [Pixel 7 delivery path](sprints/PIXEL-PATH.md)
- [Visual-system delivery roadmap](sprints/VISUAL-ROADMAP.md)
- [Work Scheduler v2 delivery roadmap](sprints/WORK-ROADMAP.md)
- [World Model / Observation delivery roadmap](sprints/WORLD-ROADMAP.md)
- [Attention projection delivery roadmap](sprints/ATTN-ROADMAP.md)
- [Unified Authority Model delivery roadmap](sprints/AUTH-ROADMAP.md)
- [Memory, Learning & Provenance delivery roadmap](sprints/MEM-ROADMAP.md)

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

S00–S32 and the single-device HIA implementation track are complete with their
documented limitations. Device delivery follows
[PIXEL-PATH.md](sprints/PIXEL-PATH.md): **VUI-04** on shell, MEM-01 runtime
flash as P0. VUI-02 is almost closed (boot-image fonts blocked). Multi-node
PCE work remains deferred until a second physical runtime node exists.

## Rules

See [ENGINEERING.md](ENGINEERING.md).
