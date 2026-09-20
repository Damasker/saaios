# ADR-245: ObservationCache lives in runtime, not a daemon

## Статус

Принято, 2026-09-20. Runtime service binary; panther flash followed
host-green. PIN stays null. Not Visual v1 sign-off. No graphs.

## Нумерация

После ADR-244 следующий свободный номер — **245**. Не S33.

## Контекст

WORLD-01 typed Observation + `system.metrics` conversion. TelemetrySampler
still published only ToolResult. Goal wave C: extend that sampler and
WORLD Observation. WORLD-02 cache belongs in runtime until several
consumers need IPC. `saai-deviced` stays later. High-frequency samples
must not enter entity-store. Model does not write Observation.

## Decision

1. **Cache.** `ObservationCache` is in-process, latest-by-key, revision
   bumps on a stored apply. Construction is empty: reboot is not Fresh.
2. **Sampler.** Successful `system.metrics` still publishes ToolResult.
   The same output becomes Observations (`source` / time / TTL) and
   `apply`s the cache.
3. **Stale ≠ Unhealthy.** Freshness is derived at snapshot time.
4. **No daemon / Prometheus / cloud / entity writes / export.**
5. **Система** still does not consume the cache (next slice).

## Consequences

WORLD-03 `saai-deviced` waits for a second consumer. Product-analytics
of screens is a later ADR, default off. Rollback: drop `with_cache`.

## Verification

Host: `cargo test -p saai-observation -p telemetry -p saaios-runtime --offline -- --test-threads=1`
13+2+7 passed, including
`sample_fills_observation_cache_without_breaking_tool_result` and
`stale_ttl_is_not_unhealthy`.
Panther: runtime `93ad729c…` pid 1865, sampler 30s. Shell `6ca55bfd…`
pid 1719. Marker on. Leave Сейчас.
