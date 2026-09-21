# ADR-409: host Qt 5.15 QLineEdit completer reload keeps v2

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-408 следующий свободный номер — **409**. Не S33.

## Контекст

PathEdit `focusInEvent` reloads `QCompleter` and does not
`complete()`. Hypothesis: that reload is enough to
`hideInputPanel` (ADR-392).

Host Qt 5.15 lone QLineEdit, keyboard-less seat,
`QT_LINEEDIT_COMPLETER_RELOAD=1` (setCompleter on focus, no
popup):

1. v2 enable. Surrounding 0.
2. No v2 disable in 2 s quiet.

PathEdit remaining is not completer reload on a real QLineEdit.
selectAll already keeps v2 (ADR-408). FolderView steal already
disables (ADR-361). Remaining is PCManFM PathEdit subclass /
`onResetFocus`, not compositor v2. Do not more PathEdit clicks.
Do not more Falkon clicks as the next default.

## Decision

1. **Do not treat PathEdit disable as compositor v2 failing
   completer reload.** Lone QLineEdit keeps IM. PathEdit still
   loses IM in PCManFM chrome.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_COMPLETER_RELOAD` and the keep-v2
  asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_completer_reload_keeps_v2`. dest-no-lock kept.
Do not flash. Leave Сейчас.
