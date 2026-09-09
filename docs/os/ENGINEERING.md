# Engineering rules (OS track)

The sprint workflow, Definition of Ready/Done and quality gates are defined in
[DEVELOPMENT_PROCESS.md](DEVELOPMENT_PROCESS.md),
[QUALITY.md](QUALITY.md), and the [sprint roadmap](sprints/README.md). This file
continues to define the mandatory shape and rollback discipline of every small
OS-track change.

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
Goal: Display a verified framebuffer mode.
Acceptance: Pixel 7 boots slot A and renders the expected color table.
Rollback: Switch to the known-good slot or restore its recorded image.
```

## Working return points

Each phase ends with an artifact that restores the previous phase without archaeology:

| Phase end | Return point |
|-----------|----------------|
| 0 | Verified factory archive and SHA-256 |
| 1 | Stock Android preserved in slot B |
| 2 | Known-good slot-A image with USB console |
| 3 | Known-good native display and input image |
| n | Image linked to source commit, hashes and Evidence |

## Hardware vs worker

Worker may: docs, ADR, code, tests, cross-compile, pack images, analyze logs.

Human with the phone may: OEM unlock, Download mode, USB/UART, flash, photo of screen, `dmesg` / last_kmsg capture.

## Hardware safety gate

- Never write bootloader, radio, secure-element, calibration or identity
  partitions as part of an ordinary sprint.
- Never flash an image whose target, source commit and SHA-256 are unknown.
- Keep the verified Android fallback slot and a known-good SaaiOS image.
- Build scripts may produce images but must not flash hardware.
- Human approval is required for every flash, active-slot change and rollback.

### Pixel 7 development-device storage authorization

The operator has confirmed that the dedicated Pixel 7 (`panther`) contains no
user data that must be preserved. This is a scoped authorization for SaaiOS
development, not a blanket authorization to overwrite partitions:

- `userdata` may be erased, reformatted and used by SaaiOS for code, mutable
  application data, tests and recovery exercises;
- `metadata` is shared device state, not general scratch space: ordinary work
  may create, replace or remove files only below `/metadata/saaios`;
- slot-A boot partitions may be written only through the existing hardware
  safety gate with an exact target, source commit, artifact hash, rollback and
  per-flash operator approval;
- slot B remains the default Android rollback and is not reusable storage;
- `persist`, `devinfo`, bootloader, radio/modem, secure-element, factory,
  calibration, hardware-identity, rollback-protection and equivalent
  device-unique partitions are read-only or forbidden;
- before using any previously unused partition, inspect and record its exact
  name, purpose and current slot. If it is not explicitly allowed above, do
  not write it without a separate operator decision.

Absence of user data does not relax the rules against committing credentials,
device-unique values or private runtime state.
