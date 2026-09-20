# ADR-284: Runtime JSON can erase one memory identity

## Статус

Принято, 2026-09-21. MEM-06 wire. Not Visual v1 sign-off. PIN stays
null. Do not flash `saaios-runtime`.

## Нумерация

После ADR-283 следующий свободный номер — **284**. Не S33.

## Контекст

ADR-263 implemented `MemoryStore::erase` (rewrite, fsync, rename).
ADR-283 gated `memory.erase` in PolicyEngine. The live JSON protocol
only had `memory_forget`, which leaves the value on disk. A user
could not erase without AI and without reading JSONL.

## Decision

1. **`{"op":"memory_erase"}`.** Same key/space/global shape as
   forget. All-scopes still rejected.
2. **UAM first.** `allow_memory_mutation("memory.erase")` then
   `store.erase`. Owner JSON Allow. Worker needs an envelope.
3. **Forget stays invalidate.** `/forget` is not `/erase`.
4. **Console.** `console-tui` `/erase --global key` (or `--space`).
5. **No flash.** Host tests. Panther runtime has no this op until
   a service week.

## Consequences

- Status `memory_records` will omit an erased Global key after the
  next status read. Shell chrome still waits for a shell experiment.
- Rollback: drop `MemoryErase` and `/erase`.

## Verification

Host: `cargo test -p saaios-runtime --offline` including
`memory_erase_op_parses`, `memory_erase_removes_the_value_from_disk`,
`memory_erase_without_scope_is_rejected`. No panther flash.
