# ADR-567: elevation token in logical units

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-566 следующий свободный номер — **567**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-02 asked for spacing, radius, stroke, touch-target, safe-inset,
and elevation tokens. The first five already exist (`SpacingToken`,
`RadiusToken`, `StrokeToken`, `MIN_TOUCH_TARGET`, `SafeInsets`).
`SurfaceLevel` named Canvas / Surface / Raised / Overlay as fills,
not a logical-unit lift. Font compare stays a device task. Long
label wrap stays open. ADR-213 thin tuning is not this slice.

## Решение

1. `ElevationToken` is Flat / Rest / Raised / Overlay at 0 / 4 / 8 /
   12 logical units (same 4-unit rhythm as spacing).
2. `SurfaceLevel::elevation` and `ElevationToken::level` round-trip.
   Raised and Overlay still share the Elevated fill.
3. Do not invent a drop shadow. Do not paint. Do not migrate screens.

## Откат

Remove `ElevationToken` and `SurfaceLevel::elevation`.

## Ссылки

- ADR-101, ADR-566
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
