# ADR-559: Observation source is named once

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-557 (`126b72c8…`).

## Нумерация

После ADR-558 следующий свободный номер — **559**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-247 keeps the Observation source on the Sistema line. ADR-555
maps `open-meteo.forecast` to Open-Meteo. Attribution already is
`Open-Meteo (CC BY 4.0)`, so the line printed the name twice.

## Решение

1. If the value already contains the chrome source, print the value.
2. Other rows still append the source. The wire id stays.
3. Do not invent weather. Camp / procfs sources stay.

## Откат

Always append ` · {source}` again.

## Ссылки

- ADR-247, ADR-555, ADR-557, ADR-558
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-09
