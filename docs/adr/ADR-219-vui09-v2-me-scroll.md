# ADR-219: VUI-09 — Me scroll hits come from `layout_v2_scrolled()`

## Статус

Принято, 2026-09-20. Linear `layout_v2` cannot place a stacked row
under the header after a drag. Stacked rows now overlay through
`Node::stack` (spacer + row). Live `me_action_at` compiles the
flatten list and hit-tests `layout_v2_scrolled`. Paint still uses
`scrolled_row_rect`. PIN stays null. Leave Сейчас. Not Visual v1
sign-off.

## Нумерация

После ADR-218 следующий свободный номер — **219**. Не S33.

## Контекст

ADR-218 left Me on `scrolled_row_rect` because a vertical linear
cannot move a row into the heading band. The goal forbids keeping
that second formula without changing the layout model.

## Decision

1. **Overlay.** `Node::stack` shares bounds; each stacked slot is a
   vertical linear of a y-spacer then the row.
2. **Scroll.** `layout_v2_scrolled(screen, w, h, offset)` subtracts
   the offset from Me stacked tops and clips like
   `scrolled_row_rect`. Lists keep offset 0.
3. **Hits.** Shell generates `SettingRow`s from `flatten_me_rows`
   and reads `layout_v2_scrolled`. Quiet rows invent no action.

## Consequences

- Me drag hits the compiler. Apps grid and overlays stay next.
  Rollback: restore linear gaps and `scrolled_row_rect` hits.
  Still not Visual v1 sign-off.

## Verification

Host: rest `cycle_timezone` matches flatten index; offset 220 moves
the same loc. Panther: flash; Сейчас; do not open Система or PIN.
