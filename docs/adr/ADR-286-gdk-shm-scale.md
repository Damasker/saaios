# ADR-286: GDK shm size from preferred_scale, not uninitialized scale

## Статус

Принято, 2026-09-21. APP-02 compositor contract. Not a GTK4 frame.
Not Visual v1 sign-off. PIN stays null. Do not flash displayd.

## Нумерация

После ADR-285 следующий свободный номер — **286**. Не S33.

## Контекст

ADR-025 localized GTK4's crash to
`gdk_wayland_display_create_shm_surface` with `height=1776831` and
an uninitialized `double *scale`. ADR-266 advertises
`wp_fractional_scale_v1` with `preferred_scale=120`. APP-02 still
needs a GTK4 frame on panther. Host can lock the size formula
without flashing: buffer = round(logical * preferred/120). At
scale 1.0 that is identity, never 1776831.

## Decision

1. **Formula.** Host test uses the GDK size from `preferred_scale`.
   Headless output is 1920×1080. Scale 120 → shm 1920×1080 (or the
   xdg configure size if non-zero).
2. **Attach.** Create that `wl_shm` buffer, attach, commit.
   displayd must stay up.
3. **Not GTK4.** This does not run `gtk4-demo`. Panther frame still
   waits. Do not flash.

## Consequences

- A regression that drops `preferred_scale` or inflates height
  fails the host test before another panther experiment.
- Rollback: drop `tests/gdk_shm_scale.rs`.

## Verification

Host: `cargo test -p saai-displayd --offline gdk_shm` including
`gdk_scale_one_is_identity_not_adr025_height`,
`preferred_scale_yields_an_honest_shm_buffer`. No panther flash.
