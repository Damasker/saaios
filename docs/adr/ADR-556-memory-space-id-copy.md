# ADR-556: memory space id is the space name

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512). Live shell is still
ADR-554 (`181cffbf…`).

## Нумерация

После ADR-555 следующий свободный номер — **556**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

«Записи» printed `home` / `work` as the value. Those are space ids.
`space_display_name` already names them «Дом» / «Работа». The record
key stays. Kind stays off chrome (ADR-527).

## Решение

1. Memory values that are known space ids use `space_display_name`.
2. Other values stay. The key stays. Restricted / hypothesis stay off.
3. Do not invent a memory taxonomy.

## Откат

Print the raw space id again.

## Ссылки

- ADR-268, ADR-527, ADR-555
- `docs/os/sprints/MEM-ROADMAP.md` MEM-08
