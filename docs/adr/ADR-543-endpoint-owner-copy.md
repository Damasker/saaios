# ADR-543: cellular endpoint owner is «процесс»

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-542 следующий свободный номер — **543**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`endpoint_phrase` printed `camp` when the camp-owner holds the
modem descriptor. That is an English helper name. «процесс» already
names that owner on the cellular row.

## Решение

1. Endpoint `owner` is «процесс».
2. `modemd` / `shared` stay. Unknown tokens stay omitted.
3. No new modem taxonomy. Do not send RadioState.

## Откат

Restore `camp`.

## Ссылки

- ADR-531, ADR-541
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
