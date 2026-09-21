# ADR-372: packed GTK 4.14 competing pane auto-enables and types OSK without a tap

## Статус

Принято, 2026-09-21. APP-04 GTK toolkit on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-371 следующий свободный номер — **372**. Не S33.

## Контекст

ADR-360/366/369: Qt competing non-IM pane holds focus until a tap
on the `QLineEdit`. ADR-356: packed gtk4-demo `--run=entry` binds
v3 after a center click and does not enable.

Same packed GTK 4.14 Entry probe, `GTK4_COMPETE=1`: frameless
320×200, Entry 40 px on top, focusable `GtkBox` pane below with
`grab_focus` at startup. `SAAIOS_SEAT_NO_KEYBOARD=1`. No click:

1. `xdg activated`, `focus set to`, `text-input-v3 enable`.
2. OSK types `GTK_ENTRY_TEXT=hi!`.

A competing pane does **not** keep GTK IM off the Entry. gtk4-demo
bind-without-enable is not this class. Do not more gtk4-demo Y
clicks. Do not claim the demo Entry typed.

## Decision

1. **Do not treat competing widgets as the reason gtk4-demo stays
   untyped.** Packed Entry in a compete window auto-enables and
   types. Demo remaining is a field that never enables. Do not add
   a fake `wl_keyboard` (ADR-012).
2. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352. AUTH-10 stays a later
   flash-week device pass.

## Consequences

- APP-04 gtk4-demo remaining is demo chrome that never
  `zwp_text_input_v3::enable`, not GTK tap-to-focus among widgets.
- Rollback: drop `GTK4_COMPETE` and
  `osk_ime_types_hi_bang_into_alpine_gtk414_entry_competing_pane`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
osk_ime_types_hi_bang_into_alpine_gtk414_entry_competing_pane`.
dest-no-lock kept. Do not flash. Leave Сейчас.
