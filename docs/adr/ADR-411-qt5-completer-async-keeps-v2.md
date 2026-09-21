# ADR-411: host Qt 5.15 QLineEdit completer async model update keeps v2

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-410 следующий свободный номер — **411**. Не S33.

## Контекст

PathEdit `focusInEvent` calls `reloadCompleter(true)`, which starts
a GIO worker then `onJobFinished` updates `QStringListModel`
without `complete()` when triggered by focusIn. Hypothesis: that
model update is enough to `hideInputPanel` (ADR-392).

Host Qt 5.15 lone QLineEdit, keyboard-less seat,
`QT_LINEEDIT_COMPLETER_ASYNC=1` (`/tmp/` text, model filled 80 ms
later, no popup):

1. v2 enable. Surrounding 5 bytes.
2. No second toplevel. No v2 disable in 2 s quiet.

PathEdit remaining is not async completer model update on a real
QLineEdit. selectAll (ADR-408), completer reload (ADR-409), and
mouse selectAll under a competing pane (ADR-410) already keep v2.
Steal-back still disables (ADR-361). Remaining is PCManFM
PathEdit/`onResetFocus`, not compositor v2. Do not more PathEdit
clicks. Do not more Falkon clicks as the next default.

## Decision

1. **Do not treat PathEdit disable as compositor v2 failing
   onJobFinished.** Lone QLineEdit keeps IM. PathEdit still loses
   IM in PCManFM chrome.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_COMPLETER_ASYNC` and the keep-v2
  asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_completer_async_keeps_v2`. dest-no-lock kept. Do
not flash. Leave Сейчас.
