# ADR-349: host Qt 5.15 QLineEdit binds v2 without `wl_keyboard` and does not enable

## Статус

Принято, 2026-09-21. APP-04 Qt5 toolkit on host glibc with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-348 следующий свободный номер — **349**. Не S33.

## Контекст

ADR-328: host glibc Qt 5.15.15 `QLineEdit` with `setFocus` types OSK
`hi!` when the host seat has `wl_keyboard`. ADR-347/348: packed musl
Qt 6.6.3 and Qt 5.15.10 on `SAAIOS_SEAT_NO_KEYBOARD=1` bind
`text-input-v2` and do not enable.

Same host probe (`tests/qt5_lineedit.cpp`), IME bound first,
keyboard-less seat:

1. Compositor logs `focus set to` and `text-input-v2 get`.
2. No `keyboard focus set`.
3. No `text-input-v2 enable` in 700 ms after focus. IME stays
   inactive.

Host glibc Qt 5.15 is the same class as packed musl Qt 5 and Qt 6:
`setFocus` is not enough without `wl_keyboard`. GTK 4.14 still types
(ADR-346). qemu is not the gap.

## Decision

1. **Do not claim host Qt 5 types without `wl_keyboard`.** Bind ≠
   enable. Do not add a fake `wl_keyboard` on panther (ADR-012).
2. **Every APP-04 Qt probe on a panther-class seat waits for
   keyboard focus.** Compositor IME ordering is not the remaining
   Qt insert gap.
3. **Do not flash panther this week.**

## Consequences

- APP-04 host Qt 5.15, packed Qt 5.15, and packed Qt 6.6.3 all
  require `wl_keyboard` to enable v2. GTK 4.14 does not. Qt chrome
  on panther still needs a later widget-focus slice after the next
  displayd flash (ADR-311+319+328+339), not a host keyboard.
- Rollback: drop
  `host_qt5_lineedit_binds_v2_without_seat_keyboard_and_does_not_enable`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
