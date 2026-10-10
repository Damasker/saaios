# ADR-435: VUI-09 — list / apps `ContextHeader` paint reads `layout_v2()`

## Статус

Принято, 2026-10-10. Overlay and compose headings already paint from
the compiled slot (ADR-433/434). Inbox, Spaces, Search, Me, apps,
Wi-Fi, Bluetooth, trusted, and diagnostic still used
`paint_context_header(content)`. Those headings now sit in the same
generated `ContextHeader` as the rows. Inset matches NOW (150/2400).
Status invents no action. PIN stays null. Do not tap list rows,
Сопряжь, or Отозвать. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-434 следующий свободный номер — **435**. Не S33.

## Контекст

List / apps paint already shared `layout_v2` for cards (ADR-226–228,
231). The title still walked the clip rect. NOW already paints
inside `chrome.header` (ADR-225).

## Decision

1. **Slot.** Each list and the apps grid look up `ContextHeader` on
   the tree they already build. Height stays the first stacked top
   (430 on 2400). No action.
2. **Paint.** `draw_context_row_list` and `draw_apps_grid` use
   `paint_overlay_context_header`. Margin is the compiled slot
   (`width/22`), not `width/20`.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas (ADR-232).

## Consequences

- Named chrome headings share `layout_v2` with NOW and overlays.
  Rollback: restore `paint_context_header(content)`. Still not
  Visual v1 sign-off.

## Verification

Host: public Inbox `ContextHeader` is `(49, 0, …, 430)` with no
action. Panther: do not flash; leave Сейчас.
