# ADR-291: Health adapter for Attention Projection

## Статус

Принято, 2026-09-21. ATTN-06 host. Not a dashboard. PIN stays null.
Do not flash shell.

## Нумерация

После ADR-290 следующий свободный номер — **291**. Не S33.

## Контекст

ADR-123 deferred Health→Attention. WORLD-06 has one CPU sampler
Health report. Stale is Unknown, not Unhealthy. Painting «всё
Healthy» would be a lie. Inbox parity must not grow a fake
notification row.

## Decision

1. **Adapter.** `project_with_health` takes one `HealthReport`.
   `project_from_entities` stays entities-only.
2. **Healthy / Unknown.** No Attention item.
3. **Degraded.** NOW only. Not Inbox. Not Orb.
4. **Unhealthy.** NOW + Orb. Inbox stays Task/Notification only.
5. **Shell.** Still calls `project_from_entities`. Chrome waits
   the next shell experiment.

## Consequences

- Sistema does not gain a Health graph. Attention does not execute.
- Rollback: drop `project_with_health` and `AttentionSource::Health`.

## Verification

Host: `cargo test -p saai-attention --offline`. No panther flash.
