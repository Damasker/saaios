# Engineering rules (OS track)

Every task, including worker-only software steps:

```text
Goal
Current state
Change
Test
Acceptance criteria
Rollback
```

## Size of a change

One verifiable step. Forbidden as a single patch: "refactored entire graphics subsystem".

Allowed:

```text
Goal: Display solid framebuffer.
Acceptance: Phone boots and screen becomes white.
Rollback: Restore boot-v003.img from the unit backup.
```

## Working return points

Each phase ends with an artifact that restores the previous phase without archaeology:

| Phase end | Return point |
|-----------|----------------|
| 0 | Stock firmware tarball + this dossier |
| 1 | `images/stock/` dump + `make restore` |
| 2 | Known-good ramdisk that prints a shell |
| 3 | Known-good splash framebuffer |
| n | Tagged image `boot-vNNN.img` |

## Hardware vs worker

Worker may: docs, ADR, code, tests, cross-compile, pack images, analyze logs.

Human with the phone may: OEM unlock, Download mode, USB/UART, flash, photo of screen, `dmesg` / last_kmsg capture.

## Forbidden until Phase 0 gate is green

- `heimdall flash`, Odin, `dd` to device
- OEM unlock
- Custom `vbmeta` / TWRP
- Writing BL / sboot / EFS / PARAM

Phase 0 is green on this unit. `make` still **refuses** to flash. Optional host artifact: PARAM-only Odin tar (`make -f os/Makefile up_param`) — human flashes **BL** slot, never sboot/TZ/EFS/boot/vbmeta. Rollback tar is mandatory.
