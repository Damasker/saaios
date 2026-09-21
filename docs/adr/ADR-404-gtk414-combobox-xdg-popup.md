# ADR-404: packed GTK 4.14 ComboBox popup maps configured xdg_popup

## Статус

Принято, 2026-09-21. APP-04 GTK combobox class on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-403 следующий свободный номер — **404**. Не S33.

## Контекст

gtk4-demo `--run=combobox` has no popup until a tap (ADR-399).
Click `200 90` missed (ADR-400). GtkMenuButton popover maps
xdg_popup (ADR-401). Packed `GtkComboBoxText` calls
`gtk_combo_box_popup` after map — no Y sweep, not gtk4-demo:

1. Keyboard-less seat.
2. Toplevel maps, Activated, frame.
3. `xdg popup` with no configure failure.

That is the gtk4-demo combobox widget class. The demo's pixel
hit-target stays unproven. Qt QMenu stays a second toplevel
(ADR-403).

## Decision

1. **ComboBox dropdown is an xdg_popup on host when GTK opens
   it.** gtk4-demo remaining is a missed tap, not missing
   compositor popup configure.
2. **Do not more combobox Y. Do not more Falkon clicks as the
   next default. Do not more PathEdit clicks.**
3. **Do not flash panther this week.** Next displayd flash must
   carry ADR-311+319+328+339+352+376+377+378+381+385+388+390+394.
   AUTH-10 stays a later flash-week device pass.

## Consequences

- Opening gtk4-demo combobox still needs a proven tap. Packed
  ComboBox already maps. Falkon URL remaining is still chrome
  (ADR-398). PathEdit remaining is still chrome (ADR-395).
- Rollback: drop the combobox probe and the popup asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk414_combobox_popup_maps_xdg_popup_without_click`.
dest-no-lock kept. Do not flash. Leave Сейчас.
