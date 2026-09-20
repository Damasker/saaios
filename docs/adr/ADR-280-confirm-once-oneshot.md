# ADR-280: Confirm Once is a OneShot grant, not AskUser fallthrough

## Статус

Принято, 2026-09-21. Host. Not Visual v1 sign-off. PIN stays null.
Phone=no this slice (do not flash runtime: boot-only respawn,
reboot drops dest-no-lock).

## Нумерация

После ADR-279 следующий свободный номер — **280**. Не S33.

## Контекст

`confirm` consumed `PendingConfirmation` then called `decide`.
Session scope wrote `grant_session` so decide Allowed. Once scope
wrote nothing, so High-risk tools stayed AskUser, and the runtime
executed anyway because it only aborted on Deny. AUTH-03 OneShot
existed and was unused on this path.

## Decision

1. **Once → OneShot.** `PolicyEngine::grant_once` writes owner +
   Any + OneShot. `decide` Allows once, then Asks again.
2. **Session unchanged.** `grant_session` stays Session validity.
3. **Execute requires Allow.** AskUser after confirm is an error,
   not a silent run.
4. **Not a worker envelope.** Runtime still executes in-process as
   the owner. AUTH-06 envelopes stay for a worker Principal.
5. **Host-only.** Do not flash `saaios-runtime` this week.

## Consequences

- Diagnose confirm Once is an honest grant, not a bypass.
- AUTH-10 still owns the panther confirm/reboot/SSH pass.
- Rollback: Session-only `grant_session`; Deny-only abort.

## Verification

Host: `cargo test -p policy-engine -p ai-runtime` including
`grant_once_allows_then_asks`. No panther flash. No reboot.
