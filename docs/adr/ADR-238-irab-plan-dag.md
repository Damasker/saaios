# ADR-238: IRAB Direct vs PlanProposal DAG vs Confirm

## Статус

Принято, 2026-09-20. `ResolutionOutcome::Plan` fell through to
`legacy_runtime_diagnose` and became one Task. Direct OAM and
`WaitingConfirmation` already created Tasks. This slice persists a
validated PlanProposal as a DAG and keeps confirmation per Action.
PIN stays null. Not Visual v1 sign-off.

## Нумерация

После ADR-237 следующий свободный номер — **238**. Не S33.

## Контекст

The Orb board's grammar is Direct / Plan / Confirmation / Result.
IRAB already resolved Action, Clarification, Unsupported without
diagnose. Plan was `NeedsModel`. WORK-01 validates DAGs in `graph.rs`
but nothing persisted them. A4 admits ready Tasks; a `depends_on`
child could not exist on panther. Goal wave A5. Never leave an Intent
without a Task.

## Decision

1. **Direct.** Explicit/pronoun OAM Action stays one Task. No DAG.
2. **Plan.** Intent property `plan` (PlanProposal JSON) is a
   deterministic Plan outcome. `validate_plan` / `bind_plan` persist
   one Task per step with `proposal_id` and `depends_on_task_ids`.
   Invalid graphs fail one Task. Dispatch runs admitted steps;
   `semantic_action_id` steps do not diagnose.
3. **Confirm.** `requires_confirmation` still writes
   `WaitingConfirmation`. A plan does not bulk-allow: a sibling plan
   step is not ready while this Intent has a confirming step.
4. **NeedsModel.** Free-form without `plan` still uses the runtime
   diagnose bridge.

## Consequences

- Intent screen can show plan → progress as real Tasks. Rollback:
  match Plan back to diagnose. Visual v1 still unsigned. Object View
  «Повторить» stays wave B.

## Verification

Host: `cargo test -p intent-resolution -p saai-taskd` 14+81 passed, including
`structured_plan_is_not_a_diagnose_fallback`,
`bind_plan_remaps_proposal_ids_in_topo_order`,
`plan_does_not_admit_sibling_while_confirming`. Panther: binary
`351b0b6f…` pid 975. Do not type an Intent; do not tap Разрешить.
Marker on.
