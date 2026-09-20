# ADR-244: Orb states follow workflow, not voice

## Статус

Принято, 2026-09-20. Orb chrome paint; panther flash followed
host-green. PIN stays null. Do not tap Orb Изменить.
Not Visual v1 sign-off.

## Нумерация

После ADR-243 следующий свободный номер — **244**. Не S33.

## Контекст

Orb already mapped Offline, Attention, Running, menu Active, Idle.
The boards' Orb is workflow: idle, confirm, run, plan, result.
Слушает is a visual destination; Pixel voice stays blocked
(ADR-092). Goal wave B, one screen. Do not invent listen, graphs,
or diagnosing without a shell-visible source.

## Decision

1. **Priority.** Offline → Attention (`WaitingConfirmation` /
   undismissed notification) → Failed → Running → Waiting
   (pending Action or derived-ready Task) → Complete (verified
   Result without error) → Active (menu) → Idle.
2. **Voice.** No Слушает state. Long-press listen stays closed.
3. **IRAB diagnose-in-flight** is not an Orb state until an entity
   fact exists. Plan is Waiting from live next work, not a fake
   «Анализирует».

## Consequences

Visual v1 still unsigned. Thin tuning (ADR-213) stays last.
Rollback: restore Attention/Running/Active/Idle-only Orb.

## Verification

Host: `cargo test -p saai-ui-compiler -p saai-ui-core -p saai-shell --offline -- --test-threads=1`
47+81+273 passed, including
`orb_visual_state_maps_failed_plan_and_result_not_voice`.
Panther: shell `6ca55bfd…` pid 1719. Leave Сейчас; do not tap
Изменить, Spaces, Поиск, Inbox. Marker on.
