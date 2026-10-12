# ADR-504: WORK-04 read-only bounded parallelism

## Статус

Принято, 2026-10-12. Host-only in `saai-taskd` scheduler. Not a phone
flash. Not WORK-09 budgets. Lock SIGTRAP stays open.

## Нумерация

После ADR-503 следующий свободный номер — **504**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

WORK-02 keeps mutating concurrency at 1. Read-only Actions (for example
`display.inspect`) do not need that lock, but the frontier treated every
Task as mutating. WORK-04 asks for measure-first then a bound. Host
lab is `mike@192.168.88.223` (`macpro`, Debian 13).

## Решение

1. Optional Task property `read_only: true` (boolean). Absent/false stays
   mutating.
2. Separate counters: `mutating_in_flight` and `read_only_in_flight`.
3. Bounds: `MAX_MUTATING_IN_FLIGHT = 1`, `MAX_READ_ONLY_IN_FLIGHT = 2`.
4. `admit_ready` fills read-only slots first, then one mutating slot.
5. `measure_parallelism` reports ready counts and free slots so host lab
   can raise the read-only bound only after evidence (WORK-09 on Pixel).

Live `dispatch_ready` walks `admit_ready` one id at a time; starting a
read-only Task no longer blocks another read-only admission in the same
loop once the first leaves Pending.

## Откат

Drop `read_only` handling; restore admit via mutating frontier only.

## Ссылки

- `docs/os/sprints/WORK-ROADMAP.md` WORK-04
- ADR-121, ADR-237
