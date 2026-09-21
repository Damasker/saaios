# ADR-407: packed GTK 4.14 DropDown search types OSK hi!

## Статус

Принято, 2026-09-21. APP-04 GTK 4.10+ dropdown field on host qemu.
Not a panther typed field. Not Visual v1 sign-off. PIN stays null.
Do not flash displayd this week.

## Нумерация

После ADR-406 следующий свободный номер — **407**. Не S33.

## Контекст

ADR-406 maps a configured `xdg_popup` from GtkDropDown
`gtk_widget_activate`. `GTK4_DROPDOWN_SEARCH=1` turns on
`gtk_drop_down_set_enable_search`:

1. Keyboard-less seat.
2. `xdg popup` configure succeeds.
3. v3 enable + IME Activate.
4. OSK grows v3 surrounding to 3 (`hi!`).

That is a foreign field inside current GTK dropdown chrome.
gtk4-demo combobox pixel hit-target stays unproven. Falkon URL
remaining is still chrome (ADR-398). PathEdit remaining is still
chrome (ADR-395).

## Decision

1. **A GTK 4.14 DropDown search Entry types OSK on host without
   wl_keyboard.** Panther still needs the unflashed displayd
   stack.
2. **Do not more combobox Y. Do not more Falkon clicks as the
   next default. Do not more PathEdit clicks.**
3. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Modern GTK dropdown search can take IME on host. Qt LocationBar
  remaining is still disable-after-focus-flash.
- Rollback: drop `GTK4_DROPDOWN_SEARCH` and the OSK asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk414_dropdown_search_osk_types_hi_bang`. dest-no-lock
kept. Do not flash. Leave Сейчас.
