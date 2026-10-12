# ADR-522: unknown OAM reasons are «недоступно»

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-521 следующий свободный номер — **522**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`explain_oam_reason` mapped three live bind failures to Russian.
Anything else, including Policy Deny strings, was printed in English.
PIXEL-PATH does not invent OAM labels; a spoofed reason is not a new
verb.

## Решение

1. Keep «нет инструмента», «нет данных объекта», «нет пространства».
2. Any other reason is «недоступно». No new Policy taxonomy on the
   Object View row.
3. Tap still does not execute the tool.

## Откат

Restore `reason.to_string()` for the fallback.

## Ссылки

- ADR-477, ADR-511, ADR-519
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-07
