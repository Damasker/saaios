# ADR-306: Packaged Falkon commits a shm hello-frame on host displayd

## Статус

Принято, 2026-09-21. APP-06 host hello-frame. Not a panther
`appd` install. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd.

## Нумерация

После ADR-305 следующий свободный номер — **306**. Не S33.

## Контекст

ADR-269 named Falkon. ADR-281 packed `QtWebEngineProcess` and
`.pak`/`v8`. `find … -maxdepth 1` missed
`usr/lib/libproxy/libpxbackend-1.0.so`. musl then exits 127 on
`libproxy` before any `xdg_toplevel`. Qt6 also prefers
`wayland-egl`; without Mali (ADR-024) that path maps a surface and
never hashes a shm buffer.

APP-06 asked for a packaged hello-frame on the proven Qt Wayland
path. This week's panther displayd flash is spent. Host
`qemu-aarch64-static -L $package` can still drive the aarch64 musl
binary against windowed `saai-displayd`, the same method ADR-021
used for GTK4.

## Decision

1. **Pack `libpxbackend`.** Copy it into `package/lib/` so sandbox
   `/lib` (and qemu `-L`) relocate `libproxy`.
2. **Do not ship Wayland EGL plugins.** Drop `libqwayland-egl`,
   `libqt-plugin-wayland-egl`, `libdrm-egl-server`, `libqeglfs`,
   `libqminimalegl`. Generic Wayland + `QBackingStore` `wl_shm`
   is the hello-frame. GPU stays ADR-024.
3. **Host qemu frame.** `tests/falkon_frame.rs` spawns displayd,
   runs packed `bin/falkon` under qemu, and requires
   `new xdg_toplevel 1280x800 fullscreen=false` plus
   `frame sha256=`. Same evidence class as ADR-026/305.
4. **Not panther.** Do not `appd install`. binfmt cannot open
   host `/lib/ld-musl-aarch64.so.1` for `QtWebEngineProcess`;
   that is a qemu-host limit. Panther has the musl loader.
5. **No NetInternet.** Capabilities stay empty. No browse.

## Consequences

- A rebuild without `libpxbackend` fails the recipe and the host
  test.
- Panther launch is still the next displayd-experiment week.
- Rollback: drop the extra copy/`rm` and `tests/falkon_frame.rs`.

## Verification

Host: `cargo test -p saai-displayd --offline falkon_commits_an_shm_frame`
with `dist/panther/packages/org.saaios.demo.falkon` from
`build-falkon-package.sh`. Needs `qemu-aarch64-static`. No panther
install. Leave Сейчас.
