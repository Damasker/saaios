# ADR-456: VUI-09 — leftover named chrome closed

## Статус

Принято, 2026-10-11. Unused leftover trees, formulas, and dispatch
are gone (ADR-440–455). The leftover clip predicate
`stacked_row_fits_above` had no live caller after ADR-455. Overflow
Back stays `layout_v2` / `stacked_control_rect`. Keyboard keys,
gallery page, and lock idle/wake stay formulas (ADR-232). Public
subset stays Experimental. Acceptance three boxes stay unchecked.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-455 следующий свободный номер — **456**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-440–455 dropped leftover unused chrome that was not on the
ADR-232 leftover-formula list. After ADR-455 the remaining unused
helper was `stacked_row_fits_above`: a clip predicate used only by
overflow tests. Inventing another named-chrome conversion, or
converting Keyboard keys / gallery page / lock idle/wake, would
break IME, 7-tap gallery, and the lock contract.

The host release gate is still Experimental, leftover formulas, and
unsigned Visual v1. Thin tuning stays last (ADR-213).

## Decision

1. **Delete** `stacked_row_fits_above`. Overflow tests compare
   `stacked_row_rect` / compiled slots to `stacked_control_rect`
   directly.
2. **Record** leftover named chrome of unused trees / formulas /
   dispatch as closed. Keep `stacked_row_rect` /
   `stacked_control_rect`.
3. **Not this slice.** Keyboard keys, gallery page, and lock
   idle/wake stay formulas. Do not promote Experimental → Stable.
   Do not check VUI-09 Acceptance. Do not start thin tuning.

## Consequences

- Host leftover cleanup stops inventing formula conversions.
  Remaining VUI-09 work is the unsigned release gate. Rollback:
  restore `stacked_row_fits_above`. Still not Visual v1 sign-off.

## Verification

Host: `stacked_row_fits_above` is gone; overflow Back still docks
on-screen; lock idle/wake helpers stay. Panther: do not flash;
leave Сейчас.
