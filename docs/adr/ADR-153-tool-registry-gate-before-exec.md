# ADR-153: Tool registry metadata + single gate before exec

## Status

Accepted, 2026-10-12. Docs only — no code, no phone binary change.

## Numbering

On `feat/pixel7-native-saaios` after ADR-152 the next free number is **153**.

## Context

Industry agent-CLI patterns (ideas only; no vendored sources) commonly
attach per-tool metadata (`read_only`, concurrency safety, permission
class) and keep the model as a proposer, not an executor. SaaiOS already
splits Planner ≠ Scheduler ([ADR-121](ADR-121-work-scheduler-v2.md)),
Policy Allow/AskUser/Deny ([ADR-006](ADR-006-system-intelligence-not-chat.md),
[ADR-124](ADR-124-unified-authority-model.md)), and rejects durable
Task/Ready/Action queues in the Sprint 0 mapping. This ADR records the
thin TL follow-up so registry evolution and the exec path stay aligned.

## Decision

1. **Registry metadata.** Every tool in `tool-registry` declares at least:
   `read_only`, `concurrency_safe`, and a **permission class** (maps to
   existing risk / Policy vocabulary; do not invent a parallel taxonomy).

2. **Cognitive Core proposes only.** The model / diagnose loop may emit
   structured tool-call proposals. It must not execute tools, claim device
   mutation success, or bypass Policy.

3. **Single gate before exec.** One ordered path only:

   ```text
   Proposal → Policy → (AskUser / plan-batch confirm) → Work Scheduler frontier → Worker
   ```

   Plan-batch confirmation does not bulk-allow individual mutating
   actions; per-action Policy still applies ([ADR-121](ADR-121-work-scheduler-v2.md)).

## Explicitly rejected

| Non-goal | Why |
|---|---|
| Importing or vendoring dump / stolen agent-CLI sources | Ideas only; own implementations |
| Agent swarms on phone MVP | Bounded frontier; default mutating concurrency remains 1 |
| Durable Task / Ready / Action queues | One workflow store + derived Ready + in-memory frontier |
| `bypassPermissions` (or equivalent) as device default | Dangerous work stays AskUser / confirm; no silent auto-allow |

## Consequences

- Evolve `tool-registry` schemas toward the three declarations; keep
  Platform vs native risk convergence under ADR-004 / open mapping item.
- No new daemon, no second PolicyEngine, no permission-bypass flag for
  production device profiles.
- Mapping doc gains a one-paragraph pointer; WORK / AUTH roadmaps unchanged.

## References

- `docs/os/architecture/cognitive-core-work-scheduler-mapping.md`
- ADR-006, ADR-030, ADR-033, ADR-119, ADR-121, ADR-124
- `crates/tool-registry`, `crates/policy-engine`
