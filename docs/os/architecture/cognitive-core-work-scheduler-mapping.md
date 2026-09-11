# Sprint 0 — Architecture Mapping: Cognitive Core and Work Scheduler

**Status:** Docs-only deliverable (no code, no binary changes).  
**Baseline:** `feat/pixel7-native-saaios` @ `2d65060` (S10 Done, CI green).  
**Audience:** TL gate after S01 closed; maps the Cognitive Core / Work Scheduler
vision onto what SaaiOS already ships and what remains backlog.

This document does **not** implement anything. It records accepted TL decisions
and binds them to current components, ADRs, and sprints S09–S11.

## Purpose

After S01 closed, produce the deferred Sprint 0 mapping for:

- **Cognitive Core** — understanding and planning (probabilistic);
- **Work Scheduler** — when and with what to run authorized work (deterministic).

Source vision (chat, 2026-09-06): embodiment, Planner ≠ Scheduler, Task DAG,
queues, Workers, Supervisor, Task Ledger, time-over-compute.  
TL accepted that vision as a **target computational model**, not a competing
roadmap. Work slots into existing OS sprints; reuse beats replace
([ADR-004](../../adr/ADR-004-os-track.md), [ADR-006](../../adr/ADR-006-system-intelligence-not-chat.md),
[system-intelligence.md](system-intelligence.md)).

## Accepted TL decisions (honor these)

| Decision | Meaning for MVP |
|---|---|
| **Planner / Scheduler split (from S09)** | Planner proposes; Workflow / Scheduler owns state transitions and the execution frontier. |
| **One workflow store + derived ready set** | Single authoritative persistence for Task/Action; Ready is a view/index, not a second store. |
| **No durable Task / Ready / Action queues** | Reject three durable queue files. Action frontier is in-memory only. |
| **Task Ledger = `audit-log`** | Extend append-only audit; do not invent a parallel ledger. |
| **UDS on device / TLS for remote TCP** | Local control plane defaults to Unix sockets; remote TCP requires TLS (prefer mTLS). |

Related earlier TL framing: Scheduler ≠ Cognitive Core; time-over-compute;
bounded fan-out; ActionIntent → Policy → Capability; Sprint 0 = map first.

## Current components vs Cognitive Core vs Work Scheduler

```text
USER / schedule / automation
        │
        ▼
   Intent (entity)
        │
   ┌────┴─────────────────────────────┐
   │         Cognitive Core           │
   │  (propose only; no execution)    │
   │                                  │
   │  saaios-runtime / AiRuntime      │
   │  + policy-engine (risk signal)   │
   │  + tool-registry schemas         │
   │  + space-scoped memory           │
   └────┬─────────────────────────────┘
        │ PlanProposal / diagnose result
        ▼
   ┌──────────────────────────────────┐
   │         Work Scheduler           │
   │  (deterministic orchestration)   │
   │                                  │
   │  saai-taskd  ≈ Workflow service  │
   │  saai-entity-store = workflow    │
   │       store (Intent/Task/Action) │
   │  schedule tick → Intent          │
   │  ready set = derived from store  │
   │  execution frontier = in-memory  │
   └────┬─────────────────────────────┘
        │ AuthorizedAction / confirm
        ▼
   Workers (tool path / entity ops)
        │
        ▼
   Policy → Capability → DEVICE
        │
        ▼
   Task Ledger (audit-log) + entity Events
```

| Vision name | Maps to today | Boundary |
|---|---|---|
| Cognitive Core | Platform `saaios-runtime` / `AiRuntime` diagnose loop; local or remote model | Proposes actions; must not claim success or mutate device without Policy |
| World Model (v0) | S01 `DeviceContext` / `system.identity` + metrics tools; not a full sensor graph | Facts from local sources only (ADR-006) |
| Capability Catalog | `tool-registry` + S07 app sandbox/portals; native risk enum in `saai-taskd` | Evolve registry; do not replace |
| Policy | Platform `policy-engine` on planner path; native confirmation for explicit dangerous Actions | Authorize only; no execution |
| Workflow service | Native `saai-taskd` + `saai-entity-store` entities | Owns Task/Action state machine |
| Work Scheduler | Sub-role of `saai-taskd` (reconcile, schedule tick, bridge, confirm) | Ready selection + budgets; no LLM |
| Event Bus | Platform in-process bus; native `saai-entityd` Subscribe/Events | Live fan-out; not durable truth |
| Embodiment UI | `saai-shell` Intent field, `Сейчас`, `TaskConfirm` | Intent metaphor, not chat-home |

Platform Track and OS Track remain two runtimes ([ADR-004](../../adr/ADR-004-os-track.md),
[ADR-030](../../adr/ADR-030-intent-task-action-native-entities.md)). S10 bridges them
with `saai-taskd` → existing TCP/JSON diagnose/confirm
([ADR-033](../../adr/ADR-033-planner-bridges-to-running-saaios-runtime.md)) — one
writer into the entity store (`saai-taskd`), one risk classifier (`policy-engine`).

## Roles and ownership

| Role | Owner today | Owns | Must not |
|---|---|---|---|
| **Planner** | `saaios-runtime` (called from `saai-taskd`) | Interpret free-text Intent → proposed tool/action under budgets (`max_tool_iters`) | Execute outside Policy; write entity store directly; declare Verification |
| **Scheduler** (Work Scheduler) | `saai-taskd` | Create/advance Task/Action entities; schedule → Intent; choose runnable work; reboot reconcile without auto-exec of `waiting_confirmation` | Call the model itself; invent a second risk taxonomy for planner path |
| **Supervisor** | Colocated in `saai-taskd` (+ runtime budgets) | Timeouts, cancel/confirm paths, idempotent resume, failure → `failed` | Re-plan; authorize |
| **Worker** | Tool executors / entity mutations invoked after Policy / confirmation | Run one authorized action to completion or timeout | Parse model prose as a tool name |
| **Policy** | `policy-engine` (planner path); native dangerous-action gate (explicit Intent) | Allow / Deny / AskUser | Execute tools; call model |
| **UI** | `saai-shell` | Capture Intent; show state; live confirmation | Bypass TaskConfirm for dangerous work |

Long-term: keep Planner probabilistic and Scheduler deterministic. Short-term
colocation of planning and execution inside `AiRuntime` for console-only paths
is technical debt; the native phone path already splits at `saai-taskd`.

## Data model: one store, derived ready set

### Authoritative workflow store

Native `saai-entity-store` (S06) with entity types
([ADR-030](../../adr/ADR-030-intent-task-action-native-entities.md)):

- `saaios.intent`
- `saaios.task`
- `saaios.action`
- `saaios.schedule` ([ADR-036](../../adr/ADR-036-schedule-triggers-native-entity.md))

Workflow status lives in entity `properties` (`pending` /
`waiting_confirmation` / `running` / `done` / `failed` / `cancelled`, etc.).
Cold-reboot resume is `list_entities` + reconcile — never auto-execute
confirmation-gated work ([ADR-032](../../adr/ADR-032-change3-dangerous-confirmation-verified.md)).

This is the **one workflow store**. It is the Task table (all states).

### Derived ready set (not a durable Ready Queue)

**Ready** = query/view over the store: tasks/actions whose dependencies are
satisfied, status is runnable, and policy/confirmation gates allow progress.
On process start, rebuild from the store (and ledger), do not read a separate
ready file.

Today: `saai-taskd` Subscribe + status filters approximate this. A materialized
`ready_index` KV is optional optimization later — still not a second source of
truth.

### Why no durable Task / Ready / Action queues for MVP

| Proposed queue | MVP ruling | Rationale |
|---|---|---|
| Durable Task Queue | **Reject as separate store** | Tasks already persist as entities. |
| Durable Ready Queue | **Reject** | Dual-write → ready-set drift and crash-recovery bugs. Ready is derived. |
| Durable Action Queue | **Reject** | Actions are records in the workflow store. The **execution frontier** is an in-memory deque (fan-out cap, default 1 mutating action). After crash, re-derive from store + audit. |

DAG / fork-join / multi-worker pools remain **non-goals for MVP** until a
linear Intent → Task → Action → Result path is proven (already true on device
for S09/S10) and budgets demand parallelism.

## Task Ledger = audit-log

Platform `audit-log` (JSONL under runtime audit path, e.g.
`/data/saaios/var/runtime/audit.jsonl` on panther) is the **Task Ledger**:

- append-only causal records (`correlation_id`, `causation_id`, message kinds);
- every policy decision, tool result, and (over time) workflow transition
  should mirror here;
- replay source for Supervisor / diagnostics (`audit-replay`).

Native entity `Event` stream is the space-scoped structural history for UI and
store consistency. It does **not** replace audit-log as the cross-runtime
causal ledger.

**Do not** create a third “Task Ledger DB”.

## Transport: UDS on device, TLS for remote TCP

| Surface | Decision | Current reality |
|---|---|---|
| On-device control | **UDS** (`/run/saaios/*.sock`, `SAAIOS_SOCK`) | Native daemons already use UDS; runtime keeps local socket alongside USB TCP |
| Remote / host TCP | **TLS required** (prefer mTLS / pinning); no anonymous LAN exposure | Bring-up still uses plaintext TCP `172.31.7.1:38127` on USB NCM ([panther README](../targets/panther/README.md)); `console-web` already has optional `--tls-cert` / `--tls-key` |
| Remote model HTTPS | rustls + secrets outside image/git/audit | In use for local Ollama / providers |

Security baseline remains a cross-cutting gate (UDS permissions, peer
credentials, authenticated USB session, TLS 1.3 for TCP). Closing plaintext
USB TCP is backlog relative to this mapping — the **architecture choice** is
already fixed: UDS default on device; TLS for remote TCP.

## Mapping to sprints S09–S11

| Sprint | Cognitive / Scheduler content | Status vs mapping |
|---|---|---|
| **S09** Intent → Task → Action | Workflow store as entity types; `saai-taskd`; confirmation survives cold reboot; Planner/Scheduler split *starts* (deterministic native path, echo placeholder) | **Exists / Done** — ADR-029…032, [S09 passport](../sprints/S09-intent-task-action.md) |
| **S10** Planner, automation, memory | Planner bridge to on-device `saaios-runtime`; schedules as entities → Intent; space-scoped memory | **Exists / Done** — ADR-033…039, [S10 passport](../sprints/S10-planner-automation-memory.md) |
| **S11** Performance, idle, daily use | Budgets, latency/memory measurements, idle energy — supports time-over-compute, not new queues | **Ready / backlog** at baseline `2d65060` — [S11 passport](../sprints/S11-performance-idle-daily-use.md) |

### Exists (do not rebuild)

- Intent/Task/Action persistence and idempotent `saai-taskd`
- Planner proposes via runtime; policy/`pending` drives confirmation UI
- Schedule → ordinary Intent → same pipeline
- Audit JSONL + entity Events
- UDS for native services; DeviceContext for planner context

### Backlog (aligned with mapping, not new roadmap IDs)

- Materialized ready-index (optional) still derived from one store
- Explicit workflow event kinds in audit-log for every Task/Action transition
- TLS/mTLS for remote TCP control plane; harden UDS credentials
- Richer World Model beyond identity/metrics
- Multi-action DAG proposals with fan-out caps
- Standalone Supervisor process (only if multi-process workers prove necessary)
- Full `Входящие` workflow catalog UI (explicitly out of S09/S10)
- ADR-004 convergence (shared tool/policy APIs across tracks)
- Opportunistic compute / preemption (time-over-compute beyond S11 measurements)
- Self-repair / code agents — deferred, human-gated forever in v1

## Open decisions / non-goals / risks

### Open decisions

1. When to require TLS on the USB bring-up TCP port (S07-class security gate vs S11/S12).
2. Whether Supervisor stays forever inside `saai-taskd` or splits after multi-worker evidence.
3. How far native risk vocabulary converges with `tool-registry::RiskLevel` under ADR-004.
4. Whether Platform console `handle_user_text` path should persist pending confirmation like S09 (today still in-memory) — out of phone-critical path.

### Non-goals (this mapping and near MVP)

- Durable Task/Ready/Action queue products
- Competing Cognitive Core sprint numbering beside S00–S12
- Replacing `audit-log`, `tool-registry`, or `event-bus` wholesale
- Always-on local model daemon without scheduler/opportunity budget
- Self-deploy / autonomous self-repair in v1
- Distributed worker pools on phone (`max_concurrent` default remains 1)

### Risks

| Risk | Mitigation |
|---|---|
| Queue proliferation / dual truth | One store + derived ready; in-memory frontier only |
| Planner becomes scheduler again | Keep `saai-taskd` as sole entity writer; planner returns proposals/`pending` only |
| Plaintext TCP treated as production API | Document as bring-up; TLS gate before Wi-Fi exposure |
| Scope explosion (DAG, multi-agent, self-repair) | Slot into S11+ backlog; Evidence-gated sprints |
| Two runtimes drift | Bridge pattern (ADR-033); no second writer into entity store from runtime |

## Explicit: no code in this change

This file and its index links are documentation only. No crate, service, image,
or protocol changes accompany this Sprint 0 mapping.

## References

- [system-intelligence.md](system-intelligence.md) — target components (Workflow, Planner, Policy)
- [application-platform.md](application-platform.md)
- [sprints/README.md](../sprints/README.md) — S09–S11 roadmap
- ADR-002 (hybrid IPC / UDS), ADR-004, ADR-006
- ADR-030…039 — Intent/Task/Action, planner bridge, schedules, space memory
- TL decisions 2026-09-06 (Planner/Scheduler split; one store; reject durable queues; audit-log = ledger; UDS/TLS)
