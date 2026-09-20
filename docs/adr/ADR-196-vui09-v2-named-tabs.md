# ADR-196: VUI-09 — name tabs in the `.sui` v2 grammar

## Статус

Принято, 2026-09-20. HEAD shell stays `e8865301…`. Nested `tab <id>`
belongs only on `BottomNavigation`. `layout_v2()` uses those ids and
`select_root:<id>` and does not call `compile_v1_rollback()`. Empty
`BottomNavigation` invents no v1 hits. `compile()` stays v1. No new
daemon. Leave Сейчас.

## Нумерация

После ADR-195 следующий свободный номер — **196**. Не S33.

## Контекст

ADR-194 borrowed live tab ids from the v1 rollback so public NOW hits
matched `layout_v1_root()`. ADR-193 named that borrow as Visual v2
item 7. An empty `BottomNavigation {}` still invented four v1 hits.
`NavigationItem` is a Rust composite, not a v2 component name.

## Decision

1. **Grammar.** After properties, `BottomNavigation` may list
   `tab <id> { loc = … }`. `id` must be a live surface. Public compile
   rejects privileged ids (`lock`, `gallery`, `diagnostic`). Duplicate
   ids fail. Nested `tab` on any other component fails.
2. **Layout.** Named tabs become equal-width hits. Action is
   `select_root:<id>`. The strip id is `BottomNavigation`. Empty
   navigation is content-only.
3. **Example.** `docs/os/ui/examples/now-public.sui` lists
   now/inbox/spaces/me so public NOW still matches v1 hits.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `layout_v1_root()`.

## Consequences

- v2 layout owns its destinations. Rollback: drop nested `tab` and
  borrow v1 ids again. Next: leftover ActionCard/tab paint, or
  Experimental→Stable after Visual v1 acceptance. Still not Visual v1
  sign-off.

## Verification

Host: nested tabs compile; header `tab` fails; empty navigation has
no 135/2250 hit; public NOW still 135/405/675/945 y=2250. Panther:
Сейчас on HEAD; leave Сейчас.
