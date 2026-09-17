# SaaiOS Work Scheduler v2 — delivery roadmap

Status: **WORK-00 complete; WORK-01 host DAG validation in progress.**
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
| WORK-01 | Task dependency model + DAG validation | **In progress** (host) | no |
| WORK-02 | Derived ready set + concurrency = 1 | Backlog | no |
| WORK-03 | Verification lifecycle (`Verifying`) | Backlog | no |
| WORK-04 | Read-only bounded parallelism | Backlog | measure first |
| WORK-05 | Priority scheduling | Backlog | no |
| WORK-06 | Retry + failure taxonomy | Backlog | no |
| WORK-07 | Bounded ReplanRequest | Backlog | no |
| WORK-08 | Сейчас / Orb / Object View visibility | Backlog | **yes** |
| WORK-09 | Pixel measurements + tuned budgets | Backlog | **yes** |

## WORK-00

**Goal:** lock architecture before code growth.

**Acceptance:** ADR-121 exists; mapping no longer treats DAG as an unnamed
S11+ maybe; this file is the track index; Visual track is not renamed.

## WORK-01

**Goal:** a PlanProposal is either a valid DAG or wholly rejected.

**Current state:** `saai-taskd` creates one Task per Intent; no
`depends_on`; IRAB `Plan` still falls through to legacy diagnose.

**Change:** `graph.rs` — unique ids, missing deps, cycle, task-count and
depth limits; optional `depends_on_task_ids` properties. No dispatch change.

**Test:** host unit tests listed in ADR-121 / spec §152 DAG rows.

**Acceptance:** cyclic / incomplete / oversized proposals never persist;
linear S09/S10 path unchanged.

**Rollback:** drop `graph.rs`; existing intents ignore unknown properties.

**Threat:** none — no new network, no executor, no phone binary change
required to accept the module.

## Non-goals until later IDs

- durable ready/action queues
- `max_mutating_actions` > 1 without measurements
- Planner-owned execution
- Worker-created child Tasks
- bulk plan confirmation
- distributed workers
