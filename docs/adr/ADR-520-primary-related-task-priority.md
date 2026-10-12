# ADR-520: primary related Task uses WORK-05 priority at the same rank

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-519 следующий свободный номер — **520**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Intent Object View, NOW ObjectSummary trailing, and Intent summary
pick `primary_related_task` by status visibility rank only. Two
waiting-confirmation Tasks of the same Intent could trail the Normal
one. Plan step order stays parent-before-child (ADR-239).

## Решение

1. After visibility rank, prefer High → Normal → Low, then UUID.
2. A Running Normal Task still beats a waiting High Task.
3. `plan_steps_for_intent` is unchanged.

## Откат

Restore `sort_by_key(task_visibility_rank)` only.

## Ссылки

- ADR-239, ADR-507, ADR-517
- `docs/os/sprints/WORK-ROADMAP.md` WORK-05 / WORK-08
