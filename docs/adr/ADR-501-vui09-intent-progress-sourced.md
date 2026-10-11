# ADR-501: VUI-09 — Intent Progress is sourced or indeterminate

## Статус

Принято, 2026-10-11. Object / Intent View paints `Progress` only from
a live running or verifying Task. A numeric `progress` / `percent`
property is determinate. Missing number is Indeterminate. Never N-of-M.
Experimental. PIN stays null. Leave Сейчас. Not Visual v1 sign-off.
No flash.

## Нумерация

После ADR-500 следующий свободный номер — **501**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The concept board shows a progress track on Intent. ADR-465 already
docks public `Progress` on `stacked_row_rect`. The overlay never
emitted it. `intent_plan_progress` is a status phrase behind
Disclosure (ADR-499), not a percent. Step counts must not become a
bar.

## Decision

1. **In-flight only.** `running` / `verifying` Task → `Progress`.
   Waiting, done, empty, and store-down omit the row.
2. **Sourced number.** Live `progress` or `percent` on that Task is
   determinate (clamped 0–100). Otherwise Indeterminate.
3. **Host-lock** `object.progress` as Status `Progress` after history
   so related / step hits stay put. Status invents no action.
4. **Not this slice.** Do not invent a Task progress writer. Do not
   convert leftover keyboard / gallery / lock formulas. Do not flash.

## Consequences

- The track appears only while work is live. Rollback: drop
  `object.progress`. Still not Visual v1 sign-off.

## Verification

Host: running is Indeterminate; `progress=40` is determinate; two-step
plan is not 50%; waiting/empty/offline omit the row; Status hit is
none. Panther: COM13 is live; do not flash; leave Сейчас.
