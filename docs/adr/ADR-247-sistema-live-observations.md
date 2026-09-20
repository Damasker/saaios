# ADR-247: Система paints only live Observations

## Статус

Принято, 2026-09-20. Система chrome; panther flash followed host-green.
PIN stays null. Do not tap Система. Not Visual v1 sign-off. No graphs.

## Нумерация

После ADR-246 следующий свободный номер — **247**. Не S33.

## Контекст

Runtime `status` already lists Fresh Observation (ADR-246). Система
must not invent CPU graphs, weather, or Unhealthy from Stale. Goal
wave C, one screen. `saai-deviced` stays later.

## Decision

1. Shell reads existing `{"op":"status"}` (UDS, then USB TCP on
   panther). Missing runtime → no observation rows.
2. Section «Наблюдения» only if the payload has live rows. Readout,
   not a chart. Source stays on the line.
3. Empty source / empty key / non-finite value dropped.

## Consequences

Product-analytics of screens is a later ADR, default off. Rollback:
drop the section and the status client.

## Verification

Host: `cargo test -p saai-ui-compiler -p saai-ui-core -p saai-shell --offline -- --test-threads=1`
47+81+275 passed, including
`me_system_sections_show_only_live_observations`.
Panther: shell `5414c57a…` pid 2073. Runtime `35b3d642…` pid 1958.
Leave Сейчас; do not tap Система, Spaces, Поиск, Inbox. Marker on.
