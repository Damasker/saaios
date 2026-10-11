# ADR-469: VUI-09 — public NOW empty stays Fill leftover

## Статус

Принято, 2026-10-11. ADR-438 kept loc `now.empty` and no action.
Geometry stayed the Fill leftover behind `ObjectSummary` without a
public rect lock. Public subset stays Experimental. Leftover named
chrome stays closed. Keyboard keys, gallery page, and lock idle/wake
stay formulas. PIN stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-468 следующий свободный номер — **469**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Apps empty docks below the header (ADR-437). NOW empty is a rest
`SurfacePattern` in the same stack as header and object, so it
overlays the content pane. Moving that Fill under the object band
would invent a second NOW body and could steal `open_object`.
Docking it on `stacked_row_rect` would invent a list row.

## Decision

1. **Host-lock** public `now.empty` shares the header origin, is
   taller than the object band, and does not match stacked row 0.
   Status invents no action. `540, 335` stays `open_object`.
   `540, 525` stays empty.
2. **Do not move** the Fill under the object band. Do not dock it
   as a stacked row.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Public NOW empty stays leftover Fill. Rollback: drop the
  geometry test. Still not Visual v1 sign-off.

## Verification

Host: `now.empty` origin matches the header; not stacked row 0.
Panther: do not flash; leave Сейчас.
