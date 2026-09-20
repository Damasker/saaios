# ADR-259: WORK-03 — Done is verified, not worker ok

## Статус

Принято, 2026-09-20. Host-only. Not Visual v1 sign-off. PIN stays null.
Phone=no this slice (WORK-ROADMAP). WORLD-05 still feeds live
Observation into the same `decide_verification`.

## Нумерация

После ADR-258 следующий свободный номер — **259**. Не S33.

## Контекст

ADR-121 splits Verifier from Supervisor. Mapping forbids declaring Done
from worker "ok". `saai-taskd` still wrote `Running → Done` after every
Action Result. `Verifying` did not exist. Children became ready as soon
as the worker returned.

## Decision

1. **Durable Verifying.** After Result the Task is `verifying`. That
   status is persisted. It is open work and mutating in-flight.
2. **No Running → Done.** `valid_transition` allows `Running → Verifying`
   and `Verifying → Done|Failed`. Worker failure without a Result still
   uses `Running → Failed`.
3. **Evidence.** `decide_verification` takes `verification_key` /
   `verification_expected` and optional `ObservationEvidence`. Fresh
   matching Observation → Done. Stale or missing → stay Verifying.
   Fresh mismatch → Failed. No contract is not success.
4. **DAG.** `is_completed` stays Done-only. A verifying parent does not
   unblock a child.

## Consequences

- Diagnose/IRAB Result no longer auto-completes the Task.
- WORLD-05 can settle Verifying without a new status machine.
- Rollback: restore `Running → Done` in `finish_task`.

## Verification

Host: `cargo test -p saai-taskd` including
`worker_ok_without_contract_is_not_done`,
`stale_observation_is_not_verified`,
`fresh_matching_observation_is_done`,
`verifying_parent_blocks_child`. No panther flash. Do not type an
Intent. Marker on.
