# SaaiOS Work Scheduler v2 — delivery roadmap

Status: **WORK-00 complete; WORK-01 host; WORK-02 host+panther dispatch (ADR-237); WORK-03 host Verifying (ADR-259); WORK-06 host FailureClass (ADR-288); WORK-07 host ReplanRequest (ADR-289); WORK-08 visibility on panther shell `63b8b64`.**
Phone visibility (WORK-08) rides VUI-05, not a separate weekend.
See [PIXEL-PATH.md](PIXEL-PATH.md).

Architecture: [ADR-121](../../adr/ADR-121-work-scheduler-v2.md),
[cognitive-core mapping](../architecture/cognitive-core-work-scheduler-mapping.md)

This is an independent track (same pattern as APP-COMPAT / VUI). It is **not**
S33 and does not reopen S00–S32. Phone/UI slices wait for the one-hardware-
experiment rule. Host-only logic may land in parallel.

## Outcome

Let SaaiOS run multi-step work with a validated DAG, derived ready set,
bounded workers, verification, and reboot recovery — without a second
queue, without persistent agent personalities, and without making the
Planner the scheduler.

## Delivery

| ID | Result | State | Phone? |
|---|---|---|---|
| WORK-00 | ADR-121 + mapping + this roadmap | **Done** | no |
| WORK-01 | Task dependency model + DAG validation | **Done** (host) | no |
| WORK-02 | Derived ready set + concurrency = 1 | **Done** (host + panther dispatch ADR-237) | **yes** |
| WORK-03 | Verification lifecycle (`Verifying`) | **Done** (host, ADR-259) | no |
| WORK-04 | Read-only bounded parallelism | Backlog | measure first |
| WORK-05 | Priority scheduling | Backlog | no |
| WORK-06 | Retry + failure taxonomy | **Done** (host, ADR-288) | no |
| WORK-07 | Bounded ReplanRequest | **Done** (host, ADR-289) | no |
| WORK-08 | Сейчас / Orb / Object View visibility | **Done** (host + panther) | **yes** |
| WORK-09 | Pixel measurements + tuned budgets | Backlog | **yes** |

## WORK-00

**Goal:** lock architecture before code growth.

**Acceptance:** ADR-121 exists; mapping no longer treats DAG as an unnamed
S11+ maybe; this file is the track index; Visual track is not renamed.

## WORK-01

**Goal:** a PlanProposal is either a valid DAG or wholly rejected.

**Current state:** `saai-taskd` persists a validated PlanProposal as
Tasks with `depends_on_task_ids` (ADR-238). IRAB `Plan` no longer
falls through to legacy diagnose.

**Change:** `graph.rs` — unique ids, missing deps, cycle, task-count and
depth limits; optional `depends_on_task_ids` properties. No dispatch change.

**Test:** host unit tests listed in ADR-121 / spec §152 DAG rows.

**Acceptance:** cyclic / incomplete / oversized proposals never persist;
linear S09/S10 path unchanged.

**Rollback:** drop `graph.rs`; existing intents ignore unknown properties.

**Threat:** none — no new network, no executor, no phone binary change
required to accept the module.

## WORK-02

**Goal:** Ready is derived from the store; mutating concurrency is 1.

**Change:** `scheduler.rs` — `derive_ready_set` / `admit_frontier` /
`next_admission`. `dispatch_ready` starts at most one admitted planner
Task (ADR-237). No `ready` status. WaitingConfirmation is not admitted.
Linear S09/S10 one-task path goes through the same frontier.

**Test:** child blocked until parent Done; failed parent blocks; two
Pending with one Running admits nothing; reboot snapshot matches;
`next_admission` unblocks the child.

**Rollback:** diagnose again inside `process_planner_intent`.

**Threat:** one live diagnose per admit; no new queue.

## WORK-03

**Goal:** a Task is not Done because the worker said ok.

**Change:** `WorkflowStatus::Verifying`. `Running → Done` is illegal.
After Result the Task is Verifying. `decide_verification` returns Done
only on a Fresh Observation whose key/value match `verification_key` /
`verification_expected`. Missing and Stale stay Verifying. Mismatch is
Failed. Verifying counts as in-flight and does not complete a parent
for DAG children. WORLD-05 feeds live Observation from runtime `status`
(ADR-274) into the same function.

**Test:** worker ok without contract is not Done; stale is not verified;
Fresh match is Done; mismatch is Failed; verifying parent blocks child.

**Acceptance:** host `cargo test -p saai-taskd`. Phone=no this slice.

**Rollback:** restore `Running → Done` in `valid_transition` and write
Done from `finish_task`.

**Threat:** none — no new network, no shell flash. Chrome that names
`verifying` is ADR-295 (host; panther waits). Result screen is ADR-301.
Orb Complete is ADR-302.

## WORK-06

**Goal:** Failed has a class. Retry ≠ Replan. Unknown idempotency is
not retried.

**Change:** `FailureClass` on the Task (`error_kind`). Timeout and
unreachable stay `retryable` and still need `retry_requested`.
Verification mismatch, malformed JSON, and unknown IO do not.
Legacy rows with `retryable` and no kind stay retryable. No taskd flash.

**Test:** host `cargo test -p saai-taskd`.

**Rollback:** drop `FailureClass`; restore boolean+optional strings.

**Threat:** none — host taxonomy, no phone binary. Shell copy is ADR-296.

## WORK-07

**Goal:** verification mismatch asks the Planner once. Retry ≠ Replan.

**Change:** `ReplanRequest` is computed. Budget is `replan_count` on
the Intent, cap 1. Scheduler calls `process_planner_intent`, not the
failed PlanProposal, not Supervisor, not the Worker. Open sibling
blocks. Timeout stays retry.

**Test:** host `cargo test -p saai-taskd`.

**Rollback:** drop `replan.rs` and `try_replan_failed_task`.

**Threat:** none — host bound, no phone binary. Shell note is ADR-297.

## WORK-08

**Goal:** Сейчас, Orb and Object View show real Task state. They do
not become a scheduler dashboard, worker counter, or second inbox.

**Change:** Object View reads the Task's own status (confirm only when
`waiting_confirmation`). Intent Object View names the related Task.
NOW ObjectSummary trails live work. «Далее» is a pending Action or
the first derived-ready Task (same `depends_on_task_ids` rule as
WORK-02). Orb still uses Running from `in_progress_work` plus ATTN-04.

**Test:** running Task has no confirm buttons; Intent shows related
Task; blocked child is omitted from «Далее»; finished related work
does not trail on the object.

**Rollback:** restore hardcoded «Ждёт подтверждения» and Action-only
«Далее».

## Non-goals until later IDs

- durable ready/action queues
- `max_mutating_actions` > 1 without measurements
- Planner-owned execution
- Worker-created child Tasks
- bulk plan confirmation
- distributed workers
