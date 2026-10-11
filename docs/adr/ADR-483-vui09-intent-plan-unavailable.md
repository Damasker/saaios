# ADR-483: VUI-09 — Intent plan unavailable when the store is down

## Статус

Принято, 2026-10-11. Handoff `intent-offline` names «План
недоступен» plus a cause. Confirmed empty stays «Нет задачи»
(ADR-473). Store down is unknown, not empty. No guessed steps.
Confirm stays hidden. Experimental. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-482 следующий свободный номер — **483**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-473 paints live `object.step.{i}` Buttons and «Нет задачи»
when the DAG is confirmed empty. A disconnected `entityd` still
had cached Tasks, so the overlay could claim a plan or the empty
caption. The board wants unavailable, never guessed.

## Decision

1. **Host-lock** Intent Object View when the store is down:
   `UniversalState::Offline`, status «План недоступен»,
   observation «Нет связи».
2. **Live.** Omit `object.step.{i}` Buttons. Do not use cached
   Tasks as current. Do not say «Нет задачи». Confirm/follow
   actions stay hidden.
3. **Empty.** Store connected and zero plan Tasks: keep
   «Нет задачи».
4. **Not this slice.** Do not change compose `intent-public.sui`.
   Do not convert leftover keyboard / gallery / lock formulas.

## Consequences

- Offline Intent matches the board without replacing ADR-473.
  Rollback: always connected content. Still not Visual v1
  sign-off.

## Verification

Host: connected empty stays «Нет задачи»; store down is «План
недоступен» / Offline / «Нет связи», no steps. Panther: do not
flash; leave Сейчас.
