# ADR-438: VUI-09 — NOW empty `SurfacePattern` keeps loc `now.empty`

## Статус

Принято, 2026-10-10. Apps empty already docks `apps.empty` (ADR-437).
Сейчас still compiled `loc = now.empty` and then looked the leaf up
as type name `SurfacePattern`. Paint now reads that loc. Status
invents no action. Placement stays the Fill leftover behind
`ObjectSummary` so `open_object` hits are unchanged. PIN stays null.
Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-437 следующий свободный номер — **438**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`now.sui` and `now-public.sui` already name `now.empty`. `layout_v2`
dropped rest leftovers on the type name, so two Status patterns
could not share a tree and the public loc was invisible. Apps empty
was the same class of leftover before ADR-437.

The leftover-formula list (ADR-232) is Keyboard keys, gallery page,
and lock idle/wake. NOW empty was not on that list.

## Decision

1. **Id.** A rest `SurfacePattern` with a loc keeps that loc as the
   node id. Missing loc still uses the type name.
2. **Paint.** `now_paint_chrome_from` looks up `now.empty`. Empty
   copy still paints only when there is no live object and no
   sections.
3. **No invented action.** Status invents none. `ObjectSummary` still
   owns `open_object`.
4. **Not this slice.** Do not move the empty Fill under the object
   band. Keyboard keys, gallery page, and lock idle/wake stay
   formulas.

## Consequences

- Public and live NOW empty share `now.empty`. Rollback: look up
  `SurfacePattern` again. Still not Visual v1 sign-off.

## Verification

Host: `now.empty` exists, has no action, and matches `chrome.empty`.
Panther: do not flash; leave Сейчас.
