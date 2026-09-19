# ADR-166: VUI-07 gallery — SurfacePattern fixtures and goldens

## Статус

Принято, 2026-09-19. Composite gallery fixtures include the five
`SurfacePattern` kinds already used on NOW / apps / Bluetooth.
Host goldens paint their marks without fonts. Labelled fixtures, not
live telemetry. Gallery marker is not required on panther this slice.

## Нумерация

После ADR-165 следующий свободный номер — **166**. Не S33.

## Контекст

VUI-07 rows already existed as gallery fixtures but
`draw_composite_gallery` never painted `SurfacePattern`. Cross-surface
empty/loading/offline/blocked/failed lived only in shell unit tests.

## Decision

1. **`CompositeGalleryFixtures.patterns`** holds empty, loading,
   offline, blocked, failed with the same copy the shell uses.
2. **`draw_composite_gallery`** paints those five marks in a strip
   even without a font, next to the existing UniversalState grid.
3. **No new daemon, no invented workers.**

## Consequences

- Gallery page 2 shows pattern marks as well as state marks.
- Rollback: drop `patterns` and the strip.

## Verification

Host: fixtures cover five kinds; failed/loading paint a mark,
empty does not. No panther gallery tap required.
