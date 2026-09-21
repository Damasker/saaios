# ADR-420: host Qt 5.15 QLineEdit inner QHBoxLayout keeps v2

## Статус

Принято, 2026-09-22. APP-04 Qt chrome on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash
displayd this week.

## Нумерация

После ADR-419 следующий свободный номер — **420**. Не S33.

## Контекст

Falkon `LineEdit::init` installs a `QHBoxLayout` on the
`QLineEdit` itself and hosts ClickFocus side widgets. Hypothesis:
that inner layout is enough to `hideInputPanel` after a URL click
(ADR-398). ADR-417 already showed loose child widgets keep v2.

Host Qt 5.15 lone QLineEdit, keyboard-less seat,
`QT_LINEEDIT_INNER_LAYOUT=1` (URL text, `QHBoxLayout(edit)`,
ClickFocus left/right, text margins):

1. v2 enable. Surrounding 19 bytes.
2. One toplevel. No v2 disable in 2 s quiet.

Falkon remaining is not LineEdit inner layout on a real QLineEdit.
WebEngine sibling/ready (ADR-418/419) already keep v2. Steal-back
still disables (ADR-361). Remaining is Falkon LocationBar chrome
(`weView()->setFocus()`, LocationCompleter), not compositor v2.
Do not more Falkon clicks as the next default. Do not more
PathEdit clicks.

## Decision

1. **Do not treat Falkon URL disable as compositor v2 failing a
   QHBoxLayout on QLineEdit.** The field keeps IM.
2. **Do not flash panther this week.** AUTH-10 stays a later
   flash-week device pass.

## Consequences

- Filter stays the typed PCManFM field. PathEdit/Falkon remaining
  is still app chrome after focus-flash.
- Rollback: drop `QT_LINEEDIT_INNER_LAYOUT` and the keep-v2
  asserts.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime
host_qt5_lineedit_inner_layout_keeps_v2`. dest-no-lock kept. Do
not flash. Leave Сейчас.
