# ADR-121: Work Scheduler v2 — DAG, verification, budgets inside saai-taskd

## Статус

Принято, 2026-09-18. WORK-00 (этот ADR + mapping/roadmap) — docs.
WORK-01 — host-only DAG validation в `saai-taskd`, без phone artifact.
Pixel dispatch, parallelism > 1, UI/NOW не заявлены.

## Нумерация

На `feat/som-v1` после ADR-120 следующий свободный номер — **121**.
Не S33: Visual Language остаётся активным hardware-changing треком.

## Контекст

Sprint 0 mapping уже зафиксировал:

- Planner ≠ Scheduler;
- `saai-taskd` владеет workflow;
- `saai-entity-store` — единственный authoritative store;
- Ready — derived, не очередь;
- audit-log — Task Ledger;
- Supervisor живёт внутри `saai-taskd`;
- DAG/fan-out/preemption — backlog.

SOM/OAM/IRAB (ADR-118–120) закрыли nouns, verbs и intent resolution.
IRAB может вернуть `Plan`. Недостающий слой: как многошаговый Plan
становится проверяемым DAG с bounded workers, не становясь agent swarm.

## Решение

**Work Scheduler v2 (WSV2)** расширяет `saai-taskd`. Не daemon, не
вторая queue, не persistent worker personalities, не второй PolicyEngine.

```text
IRAB Plan → Planner PlanProposal → validate DAG → persist Tasks
       → derive Ready Set → frontier (in-memory, default mutating=1)
       → Worker → OAM → Policy → Action → Verification → Result
       → VerificationFailed → ReplanRequest → Planner
```

Инварианты:

1. Proposal — untrusted structured suggestion, не prose.
2. Cycle / missing dep / duplicate id / over-budget graph — reject целиком.
3. `Ready` и `WaitingDependency` не становятся durable status.
4. Plan v1 — projection Intent+Tasks+edges, не обязательный `saaios.plan`.
5. Semantic `action_id` + `ObjectRef`, не raw tool names.
6. Plan approval не отменяет per-action Policy/confirmation.
7. Worker не пишет children в store; decomposition только через Planner.
8. Verifier ≠ Supervisor; Supervisor не перепланирует.
9. Retry ≠ Replan. Unknown idempotency → no auto-retry.
10. После reboot: rebuild graph from store; `WaitingConfirmation` не
    авто-исполняется.

Временные `depends_on_task_ids` в properties допустимы, пока SOM
relationship `saaios.depends-on` не станет единственным форматом.

## Roadmap

Отдельный track `WORK-*` (`docs/os/sprints/WORK-ROADMAP.md`), не смешивать
с VUI и не нумеровать как S33. Phone/UI slices ждут очереди hardware
experiment. Host DAG/ready logic может идти параллельно.

## Что не делается в этом ADR

Нет Redis/SQLite очередей, нет multi-agent pool, нет SIGKILL preemption,
нет bulk «allow all 12 actions», нет distributed workers.

## Ссылки

- `docs/os/architecture/cognitive-core-work-scheduler-mapping.md`
- ADR-006, ADR-030, ADR-032, ADR-033, ADR-118, ADR-119, ADR-120
- `docs/os/sprints/WORK-ROADMAP.md`
