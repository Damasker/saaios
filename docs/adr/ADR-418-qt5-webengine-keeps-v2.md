# ADR-418: host Qt 5.15 QLineEdit above QWebEngineView keeps v2

## Статус

Принято, 2026-09-22. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-417 следующий свободный номер — **418**. Не S33.

## Контекст

Falkon LocationBar sits above `QWebEngineView`. Hypothesis: the
webview loading `about:blank` is enough to `hideInputPanel` after
a URL click (ADR-398) without an explicit `STEAL_MS` (ADR-361).

Host Qt 5.15, keyboard-less seat, `QT_LINEEDIT_WEBENGINE=1`
(frameless window, URL `QLineEdit` 40 px, `QWebEngineView`
`about:blank`, `setFocus` on the field):

1. v2 enable. Surrounding 19 bytes. Caret in the 40 px bar.
2. One toplevel. No v2 disable in 2 s quiet.

Falkon remaining is not a sibling `QWebEngineView` on a real
QLineEdit. Explicit steal-back still disables (ADR-361). Remaining
is Falkon LocationBar chrome after focus-flash (load/urlChanged /
`weView()->setFocus()`), not compositor v2. Do not more Falkon
clicks as the next default. Do not more PathEdit clicks.

## Decision

1. **Do not treat Falkon URL disable as compositor v2 failing a
   sibling WebEngine view.** Lone LocationBar-class field keeps IM.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_WEBENGINE` and the keep-v2 asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_webengine_keeps_v2`. dest-no-lock kept. Do not
flash. Leave Сейчас.
