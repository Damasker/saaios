# ADR-412: host Qt 5.15 QLineEdit BlockingQueuedConnection completer update keeps v2

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-411 следующий свободный номер — **412**. Не S33.

## Контекст

PathEdit GIO worker emits `finished` with
`Qt::BlockingQueuedConnection` into `onJobFinished`, which updates
`QStringListModel` without `complete()` on focusIn. Hypothesis:
that blocking hop is enough to `hideInputPanel` (ADR-392). ADR-411
already showed a `QTimer` model update keeps v2.

Host Qt 5.15 lone QLineEdit, keyboard-less seat,
`QT_LINEEDIT_COMPLETER_BLOCK=1` (worker `invokeMethod` +
`BlockingQueuedConnection` 80 ms after map):

1. v2 enable. Surrounding 5 bytes.
2. No second toplevel. No v2 disable in 2 s quiet.

PathEdit remaining is not GIO BlockingQueuedConnection on a real
QLineEdit. selectAll (ADR-408), completer reload (ADR-409), mouse
selectAll (ADR-410), and async model update (ADR-411) already keep
v2. Steal-back still disables (ADR-361). Remaining is PCManFM
PathEdit chrome / FolderView steal, not compositor v2. Do not more
PathEdit clicks. Do not more Falkon clicks as the next default.

## Decision

1. **Do not treat PathEdit disable as compositor v2 failing a
   BlockingQueuedConnection job.** Lone QLineEdit keeps IM.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_COMPLETER_BLOCK` and the keep-v2
  asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_completer_block_keeps_v2`. dest-no-lock kept. Do
not flash. Leave Сейчас.
