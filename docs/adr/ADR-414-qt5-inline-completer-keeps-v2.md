# ADR-414: host Qt 5.15 QLineEdit inline completer keeps v2

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-413 следующий свободный номер — **414**. Не S33.

## Контекст

Falkon LocationBar installs a `QCompleter` with
`InlineCompletion` for domain complete, then calls `complete()`
when the model updates. Hypothesis: that inline complete is
enough to `hideInputPanel` (ADR-398).

Host Qt 5.15 lone QLineEdit, keyboard-less seat,
`QT_LINEEDIT_INLINE=1` (URL text, inline completer, `complete()`
at 80 ms):

1. v2 enable. Surrounding 19 bytes.
2. One toplevel. No v2 disable in 2 s quiet.

Falkon remaining is not InlineCompletion on a real QLineEdit.
URL hints already keep v2 (ADR-413). Steal-back still disables
(ADR-361). Remaining is Falkon LocationBar/WebEngine chrome after
focus-flash, not compositor v2. Do not more Falkon clicks as the
next default. Do not more PathEdit clicks.

## Decision

1. **Do not treat Falkon URL disable as compositor v2 failing
   InlineCompletion.** Lone QLineEdit keeps IM.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_INLINE` and the keep-v2 asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_inline_completer_keeps_v2`. dest-no-lock kept.
Do not flash. Leave Сейчас.
