# ADR-356: packed gtk4-demo click binds v3 without `wl_keyboard` and does not enable

## Статус

Принято, 2026-09-21. APP-04 GTK chrome on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-355 следующий свободный номер — **356**. Не S33.

## Контекст

ADR-344: packed gtk4-demo `--run=entry` on a keyboard seat binds
v3 after center click `640 400` and does not enable. Packed GTK
4.14 Entry types OSK without `wl_keyboard` (ADR-346). xdg
Activated is window activation (ADR-352), not a demo field.

Same gtk4-demo `--run=entry`, `SAAIOS_SEAT_NO_KEYBOARD=1`, **one**
click `640 400` after Activated (not a Y sweep, not OSK):

1. Pre-click: framed shm `aa71d2ae…`, `xdg activated`,
   `focus set to`. No `keyboard focus set`. No v3 get.
2. `injected click`. Cursor surfaces `849af246…` / `8c6de10e…`.
3. `text-input-v3 get`. No `text-input-v3 enable`.

Pointer tap is enough to bind. `wl_keyboard` is not required.
Demo Entry still does not enable. Packed `gtk414-entry` is not
this window.

## Decision

1. **Do not claim gtk4-demo Entry typed on a panther-class seat.**
   Bind ≠ enable. Packed Entry (ADR-346) does not transfer to
   `--run=entry`.
2. **Do not add a fake `wl_keyboard` (ADR-012). Do not flash
   panther this week.** Next displayd flash must carry
   ADR-311+319+328+339+352. AUTH-10 stays a later flash-week
   device pass.

## Consequences

- APP-04 GTK chrome on a panther-class seat still needs a focused
  demo Entry, not only Activated plus a center click.
- Rollback: drop
  `packed_gtk4_demo_click_without_seat_keyboard_does_not_enable_v3`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk4_demo_click_without_seat_keyboard_does_not_enable_v3`.
dest-no-lock kept. Do not flash. Leave Сейчас.
