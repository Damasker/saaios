# ADR-528: unknown device_class is not a node label

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-527 следующий свободный номер — **528**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-249 / ADR-298 name two classes: `phone` and `computer`. Chrome
already says «Телефон» / «Компьютер». Any other `device_class`
was printed as the schema token.

## Решение

1. Live `status.device` accepts only `phone` and `computer`.
   Anything else falls back to the local phone-gate guess.
2. Paint maps unknown class to «Узел», not the raw token.
3. No third device class.

## Откат

Restore `other => other` and accept any non-empty class.

## Ссылки

- ADR-249, ADR-298, ADR-299
- `docs/os/sprints/PIXEL-PATH.md` D
