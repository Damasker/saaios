# ADR-403: host Qt 5.15 QMenu is a second toplevel, not xdg_popup

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-402 следующий свободный номер — **403**. Не S33.

## Контекст

GTK 4.14 `GtkPopover` maps a configured `xdg_popup` (ADR-401) and
an Entry in that popup types OSK (ADR-402). Qt 5.15 `QCompleter`
maps a second `xdg_toplevel` (ADR-394). `QMenu::popup` after map,
no click:

1. Keyboard-less seat.
2. Two `xdg_toplevel`s. `mapped toplevel without steal`.
3. No `xdg popup`.
4. v2 enable on the field.

Qt Wayland menus are windows, not xdg_popup. PCManFM/Falkon
context menus will not exercise ADR-394 popup configure.

## Decision

1. **Do not treat Qt QMenu as the compositor popup path.** GTK
   popover already did. Next displayd flash still carries
   ADR-394 with the IME stack.
2. **Do not more QMenu variants. Do not more combobox Y. Do not
   more Falkon clicks as the next default. Do not more PathEdit
   clicks.**
3. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Qt chrome dropdowns stay in the no-steal toplevel class.
  Opening gtk4-demo combobox still needs a proven tap.
- Rollback: drop `QT_LINEEDIT_MENU` and the second-toplevel
  asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_qmenu_popup_is_second_toplevel_not_xdg_popup`. dest-no-lock
kept. Do not flash. Leave Сейчас.
