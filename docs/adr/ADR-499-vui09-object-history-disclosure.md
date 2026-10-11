# ADR-499: VUI-09 — Object View history stays behind Disclosure

## Статус

Принято, 2026-10-11. Activity and live Action execution sit behind
`object.history`. Closed until the owner opens it. No history means
no row. Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off. No flash.

## Нумерация

После ADR-498 следующий свободный номер — **499**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The concept board puts activity/history on disclosure, not as always
-visible facts. ADR-490 already hides workers without an Action.
`object_view_details` still painted `AgentSummary` and activity on
the main stack. A closed Object View must not dump execution lines.

## Decision

1. **History lines** are `agent` + `activity`. Observation, blocker,
   and consequence stay on the main stack.
2. **Host-lock** `object.history` as a Button `Disclosure`. Closed
   shows only that row. Open adds `object.history.{i}` Status
   DataRows. Offset resets when the viewed entity changes.
3. **Compiler.** `Disclosure` with `a11y = Button` uses `loc` as the
   hit. Status Disclosure stays inert (ADR-465).
4. **Not this slice.** Do not invent empty history. Do not convert
   leftover keyboard / gallery / lock formulas. Do not flash.

## Consequences

- Execution stays behind a tap. Rollback: fold lines back into
  details. Still not Visual v1 sign-off.

## Verification

Host: running Action produces history lines; closed tree has
`object.history` only; tap is `ObjectViewHit::History`; no Action
omits the row. Panther: COM13 is live; do not flash; leave Сейчас.
