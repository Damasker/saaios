# ADR-473: VUI-09 — Intent plan steps are live Task rows

## Статус

Принято, 2026-10-11. Handoff `intent-default` shows 1/5/12 DAG
steps, not a joined caption. `plan_steps_for_intent` already lists
those Tasks. Each step is `object.step.{i}` Button. Empty stays
«Нет задачи». Confirm stays the existing AskUser pair on the
waiting Task, not a board Confirm. Public compose
`intent-public.sui` stays header+Field. Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-472 следующий свободный номер — **473**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-239 painted two-or-more Tasks as `План: A → B`. The handoff
wants one row per Task, including the one-item case. Inventing
steps or a bulk Confirm would skip IRAB/AskUser.

## Decision

1. **Host-lock** an Intent overlay with N `object.step.{i}` Buttons
   docks them below `ObjectSummary`. Tap follows that Task.
2. **Live.** When the viewed entity is `saaios.intent`, emit one
   Button per `plan_steps_for_intent`. Zero steps: no rows, status
   stays «Нет задачи». Plan Tasks are omitted from related peers
   so they are not duplicated.
3. **Confirm.** Do not add a Confirm control unless the existing
   waiting-Task actions already exist. No bulk DAG confirm.
4. **Not this slice.** Do not change compose `intent-public.sui`.
   Do not invent workers. Do not promote Stable.

## Consequences

- Intent Object View matches the handoff step list. Rollback: joined
  caption only. Still not Visual v1 sign-off.

## Verification

Host: step Buttons hit `object.step.{i}`; empty has none. Panther:
do not flash; leave Сейчас.
