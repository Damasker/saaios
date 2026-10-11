# ADR-448: VUI-09 — drop leftover `content_action_at` live dispatch

## Статус

Принято, 2026-10-11. Every `RootPage` already has a dedicated tap
handler (ADR-138–141, 240). The leftover `else` still walked empty
`ROOT_CONTENT_ACTIONS` and called `invoke_content_action`. That
path is gone. `content_action_at` stays a host leftover-empty
oracle (ADR-187). Status invents no action. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-447 следующий свободный номер — **448**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-187 emptied `ROOT_CONTENT_ACTIONS`. ADR-446 removed the live
`Frame::Root` paint path. Touch still had a catch-all that hit
`content_action_at` after Inbox, Search, Spaces, NOW, apps, and
Система already handled their own rows. That leftover table never
found a card.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover content-action dispatch was not
on that list.

## Decision

1. **Delete** the live `content_action_at` / `invoke_content_action`
   `else`. Every `RootPage` keeps its dedicated handler.
2. **Keep** `content_action_at` and empty `ROOT_CONTENT_ACTIONS`
   for host leftover-empty tests. Live space taps stay
   `invoke_select_space`. Live compose stays `begin_compose_context`.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Live chrome never walks the leftover inspect/intent table.
  Rollback: restore the `else` and `invoke_content_action`.
  Still not Visual v1 sign-off.

## Verification

Host: `invoke_content_action` is gone; leftover table stays empty.
Panther: do not flash; leave Сейчас.
