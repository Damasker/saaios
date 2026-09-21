# ADR-422: host Qt 6.8 URL mouse click above WebEngine keeps v2

## Статус

Принято, 2026-09-22. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-421 следующий свободный номер — **422**. Не S33.

## Контекст

Falkon is packed musl Qt 6.6.3 + QtWebEngine. ADR-421 kept v2 on
host Qt 5.15 `QLineEdit` + `QWebEngineView` after a pointer click.
Hypothesis: Qt 6 WebEngine `hideInputPanel` after that same click.

Host Qt 6.8.2, keyboard-less seat, same
`QT_LINEEDIT_WEBENGINE_CLICK=1` probe compiled with
`Qt6Widgets`/`Qt6WebEngineWidgets` (not a Falkon click):

1. Click enables v2. Surrounding 19 bytes.
2. One toplevel. Cursor surface is not the field.
3. No v2 disable in 2 s quiet.

Falkon remaining is not Qt 6 vs Qt 5 WebEngine on a real
QLineEdit. Steal-back still disables (ADR-361/367). Remaining is
Falkon LocationBar chrome (`weView()->setFocus()`,
LocationCompleter), not compositor v2. Do not more Falkon clicks
as the next default. Do not more PathEdit clicks.

## Decision

1. **Do not treat Falkon URL disable as compositor v2 failing a
   mouse click on QLineEdit above Qt 6 WebEngine.** The field
   keeps IM the same as Qt 5.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `compile_qt6_probe` and the keep-v2 asserts.

## Связанные

- ADR-336 packed musl Qt 6.6.3 QLineEdit types OSK hi!
- ADR-367 packed musl Qt 6.6.3 steal-back disables
- ADR-421 host Qt 5.15 URL click above WebEngine keeps v2

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt6_lineedit_webengine_click_keeps_v2`. dest-no-lock kept. Do
not flash. Leave Сейчас.
