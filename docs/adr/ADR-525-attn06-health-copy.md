# ADR-525: NOW Health does not print schema tokens

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-524 следующий свободный номер — **525**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ATTN-06 already puts Degraded on NOW and Unhealthy on NOW+Orb.
The crate keeps English domain summaries (`degraded` / `unhealthy`)
and `component_id` as `system.cpu.sampler`. Chrome printed both.
That is a schema token, not a board label. Presentation lives in
the shell.

Система already names the same sampler observation «Процессор».

## Решение

1. `system.cpu.sampler` is «Процессор». Any other component id is
   «Наблюдения», not the dotted token.
2. `unhealthy` is «Ошибка». `degraded` and any other summary stay
   empty. No new Health taxonomy.
3. The crate tokens stay as they are. Inbox still omits Health.
   Tap still does not follow Health.

## Откат

Restore `item.title` / `item.summary` on the NOW Health row.

## Ссылки

- ADR-291, ADR-293, ADR-522
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-06
