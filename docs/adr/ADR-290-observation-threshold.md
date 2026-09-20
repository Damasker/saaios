# ADR-290: ObservationThreshold on native schedules

## Статус

Принято, 2026-09-21. WORLD-07 host. Legacy `every_secs` stays.
PIN stays null. Do not flash taskd.

## Нумерация

После ADR-289 следующий свободный номер — **290**. Не S33.

## Контекст

Platform `automation-engine` still has `ToolResultThreshold` on tool
JSON. Native schedules (ADR-036) only had `every_secs`. WORLD-05
already feeds Fresh Observation into verification. Automation should
gate on the same facts, not worker "ok" or Stale numbers.

## Decision

1. **Extra gate.** `observation_key` + `observation_gte` on
   `saaios.schedule`. Interval remains required.
2. **Fresh only.** Missing or Stale evidence is not due. Below
   threshold is not due. `>=` on the numeric Observation text.
3. **Legacy.** Schedules without those properties still fire on
   interval alone. `TriggerKind::ToolResultThreshold` is unchanged.
4. **No graphs.** Magnitude is a gate, not a dashboard. No shell
   chrome. Preserve the properties when the schedule fires.

## Consequences

- Unreachable runtime keeps a threshold schedule quiet, not looping.
- Rollback: drop `is_schedule_due_with` and the two properties.

## Verification

Host: `cargo test -p saai-taskd --offline`. No panther flash.
