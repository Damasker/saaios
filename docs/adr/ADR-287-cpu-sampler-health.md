# ADR-287: CPU sampler Health from Observation freshness

## Статус

Принято, 2026-09-21. WORLD-06 host. Not a dashboard. Not Система chrome.
PIN stays null. Do not flash runtime.

## Нумерация

После ADR-286 следующий свободный номер — **287**. Не S33.

## Контекст

ADR-122 separates Freshness from HealthState. WORLD-05 already feeds
Fresh rows into verification. WORLD-06 is one deterministic component,
not Health for every subsystem and not invented graphs.

`system.cpu.usage` already exists from TelemetrySampler. Magnitude is
not a threshold: 97% Fresh is still Healthy for the sampler.

## Decision

1. **Component.** `system.cpu.sampler` reads LocalDevice
   `system.cpu.usage`.
2. **Mapping.** Missing or Stale → `Unknown`. Fresh Direct/Authoritative
   → `Healthy`. Fresh Approximate → `Degraded`. Fresh JSON `false` →
   `Unhealthy` (boolean facts only; CPU percent never Unhealthy).
3. **Not last-known.** A sample that was Healthy and then went Stale
   is Unknown, not Healthy.
4. **No wire.** Runtime `status` and shell stay unchanged. No graphs.

## Consequences

- Attention (ATTN-06) can later consume this report. Do not paint it
  as «всё Healthy».
- Rollback: drop `crates/saai-observation/src/health.rs`.

## Verification

Host: `cargo test -p saai-observation --offline health`. No panther
flash.
