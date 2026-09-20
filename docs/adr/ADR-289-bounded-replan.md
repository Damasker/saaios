# ADR-289: Bounded ReplanRequest after verification mismatch

## Статус

Принято, 2026-09-21. WORK-07 host. Not a second queue. PIN stays null.
Do not flash taskd.

## Нумерация

После ADR-288 следующий свободный номер — **289**. Не S33.

## Контекст

ADR-121: VerificationFailed → ReplanRequest → Planner. Retry ≠ Replan.
WORK-06 classifies mismatch as not retryable. Without a bound, a
Failed mismatch would either stall the Intent or loop diagnose.

Supervisor must not rewrite the plan. Workers must not spawn children.

## Decision

1. **Request.** `ReplanRequest` is computed, not a durable queue.
   Stored budget is `replan_count` on the Intent.
2. **When.** Only `FailureClass::VerificationMismatch`. No open
   sibling Task. `replan_count < 1` (one Planner pass).
3. **Who.** Scheduler calls existing `process_planner_intent`
   (runtime diagnose). Not `process_plan_intent` (same DAG again).
   Not Supervisor. Not the Worker.
4. **Timeout.** Stays WORK-06 retry. Unknown stays no-retry.

## Consequences

- A second mismatch on the same Intent stays Failed. No loop.
- Rollback: drop `replan.rs` and `try_replan_failed_task`.

## Verification

Host: `cargo test -p saai-taskd --offline`. No panther flash.
