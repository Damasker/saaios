# ADR-228: VUI-09 — apps grid paint reads `layout_v2()`

## Статус

Принято, 2026-09-20. Me paint already reads `layout_v2_scrolled`
(ADR-227). Live Приложения tiles now take rects from the same
generated `compile_v2()` / `layout_v2()` tree hit-test already uses
(ADR-220). Empty stays a SurfacePattern, not invented tiles. Overlay
Field/decision, OrbHost, and diagnostic paint stay later slices. PIN
stays null. Do not open Приложения. Leave Сейчас. Not Visual v1
sign-off.

## Нумерация

После ADR-227 следующий свободный номер — **228**. Не S33.

## Контекст

Hits for the apps grid already generate `apps_v2_source` `Button`
tiles with `manage_app:{id}` and dock them like `now_grid_rect`.
Paint still placed cards on `now_grid_rect`, a second formula. Tabs
came from `root_view` even though the live document names
`BottomNavigation`.

## Decision

1. **One tree.** Paint builds `layout_v2` from `apps_v2_source`.
2. **Tile ids.** Each card looks up `manage_app:{id}`. BTreeMap
   order matches the generated document.
3. **Tabs.** Tab rects come from that document's `BottomNavigation`.
4. **No invented Buttons.** Empty copy stays a SurfacePattern leftover
   (source loc `apps.empty`), not a tile.

## Consequences

- Apps paint and hit-test share the generated tree. Overlay paint
  stays procedural until its slice. Rollback: restore `now_grid_rect`
  in the tile builder. Still not Visual v1 sign-off.

## Verification

Host: `manage_app:alpha` / `manage_app:demo` rects match
`now_grid_rect(0/1)`; empty document keeps a SurfacePattern leftover.
Panther: flash; Сейчас; do not open Приложения; leave Сейчас.
