# ADR-507: WORK-05 priority among Ready

## Статус

Принято, 2026-10-12. Host-only in `saai-taskd`. Not a phone flash.
Does not raise mutating concurrency. Lock SIGTRAP stays open.

## Нумерация

После ADR-506 следующий свободный номер — **507**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The ready frontier was UUID-stable. WORK-05 asks for priority
scheduling. ADR-121 forbids SIGKILL preemption and a second queue.

## Решение

1. Optional Task property `priority`: `low` | `high`. Missing,
   `normal`, and unknown values (including spoofed `critical`) are
   Normal.
2. `derive_ready_set` / `admit_ready` sort High → Normal → Low, then
   UUID. Bounds stay WORK-02/04.
3. A High Pending Task does not displace a Running or Verifying Task.

## Откат

Drop `priority` handling; restore UUID-only ready order.

## Ссылки

- `docs/os/sprints/WORK-ROADMAP.md` WORK-05
- ADR-121, ADR-237, ADR-504
