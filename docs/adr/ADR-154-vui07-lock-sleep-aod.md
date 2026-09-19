# ADR-154: VUI-07 lock sleep — AOD clock, wake is not unlock

## Статус

Принято, 2026-09-19. Pseudo-sleep paints Canvas and the live clock,
not a blank black fill and not diagnostic red. Flashed panther
`c3fd2fcf…`: AOD `18:03` without hint; first tap woke to hint +
`Заряд 98%` still locked. PIN keypad, Inbox bodies, and Space detail
are not this slice. Do not save a PIN.

## Нумерация

После ADR-153 следующий свободный номер — **154**. Не S33.

## Контекст

Visual Language 9.6: wake-on-touch must not perform a privileged
action. HIA-38 AOD is time plus an optional mark, not a second lock
with cards. ADR-051 blanks the lock surface to
`SLEEP_INDICATOR_COLOR` and wakes to `LOCK_SCREEN_COLOR` (the S04
red). ADR-134 already replaced the idle lock; wake and deep-idle
still used the old fills. ADR-153 left deep-idle black.

The two-step already exists in `TouchHandler::down`: sleeping
consumes the first tap. It needs a named view, a library paint, and
host proof that ShowLock is not unlock.

## Decision

1. **`lock_sleep_view(time) -> { time }`**. No hint, no battery, no
   Inbox text. Time is the same `current_time_string` as lock idle.
2. **`draw_lock_sleep` paints Canvas and the Display-role clock** at
   the same `time_y` as idle, so the clock does not jump on wake.
   No unlock copy. No fuel-gauge Caption.
3. **`lock_wake_tap(sleeping)`** is `ShowLock` while sleeping, else
   `Continue`. `ShowLock` is not unlock. The existing down-handler
   still consumes that tap and calls `present_lock_pin_entry`.
4. **`check_deep_idle` paints through `present_lock_pin_entry`**
   (sleep branch), not `present_lock_surface(SLEEP_INDICATOR_COLOR)`.
   Minute refresh continues while sleeping.

## Consequences

- AOD is a clock, not a black diagnostic gap and not a widget host.
- Wake remains two-step. Rollback: restore the black fill.

## Verification

Host: sleep view keeps the passed time and has no hint; wake while
sleeping is `ShowLock`; fill is Canvas, not the diagnostic red.
Panther `c3fd2fcf…` pid 6331: AOD clock `18:03` without hint or
battery; first tap `18:04` + hint + `Заряд 98%` still locked; second
tap unlocks. Do not set a PIN. Do not 7-tap.
