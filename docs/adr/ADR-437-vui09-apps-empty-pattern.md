# ADR-437: VUI-09 — apps empty `SurfacePattern` reads `layout_v2()`

## Статус

Принято, 2026-10-10. Apps tiles already paint from generated
`layout_v2` (ADR-228). An empty grid still dropped `ContextHeader`
(`header_height` was 0) and painted `SurfacePattern` into the whole
content rect. The empty copy now sits in a compiled Status slot
below the header. No invented tiles. PIN stays null. Do not open
Приложения. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-436 следующий свободный номер — **437**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-228 left `apps.empty` as a leftover Fill leaf named
`SurfacePattern`. Without tiles, `v2_content_node` treated the
screen as header-only and omitted the heading slot. Paint then
centered the empty copy on `content`, a second tree.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. Apps empty was not on that list.

## Decision

1. **Header.** A tabbed screen with Status `SurfacePattern` and no
   `ObjectSummary` keeps the stacked header (430 on 2400).
2. **Slot.** That pattern docks below the header as Status. Loc
   `apps.empty` is the node id. No action.
3. **Paint.** `draw_apps_grid` reads that slot. Live tiles unchanged.
4. **Public example.** `apps-public.sui` is header plus Status
   `SurfacePattern` plus the primary tabs. No `Button` tiles. Still
   Experimental.

## Consequences

- Empty Приложения heading and copy share `layout_v2`. Hits invent
  no `manage_app`. Rollback: restore the Fill leftover and
  `draw_surface_pattern(..., content)`. Still not Visual v1 sign-off.

## Verification

Host: public `apps.empty` is `(49, 430, 982, …)` with no action;
`ContextHeader` stays `(49, 0, 982, 430)`. Panther: do not flash;
leave Сейчас.
