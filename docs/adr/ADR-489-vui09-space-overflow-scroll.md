# ADR-489: VUI-09 — Space overflow scrolls above pinned Назад

## Статус

Принято, 2026-10-11. Handoff Space overflow asks for independent
member scroll with a fixed identity header and docked Назад.
Focus and `spaces.member.{i}` honor `layout_v2_scrolled`. Header,
Назад, and tabs stay at offset 0. Experimental. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-488 следующий свободный номер — **489**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-480/482 host-locked Space focus and members. Twelve members
already cross the docked back card. The compiler already clipped
stacked rows against trailing Назад, but the shell never passed
`scroll_offset`, so overflow members stayed unreachable. Members
must not slide under ContextHeader (space identity stays visible).

## Decision

1. **Compiler.** When a trailing control exists, stacked rows do
   not slide above the first stacked slot. Trailing Назад docks
   above the tab strip and ignores `scroll_offset`.
2. **Host-lock** `space_scroll_offset`. Paint and hits for focus /
   members read `layout_live_v2_scrolled`. Назад hits stay the
   offset-0 tree. Offset resets when detail opens or closes.
3. **Gesture.** Space detail arms the same drag recognizer as Me
   and NOW, only inside the stacked content pane. A drag is not a
   tap.
4. **Not this slice.** Do not scroll the Space list. Do not invent
   workers/queue. Do not convert leftover keyboard / gallery /
   lock formulas.

## Consequences

- Overflow Space members stay reachable. Identity and Назад stay
  pinned. Rollback: offset-0 paint/hit. Still not Visual v1
  sign-off.

## Verification

Host: 12 members produce `max_offset > 0`; last member is missing
at offset 0 and hittable after scroll; header and Назад rects
stay put. Panther: do not flash; leave Сейчас.
