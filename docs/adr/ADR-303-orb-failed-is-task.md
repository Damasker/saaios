# ADR-303: Orb Failed is the Task's class, not Result.error

## Статус

Принято, 2026-09-21. WORK-03/06 chrome. Not Visual v1 sign-off.
PIN stays null. Do not flash shell.

## Нумерация

После ADR-302 следующий свободный номер — **303**. Не S33.

## Контекст

ADR-302 stopped Orb Complete from an orphan Result. Failed still
lit from `saaios.result.error` even while the Task was Verifying.
WORK-06 named failure on the Task (`error_kind`). A worker error
string is a claim, not the verified Failed class.

## Decision

1. **Orb.** `orb_failed_work` is Task `failed` only. Result.error
   does not light Failed.
2. **Result screen.** If the related Task is Failed, status is
   `failed_status_text` (Таймаут / Нет связи / …), not `Готово`
   and not the raw worker error.
3. **In-flight.** Verifying + Result.error stays Running on Orb
   (in-progress wins; Result.error is ignored).
4. **Host only.** Panther paint waits the next shell experiment.

## Consequences

- Failure class is one place: the Task.
- Rollback: restore Result.error as an Orb Failed source.

## Verification

Host: `cargo test -p saai-shell --offline orb_visual_state_maps_failed`
and `object_view_result_names_the_task_failure`. No panther flash.
Leave Сейчас.
