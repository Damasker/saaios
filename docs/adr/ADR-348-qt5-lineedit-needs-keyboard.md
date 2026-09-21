# ADR-348: packed Qt 5.15 QLineEdit binds v2 without `wl_keyboard` and does not enable

## Статус

Принято, 2026-09-21. APP-04 Qt5 toolkit on host qemu with a
panther-like seat. Not a panther typed field. Not Visual v1
sign-off. PIN stays null. Do not flash displayd this week.

## Нумерация

После ADR-347 следующий свободный номер — **348**. Не S33.

## Контекст

ADR-330: packed musl Qt 5.15.10 `QLineEdit` with `setFocus` types OSK
`hi!` when the host seat has `wl_keyboard`. ADR-347: packed Qt 6.6.3
on `SAAIOS_SEAT_NO_KEYBOARD=1` binds `text-input-v2` and does not
enable. PCManFM-Qt is that Qt 5.15 toolkit.

Same packed Qt 5 probe, IME bound first, keyboard-less seat:

1. Compositor logs `focus set to` and `text-input-v2 get`.
2. No `keyboard focus set`.
3. No `text-input-v2 enable` in 700 ms after focus. IME stays
   inactive.

Qt 5.15 `setFocus` is the same class as Qt 6.6.3: not enough without
`wl_keyboard`. GTK 4.14 still types (ADR-346). PCManFM Filter/PathEdit
stay chrome gaps (ADR-341), not this probe.

## Decision

1. **Do not claim packed Qt 5 types without `wl_keyboard`.** Bind ≠
   enable. Do not add a fake `wl_keyboard` on panther (ADR-012).
2. **The Qt Wayland v2 client waits for keyboard focus.** This is
   not Qt 6-only and not compositor IME ordering.
3. **Do not flash panther this week.**

## Consequences

- APP-04 packed Qt 5 and Qt 6 probes both require `wl_keyboard` to
  enable v2. GTK 4.14 does not. Panther chrome still needs a later
  widget-focus slice after the next displayd flash
  (ADR-311+319+328+339), not a host keyboard.
- Rollback: drop
  `packed_qt5_lineedit_binds_v2_without_seat_keyboard_and_does_not_enable`.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime`. dest-no-lock kept.
Do not flash. Leave Сейчас.
