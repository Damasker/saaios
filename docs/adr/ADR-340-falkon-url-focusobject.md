# ADR-340: Falkon URL OSK is silent `focusObject() == null`, not discard

## Статус

Принято, 2026-09-21. APP-04 Qt6 LocationBar on host qemu. Not a
panther typed field. Not Visual v1 sign-off. PIN stays null. Do not
flash displayd this week.

## Нумерация

После ADR-339 следующий свободный номер — **340**. Не S33.

## Контекст

Qt 6.6 `zwp_text_input_v2_commit_string` drops the string in two
places: `m_resetCallback` still pending logs
`discard commit_string: reset not confirmed` on
`qt.qpa.input.methods`; `QGuiApplication::focusObject() == null`
returns with no log.

ADR-339 deferred v2 IME send until after `dispatch_clients` so the
sync callback can finish. Packed `QLineEdit` with `setFocus` still
types (ADR-336). Packed Falkon URL still protocol-without-paint
(ADR-335/338).

OSK spawn now carries `QT_LOGGING_TO_CONSOLE=1`,
`QT_ASSUME_STDERR_HAS_CONSOLE=1`, and
`QT_LOGGING_RULES=qt.qpa.wayland.textinput.debug=true;qt.qpa.input.methods.debug=true`.
Host qemu, same sequential second click `640 20`, immediate OSK:

1. stderr contains `qt.qpa.input.methods` (`QWaylandInputContext`
   `showInputPanel` / `update` / `commit` / `setFocusObject` /
   `hideInputPanel`).
2. stderr does **not** contain `discard commit_string`.
3. Incoming `commit_string` has no other debug in that function.
   After the discard check, Qt returns silently when
   `focusObject()` is null.
4. Main shm at enable still equals the hash 700 ms after OSK.

LocationBar `enable` is therefore not a focused `QObject` at IME
commit time. Compositor defer does not create that focus.

## Decision

1. **Do not claim LocationBar was typed.** The remaining Qt drop is
   silent `focusObject() == null`, not `m_resetCallback`.
2. **Do not sweep Y.** URL band stays `640 20`.
3. **Do not flash panther.** AUTH-10 stays a later flash-week device
   pass (reboot drops dest-no-lock).

## Consequences

- APP-04 Falkon chrome stays enable + OSK forward without a focused
  insert target. Probe `QLineEdit` remains the only typed Qt 6 field.
- Next compositor IME ordering will not paint this bar.
- Rollback: drop `QT_LOGGING_RULES` from the OSK spawn.

## Verification

Host `cargo test -p saai-displayd --test falkon_frame
falkon_url_osk_hi_bang_reaches_v2`. dest-no-lock kept. Do not flash.
Leave Сейчас.
