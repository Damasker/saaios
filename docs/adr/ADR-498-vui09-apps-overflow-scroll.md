# ADR-498: VUI-09 — apps grid overflow scrolls above the tab strip

## Статус

Принято, 2026-10-11. Overflow `manage_app:` tiles honor
`layout_v2_scrolled`. Header and tabs stay at offset 0. Clipped
tiles omit paint instead of inventing a rect. Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. No flash.

## Нумерация

После ADR-497 следующий свободный номер — **498**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-220/228 host-locked the apps grid. Eighteen tiles already cross
the tab strip. `layout_v2_scrolled` moved stacked rows, but grid
cells ignored `scroll_offset`, so overflow apps stayed unreachable.
`list_paint_cards` also panicked when a clipped loc was missing.

## Decision

1. **Compiler.** A tabbed apps grid honors `scroll_offset`. Tiles
   clip under the first grid row and above the tab strip. Header
   and tabs stay at offset 0.
2. **Host-lock** `apps_scroll_offset`. Paint and `manage_app:` hits
   read `layout_live_v2_scrolled`. Offset resets when the grid
   opens or closes.
3. **Paint.** Missing locs skip the card (`apps_paint_cards` and
   `list_paint_cards`). Do not invent a rect for a clipped tile.
4. **Not this slice.** Do not convert leftover keyboard / gallery /
   lock formulas. Do not flash. Phone was not on adb.

## Consequences

- Overflow apps stay reachable. Rollback: offset-0 paint/hit.
  Still not Visual v1 sign-off.

## Verification

Host: 18 tiles produce `max_offset > 0`; last tile is missing at
offset 0 and hittable after scroll; header and tabs stay put.
Panther: do not flash; leave Сейчас.
