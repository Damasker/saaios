# ADR-301: Result is not Complete while the Task is still open

## Статус

Принято, 2026-09-21. WORK-03 chrome. Not Visual v1 sign-off.
PIN stays null. Do not flash shell.

## Нумерация

После ADR-300 следующий свободный номер — **301**. Не S33.

## Контекст

WORK-03 made Verifying durable: a worker Result is not Done.
ADR-295 stopped Orb Complete while the Task is Verifying. Object
View still named `Результат:` on the related path and painted the
Result screen `Готово` as soon as the Result entity existed, even
while the Task was `running` or `verifying`. That is the board
failure mode: worker ok as verified outcome.

## Decision

1. **Path.** `workflow_path_caption` omits Result while the related
   Task is `running` or `verifying`. Action and Intent still appear.
2. **Result screen.** If that Task is in-flight, Result uses the
   Task's UniversalState and status (`Выполняется` /
   `Проверяется`), not `Готово`. Worker `summary` may still show
   as observation — it is a claim, not verification.
3. **Done.** After Task `done`, Result is `Готово` and the path
   includes `Результат:`.
4. **Host only.** Panther paint waits the next shell experiment.

## Consequences

- Intent plan→progress→result does not skip verification.
- Rollback: restore the always-Complete Result facts and always
  append Result on the path.

## Verification

Host: `cargo test -p saai-shell --offline object_view_result`. No
panther flash. Leave Сейчас.
