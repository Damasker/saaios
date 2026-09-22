# ADR-237: WORK-02 live dispatch — derived ready, concurrency=1

## Статус

Принято, 2026-09-20. WORK-02's `derive_ready_set` / `admit_frontier`
lived in `saai-taskd` but never started work. Planner intents diagnosed
immediately at create, so a `depends_on` child stayed Pending after its
parent became Done. This slice admits at most one ready Task and runs
the planner bridge. PIN stays null. Not Visual v1 sign-off. A5
PlanProposal DAG is still next.

## Нумерация

После ADR-236 следующий свободный номер — **237**. Не S33.

## Контекст

ADR-121 forbids persisting `ready`. The host tests already proved the
derived set and `MAX_MUTATING_IN_FLIGHT = 1`. Live panther still ran
diagnose inside `process_planner_intent`, ignoring the frontier.
WaitingConfirmation must not be auto-admitted. Goal wave A4.

## Decision

1. **Create then admit.** A planner Intent persists a Pending Task and
   calls `dispatch_ready`. Diagnose runs only for an admitted id.
2. **Frontier.** `next_admission` = ready ∩ (in_flight < 1). Running
   counts as in-flight. Confirmation/clarification waits are not ready.
3. **Unblock.** Task `Done`/`Failed`/`Pending` and boot/follow
   reconcile call `dispatch_ready` so a child whose parents are Done
   starts without a new Intent.
4. **No Ready status.** Stale Task Created events must not downgrade a
   newer revision in `known_tasks`.

## Consequences

- Linear S10 still works: one Pending, idle frontier, one diagnose.
- A `depends_on` child no longer stalls after the parent finishes.
- Rollback: diagnose again inside `process_planner_intent`. PlanProposal
  persist stays A5. Still not Visual v1 sign-off.

## Verification

Host: `cargo test -p saai-taskd` including `next_admission_*`. Panther:
flash `saai-taskd`; do not type an Intent; do not tap Разрешить.
Marker on.
