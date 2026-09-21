# ADR-423: host Qt 6.8 Falkon LineEdit mouse-ignore keeps v2

## Статус

Принято, 2026-09-22. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-422 следующий свободный номер — **423**. Не S33.

## Контекст

Falkon `LineEdit::focusInEvent` on `MouseFocusReason` with
`selectAllOnClick` sets `m_ignoreMousePress` and `selectAll()`,
then swallows the focusing `mousePressEvent`. ADR-422 kept v2 on
a Qt 6 `QLineEdit` click over WebEngine without that swallow.
Hypothesis: ignoring the focusing press is enough to
`hideInputPanel`.

Host Qt 6.8.2, keyboard-less seat,
`QT_LINEEDIT_WEBENGINE_CLICK=1` plus `QT_LINEEDIT_FALKON_MOUSE=1`
(not a Falkon click):

1. Click enables v2. Surrounding 19 bytes.
2. One toplevel. Cursor surface is not the field.
3. No v2 disable in 2 s quiet.

Falkon remaining is not LineEdit selectAllOnClick + ignore first
press. Steal-back still disables (ADR-361/367). Remaining is
LocationBar chrome (`weView()->setFocus()`, LocationCompleter on
textEdited), not compositor v2. Do not more Falkon clicks as the
next default. Do not more PathEdit clicks.

## Decision

1. **Do not treat Falkon URL disable as compositor v2 failing
   Falkon LineEdit mouse-ignore.** The field keeps IM.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_FALKON_MOUSE` and the keep-v2
  asserts.

## Связанные

- ADR-408 host QLineEdit selectAll keeps v2
- ADR-421/422 URL click above WebEngine keeps v2
- ADR-361/367 steal-back disables v2

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt6_lineedit_falkon_mouse_keeps_v2`. dest-no-lock kept. Do
not flash. Leave Сейчас.
