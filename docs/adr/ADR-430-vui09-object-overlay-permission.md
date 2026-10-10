# ADR-430: VUI-09 — Object View permission and decision facts read `layout_v2()`

## Статус

Принято, 2026-10-10. Related/detail `DataRow`s already dock
(ADR-429). DecisionOverlay fact lines and OAM permission still
walked a y-cursor. Decision facts are now Status `DataRow`s
(`object.decision.{i}`). Permission is a Status `SurfacePattern`
at `object.permission`. Neither invents an action. Privileged
`DecisionOverlay` stays off `compile_v2_public()`. PIN stays null.
Do not open Object View this slice. Leave Сейчас. Not Visual v1
sign-off.

## Нумерация

После ADR-429 следующий свободный номер — **430**. Не S33.

## Контекст

ADR-429 left two cursor paints: confirmation fact lines and the
blocked OAM band. Public `object-public.sui` could not name a
permission pattern. Flattening confirmation into `details` would
undo ADR-157; the lines stay distinct locs.

## Decision

1. **Decision facts.** Live overlay emits `object.decision.{i}`
   Status `DataRow`s from `DecisionOverlay::fact_lines`. Same
   Body-height slots as related/details. No action.
2. **Permission.** When OAM has a blocked line,
   `SurfacePattern` docks at `object.permission` after the last
   fact. Status invents no action. Paint reads that rect.
3. **Public example.** `object-public.sui` names Status
   `SurfacePattern` at `object.permission`. Still Experimental.
   `compile_v2_public()` still rejects `DecisionOverlay`.
4. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas (ADR-232).

## Consequences

- Remaining Object View chrome except leftover formulas shares
  `layout_v2`. Consent overlays without `ObjectSummary` are
  unchanged. Rollback: walk decision/permission with the y-cursor
  again. Still not Visual v1 sign-off.

## Verification

Host: public `object.permission` sits below `object.related` with
no action; two-action buttons still hit `y=2250`. Panther: do not
flash; leave Сейчас.
