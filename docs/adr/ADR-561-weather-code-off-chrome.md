# ADR-561: weather code stays off chrome

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-560 (`72cba91c…`).

## Нумерация

После ADR-560 следующий свободный номер — **561**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Sistema printed `locality.weather.code` as «Код» plus the numeric
code. «Погода» already names that observation. The number is a
helper, not a board word. The widget still reads the code.

## Решение

1. Observation `locality.weather.code` has no row.
2. The fact is still read. Wire key stays. «Погода» stays.
3. Do not invent weather. Do not mark weather Готово.

## Откат

Restore the «Код» row.

## Ссылки

- ADR-555, ADR-559, ADR-560
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-09
