# ADR-421: host Qt 5.15 URL mouse click above WebEngine keeps v2

## Статус

Принято, 2026-09-22. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-420 следующий свободный номер — **421**. Не S33.

## Контекст

Falkon URL click is a pointer hit on LocationBar while
`QWebEngineView` already holds focus (ADR-355/358). ADR-419 used
`setFocus(OtherFocusReason)` after `loadFinished`. Hypothesis: a
real mouse click over a live WebEngine is enough to
`hideInputPanel`.

Host Qt 5.15, keyboard-less seat,
`QT_LINEEDIT_WEBENGINE_CLICK=1` (frameless, URL `QLineEdit` 40 px,
WebEngine `about:blank` focused, then `inject-click 160 20` — not
a Falkon click):

1. Click enables v2. Surrounding 19 bytes.
2. One toplevel. Cursor surface is not the field.
3. No v2 disable in 2 s quiet.

Falkon remaining is not mouse+WebEngine on a real QLineEdit.
Inner layout (ADR-420) and ready-engine setFocus (ADR-419) already
keep v2. Steal-back still disables (ADR-361). Remaining is Falkon
LocationBar chrome (`weView()->setFocus()`, LocationCompleter),
not compositor v2. Do not more Falkon clicks as the next default.
Do not more PathEdit clicks.

## Decision

1. **Do not treat Falkon URL disable as compositor v2 failing a
   mouse click on QLineEdit above WebEngine.** The field keeps IM.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_WEBENGINE_CLICK` and the keep-v2
  asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_webengine_click_keeps_v2`. dest-no-lock kept. Do
not flash. Leave Сейчас.
