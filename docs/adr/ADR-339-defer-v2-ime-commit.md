# ADR-339: defer v2 IME commit until after client dispatch

## Статус

Принято, 2026-09-21. APP-04 compositor on host. Not a panther typed
field. Not Visual v1 sign-off. PIN stays null. Do not flash displayd
this week.

## Нумерация

После ADR-338 следующий свободный номер — **339**. Не S33.

## Контекст

Qt 6.6 `qwaylandtextinputv2.cpp` discards `commit_string` while
`m_resetCallback` (`wl_display.sync` after `update_state`) is
pending. IME and the Qt app are different Wayland clients. Forwarding
v2 `commit_string` inside the IME `Dispatch` can run before that
client's remaining `update_state` + sync in the same wakeup.

v3 (GTK) stays immediate. v2 commit/delete are queued on the seat and
applied in the event-loop idle, after `dispatch_clients`, before
`flush_clients`.

Host:

1. Packed Qt 5.15 and Qt 6.6.3 `QLineEdit` still type OSK `hi!`.
2. `text_input_v2` still receives `commit_string`.
3. Packed Falkon URL second-click OSK still forwards; main shm
   unchanged (ADR-335/338). LocationBar is still not a typed field.

## Decision

1. **Queue v2 IME commit/delete until after the current dispatch.**
   Do not delay v3.
2. **Do not claim LocationBar was typed.** The settle helps Qt's
   sync order; it does not hold focus.
3. **Do not flash panther this week.** Next displayd flash must carry
   ADR-311 bounds + ADR-319 v2 + ADR-328 delete + this queue.

## Consequences

- APP-04 host Qt probes stay typed. Falkon/PCManFM chrome stay
  protocol-without-paint.
- Rollback: send v2 commit_string inside the IME Dispatch again.

## Verification

Host `cargo test -p saai-displayd --test qt5_ime --test text_input_v2
--test falkon_frame --test gtk4_ime --test pcmanfm_frame`. dest-no-lock
kept. Do not flash. Leave Сейчас.
