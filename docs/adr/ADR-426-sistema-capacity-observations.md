# ADR-426: Система capacity comes from live Observations

## Статус

Принято, 2026-10-01. Host only: runtime/telemetry/shell logic, no panther
flash, no hardware-changing experiment. Not Visual v1 sign-off.
Номер 426, а не 424: 424–425 заняты веткой `feat/lockscreen-policy-widgets`.

## Контекст

Board «Система» ([product-visual-target-v1](../os/ui/product-visual-target-v1.md))
shows resources as capacity readouts: RAM `3.2 / 8 ГБ`, storage used / total.
ADR-246/247 gave Система live Observations, but the cache only carried CPU,
load and memory **percent**. There was no byte-level memory and no storage at
all, so the screen could not show the board's resource block without
inventing numbers. «Нет данных» is correct; a painted `64 / 256 ГБ` is not.

## Decision

1. `saai-observation` adds `system.memory.{used,total}_mb` (from the existing
   `system.metrics` fields) and `system.storage.{used,total}_mb` (new
   `observations_from_system_disk` over the existing `system.disk` tool).
   Unit is `megabyte`.
2. Storage reads one mount: `/data` when present (the user-visible capacity
   on a phone), otherwise `/`. `system.disk` now also asks `df` for `/data`.
3. Impossible numbers (non-finite, `total <= 0`, `used < 0`, `used > total`)
   produce no rows. Nothing is clamped.
4. `TelemetrySampler` fetches `system.disk` best-effort in the same tick and
   applies metrics + storage in **one** `cache.apply`, so the revision still
   bumps once per sample. A missing or failing disk tool omits storage and
   never fails the metrics sample. Disk output is not published on the bus.
5. Storage TTL is 120 s (`METRICS_STORAGE_TTL`): it changes slowly and `df`
   is a subprocess. It still goes Stale, and runtime `status` still lists
   only Fresh rows (ADR-246). CPU/load/memory TTL is unchanged.
6. Shell folds each used/total pair into one readout: `Память  3.2 / 8.0 ГБ
   (40%)` (percent merged when present) and `Хранилище  40.0 / 128.0 ГБ`.
   A pair missing either half, or with `used > total`, is dropped, never
   shown as half a number. Source stays on the line. No graph.

## Consequences

Runtime `status` DTO is unchanged (generic Observation rows). Android VM /
Linux runtime rows, CPU history graph and services list from the board stay
absent until a real source exists (APP-COMPAT, `saai-deviced`).
Rollback: drop the four keys and `observations_from_system_disk`; the shell
fold is inert without them and falls back to the percent row.

## Verification

Host: `cargo test -p saai-observation -p telemetry -p system-tools -p saaios-runtime`
and `cargo test -p saai-shell -- --test-threads=1` (309 passed), including
`storage_prefers_data_mount_over_root`,
`impossible_capacity_numbers_are_dropped_not_clamped`,
`storage_outlives_the_cpu_ttl_but_still_goes_stale`,
`missing_disk_tool_keeps_metrics_and_omits_storage`,
`capacity_observations_fold_into_one_readout_or_vanish`.
Panther: not flashed. `df -Bk` behaviour on the phone image is unverified;
if it fails, storage is simply absent.
