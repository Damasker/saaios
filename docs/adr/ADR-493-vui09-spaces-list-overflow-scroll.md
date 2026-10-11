# ADR-493: VUI-09 — Space list overflow scrolls above the tab strip

## Статус

Принято, 2026-10-11. Space switcher `SpaceRow`s honor
`layout_v2_scrolled`. Header and tabs stay at offset 0.
Experimental. PIN stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-492 следующий свободный номер — **493**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-489 scrolled Space *detail*. The switcher still painted overflow
behind tabs because SpaceRow was outside `v2_me_scroll_clip` and the
shell never passed `scroll_offset`.

## Decision

1. **Compiler.** SpaceRow joins the Me-style tab clip.
2. **Host-lock** `spaces_list_scroll_offset`. Paint and hits read
   `layout_live_v2_scrolled`. Offset resets when leaving Spaces.
3. **Gesture.** The switcher arms the same drag recognizer as Search,
   only while detail is closed.
4. **Not this slice.** Do not convert leftover keyboard / gallery /
   lock formulas. Do not flash.

## Consequences

- Overflow spaces stay reachable. Rollback: offset-0 paint/hit and
  SpaceRow outside the clip set. Still not Visual v1 sign-off.

## Verification

Host: 12 spaces produce `max_offset > 0`; last row is missing at
offset 0 and hittable after scroll; header stays put. Panther: do
not flash; leave Сейчас.
