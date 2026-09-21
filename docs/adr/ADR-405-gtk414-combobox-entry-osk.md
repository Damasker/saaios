# ADR-405: packed GTK 4.14 ComboBox with_entry types OSK hi!

## Статус

Принято, 2026-09-21. APP-04 GTK combobox field on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-404 следующий свободный номер — **405**. Не S33.

## Контекст

ADR-404 maps a configured `xdg_popup` from
`gtk_combo_box_popup`. gtk4-demo `--run=combobox` also has an
editable `gtk_combo_box_text_new_with_entry`.
`GTK4_COMBOBOX_ENTRY=1` uses that constructor, clears the child,
popups, then grab_focus:

1. Keyboard-less seat.
2. `xdg popup` configure succeeds.
3. v3 enable + IME Activate.
4. OSK types `hi!` (`GTK_ENTRY_TEXT=hi!`).

gtk4-demo pixel hit-target stays unproven (ADR-400). ComboBox is
deprecated for GtkDropDown in 4.10; gtk4-demo 4.14 still ships it.

## Decision

1. **A GTK 4.14 ComboBoxText with_entry types OSK on host without
   wl_keyboard.** Panther still needs the unflashed displayd
   stack.
2. **Do not more combobox Y. Do not more Falkon clicks as the
   next default. Do not more PathEdit clicks.**
3. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- gtk4-demo editable combo is the same widget class, still needs
  a proven tap in the demo window. Falkon URL remaining is still
  chrome (ADR-398). PathEdit remaining is still chrome (ADR-395).
- Rollback: drop `GTK4_COMBOBOX_ENTRY` and the OSK asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk414_combobox_entry_osk_types_hi_bang`. dest-no-lock
kept. Do not flash. Leave Сейчас.
