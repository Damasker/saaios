# ADR-401: packed GTK 4.14 popover maps configured xdg_popup

## Статус

Принято, 2026-09-21. APP-04 compositor popup on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-400 следующий свободный номер — **401**. Не S33.

## Контекст

ADR-394 sends `xdg_popup` configure. gtk4-demo combobox does not
map a popup without a tap (ADR-399) and click `200 90` missed
(ADR-400). A packed GTK 4.14 `GtkMenuButton` probe calls
`gtk_menu_button_popup` after map — no Y sweep, not gtk4-demo:

1. Keyboard-less seat.
2. Toplevel maps, Activated, frame.
3. `xdg popup` with no configure failure.

That is the compositor popup path. Combobox dropdown hit-target
stays unproven.

## Decision

1. **xdg_popup configure is exercised on host.** Next displayd
   flash must still carry ADR-394 with the rest of the IME stack
   (ADR-311+319+328+339+352+376+377+378+381+385+388+390).
2. **Do not treat gtk4-demo combobox as mapped.** Do not more
   combobox Y. Do not more Falkon clicks as the next default.
3. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- GTK chrome that opens a popover without a guessed pixel can
  map. Combo/menu hit-testing is a later coordinate, not a
  compositor gap.
- Rollback: drop the popover probe and the popup asserts.

## Verification

Host `cargo test -p saai-displayd --test gtk4_ime
packed_gtk414_popover_maps_xdg_popup_without_click`. dest-no-lock
kept. Do not flash. Leave Сейчас.
