# ADR-408: host Qt 5.15 QLineEdit selectAll keeps v2

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-407 следующий свободный номер — **408**. Не S33.

## Контекст

PathEdit click selects the path then disables (ADR-392). Hypothesis:
`QLineEdit::selectAll` after focus is enough to `hideInputPanel`.

Host Qt 5.15 lone QLineEdit, keyboard-less seat,
`QT_LINEEDIT_SELECTALL=1` (path text, `selectAll` on focus):

1. v2 enable. Surrounding 31 bytes (the path).
2. Line caret. Frame clock commits.
3. No v2 disable in 2 s quiet. No OSK sent.

PathEdit remaining is not selectAll on a real QLineEdit. Same for
Falkon URL: remaining is not this. Do not more PathEdit clicks. Do
not more Falkon clicks as the next default.

## Decision

1. **Do not treat PathEdit disable as compositor v2 failing
   selectAll.** Lone QLineEdit keeps IM. PathEdit still loses IM
   in PCManFM chrome (completer reload / `onResetFocus` /
   FolderView).
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash, not selectAll.
- Rollback: drop `QT_LINEEDIT_SELECTALL` and the keep-v2 asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_selectall_keeps_v2`. dest-no-lock kept. Do not
flash. Leave Сейчас.
