# ADR-167: VUI-08 — shared MotionClock and compositor frame callbacks

## Статус

Принято, 2026-09-19. One `MotionClock` owns `MotionToken` duration.
The shell requests `wl_surface.frame` only while the clock still
needs a frame. Keyboard `ColorRole::Pressed` holds through
`MicroFeedback` after release. Do not send an intent. Leave Отмена.
Do not type a PIN. Space detail is not this slice. Orb activity
pulse stays the still-frame inset until a later VUI-08 slice.

## Нумерация

После ADR-166 следующий свободный номер — **167**. Не S33.

## Контекст

VUI-07 is closed. `MotionToken` already names 120 / 180 / 240 ms and
zeros them under reduced motion, but nothing in the shell advances a
clock or asks the compositor for another frame. S04 forbade requesting
a frame on every commit: idle UI must not redraw without a state
reason. Keyboard press already paints `Pressed` while the finger is
down (ADR-151); a short tap vanished on up before the micro window.

## Decision

1. **`MotionClock`** in `saai-ui-core` next to `MotionToken`. Tests
   inject elapsed milliseconds. Reduced motion never `needs_frame`.
2. **Frame callbacks** only while `needs_frame()`. The 16 ms event
   loop ticks the same clock as a watchdog if `done` is late. Instant
   measures duration; compositor timestamps are not trusted (touch
   time is already always 0).
3. **Keyboard micro hold:** on a new key down, start
   `MotionToken::MicroFeedback`. After up, keep `Pressed` until the
   clock finishes, then clear. Finger leaving the keys still clears
   immediately (pressed follows the finger). Reduced motion is today's
   clear-on-up. Cancel / AOD-wake drop the clock.
4. **No Orb loop, no new daemon, no invented duration token.**

## Consequences

- A 50 ms tap still shows ~120 ms of `Pressed`. Idle screens do not
  grow a commit loop.
- Rollback: drop `MotionClock` and restore `pressed_key.take()` on up.

## Verification

Host: clock finishes at 120 ms; reduced motion never needs a frame;
pressed is kept after release only while the clock needs a frame.
Panther: open intent, tap one letter, screenshot `Pressed` or the
letter, leave Отмена. Do not send.
