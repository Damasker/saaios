# ADR-261: Session grants are scoped records, not a tool-name set

## Статус

Принято, 2026-09-20. AUTH-03 host. Not Visual v1 sign-off. PIN stays
null. Session grants still die on reboot. Phone=no this slice.

## Нумерация

После ADR-260 следующий свободный номер — **261**. Не S33.

## Контекст

ADR-124: session grant was `HashSet<tool_name>` without Principal,
target, or TTL. AUTH-04 already made `decide_named` see live grants.
A grant for one object or one principal still covered every later
call of that tool.

## Decision

1. **Record.** `SessionGrant` is Principal + operation + TargetScope +
   SpaceScope + GrantValidity. Matching is `grant_covers`.
2. **Compat.** `grant_session(tool)` writes owner + Any + Session, so
   existing confirm → Allow still works.
3. **Hard deny wins.** `grant_scoped` refuses hard-denied tools.
   Persistent validity is not stored here (GrantStore / AUTH-08).
4. **OneShot** is consumed on Allow. Expired `Until` does not Allow.
   A worker does not inherit the owner's grant. Exact-object does not
   leak to another object.

## Consequences

- `session_grants()` still returns tool names for the console readout.
- Rollback: restore `HashSet<String>` in `PolicyEngine`.

## Verification

Host: `cargo test -p saai-authority -p policy-engine` including
`owner_grant_does_not_cover_worker`,
`exact_object_grant_does_not_cover_other_object`,
`oneshot_grant_is_consumed`, `expired_grant_does_not_allow`.
No panther flash. Do not tap Разрешить.
