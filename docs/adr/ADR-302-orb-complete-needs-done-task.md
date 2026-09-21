# ADR-302: Orb Complete requires a Done Task, not a Result entity

## Статус

Принято, 2026-09-21. WORK-03 chrome. Not Visual v1 sign-off.
PIN stays null. Do not flash shell.

## Нумерация

После ADR-301 следующий свободный номер — **302**. Не S33.

## Контекст

ADR-295 stopped Complete while a Verifying Task is in the same
entity list (in-progress wins). Orb still treated any `saaios.result`
without `error` as verified, including an orphan Result whose Task
is missing or still open. That is worker ok as Orb Complete.

## Decision

1. **Verified.** `orb_verified_result` is true only when the Result
   has no error **and** its `task_id` is a `saaios.task` in the same
   list with status `done`.
2. **Orphan.** A Result without that Task does not light Complete.
   Menu-open then stays Active; otherwise Idle.
3. **In-flight.** Running/Verifying still wins via `in_progress_work`
   when the Task is present.
4. **Host only.** Panther paint waits the next shell experiment.

## Consequences

- Orb Complete is verification, not a Result row.
- Rollback: restore “any Result without error”.

## Verification

Host: `cargo test -p saai-shell --offline orb_visual_state_maps_failed`.
No panther flash. Leave Сейчас.
