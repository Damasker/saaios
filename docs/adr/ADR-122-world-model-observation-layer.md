# ADR-122: World Model / Observation Layer v1

## Статус

Принято, 2026-09-18. WORLD-00 (этот ADR + mapping/roadmap) — docs.
WORLD-01 — host Observation types + conversion из `system.metrics`.
`saai-deviced`, Health, Attention, Pixel sampling не заявлены.

## Нумерация

После ADR-121 следующий свободный номер — **122**. Не S33.
Visual Language остаётся hardware-changing треком.

## Контекст

`system-intelligence.md` уже предусматривает Observation, HealthState,
Verification и `saai-deviced`. В коде есть `system-tools` (реальные
read tools) и `TelemetrySampler` (`ToolResult` → EventBus). Отдельного
`saai-deviced` нет. Цифра `cpu_usage: 37` не несёт источник, время и
freshness.

## Решение

**World Model v1** — слой «что наблюдается сейчас». Не AI memory, не
SOM, не Prometheus, не Policy.

```text
source → Observation(source, time, TTL)
       → Freshness (Fresh | Stale)
       → (later) HealthState (Healthy | Degraded | Unhealthy | Unknown)
       → WorldSnapshot
```

Инварианты:

1. Observation всегда имеет subject, source, timestamp, TTL.
2. **Stale не HealthState.** Устаревшее наблюдение → Health `Unknown`,
   не `Unhealthy` и не last-known `Healthy`.
3. `Unknown` — нормальное состояние.
4. Subject: `LocalDevice` или existing `ObjectRef`. Нет второго id.
5. Observation key ≠ tool name (`system.metrics` → `system.cpu.usage`).
6. High-frequency samples не пишутся в `saai-entity-store`.
7. После reboot cache не поднимается как fresh.
8. Model не создаёт authoritative Observation.
9. `TelemetrySampler` / `system.metrics` JSON contract не ломаются.
10. Linux `/proc` parsing не копируется в новый crate в WORLD-01.
11. VerificationContract остаётся у OAM/WSV2; World Model даёт evidence.
12. `saai-deviced` (WORLD-03) не вызывает модель, не исполняет Action,
    не принимает Policy.

## Roadmap

Независимый track `WORLD-*` (`docs/os/sprints/WORLD-ROADMAP.md`).

## Что не делается здесь

Нет daemon, Health evaluator, Attention, self-healing, time-series DB,
plugin observers, TCP listener.

## Ссылки

- `docs/os/architecture/system-intelligence.md`
- ADR-006, ADR-118, ADR-119, ADR-120, ADR-121
- `docs/os/sprints/WORLD-ROADMAP.md`
