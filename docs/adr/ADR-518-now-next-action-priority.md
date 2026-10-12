# ADR-518: NOW pending Action follows the owner Task priority

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-517 следующий свободный номер — **518**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-517 ordered ready Tasks High → Normal → Low. «Далее» still prefers
any pending Action over a ready Task, but `next_pending_action` took
the first Action in list order. A Normal Task's Action could hide a
High Task's Action.

## Решение

1. Among pending Actions, pick the one whose `task_id` Task has the
   higher WORK-05 priority. Missing owner is Normal.
2. Same UUID tie-break as ADR-517.
3. A pending Action still beats any ready Task.

## Откат

Restore first-match `find` on pending Actions.

## Ссылки

- ADR-507, ADR-517
- `docs/os/sprints/WORK-ROADMAP.md` WORK-05 / WORK-08
