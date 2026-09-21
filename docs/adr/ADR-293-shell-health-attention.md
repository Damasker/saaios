# ADR-293: shell Attention reads status.health

## Статус

Принято, 2026-09-21. ATTN-06 wire. Not a dashboard. PIN stays null.
Do not flash shell.

## Нумерация

После ADR-292 следующий свободный номер — **293**. Не S33.

## Контекст

ADR-291 can turn Unhealthy into Attention. ADR-292 puts one Health
report on runtime `status`. Shell still called `project_from_entities`
and ignored that field, so the next shell experiment would still
guess. Sistema already shows Observation rows; Health is not a second
graph.

## Decision

1. **Same blob.** `live_health_from_status_json` reads
   `status.health.{component_id,state}`. Missing or unknown state is
   `None`, never a guessed Healthy.
2. **Same projection.** NOW and Orb call `project_with_health`. Inbox
   still skips Health. Healthy/Unknown stay omitted. Degraded is NOW
   only. Unhealthy lights Orb.
3. **Not chrome.** Система still shows Observation, not Health.
   Labels stay the component id and the state word. No graphs.
4. **Host only.** Binary on panther is unchanged until the next
   allowed shell experiment.

## Consequences

- Next shell flash plus a runtime that already has ADR-292 is enough
  for Unhealthy to light Orb without guessing.
- Rollback: drop the parser and keep `project_from_entities`.

## Verification

Host: `cargo test -p saai-shell --offline live_health unhealthy_health healthy_health`.
No panther flash.
