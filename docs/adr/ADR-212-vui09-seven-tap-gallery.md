# ADR-212: VUI-09 — 7-tap DevSurface on panther HEAD

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. Operator-approved
7-tap gallery: Система `SaaiOS · сборка 96a503bd72d3` opened
`Работа · Диагностика` after seven silent taps. Назад returned to
Система. Leave Сейчас. No Изменить, no PIN, no Orb, no Me app tap.
`/run/saaios/dev-no-lock` stayed. `compile()` stays `sui 1`. No new
daemon.

## Нумерация

После ADR-211 следующий свободный номер — **212**. Не S33.

## Контекст

The ledger left 7-tap gallery open as a DevSurface-chrome cell, not
a leftover-text slice. ADR-201 forbade 7-tap while locking named
sizes. Operator carte blanche now allows the gesture. The
`/run/saaios/ui-gallery` boot marker is a different path and was
not used. Daylight booth stays deferred.

## Decision

1. **Seven taps on the silent build row.** Threshold remains 7.
2. **Leave with Назад, then Сейчас.** Do not tap Изменить, lock,
   PIN, or apps.
3. **Do not reflash.** Binary stays ADR-198.

## Consequences

- 7-tap DevSurface is proven on HEAD chrome. Rollback: do not 7-tap
  unless the cell requires it. Daylight booth stays open. Still not
  Visual v1 sign-off.

## Verification

Host: ledger cites ADR-212 and 7-tap. Panther: Диагностика header,
docked Назад, back to Система, leave Сейчас; PIN null; marker on.
