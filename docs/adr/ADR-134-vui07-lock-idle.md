# ADR-134: VUI-07 lock idle — time, not diagnostic red

## Статус

Принято, 2026-09-18. No-PIN lock paints Canvas, the live clock, and
a tap-unlock hint. Flashed panther `a19327cb…`: `23:02` and
`Коснитесь, чтобы разблокировать`, no diagnostic red, no Inbox
content. PIN entry stays `draw_lock_pin_entry` dots. Deep-idle stays
black. Intent input and Space detail are not this slice.

## Нумерация

После ADR-133 следующий свободный номер — **134**. Не S33.

## Контекст

VUI-07 next remaining surface after PIN setup Field. Visual Language
9.6: the lock screen prioritizes time, device state, essential
attention, and a clear unlock affordance; wake-on-touch must not
perform a privileged action; sensitive content respects lock-state
policy. The no-PIN lock is still S04's diagnostic
`LOCK_SCREEN_COLOR` fill (empirical red). Displayd ignores status-bar
commits while locked, so that clock is not the lock's clock. Device
currently has no PIN (tap-to-unlock). Do not paint Inbox bodies,
do not log a PIN, do not change unlock or deep-idle.

## Decision

1. **`lock_idle_view(time) -> { time, hint }`**. Hint is
   `Коснитесь, чтобы разблокировать` — the real no-PIN affordance,
   not invented attention. Time is passed in from
   `current_time_string`, never fabricated.
2. **`draw_lock_idle` paints that view.** Canvas fill, Display-role
   clock, Body hint. No status layer inset (the lock surface is the
   visible frame). `draw_lock_pin_entry` stays dots. Deep-idle still
   `present_lock_surface(SLEEP_INDICATOR_COLOR)`.
3. **Repaint when the minute changes**, not every status tick, and
   never while `sleeping`. Unlock and wake-from-pseudo-sleep stay
   the existing two-step.

## Consequences

- The lock is a clock, not a red diagnostic panel.
- PIN unlock still never paints digits.
- Rollback: restore `present_lock_surface(LOCK_SCREEN_COLOR)` for
  the no-PIN branch.

## Verification

Host: idle view keeps the passed time; hint names tap-unlock; fill
is Canvas, not the diagnostic red. Panther `a19327cb…`: dark Canvas,
clock `23:02` below the punch-hole, hint visible, no Inbox bodies.
PIN was not set for the screenshot.
