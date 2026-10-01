# ADR-428: WORK-05 — deterministic admission order and optional priority

## Статус

Принято, 2026-10-01. Host only (`saai-taskd` scheduler, `saai-shell` «Далее»).
No taskd or shell flash. Не S33.

## Контекст

`derive_ready_set` ended with `ready.sort()` on the Task UUID. UUIDs are
random, so "which ready Task starts first" was arbitrary, not oldest-first.
Meanwhile NOW «Далее» (WORK-08) picked the first ready Task in *store*
order. The screen could name one Task while the scheduler admitted another.
The shell also ignored the scheduler's rule that sibling plan steps are not
ready while one step awaits confirmation.

## Decision

1. Admission order is one total order: `priority` (high, normal, low),
   then `created_at` ascending, then id. `derive_ready_set` returns that
   order; `next_admission` starts the first element.
2. `priority` is an optional Task property: `low | normal | high`. Missing
   or unknown means `normal`; it is not written when `normal`. A malformed
   value neither promotes nor strands a Task. No component sets it yet:
   there is no AI ranking and no learned priority. It is the contract for a
   later explicit user action.
3. Priority orders only the *ready* set. It never bypasses dependencies,
   `WaitingConfirmation`, the plan-confirmation rule, or
   `MAX_MUTATING_IN_FLIGHT`.
4. `task_properties_after_result` preserves `priority`.
5. `saai-shell` `next_ready_task` applies the same rank and the
   plan-confirmation rule, so «Далее» names what the scheduler admits next.

## Consequences

FIFO is the practical change today. Rollback: restore `ready.sort()` and
first-in-store `next_ready_task`; the `priority` property is ignored by older
code. The shell keeps a small copy of the rank because the shell does not
depend on the `saai-taskd` crate; a test pins the same cases on both sides.

## Verification

Host: `cargo test -p saai-taskd` (110 passed) and
`cargo test -p saai-shell -- --test-threads=1` (311 passed), including
`equal_priority_admits_oldest_first_not_random_id_order`,
`high_priority_jumps_the_queue_low_waits`,
`unknown_priority_is_normal_not_promoted_or_stranded`,
`priority_never_bypasses_dependencies_or_the_cap`,
`priority_survives_the_result_property_rebuild`,
`next_ready_task_follows_scheduler_admission_order`,
`next_ready_task_skips_plan_steps_while_a_sibling_awaits_confirmation`.
