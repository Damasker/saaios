# ADR-004: Native OS track is separate from Platform track

## Status

Accepted

## Context

SaaiOS Platform (this repo's original product) is a Linux userspace AI runtime. ADR-001 forbids tying the product to a custom kernel.

A second, longer effort is now in scope: a native OS that can boot on a phone (first target: Galaxy A12 SM-A127F/DSN), later on Pi 5 and x86. Mixing that into Platform crates would stall both tracks.

## Decision

- **Platform Track** stays the AI userspace on an existing Linux (VM, Pi, later phone Linux).
- **OS Track** lives under `docs/os/` and (when code appears) `os/`. It owns boot, init, display, input, HAL, and a tiny userspace.
- Galaxy A12 is the **first hardware target**, not the OS. Backends: `samsung-a127f` | `raspberry-pi` | `x86`.
- Platform later consumes OS services through the same tool/policy/event APIs it already has.
- No flashing, OEM unlock, or partition writes until the target's recovery procedure is written **and** a matching stock firmware archive exists for that exact binary.

## Consequences

- Platform keeps shipping on the VM without waiting for phone bring-up.
- Phone work proceeds as one-step experiments with rollback images.
- Naming on device splash: `SaaiOS` (product). Informal "SAII OS" in notes maps to the same track.
