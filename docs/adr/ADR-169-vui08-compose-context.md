# ADR-169: VUI-08 — compose Field Focus during Context

## Статус

Принято, 2026-09-20. Opening intent / Wi-Fi password / PIN setup starts
`MotionToken::Context` (240 ms) on the shared `MotionClock`. The compose
`Field` paints a `StrokeToken::Focus` outline while that clock needs a
frame. Reduced motion is today's instant Field, no extra frames. Lock
unlock PIN is not this slice. Do not send an intent. Do not type a
PSK. Do not type a PIN. Leave Отмена. Orb activity pulse stays the
still-frame inset.

## Нумерация

После ADR-168 следующий свободный номер — **169**. Не S33.

## Контекст

ADR-167/168 consume `MicroFeedback` and `Selection`. `Context` (200–300
ms) still had no shell consumer. Opening «Намерение» is a context
change: the Field is already focus stop 0 (ADR-162), but nothing
showed the 2-unit Focus outline the component contract already names.

## Decision

1. **On overlay open** (`intent_input`, `wifi_password`, `pin_setup`),
   start `MotionClock::one_shot(Context)` unless reduced motion.
2. **Paint Focus** on that overlay's Field while the clock's token is
   still `Context` and `needs_frame()`. Size does not change. A later
   key down may replace the clock with `MicroFeedback` (one clock).
3. **On overlay close** (Отмена / send / save), drop a Context clock.
   Lock PIN Field stays without this outline.
4. **No Orb loop, no new daemon, no invented duration.**

## Consequences

- Entering compose is visible for ~240 ms. Idle NOW does not grow a
  commit loop after the clock ends.
- Rollback: stop starting a Context clock; Field paint stays as today.

## Verification

Host: Context finishes at 240 ms; reduced motion never needs a frame;
Selection/Micro clocks do not show Field Focus. Panther: open intent,
screenshot the Field outline, leave Отмена. Do not send.
