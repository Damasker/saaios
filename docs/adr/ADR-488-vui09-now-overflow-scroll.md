# ADR-488: VUI-09 — NOW overflow scrolls above the pinned footer

## Статус

Принято, 2026-10-11. Handoff NOW overflow asks for independent
content scroll with a fixed footer. Weather, attention, current
tasks, and next-action rows honor `layout_v2_scrolled`. Header,
ObjectSummary, Приложения / Новое намерение, and tabs stay at
offset 0. Experimental. PIN stays null. Leave Сейчас. Not Visual
v1 sign-off.

## Нумерация

После ADR-487 следующий свободный номер — **488**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-474/476/478/479 host-locked live NOW rows. Three attention
rows plus three tasks plus next already cross the footer band.
`layout_v2` clipped only against the tab strip, so work painted
behind Приложения.

## Decision

1. **Compiler.** Stacked NOW rows clip above the footer band and
   do not slide under the first stacked slot (ObjectSummary stays
   visible). Footer and tabs ignore `scroll_offset`.
2. **Host-lock** `now_scroll_offset`. Paint and hits for weather /
   attention / tasks / next read `layout_live_v2_scrolled`. Footer
   hits stay `now_view()` at offset 0.
3. **Gesture.** NOW arms the same drag recognizer as Me, only
   inside the stacked content pane. A drag is not a tap.
4. **Not this slice.** Do not change `now-public.sui`. Do not
   scroll Space. Do not convert leftover keyboard / gallery /
   lock formulas.

## Consequences

- Overflow NOW work stays reachable. Footer stays pinned.
  Rollback: offset-0 paint/hit. Still not Visual v1 sign-off.

## Verification

Host: 3+3+next produce `max_offset > 0`; scrolled next is
hittable; weather at 438 is gone after scroll; footer stays.
Panther: do not flash; leave Сейчас.
