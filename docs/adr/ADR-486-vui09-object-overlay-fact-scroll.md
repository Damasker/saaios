# ADR-486: VUI-09 — Object / Intent overlay fact scroll

## Статус

Принято, 2026-10-11. Handoff Object / Intent boards ask for
overflow scroll of related peers and plan steps without moving
the footer or back. Facts and permission read
`layout_live_v2_scrolled`. `object.summary` and trailing Buttons
stay at offset 0. Experimental. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-485 следующий свободный номер — **486**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-472/473 host-locked live `object.related.{i}` and
`object.step.{i}` DataRows. Twelve plan steps or four tall peers
overflow the panel. Scrolling the whole overlay would drag
Confirm / Hide / back off-screen.

## Decision

1. **Host-lock** Object View fact scroll. `object_overlay_paint`
   stays the offset-0 wrapper. `object_overlay_paint_scrolled`
   lays facts/permission on `layout_live_v2_scrolled` and pins
   summary + Buttons on a dock tree with no facts.
2. **Compiler.** `v2_decision_node` keeps fact pitch at the
   stacked row height, clips rows that would cover the summary
   or footer, and honors `scroll_offset`. Fill no longer
   compresses overflow facts onto the panel.
3. **Hits.** `object_view_hit` stays offset 0.
   `object_view_hit_scrolled` keeps Action hits on the dock tree
   and reads related / step / quick through
   `live_v2_hit_scrolled`.
4. **Gesture.** Object View arms the same drag recognizer as Me.
   Offset lives in `object_scroll_offset` and resets when the
   viewed entity changes. A drag is not a tap.
5. **Not this slice.** Do not change `object-public.sui`. Do not
   scroll Space / NOW. Do not convert leftover keyboard /
   gallery / lock formulas.

## Consequences

- Long Object / Intent fact lists stay reachable. Footer stays
  pinned. Rollback: offset-0 paint/hit. Still not Visual v1
  sign-off.

## Verification

Host: twelve plan steps produce `max_offset > 0`; scrolled last
step is hittable; buttons stay at the docked footer rect.
Panther: do not flash; leave Сейчас.
