# SaaiOS World Model / Observation Layer — delivery roadmap

Status: **WORLD-00/01 host complete** (types + metrics conversion).
Daemon (WORLD-03) is not on the Pixel path until ObservationCache is
used in runtime. See [PIXEL-PATH.md](PIXEL-PATH.md).

Architecture: [ADR-122](../../adr/ADR-122-world-model-observation-layer.md),
[system-intelligence.md](../architecture/system-intelligence.md)

Independent track (same pattern as WORK / VUI / APP-COMPAT). Not S33.

## Outcome

Make current reality typed: source, time, freshness. Planner, Scheduler
and UI consume facts, not tool-success guesses or stale numbers.

Order is mandatory:

```text
Observation → freshness → (later) Verification → Health → Attention
```

Do not start with a monitoring product or HealthState.

## Delivery

| ID | Result | State | Phone? |
|---|---|---|---|
| WORLD-00 | ADR-122 + mapping + this roadmap | **Done** | no |
| WORLD-01 | Observation types + `system.metrics` conversion | **Done** (host) | no |
| WORLD-02 | ObservationCache + snapshot revision | Backlog | no |
| WORLD-03 | `saai-deviced` UDS GetSnapshot/Subscribe | Backlog | no |
| WORLD-04 | Shared Linux observers (no `/proc` copy) | Backlog | no |
| WORLD-05 | Verification uses fresh observation | Backlog | measure |
| WORLD-06 | One deterministic Health component | Backlog | no |
| WORLD-07 | Automation ObservationThreshold (legacy stays) | Backlog | no |
| WORLD-08 | NOW/Attention feed (deviced does not notify) | Backlog | **yes** |

## WORLD-01

**Goal:** mock `SystemMetrics` become Observations with keys, units,
source, timestamps and freshness. Stale ≠ Unhealthy.

**Change:** crate `saai-observation`. No daemon. No entity writes.
`system.metrics` tool output unchanged.

**Test:** host unit tests (keys, units, source, TTL → Stale).

**Rollback:** drop the crate; tools/telemetry untouched.

**Threat:** none — no network, no phone binary, no secrets in observations.
