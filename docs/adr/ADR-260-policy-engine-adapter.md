# ADR-260: PolicyEngine adapter keeps the same verdicts

## Статус

Принято, 2026-09-20. AUTH-02 host. Not Visual v1 sign-off. PIN stays
null. No new policyd. Phone=no this slice.

## Нумерация

После ADR-259 следующий свободный номер — **260**. Не S33.

## Контекст

AUTH-01 added Principal / AuthorityRequest / reason codes but nothing
called PolicyEngine with them. `decide` / `decide_named` still spoke
only tool name + ToolSpec. A second evaluator would drift.

## Decision

1. **One engine.** `PolicyEngine::decide_request` is the adapter.
   Unverified proof is Deny. Known `SemanticAction` with a matching
   ToolSpec uses the same Allow/AskUser/Deny path as `decide_named`.
2. **Same strings.** For a local-user request the verdict and reason
   match `decide_named` on the same spec and args.
3. **No policyd.** Portal, SSH `authorized_keys`, and appd GrantStore
   stay. AUTH-03 scoped grants live in this same process-local engine.

## Consequences

- OAM/IRAB can pass an AuthorityRequest without a second policy.
- Rollback: drop `decide_request`; `decide` / `decide_named` stay.

## Verification

Host: `cargo test -p saai-authority -p policy-engine` including
`adapter_matches_decide_named_verdicts` and
`unverified_request_is_denied`. No panther flash.
