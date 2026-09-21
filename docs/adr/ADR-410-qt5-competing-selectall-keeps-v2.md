# ADR-410: competing pane + mouse selectAll keeps v2

## Статус

Принято, 2026-09-21. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-409 следующий свободный номер — **410**. Не S33.

## Контекст

PathEdit mouse focus defers `selectAll` (MouseFocusReason). A
FolderView-like pane holds focus until the tap. Hypothesis: that
mouse path is enough to `hideInputPanel` (ADR-392).

Host Qt 5.15 `QT_LINEEDIT_COMPETE=1` + `QT_LINEEDIT_SELECTALL=1`,
keyboard-less seat, one click `160 20` (same as ADR-360, not a
PathEdit Y):

1. Pre-click: no v2 enable (pane holds focus).
2. Click enables v2. Surrounding 31 bytes. Caret `10x16`.
3. No v2 disable in 2 s quiet.

PathEdit remaining is not mouse `selectAll` on a real QLineEdit
under a competing pane. Lone selectAll (ADR-408) and completer
reload (ADR-409) already keep v2. Steal-back still disables
(ADR-361). Remaining is PCManFM PathEdit/`onResetFocus`, not
compositor v2. Do not more PathEdit clicks. Do not more Falkon
clicks as the next default.

## Decision

1. **Do not treat PathEdit disable as compositor v2 failing a
   mouse selectAll.** Competing QLineEdit keeps IM after the tap.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop the competing selectAll click asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_competing_selectall_click_keeps_v2`. dest-no-lock kept.
Do not flash. Leave Сейчас.
