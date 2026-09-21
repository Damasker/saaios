# ADR-415: host Qt 5.15 LocationCompleterView ToolTip keeps v2

## Статус

Принято, 2026-09-22. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-414 следующий свободный номер — **415**. Не S33.

## Контекст

Falkon LocationCompleterView is a `QWidget` with
`Qt::ToolTip | FramelessWindowHint | BypassWindowManagerHint`,
`WA_ShowWithoutActivating`, `setFocusProxy(LocationBar)`, then
`createWinId` + `setTransientParent`. Not QCompleter (ADR-394) and
not QMenu (ADR-403). Hypothesis: showing that view is enough to
`hideInputPanel` after a URL click (ADR-398).

Host Qt 5.15 lone QLineEdit, keyboard-less seat,
`QT_LINEEDIT_TOOLTIP_POPUP=1` (URL text, ToolTip widget 80 ms after
map):

1. v2 enable. Surrounding 19 bytes.
2. One toplevel. Log `xdg popup` (not a second toplevel).
3. No v2 disable in 2 s quiet.

Falkon remaining is not LocationCompleterView ToolTip+focusProxy
on a real QLineEdit. InlineCompletion (ADR-414) and URL hints
(ADR-413) already keep v2. Steal-back still disables (ADR-361).
Remaining is Falkon LocationBar/WebEngine chrome after
focus-flash, not compositor v2. Do not more Falkon clicks as the
next default. Do not more PathEdit clicks.

## Decision

1. **Do not treat Falkon URL disable as compositor v2 failing a
   ToolTip completer view.** Lone QLineEdit keeps IM. The view
   maps as `xdg_popup`.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_TOOLTIP_POPUP` and the keep-v2
  asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_tooltip_popup_keeps_v2`. dest-no-lock kept. Do
not flash. Leave Сейчас.
