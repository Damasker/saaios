# ADR-347: packed Qt 6.6.3 QLineEdit binds v2 without `wl_keyboard` and does not enable

## Статус

Принято, 2026-09-21. APP-04 Qt6 toolkit on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-346 следующий свободный номер — **347**. Не S33.

## Контекст

ADR-336: packed musl Qt 6.6.3 `QLineEdit` with `setFocus` types OSK
`hi!` when the host seat has `wl_keyboard`. ADR-346: packed GTK 4.14
Entry types on `SAAIOS_SEAT_NO_KEYBOARD=1` (`focus set to`, no
`keyboard focus set`). Panther has no keyboard capability (ADR-012).

Same packed Qt 6 probe, IME bound first, keyboard-less seat:

1. Compositor logs `focus set to` and `text-input-v2 get`.
2. No `keyboard focus set`.
3. No `text-input-v2 enable` in 700 ms after focus. IME stays
   inactive.

Qt `setFocus` is not enough without `wl_keyboard`. GTK Entry still
types on that seat. Falkon LocationBar stays a chrome gap
(ADR-340), not this probe.

## Decision

1. **Do not claim packed Qt 6 types without `wl_keyboard`.** Bind ≠
   enable. Do not add a fake `wl_keyboard` on panther (ADR-012).
2. **Do not treat compositor IME ordering as the remaining Qt
   insert gap.** The toolkit waits for keyboard focus.
3. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass (reboot drops dest-no-lock).

## Consequences

- APP-04 GTK 4.14 probe types on a panther-class seat. Qt 6 probe
  does not. Qt chrome on panther still needs a later compositor
  and widget-focus slice, not a host `wl_keyboard`.
- Rollback: drop
  `packed_qt6_lineedit_binds_v2_without_seat_keyboard_and_does_not_enable`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
