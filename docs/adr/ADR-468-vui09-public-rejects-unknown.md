# ADR-468: VUI-09 — public subset rejects unknown names

## Статус

Принято, 2026-10-11. `compile_v2` already rejects `WidgetCard` and
surface `home` as unknown. `compile_v2_public` used the same
parser without a named public lock. `WidgetCard` stays unknown,
not deferred (ADR-463). Public subset stays Experimental.
Leftover named chrome stays closed. Keyboard keys, gallery page,
and lock idle/wake stay formulas. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-467 следующий свободный номер — **468**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Privileged rejects are ADR-462. Deferred rejects are ADR-463.
Unknown names return `None` from `sui_v2_stability`. Promoting
`WidgetCard` or inventing surface `home` would skip that gate.

## Decision

1. **Host-lock** `compile_v2_public` rejects component `WidgetCard`
   and surface `home` as unknown. Stability stays `None`.
2. **Public examples omit those names.** Do not add them to the
   vocabulary.
3. **Not this slice.** Do not promote Experimental → Stable. Do
   not check VUI-09 Acceptance. Do not convert ADR-232 leftovers.
   Do not start thin tuning.

## Consequences

- Unknown names fail the public gate. Rollback: drop the reject
  test. Still not Visual v1 sign-off.

## Verification

Host: public compile rejects `WidgetCard` and `home`. Panther: do
not flash; leave Сейчас.
