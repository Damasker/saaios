# ADR-402: packed GTK 4.14 popover Entry types OSK hi!

## Статус

Принято, 2026-09-21. APP-04 GTK field in xdg_popup on host qemu.
Not a panther typed field. Not Visual v1 sign-off. PIN stays null.
Do not flash displayd this week.

## Нумерация

После ADR-401 следующий свободный номер — **402**. Не S33.

## Контекст

ADR-401 maps a configured `xdg_popup` from
`gtk_menu_button_popup`. `GTK4_POPOVER_ENTRY=1` puts a GtkEntry
in that popover and grab_focus after popup:

1. Keyboard-less seat.
2. `xdg popup` configure succeeds.
3. v3 enable + IME Activate.
4. OSK types `hi!` (`GTK_ENTRY_TEXT=hi!`).

That is a foreign field inside a popup. gtk4-demo combobox
hit-target stays unproven (ADR-400). Falkon URL remaining is still
chrome (ADR-398). PathEdit remaining is still chrome (ADR-395).

## Decision

1. **A GTK 4.14 Entry in an xdg_popup types OSK on host without
   wl_keyboard.** Panther still needs the unflashed displayd
   stack (ADR-311+319+328+339+352+376+377+378+381+385+388+390+394).
2. **Do not more combobox Y. Do not more Falkon clicks as the
   next default. Do not more PathEdit clicks.**
3. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- GTK dialogs that present a popover Entry can take IME on host.
  Combo/menu hit-testing is still a later coordinate.
- Rollback: drop `GTK4_POPOVER_ENTRY` and the OSK asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk414_popover_entry_osk_types_hi_bang`. dest-no-lock kept.
Do not flash. Leave Сейчас.
