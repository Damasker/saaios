# ADR-450: VUI-09 — drop leftover `content_action_at` table

## Статус

Принято, 2026-10-11. Live touch already ignores leftover
`ROOT_CONTENT_ACTIONS` (ADR-448). The leftover-empty table and
`content_action_at` oracle are gone. `root_view` still supplies
tab/content rects. Status invents no action. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-449 следующий свободный номер — **450**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-187 emptied `ROOT_CONTENT_ACTIONS`. ADR-448 removed the live
`else` that walked it. The leftover table, `ContentActionDefinition`,
and `content_action_at` then existed only as a host leftover-empty
oracle. Live NOW hits ObjectSummary + footer. Live spaces hit
`space_row_at`.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover content-action table was not on
that list.

## Decision

1. **Delete** `content_action_at`, `ContentActionDefinition`,
   generated `ROOT_CONTENT_ACTIONS`, and leftover `RootPage::id`.
2. **Keep** `ROOT_TABS` / `root_view` / `now_footer_action_at` /
   `space_row_at`.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Live chrome has no leftover inspect/intent table. Rollback:
  restore the generated empty slice and `content_action_at`.
  Still not Visual v1 sign-off.

## Verification

Host: `content_action_at` is gone; footer and space hits remain.
Panther: do not flash; leave Сейчас.
