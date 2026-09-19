# ADR-170: VUI-08 — Orb ActivityPulse loop while Running

## Статус

Принято, 2026-09-20. A second `MotionClock` loops while `OrbHost`
motion is `ActivityPulse`. The inset hairline is on for one Context
window (240 ms) and off for the next. Reduced motion keeps today's
static mark, no extra frames. Idle NOW does not loop. Do not invent
a running task. Do not send an intent. Do not 7-tap.

## Нумерация

После ADR-169 следующий свободный номер — **170**. Не S33.

## Контекст

ADR-167–169 consume one-shot `MicroFeedback` / `Selection` / `Context`
on the shared interaction clock. Context Light still paints Running as
a still-frame inset (ADR-116). Visual Language §8: running is quiet,
low-frequency, and cancellable; idle screens must not decorate. S04
forbids `wl_surface.frame` without a state reason. ADR-167 deferred
the Orb loop until a quieter duration existed; this slice reuses
Context's 240 ms as one on/off window (480 ms period), not a new
number.

## Decision

1. **`MotionClock::looping`**. Elapsed wraps at `2 × duration`.
   `needs_frame` stays true until the caller drops the clock.
   Reduced motion never needs a frame. `pulse_visible` is the first
   half of the cycle.
2. **`activity_clock` is separate** from the interaction `motion_clock`.
   Compose Field Focus (ADR-169) still reads only `motion_clock`.
   A live one-shot does not cancel the pulse; leaving Running,
   reduced motion, lock, or sleep does.
3. **Paint.** `draw_orb`'s inset hairline follows `pulse_visible`.
   Commits only when the phase flips, not every vsync. The
   `StatusMark::Activity` shape stays every frame so Running is
   understandable without animation. No Orb haptic. No new daemon.
4. **Idle / no in-progress work.** `activity_clock` is `None`. No
   extra compositor frames. Do not map Bluetooth scan or Orb menu
   `Active` onto Running.

## Consequences

- Real `saaios.task` `running` is visible as a slow inset blink.
  Quiet NOW does not grow a commit loop.
- Rollback: drop `activity_clock`; restore the still-frame inset
  whenever `motion() == ActivityPulse`.

## Verification

Host: looping needs a frame past 240 ms; pulse on then off; reduced
and Idle never need a frame; Field Focus ignores a looping clock.
Panther: Idle Orb has no inset blink. If a live running task exists,
screenshot the inset on-phase. Do not create a task. Leave Сейчас.
