# ADR-552: locality empty forecast is «нет данных»

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-551 (`e1205a3e…`).

## Нумерация

После ADR-551 следующий свободный номер — **552**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

An empty Open-Meteo document printed `current` on the locality
row. That is an API field. «нет данных» already names a missing
observation. Open-Meteo stays the source.

## Решение

1. Empty forecast reason is `Open-Meteo · нет данных`.
2. Do not invent weather. Do not mark weather Готово.

## Откат

Restore `Open-Meteo не вернул current`.

## Ссылки

- ADR-475, ADR-476, ADR-551
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-09
