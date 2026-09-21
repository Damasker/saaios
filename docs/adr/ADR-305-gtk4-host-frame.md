# ADR-305: Host GTK4 commits a shm frame on saai-displayd

## Статус

Принято, 2026-09-21. APP-02 toolkit half on host. Not a panther
frame. Not Visual v1 sign-off. PIN stays null. Do not flash displayd.

## Нумерация

После ADR-304 следующий свободный номер — **305**. Не S33.

## Контекст

ADR-025 crashed Alpine GTK 4.14.4 musl on panther at
`gdk_wayland_display_create_shm_surface` (`height=1776831`).
ADR-266 advertises `wp_fractional_scale_v1` (`preferred_scale=120`)
and `wp_viewporter`. ADR-286 locks the GDK size formula on a
wayland-client probe. ADR-294 keeps native clipboard deny-by-default
so GTK can open a display without smithay's ungated path.

APP-02 still needed a **real GTK4** commit, not another synthetic
shm attach. This week's displayd flash is spent; host x86 GTK 4.18
glibc can still prove the compositor contract.

## Decision

1. **Real toolkit.** `tests/gtk4_hello.py` opens one GTK4 window
   (`GDK_BACKEND=wayland`, `GSK_RENDERER=cairo`) against a spawned
   `saai-displayd`.
2. **Same Qt evidence.** displayd must log `client connected`,
   `new xdg_toplevel 1280x800 fullscreen=false`, and
   `frame sha256=…`. Exit 0. No `1776831`.
3. **Not panther.** Debian GTK 4.18 glibc ≠ Alpine 4.14.4 musl on
   Tensor G2. APP-02 stays open until a GTK4 frame on panther
   (ADR-026 method) or a later ADR refuses GTK4.
4. **No flash.** Do not replace panther `saai-displayd`.

## Consequences

- Host GTK4 cairo/shm maps and commits on the compositor that
  already ships fractional-scale + honest shm size.
- Panther GTK4 remains the hardware gate.
- Rollback: drop `tests/gtk4_frame.rs` and `tests/gtk4_hello.py`.

## Verification

Host: `cargo test -p saai-displayd --offline gtk4_commits_an_shm_frame`.
Needs `python3` + `gir1.2-gtk-4.0`. No panther flash. Leave Сейчас.
