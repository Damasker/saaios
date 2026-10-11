# ADR-466: VUI-09 — public paint primitives are not stacked rows

## Статус

Принято, 2026-10-11. ADR-465 docked row-like public primitives.
`SemanticText`, `Icon`, and `Divider` stay paint primitives inside
composites. They compile through `compile_v2_public` and do not
occupy `stacked_row_rect`. Status invents no action. Public
subset stays Experimental. Leftover named chrome stays closed.
Keyboard keys, gallery page, and lock idle/wake stay formulas.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-465 следующий свободный номер — **466**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Treating `SemanticText` / `Icon` / `Divider` as list rows would
invent a second layout for tokens that already live inside
`DataRow` / `ContextHeader` / cards. Inventing Buttons on them is
not this gate.

## Decision

1. **Host-lock** those three names compile public and do not match
   `stacked_row_rect` index 0. `540, 525` stays empty.
2. **Status invents no action.** Do not dock them as rows.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Paint primitives stay paint. Rollback: drop the not-row test.
  Still not Visual v1 sign-off.

## Verification

Host: three paint primitives miss stacked row 0. Panther: do not
flash; leave Сейчас.
