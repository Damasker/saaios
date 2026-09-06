# ADR-006: Minimal wayland-rs server for S01

## Status

Accepted, 2026-09-06.

## Context

S01 needs one headless `wl_shm`/stable `xdg-shell` toplevel and evidence for
configure/ack/commit, focus, synthetic input, close, malformed clients and a
deterministic software frame. It does not need output discovery, DRM/KMS,
rendering, scene graphs or desktop policy.

Two host spikes were measured on Rust 1.97.1 in Ubuntu/WSL2 on an NTFS
worktree. The timings are local comparison data, not CI performance targets:

- direct `wayland-server` 0.31.14, `wayland-protocols` 0.32.13 and
  `wayland-client` 0.31.15: 44 unique `cargo tree` lines for the
  `saai-displayd` package; the first all-target test build took 89 seconds;
- Smithay 0.7.0 with `default-features = false` and only
  `wayland_frontend`: 72 unique `cargo tree` lines; a clean compile-only
  frontend probe took 83.86 seconds and 472264 KiB maximum resident memory.

The direct spike additionally completed the real multi-process protocol test.
The Smithay spike proved that its smallest relevant frontend builds, but did
not implement a second compositor.

## Decision

Use the maintained `wayland-rs` server and protocol crates directly for S01.
Keep protocol state in a small testable state machine and implement only the
globals exercised by the vertical slice.

This avoids adopting Smithay's broader compositor abstractions before SaaiOS
has a renderer, backend or shell policy that benefits from them. The selected
pure-Rust backend does not link `libwayland-server`; its Linux cross-build
inputs are Rust crates plus libc syscalls. Smithay remains the preferred
re-evaluation candidate when DRM/KMS, libinput, multiple outputs or a scene
graph enter scope.

## Consequences

- The S01 protocol boundary is real Wayland, not a mock socket.
- SaaiOS owns more request validation and lifecycle code.
- The implementation deliberately supports one XRGB8888 toplevel and one
  synthetic keyboard path.
- Framework migration remains possible because clients use standard Wayland
  protocols and compositor policy is isolated from the demo client.

## Rejected alternative

Adopting Smithay now would provide mature helpers for future backends, but S01
would pay for abstractions and cross-compile inputs it does not exercise. The
measured minimal frontend graph is larger, while the direct implementation is
already covered by the required protocol and recovery scenarios.
