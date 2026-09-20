# ADR-277: Automation is not the local user

## Статус

Принято, 2026-09-21. AUTH-07 host. Not Visual v1 sign-off. PIN stays
null. Phone=no this slice. No scheduler/cron daemon.

## Нумерация

После ADR-276 следующий свободный номер — **277**. Не S33.

## Контекст

`PrincipalKind::Automation` existed as vocabulary. Nothing stopped a
caller from pairing it with `LocalSystemSurface` and looking like the
owner. AUTH-03 already keeps grants on PrincipalId; AUTH-07 makes the
proof match the kind.

## Decision

1. **Constructor.** `Principal::automation(id)` is `automation:{id}` +
   `Automation`.
2. **Proof match.** `proof_matches_principal`: LocalUser uses
   LocalSystemSurface (or a key); Worker uses DelegatedWorker;
   Automation uses InternalServiceBoundary. A mismatch is Deny.
3. **No owner inheritance.** An owner session grant does not cover an
   automation Principal. High-risk still AskUser until that principal
   has its own grant.
4. **No cron this slice.** Rules, timers, and `saai-taskd` wiring stay
   later. This is the identity, not a scheduler.

## Consequences

- A spoofed Automation + LocalSystemSurface is Deny even on low-risk
  tools.
- AUTH-09 revocation review is still later.
- Rollback: drop `proof_matches_principal` from `decide_request`.

## Verification

Host: `cargo test -p saai-authority -p policy-engine` including
`automation_is_not_local_user`, `automation_cannot_use_local_user_surface`,
`owner_grant_does_not_cover_automation`. No panther flash.
