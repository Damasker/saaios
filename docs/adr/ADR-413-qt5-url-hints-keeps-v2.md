# ADR-413: host Qt 5.15 QLineEdit URL input-method hints keep v2

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-412 следующий свободный номер — **413**. Не S33.

## Контекст

Falkon LocationBar sets `ImhNoAutoUppercase | ImhUrlCharactersOnly`
and flash-disables v2 after a URL click (ADR-398). Hypothesis:
those hints are enough to `hideInputPanel`.

Host Qt 5.15 lone QLineEdit, keyboard-less seat,
`QT_LINEEDIT_URL_HINTS=1` (`https://example.com`, those hints):

1. v2 enable. Surrounding 19 bytes.
2. No second toplevel. No v2 disable in 2 s quiet.

Falkon remaining is not URL input-method hints on a real QLineEdit.
PathEdit GIO/selectAll isolations already keep v2 (ADR-408–412).
Steal-back still disables (ADR-361). Remaining is Falkon/PCManFM
chrome after focus-flash, not compositor v2. Do not more Falkon
clicks as the next default. Do not more PathEdit clicks.

## Decision

1. **Do not treat Falkon URL disable as compositor v2 failing
   ImhUrlCharactersOnly.** Lone QLineEdit keeps IM.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_URL_HINTS` and the keep-v2 asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_url_hints_keeps_v2`. dest-no-lock kept. Do not
flash. Leave Сейчас.
