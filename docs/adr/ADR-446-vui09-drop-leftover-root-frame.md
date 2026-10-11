# ADR-446: VUI-09 — live draw does not construct `Frame::Root`

## Статус

Принято, 2026-10-11. Every `RootPage` already has a dedicated frame
(ADR-138–141, 240). The leftover `else` still built `Frame::Root` and
painted the diagnostic Surface strip. That path is gone. `draw_root`
stays a test painter (ADR-165). Status invents no action. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-445 следующий свободный номер — **446**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

NOW, apps, Inbox, Search, Spaces, and Система each paint through
`layout_v2`. The catch-all still assembled leftover inspect/intent
cards from empty `ROOT_CONTENT_ACTIONS` and called `draw_root`.
That was a second tree: hardcoded Surface at `y=150` and
placeholder rows.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. The leftover root fallback was not on that
list.

## Decision

1. **Live.** `draw()` is `unreachable!` if a `RootPage` has no
   dedicated frame. No `Frame::Root` variant.
2. **Keep** `draw_root` for host tests. `root_view` still supplies
   tab/content rects. `ROOT_CONTENT_ACTIONS` stays empty (ADR-187).
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas.

## Consequences

- Live chrome never paints the leftover Surface strip. Rollback:
  restore `Frame::Root` and the `else`. Still not Visual v1
  sign-off.

## Verification

Host: `Frame::Root {` is gone from `main.rs`; `draw_root` remains.
Panther: do not flash; leave Сейчас.
