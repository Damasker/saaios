# ADR-416: host Qt 5.15 empty QInputMethodEvent keeps v2

## Статус

Принято, 2026-09-22. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-415 следующий свободный номер — **416**. Не S33.

## Контекст

Falkon `LineEdit::clearTextFormat` (called from
`LocationBar::focusInEvent`) sends a `QInputMethodEvent` with an
empty commit string and no attributes. Hypothesis: that event is
enough to `hideInputPanel` after a URL click (ADR-398).

Host Qt 5.15 lone QLineEdit, keyboard-less seat,
`QT_LINEEDIT_IM_FORMAT=1` (URL text, empty `QInputMethodEvent` 80
ms after map):

1. v2 enable. Surrounding 19 bytes.
2. One toplevel. No v2 disable in 2 s quiet.

Falkon remaining is not `clearTextFormat` on a real QLineEdit.
LocationCompleterView ToolTip (ADR-415), InlineCompletion
(ADR-414), and URL hints (ADR-413) already keep v2. Steal-back
still disables (ADR-361). Remaining is Falkon LocationBar/WebEngine
chrome after focus-flash, not compositor v2. Do not more Falkon
clicks as the next default. Do not more PathEdit clicks.

## Decision

1. **Do not treat Falkon URL disable as compositor v2 failing an
   empty QInputMethodEvent.** Lone QLineEdit keeps IM.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_IM_FORMAT` and the keep-v2 asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_im_format_keeps_v2`. dest-no-lock kept. Do not
flash. Leave Сейчас.
