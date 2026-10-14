# ADR-665: empty Sistema v2 keeps a header dock

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
is `81e7d8f9…` (ADR-664). dest-no-lock stays absent. That
binary SIGABRT'd on the first empty Sistema paint
(`missing ContextHeader`). Unlock stays the proven path
(ADR-564). Experimental. PIN stays null. Leave Сейчас. Not
Visual v1 sign-off. Not VUI-09 Acceptance. Wrap Acceptance
stays unchecked.

## Нумерация

После ADR-664 следующий свободный номер — **665**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-664 paints Sistema chrome with no rows so `me_facts()`
can wait a turn. `layout_v2` only emits `ContextHeader` when
a stacked row, grid, or SurfacePattern is present. Empty
`me_v2_source` therefore has no header slot.
`v2_named_rect(..., "ContextHeader")` panicked (`panic =
abort` → SIGABRT). Host test
`empty_me_v2_has_header_and_content_child` reproduced it.

## Решение

1. When `me_v2_source` has no rows, emit one inert
   `SettingRow` loc `me.pending` so the header docks.
2. `me_paint_cards` still paints nothing for an empty row
   list. Host test: empty layout has `ContextHeader`.
3. Do not flash this ADR. dest-no-lock stays off.

## Откат

Revert the empty-row `me.pending` block. Live `81e7d8f9…`
still aborts on Sistema.

## Ссылки

- ADR-664, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
