# ADR-346: packed GTK 4.14 Entry types OSK without `wl_keyboard`

## Статус

Принято, 2026-09-21. APP-04 GTK 4.14 toolkit on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-345 следующий свободный номер — **346**. Не S33.

## Контекст

ADR-345: a packed musl 4.14.4 Entry types OSK `hi!` without
`grab_focus` once the host seat gives `wl_keyboard` focus. Panther
has no keyboard capability (ADR-012). Host `saai-displayd` now
honors `SAAIOS_SEAT_NO_KEYBOARD=1`: skip `seat.add_keyboard`,
`activate_toplevel` logs `focus set to` (not `keyboard focus set`).

Same `gtk414-entry`, `GTK4_NO_GRAB=1`, IME bound first:

1. No `keyboard focus set`.
2. `text-input-v3 enable` + IME `Activate` still happen
   (`text_ime::on_focus`).
3. OSK prints `GTK_ENTRY_TEXT=hi!`.

A focused GTK Entry does not need `wl_keyboard` for IME insert.
gtk4-demo chrome still does not enable (ADR-344): the focused widget
is not that Entry. Qt chrome still has `focusObject() == null`.

## Decision

1. **APP-04 packed GTK 4.14 types on a keyboard-less seat.** This is
   the panther seat class on host, not a phone flash.
2. **Do not claim a panther field was typed.** Touch-only chrome
   still needs a focused widget after the next displayd flash
   (ADR-311+319+328+339).
3. **Do not flash panther this week.**

## Consequences

- Host GTK 4.14 IME no longer hides behind a fake `wl_keyboard`.
- Rollback: always `add_keyboard` on the host build.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
