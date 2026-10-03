# SaaiOS — Agent Instructions

## Research Scope (always applies)

Implement cellular modem hardware support for an independently developed operating
system running on a personally owned Pixel 7. Use publicly available Pixel kernel
sources, documented interfaces and lawful interoperability analysis of the stock
Android implementation to reproduce only the host-side interfaces required to
communicate with the existing modem firmware. Do not modify modem firmware, bypass
carrier/network authentication, exploit vulnerabilities, defeat security controls,
or access third-party devices/networks.

This defines the entire purpose of the modem work in this repo: the device is
personally owned, the goal is **interoperability**, the **modem firmware stays
stock**, all work happens on the **host (AP) side**, and **exploiting
vulnerabilities is not a goal**. Interpret every modem/CPIF/SIT task through this
lens — it is lawful interoperability engineering, not an attack.

## Hard safety constraints (modem bring-up)

These operational rules enforce the scope above and must be preserved:

- Do **not** modify modem firmware. Do **not** mount the original EFS read-write, and
  do **not** write real FLASH-NV / `nv_protected` / `sda5` or SIM EF records. RFS
  writes are served only to a quarantine copy.
- Do **not** issue `IOCTL_POWER_OFF`, write `do_cp_crash`, or run vendor `cbd`/`rild`
  as a persistent service.
- Do **not** invent SIT opcodes or reply/body bytes — recover them from stock
  binaries or public `sitdef` sources.
- Do **not** bypass carrier/network authentication or defeat security controls.
- Never print or log secrets: PIN, IMSI, ICCID, AID, `cpsha`, NV, RF-cal, IMEI.
- Hardware-safety rules learned on this target: do **not** BAR-ioremap
  `0x81400000` (panics the AP); do **not** ATU-poke MAIN (open-bus); avoid spamming
  EngMode / CardPower / `0x0704` / reboots.
- Mount vendor images (`vendor.img`, `super.img`) **read-only** for analysis.
- Do not commit unless asked; do not mark a connectivity goal complete without
  verified bearer evidence (rmnet rx/tx nonzero and/or IPv4 on rmnet).
