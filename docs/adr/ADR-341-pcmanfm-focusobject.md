# ADR-341: PCManFM v2 OSK is silent `focusObject() == null`, not discard

## Статус

Принято, 2026-09-21. APP-04 Qt 5.15 PathEdit/Filter on host qemu.
Not a panther typed field. Not Visual v1 sign-off. PIN stays null.
Do not flash displayd this week.

## Нумерация

После ADR-340 следующий свободный номер — **341**. Не S33.

## Контекст

ADR-340: packed Falkon URL OSK with `QT_LOGGING_RULES` has
`qt.qpa.input.methods` logs and no `discard commit_string`. Qt 6.6
`commit_string` then returns silently when `focusObject()` is null.

Packed PCManFM-Qt 1.4.1 (Qt 5.15.10) is the other chrome `QLineEdit`.
The same rules plus `QT_LOGGING_TO_CONSOLE=1` were already on the
spawn, but `dbus-run-session` + qemu left stderr open after a
plain `kill`, so the test collected 0 bytes. `process_group(0)` and
group `SIGKILL` (same as Falkon `reap_falkon`) close the pipe.

Host qemu, IME `commit_string("hi!")` on the first v2 enable, then
Ctrl+L:

1. stderr contains `qt.qpa.input.methods`
   (`setFocusObject` / `update` / `commit` / `hideInputPanel`).
2. stderr does **not** contain `discard commit_string`.
3. Ctrl+L still disables v2 with no second enable (ADR-332).

Chrome PathEdit/Filter is the same class as LocationBar: compositor
forward is live; insert target is not a focused `QObject`.

## Decision

1. **Do not claim PathEdit or Filter was typed.** Same silent
   `focusObject() == null` as ADR-340, not `m_resetCallback`.
2. **Do not sweep Y.** Filter/PathEdit bands stay where ADR-331/327
   left them.
3. **Do not flash panther.** AUTH-10 stays a later flash-week device
   pass (reboot drops dest-no-lock).

## Consequences

- APP-04 host chrome (Falkon + PCManFM) is protocol-without-paint
  for the same Qt reason. Probe `QLineEdit` with `setFocus` stays
  the only typed Qt 5/6 field.
- Next compositor IME ordering will not paint these bars.
- Rollback: drop process-group reap / console logging env.

## Verification

Host `cargo test -p saai-displayd --test pcmanfm_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
