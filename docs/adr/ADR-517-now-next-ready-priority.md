# ADR-517: NOW «Далее» uses WORK-05 ready priority

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-516 следующий свободный номер — **517**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-507 sorts ready Tasks High → Normal → Low, then UUID, inside
`saai-taskd`. NOW `next_ready_task` still took the first Pending in
list order (newest `updated_at`). «Далее» could name a Normal Task
while a High one waited.

## Решение

1. `next_ready_task` uses the same `priority` rule: only `low` /
   `high`. Missing, `normal`, and unknown (including spoofed
   `critical`) are Normal.
2. Among derived-ready Tasks, pick High, then Normal, then Low, then
   smaller UUID.
3. A pending Action still beats any ready Task (WORK-08).
4. No `saai-taskd` dependency in the shell. No preemption.

## Откат

Restore first-match `find` on Pending + ready deps.

## Ссылки

- ADR-507, ADR-241
- `docs/os/sprints/WORK-ROADMAP.md` WORK-05 / WORK-08
