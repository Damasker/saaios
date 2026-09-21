# ADR-406: packed GTK 4.14 DropDown activate maps xdg_popup

## Статус

Принято, 2026-09-21. APP-04 GTK 4.10+ dropdown on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-405 следующий свободный номер — **406**. Не S33.

## Контекст

GtkComboBox is deprecated for GtkDropDown in 4.10. Packed 4.14
still ships ComboBox (ADR-404/405). GtkDropDown
`gtk_widget_activate` after map, no tap:

1. Keyboard-less seat.
2. Toplevel maps, Activated, frame.
3. `xdg popup` with no configure failure.

That is current GTK chrome. gtk4-demo combobox pixel hit-target
stays unproven. Falkon URL remaining is still chrome (ADR-398).

## Decision

1. **GtkDropDown opens an xdg_popup on host without a tap.** Next
   displayd flash still carries ADR-394 with the IME stack.
2. **Do not more combobox Y. Do not more Falkon clicks as the
   next default. Do not more PathEdit clicks.**
3. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Modern GTK 4.14 dropdowns can map. Search-in-dropdown IME is a
  later probe. PathEdit/Falkon remaining is still Qt chrome.
- Rollback: drop the dropdown probe and the popup asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk414_dropdown_activate_maps_xdg_popup_without_click`.
dest-no-lock kept. Do not flash. Leave Сейчас.
