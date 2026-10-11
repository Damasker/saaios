# ADR-500: VUI-09 — Intent Confirm only when AskUser exists

## Статус

Принято, 2026-10-11. Confirm/Decline sit on Intent (and Action) only
when a related Task is `waiting_confirmation`. No AskUser means no
Confirm. Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off. No flash.

## Нумерация

После ADR-499 следующий свободный номер — **500**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The concept board paints Confirm on every Intent fixture. ADR-473
already said Confirm stays AskUser. `decision_overlay_for` already
attached a DecisionOverlay when the primary Task was waiting, but
Intent actions stayed «Открыть задачу». Paint then relabeled those
buttons as Confirm while the tap still opened the Task. Offline
Intent already hid actions (ADR-483).

## Decision

1. **Same gate as the overlay.** `waiting_confirmation_task` is the
   AskUser source for both DecisionOverlay and the action row.
2. **Tap writes the waiting Task**, not the viewing Intent. Decline
   still cancels that Task. No bulk DAG confirm.
3. **No AskUser.** Follow actions stay «Открыть задачу» /
   «Открыть действие». Empty and store-down stay buttonless.
4. **Not this slice.** Do not invent Progress percent. Do not convert
   leftover keyboard / gallery / lock formulas. Do not flash.

## Consequences

- Confirm matches the pending request. Rollback: Intent actions
  return to follow-only. Still not Visual v1 sign-off.

## Verification

Host: waiting Intent is Confirm/Decline; running/empty/offline have
no Confirm. Panther: COM13 is live; do not flash; leave Сейчас.
