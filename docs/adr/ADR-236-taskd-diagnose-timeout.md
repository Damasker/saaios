# ADR-236: diagnose timeout is Failed + retryable, not Pending

## Статус

Принято, 2026-09-20. ADR-233's live Work intent became Failed after
`saaios-runtime` hit its 60s budget. Hung TCP had no client timeout, so
a silent drop would leave the Task `Pending` and block the daemon loop.
Retry of mutating Actions stays forbidden (ADR-121). PIN stays null.
Not Visual v1 sign-off. WORK-02 stays host-only.

## Нумерация

После ADR-235 следующий свободный номер — **236**. Не S33.

## Контекст

`runtime_bridge::call` waited on `read_to_end` with no budget. When
runtime answered `ok:false` ("request timed out after 60s") the Task
already became Failed. When the socket hung, the Task stayed Pending
and `run()` could not follow selection or other intents. Failed is
terminal: there was no retry hook, so Object View could only show
«Ошибка». Goal wave A3: honest Failed + повтор, not an empty Pending.

## Decision

1. **Client budget.** `DIAGNOSE_TIMEOUT` is 65s — slightly above
   panther's 60s runtime budget so a live timeout still arrives as
   `ok:false`. Hung TCP becomes `BridgeError::Timeout` and fails the
   Task.
2. **Classify.** Timeout and connect failures set `retryable: true`
   and `error_kind` (`timeout` / `unreachable`). Malformed JSON is
   not retryable. Runtime's own "timed out after" string is the same
   timeout class.
3. **Retry is a sibling Task.** `retry_requested: true` on a
   retryable Failed Task creates a new Task on the same Intent.
   Failed stays Failed. No `Failed → Pending`. No auto-retry of
   diagnose or of confirmed Actions. Object View «Повторить» is wave
   B; this slice only owns the flag.

## Consequences

- A hung diagnose cannot stall the watch. Rollback: drop the timeout
  and `retry_requested` handler. WORK-02 live dispatch stays A4.
  Still not Visual v1 sign-off.

## Verification

Host: `cargo test -p saai-taskd` including
`diagnose_times_out_instead_of_hanging` and
`timeout_failed_task_retries_only_when_requested`. Panther: flash
`saai-taskd`; do not type an Intent; do not tap Повторить/Разрешить.
Marker on.
