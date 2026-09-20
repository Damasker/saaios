# ADR-292: runtime status includes one Health report

## Статус

Принято, 2026-09-21. WORLD-06 wire. Not a dashboard. PIN stays null.
Do not flash runtime.

## Нумерация

После ADR-291 следующий свободный номер — **292**. Не S33.

## Контекст

ADR-287 derived CPU sampler Health in-process. ADR-291 can turn
Unhealthy into Attention. Shell-legal read for Observation is already
runtime `status`. Without Health on that blob the next shell experiment
would have to guess or re-derive.

## Decision

1. **`health`.** `RuntimeStatusDto.health` is one object:
   `component_id` + `state`. Always present on a live status call.
2. **Same function.** `cpu_sampler_health` on the same cache snapshot
   as observations. Fresh Direct → Healthy. Stale/missing → Unknown.
   CPU percent is not a field.
3. **Not chrome.** Система still shows Observation rows. Attention
   still waits for the next shell experiment to call
   `project_with_health`.
4. **Compat.** Unknown field for old clients. `#[serde(default)]`.

## Consequences

- Flashing runtime later is enough for shell to read Health.
- Rollback: drop `health` from the DTO.

## Verification

Host: `cargo test -p saaios-runtime --offline status`. No panther
flash.
