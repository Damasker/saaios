# ADR-555: locality source on chrome is Open-Meteo

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-554 (`181cffbf…`).

## Нумерация

После ADR-554 следующий свободный номер — **555**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-247 keeps the Observation source on the Sistema line. Locality
rows printed the helper id `open-meteo.forecast`. The widget and
empty-forecast copy already name Open-Meteo. Camp / procfs sources
stay.

## Решение

1. Chrome source for `open-meteo.forecast` is `Open-Meteo`.
2. The wire id stays. Other sources stay.
3. Do not invent weather. Do not mark weather Готово.

## Откат

Print `open-meteo.forecast` on the line again.

## Ссылки

- ADR-247, ADR-552, ADR-553, ADR-554
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-09
