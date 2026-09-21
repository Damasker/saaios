# ADR-296: Failed status names FailureClass, not one Ошибка

## Статус

Принято, 2026-09-21. WORK-06 chrome. Not a Retry button. PIN stays null.
Do not flash shell or taskd.

## Нумерация

После ADR-295 следующий свободный номер — **296**. Не S33.

## Контекст

ADR-288 stores `error_kind` on Failed Tasks. Retry ≠ Replan. The
shell painted every Failed as «Ошибка», so timeout looked the same
as verification mismatch. Object View still has no `retry_requested`
control — that wait stands. Chrome can still tell the classes apart
from properties that already exist.

## Decision

1. **Copy from `error_kind`.** `timeout` → «Таймаут». `unreachable`
   → «Нет связи». `verification_mismatch` → «Не подтвердилось».
   `malformed` → «Ошибка формата». `unknown` / missing → «Ошибка».
2. **Legacy.** ADR-236 rows with `retryable` and no `error_kind`
   show «Таймаут».
3. **No new tap.** Do not add Повторить. `retry_requested` stays
   taskd. Do not flash.

## Consequences

- NOW / Intent / Object View use the same `task_status_text`.
- Rollback: restore the single «Ошибка» arm.

## Verification

Host: `cargo test -p saai-shell --offline failed_status_text`. No panther flash.
Leave Сейчас.
