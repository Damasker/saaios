# ADR-454: VUI-09 — drop leftover `scrolled_row_rect`

## Статус

Принято, 2026-10-11. Me and diagnostic paint/hits already read
`layout_v2_scrolled` (ADR-219/227/231). The leftover offset formula
had no live caller. Scroll clamp stays `me_max_scroll_offset`.
Status invents no action. PIN stays null. Leave Сейчас. Not Visual
v1 sign-off.

## Нумерация

После ADR-453 следующий свободный номер — **454**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-215 named Me flatten/scroll. ADR-219 switched hits to
`layout_v2_scrolled`. ADR-227 switched paint. The helper stayed as
a second tree: `stacked_row_rect` minus offset, clipped to the
content box.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover scroll formula was not on that
list.

## Decision

1. **Delete** `scrolled_row_rect`.
2. **Keep** `me_dispatch_at` / `me_paint_cards` /
   `diagnostic_paint_cards` on `layout_v2_scrolled`. Keep
   `me_max_scroll_offset` / `stacked_row_rect`.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Me and diagnostic scroll share one compiled tree. Rollback:
  restore `scrolled_row_rect`. Still not Visual v1 sign-off.

## Verification

Host: `scrolled_row_rect` is gone; timezone still hits after a
drag. Panther: do not flash; leave Сейчас.
