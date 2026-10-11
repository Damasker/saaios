# ADR-491: VUI-09 — Search overflow scrolls above the tab strip

## Статус

Принято, 2026-10-11. Search results honor `layout_v2_scrolled`.
Header and tabs stay at offset 0. Experimental. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-490 следующий свободный номер — **491**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-484 host-locked live `search.item.{i}`. Twelve results already
cross the tab strip. The compiler already clips Me-style stacked
rows against tabs, but the shell never passed `scroll_offset`.

## Decision

1. **Host-lock** `search_scroll_offset`. Paint and hits read
   `layout_live_v2_scrolled`. Offset resets when leaving Search.
2. **Gesture.** Search arms the same drag recognizer as Me / NOW,
   only inside the stacked content pane. A drag is not a tap.
3. **Not this slice.** Do not scroll Inbox. Do not convert leftover
   keyboard / gallery / lock formulas. Do not flash.

## Consequences

- Overflow Search results stay reachable. Rollback: offset-0
  paint/hit. Still not Visual v1 sign-off.

## Verification

Host: 12 results produce `max_offset > 0`; last item is missing at
offset 0 and hittable after scroll; header stays put. Panther: do
not flash; leave Сейчас.
