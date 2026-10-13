# ADR-553: locality network fail is «сеть»

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-551 (`e1205a3e…`).

## Нумерация

После ADR-552 следующий свободный номер — **553**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

A failed fetch printed `сеть: {reqwest}` and `Open-Meteo HTTP` /
`JSON` helper text on the locality row. «сеть» and «нет данных»
already name those misses.

## Решение

1. Transport fail is «сеть».
2. Bad Open-Meteo body is `Open-Meteo · нет данных` (ADR-552).
3. Do not invent weather. Do not mark weather Готово.

## Откат

Restore `сеть: {error}` and the HTTP/JSON helper strings.

## Ссылки

- ADR-552
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-09
