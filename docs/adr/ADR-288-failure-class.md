# ADR-288: FailureClass — retry is not replan

## Статус

Принято, 2026-09-21. WORK-06 host. Not a ReplanRequest. PIN stays null.
Do not flash taskd.

## Нумерация

После ADR-287 следующий свободный номер — **288**. Не S33.

## Контекст

ADR-121: Retry ≠ Replan. Unknown idempotency → no auto-retry. ADR-236
marks timeout/unreachable Failed as `retryable` and waits for
`retry_requested`. WORK-03 writes Failed on Observation mismatch
without a class. A boolean `retryable` cannot tell timeout from
verification mismatch from malformed JSON.

## Decision

1. **`FailureClass`.** `timeout` | `unreachable` | `malformed` |
   `verification_mismatch` | `unknown`. Stored as `error_kind`.
2. **Retry.** Only `timeout` and `unreachable` set `retryable`.
   Still requires `retry_requested`. No auto-retry of mutating work.
3. **Not retry.** `malformed`, `verification_mismatch`, `unknown`.
   Mismatch is Failed for WORK-07 Replan, not a sibling retry.
   Legacy ADR-236 rows with `retryable` and no `error_kind` stay
   retryable.
4. **Bridge.** `Timeout` → timeout, `Connect` → unreachable,
   `Json` → malformed, `Io` → unknown.

## Consequences

- Object View retry chrome still waits for `retry_requested`. Do not
  flash taskd. Do not invent ReplanRequest (WORK-07).
- Rollback: drop `FailureClass`; restore boolean+optional kind strings.

## Verification

Host: `cargo test -p saai-taskd --offline`. No panther flash.
