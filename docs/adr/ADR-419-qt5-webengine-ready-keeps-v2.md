# ADR-419: host Qt 5.15 URL focus after WebEngine loadFinished keeps v2

## Статус

Принято, 2026-09-22. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-418 следующий свободный номер — **419**. Не S33.

## Контекст

Falkon URL click happens after `QWebEngineView` has already
loaded `about:blank`. ADR-418 focused the field at map time, while
Chromium was still starting. Hypothesis: focusing the URL bar
after `loadFinished` is enough to `hideInputPanel` (ADR-398).

Host Qt 5.15, keyboard-less seat,
`QT_LINEEDIT_WEBENGINE_READY=1` (frameless, URL `QLineEdit` 40 px,
`loadFinished` then `setFocus` on the field):

1. v2 enable. Surrounding 19 bytes. Caret in the 40 px bar.
2. One toplevel. No v2 disable in 2 s quiet.

Falkon remaining is not “ready WebEngine then URL focus” on a real
QLineEdit. Sibling about:blank (ADR-418) already keeps v2.
Steal-back still disables (ADR-361). Remaining is Falkon
LocationBar chrome (`weView()->setFocus()`, LocationCompleter,
LineEdit subclass), not compositor v2. Do not more Falkon clicks
as the next default. Do not more PathEdit clicks.

## Decision

1. **Do not treat Falkon URL disable as compositor v2 failing
   URL focus after WebEngine loadFinished.** The field keeps IM.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_WEBENGINE_READY` and the keep-v2
  asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_webengine_ready_keeps_v2`. dest-no-lock kept. Do
not flash. Leave Сейчас.
