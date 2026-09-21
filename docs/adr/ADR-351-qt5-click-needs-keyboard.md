# ADR-351: host Qt 5.15 pointer click without `wl_keyboard` does not enable v2

## Статус

Принято, 2026-09-21. APP-04 Qt5 toolkit on host glibc with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-350 следующий свободный номер — **351**. Не S33.

## Контекст

ADR-349: host glibc Qt 5.15.15 `QLineEdit` with `setFocus` on
`SAAIOS_SEAT_NO_KEYBOARD=1` binds `text-input-v2` and does not
enable. GTK 4.18 types on that seat (ADR-350). Panther has touch,
not `wl_keyboard` (ADR-012). Host seat still has `wl_pointer`.

Same host probe, IME bound first, keyboard-less seat, one
`inject-click 160 20` after compositor focus (widget origin, not a
Y sweep):

1. Before the click: `focus set to`, `text-input-v2 get`, no
   enable, no `keyboard focus set`.
2. Compositor logs `injected click 160 20`.
3. No `text-input-v2 enable` in 700 ms after the click. IME stays
   inactive.

Pointer activation does not substitute `wl_keyboard` for Qt v2
enable. `setFocus` plus a click is still not enough. GTK still
types without either.

## Decision

1. **Do not claim a pointer or panther touch click will enable a
   Qt field without `wl_keyboard`.** Bind ≠ enable.
2. **Do not add a fake `wl_keyboard` on panther (ADR-012).** Do
   not sweep Y.
3. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass (reboot drops dest-no-lock).

## Consequences

- APP-04 Qt probes on a panther-class seat need keyboard focus to
  enable v2. Pointer/touch is not the remaining compositor IME
  gap. GTK 4.14/4.18 still type. Qt chrome still needs a later
  widget-focus slice after the next displayd flash
  (ADR-311+319+328+339).
- Rollback: drop
  `host_qt5_lineedit_click_without_seat_keyboard_does_not_enable_v2`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
