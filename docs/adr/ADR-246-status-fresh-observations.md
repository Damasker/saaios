# ADR-246: runtime status lists only Fresh Observations

## Статус

Принято, 2026-09-20. Runtime service binary; panther flash followed
host-green. PIN stays null. Not Visual v1 sign-off. No graphs.

## Нумерация

После ADR-245 следующий свободный номер — **246**. Не S33.

## Контекст

WORLD-02 cache is in runtime. Система must show live Observation only.
Stale ≠ Unhealthy: stale rows are omitted, not painted as broken.
High-frequency samples stay out of entity-store. Existing `{"op":"status"}`
is the read path until a second consumer needs `saai-deviced`.

## Decision

1. `RuntimeStatusDto.observations` is Fresh rows only: key, value,
   unit, source, observed_at.
2. Stale TTL → empty list, revision still counts the cache.
3. No time-series, no Prometheus, no export, no Intent text.

## Consequences

Shell Система paint is the next screen slice. Rollback: drop the
`observations` field.

## Verification

Host: `cargo test -p saaios-runtime --offline -- --test-threads=1`
8 passed, including `status_lists_only_fresh_observations`.
Panther: runtime `35b3d642…` pid 1958. Shell `6ca55bfd…` pid 1719.
Marker on. Leave Сейчас.
