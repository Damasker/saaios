# ADR-275: OAM/IRAB pass Principal on AuthorityRequest

## Статус

Принято, 2026-09-21. AUTH-05 host. Not Visual v1 sign-off. PIN stays
null. Phone=no this slice.

## Нумерация

После ADR-274 следующий свободный номер — **275**. Не S33.

## Контекст

AUTH-02 `decide_request` only used scoped Principal when
`ToolSpec.name` equalled the request operation. OAM `preflight` /
`execute_if_allowed` still called `decide_named`, which hardcodes
owner. Semantic action `display.inspect` and bound tool
`system.identity` are different strings, so a real AuthorityRequest
would fall through and lose Principal/target.

## Decision

1. **Operation is semantic.** `AuthorityRequest` carries the OAM/IRAB
   action id. `ToolSpec` is the bound tool's risk metadata even when
   the names differ.
2. **OAM builds the request.** `authority_request` names Principal,
   proof, semantic action, target, and arguments. Shell Object View
   keeps `preflight()` as LocalUser + LocalSystemSurface.
3. **IRAB does not evaluate policy.** Direct
   `ActionResolution::authority_request` builds the same shape.
   Worker Principal does not become owner.
4. **Grants.** A session grant may match the semantic action or the
   bound tool name. Worker still does not inherit the owner's grant.

## Consequences

- Unverified remains Deny. Hard-denied tools still Deny from ToolSpec.
- AUTH-07 Automation Principal is still later.
- Rollback: restore `decide_named` in OAM and name-equal spec match.

## Verification

Host: `cargo test -p saai-object-actions -p intent-resolution -p policy-engine`
including `preflight_request_is_semantic_action_not_tool_name`,
`unverified_principal_is_denied`, `worker_does_not_inherit_owner_grant`,
`direct_action_request_names_local_user_and_target`. No panther flash.
