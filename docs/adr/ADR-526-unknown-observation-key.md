# ADR-526: unknown Observation keys stay off Система

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-525 следующий свободный номер — **526**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`observation_row_label` mapped known WORLD keys to board words.
Anything else, including `weather.temp` with a source, became the
dotted key. That is a schema token, not a Система row. PIXEL-PATH
does not invent weather.

## Решение

1. Known keys keep their Russian labels.
2. An unmapped key is omitted. No new observation name.
3. `locality.weather.*` stays the ADR-475 rows. Empty source still
   drops the row.

## Откат

Restore `other => other.to_string()`.

## Ссылки

- ADR-247, ADR-475, ADR-525
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
