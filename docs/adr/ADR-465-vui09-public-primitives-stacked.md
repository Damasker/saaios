# ADR-465: VUI-09 — public primitives dock on `stacked_row_rect`

## Статус

Принято, 2026-10-11. `Metric`, `Disclosure`, `StatusIndicator`, and
`Progress` are Experimental public primitives. `layout_v2` dropped
them as leftover Fill leaves. They now occupy stacked slots.
Status invents no action. `SemanticText` / `Icon` / `Divider` stay
paint primitives, not rows. Public subset stays Experimental.
Leftover named chrome stays closed. Keyboard keys, gallery page,
and lock idle/wake stay formulas. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-464 следующий свободный номер — **465**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-464 docked public summaries. The remaining row-like public
primitives still compiled, then lost stacked geometry. Inventing
Buttons or treating `SemanticText` as a row would break composite
paint.

## Decision

1. **Dock** `Metric` / `Disclosure` / `StatusIndicator` /
   `Progress` on `stacked_row_rect` through `layout_v2`.
2. **Status invents no action.** Button a11y still invents none.
   Do not dock `SemanticText` / `Icon` / `Divider` as rows.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Public row primitives share list geometry. Rollback: drop them
  from the stacked match. Still not Visual v1 sign-off.

## Verification

Host: four Status primitives match stacked rows; tabs stay.
Panther: do not flash; leave Сейчас.
