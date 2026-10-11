# ADR-492: VUI-09 — Inbox overflow scrolls above the tab strip

## Статус

Принято, 2026-10-11. Inbox EventRows honor `layout_v2_scrolled`.
Header and tabs stay at offset 0. Experimental. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-491 следующий свободный номер — **492**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Inbox EventRows were stacked but not in `v2_me_scroll_clip`, so
overflow painted behind tabs and the shell never passed
`scroll_offset`.

## Decision

1. **Compiler.** EventRow joins the Me-style tab clip.
2. **Host-lock** `inbox_scroll_offset`. Paint and hits read
   `layout_live_v2_scrolled`. Offset resets when leaving Inbox.
3. **Gesture.** Inbox arms the same drag recognizer as Search.
4. **Not this slice.** Do not scroll the Space list. Do not convert
   leftover keyboard / gallery / lock formulas. Do not flash.

## Consequences

- Overflow Inbox events stay reachable. Rollback: offset-0 paint/hit
  and EventRow outside the clip set. Still not Visual v1 sign-off.

## Verification

Host: 12 waiting tasks produce `max_offset > 0`; last row is missing
at offset 0 and hittable after scroll; header stays put. Panther: do
not flash; leave Сейчас.
