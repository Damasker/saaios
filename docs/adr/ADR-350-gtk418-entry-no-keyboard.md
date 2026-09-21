# ADR-350: host GTK 4.18 Entry types OSK without `wl_keyboard`

## Статус

Принято, 2026-09-21. APP-04 GTK 4.18 toolkit on host glibc with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-349 следующий свободный номер — **350**. Не S33.

## Контекст

ADR-325: host glibc GTK 4.18 `Entry` with `grab_focus` types OSK
`hi!` when the host seat has `wl_keyboard`. ADR-346: packed musl
GTK 4.14.4 types on `SAAIOS_SEAT_NO_KEYBOARD=1`. ADR-347–349: Qt
5.15 and Qt 6.6.3 bind v2 on that seat and do not enable.

Same host `tests/gtk4_hello.py`, IME bound first, keyboard-less
seat:

1. Compositor logs `focus set to`. No `keyboard focus set`.
2. `text-input-v3 enable` + IME `Activate`.
3. OSK prints `GTK_ENTRY_TEXT=hi!`.

Host glibc GTK 4.18 is the same class as packed 4.14: a focused
Entry does not need `wl_keyboard`. gtk4-demo chrome still does not
enable (ADR-344). Qt still waits for keyboard focus.

## Decision

1. **APP-04 host GTK 4.18 types on a keyboard-less seat.** This is
   the panther seat class on host, not a phone flash.
2. **Do not claim a panther field was typed.** Touch-only chrome
   still needs a focused widget after the next displayd flash
   (ADR-311+319+328+339).
3. **Do not flash panther this week.**

## Consequences

- Host GTK 4.18 IME no longer hides behind a fake `wl_keyboard`.
  Qt probes still do (ADR-349).
- Rollback: drop
  `osk_ime_types_hi_bang_into_gtk4_entry_without_seat_keyboard`.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
