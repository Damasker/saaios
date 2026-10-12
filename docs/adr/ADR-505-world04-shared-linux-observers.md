# ADR-505: WORLD-04 shared Linux observers

## Статус

Принято, 2026-10-12. Host. Not `saai-deviced`. Not WORLD-08 Attention.
No runtime flash. Lock SIGTRAP stays open.

## Нумерация

После ADR-504 следующий свободный номер — **505**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

WORLD-01/02 convert `system.metrics` JSON into Observations. The
JSON still came from `system-tools` re-reading `/proc`. WORLD-04 asks
for shared Linux observers so Observation and tools do not each own a
copy of loadavg/meminfo/stat parsers. Host lab is `mike@macpro`
(`192.168.88.223`, Debian 13).

## Решение

1. `saai-observation::linux` owns `read_load_average`, `read_meminfo_mb`,
   `read_cpu_times`, `cpu_usage_between`, `HostMetricsSample`.
2. `system-tools` `LinuxBackend::metrics` calls those helpers (async sleep
   between CPU samples stays in the tool).
3. `HostMetricsSample::to_observations` reuses WORLD-01 JSON conversion
   so ToolResult and Observation stay the same shape.
4. No UDS daemon (WORLD-03 stays backlog until a second process needs it).

## Откат

Restore inline `/proc` reads in `system-tools`; drop `linux.rs`.

## Ссылки

- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-04
- ADR-122
