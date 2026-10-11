# ADR-490: VUI-09 — workers/queue stay hidden without an Action

## Статус

Принято, 2026-10-11. Spec §7: Workers / queue are derived ready
tasks / WorkerExecution. Hidden without source. No persistent
agents. Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off.

## Нумерация

После ADR-489 следующий свободный номер — **490**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Object View always painted `AgentSummary::unassigned()` («Нет
исполнения») on every workflow entity. That is a manufactured
queue row. A live `saaios.action` is the only WorkerExecution
source. Notifications and unknown types already omitted the row.

## Decision

1. **Host-lock.** `agent_summary_from_lineage` returns `Some` only
   when lineage has a real Action. No Action → `None`. Details omit
   the execution line.
2. **Not this slice.** Do not invent `Воркеры (N)`. Do not convert
   leftover keyboard / gallery / lock formulas. Do not flash.

## Consequences

- Object View no longer pretends a worker slot exists. A live
  Action still shows «Исполнение: …». Rollback: restore
  `AgentSummary::unassigned()`. Still not Visual v1 sign-off.

## Verification

Host: waiting Task and empty Intent have `agent = None`; running
Action still names the execution. Panther: do not flash; leave
Сейчас.
