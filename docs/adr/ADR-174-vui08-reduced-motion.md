# ADR-174: VUI-08 — reduced motion is immediate

## Статус

Принято, 2026-09-20. Every in-flight `MotionClock` is created through
`motion_clock_for` / `activity_clock_for`. «Меньше движения» drops
clocks on the same tap. Pressed still follows a live finger. No new
daemon. Do not tap PIN / Orb / Inbox rows. Restore Выкл. Leave Сейчас.

## Нумерация

После ADR-173 следующий свободный номер — **174**. Не S33.

## Контекст

Keyboard, tab, compose Focus, and Orb already refuse to *start* a
clock when `reduced_motion` is set. Toggling the setting mid-hold or
mid-pulse left the old clock running until it expired. Gallery
Progress stays a still frame (ADR-166). Haptics stay independent
(ADR-171).

## Decision

1. **`motion_clock_for(token, reduced)`** returns `None` when reduced,
   otherwise a one-shot. `activity_clock_for(wants)` returns a looping
   Context clock only when Orb actually wants a pulse (already false
   under reduced motion).
2. **Toggle drops clocks now.** Finger-down Pressed is not a clock and
   stays. No compositor frame is requested after the drop unless scroll
   is dirty.
3. **Caption** is `Вкл -- без анимации` / `Выкл`. Not Orb-only.

## Consequences

- A running Orb pulse stops on the same tap as the setting. Rollback:
  the previous per-call `if !reduced` branches.

## Verification

Host: `motion_clock_for` is `None` when reduced; drop helper clears
both clocks; caption. Panther: Система → «Меньше движения» Вкл, tab
sample has `frame=0`, restore Выкл, leave Сейчас.
