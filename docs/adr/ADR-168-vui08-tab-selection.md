# ADR-168: VUI-08 — tab Selection hold after release

## Статус

Принято, 2026-09-19. Tab `NavigationItem::pressed` holds through
`MotionToken::Selection` (180 ms) after release, using the same
`MotionClock` as ADR-167. Reduced motion is today's clear-on-up.
Finger leaving the tab strip still clears immediately. Do not 7-tap.
Do not tap Inbox/Spaces rows. Leave by Сейчас. Space detail is not
this slice. Orb activity pulse stays the still-frame inset. Context
transitions are a later VUI-08 slice.

## Нумерация

После ADR-167 следующий свободный номер — **168**. Не S33.

## Контекст

ADR-167 gave the keyboard a 120 ms `MicroFeedback` hold so a short tap
is still visible. The tab strip already paints `ColorRole::Pressed`
while the finger is down (ADR-116) and clears on `up()`, so a 50 ms
tap never shows the Selection window `MotionToken` already names.

## Decision

1. **Same clock, Selection token.** On a new tab down, start
   `MotionClock::one_shot(MotionToken::Selection, reduced_motion)`.
   After up, keep `pressed_tab` until the clock finishes, then clear.
2. **Pressed still follows the finger** while down. Sliding off the
   strip drops the clock. Cancel / AOD-wake drop it.
3. **No haptic on tabs, no Orb loop, no new daemon, no invented
   duration.** Context overlays stay instantaneous until their own
   slice.

## Consequences

- A short tab tap still shows ~180 ms of `Pressed` on the destination
  before it settles to `selected`. Idle screens do not grow a commit
  loop.
- Rollback: restore `pressed_tab.take()` on up.

## Verification

Host: Selection finishes at 180 ms; reduced motion never needs a
frame; `pressed_tab` is kept after release only while the clock needs
a frame. Panther: from Сейчас tap Входящие, screenshot `Pressed` or
the selected tab, leave Сейчас. Do not 7-tap. Do not tap a row.
