# ADR-283: Memory mutation goes through PolicyEngine

## Статус

Принято, 2026-09-21. MEM-07 host. Not Visual v1 sign-off. PIN stays
null. Do not flash `saaios-runtime`.

## Нумерация

После ADR-282 следующий свободный номер — **283**. Не S33.

## Контекст

MEM-03/06 typed record and erase. Console `{"op":"memory_remember"}`
and `memory_forget` still wrote the store with no Principal.
`decide_request` would Allow a Worker on a Medium tool. Memory
mutation is not metrics.

## Decision

1. **Mutations only.** `memory.remember`, `memory.forget`,
   `memory.invalidate`, `memory.erase`. Recall/tail/status stay reads.
2. **Owner JSON is the user channel.** LocalUser + LocalSystemSurface
   Allow. The TCP console is still attributed as owner (no SO_PEERCRED
   on this socket).
3. **Worker needs an envelope.** No envelope is Deny, not AskUser.
   Automation and Application Deny.
4. **Value is not policy input.** Arguments are key + scope. The
   secret stays in the store write.
5. **No flash.** Host tests. Panther runtime still writes without
   this gate until a service week.

## Consequences

- Model still has no remember/forget tools (MEM-05).
- MEM-08 chrome is still status `memory_records`, not a second writer.
- Rollback: drop `decide_memory_mutation` and the two runtime checks.

## Verification

Host: `cargo test -p policy-engine -p ai-runtime --offline` including
`owner_memory_remember_is_allowed`,
`worker_cannot_remember_without_envelope`,
`automation_cannot_erase_memory`. No panther flash.
