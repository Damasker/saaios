# ADR-263: Invalidate keeps history; erase rewrites the file

## Статус

Принято, 2026-09-20. MEM-06 host. Not Visual v1 sign-off. PIN stays
null. Forget is not physical removal. Phone=no this slice.

## Нумерация

После ADR-262 следующий свободный номер — **263**. Не S33.

## Контекст

ADR-125: forget must have honest storage semantics. `forget` appended
a tombstone that still contained the value. A reader of
`saaios-memory.jsonl` could recover it. Physical erase was named
MEM-06, not a silent compact.

## Decision

1. **Invalidate.** `forget` / `invalidate` append an `Invalidated`
   record for one `(space_id, key)`. Recall skips it. The previous
   value stays on disk. All-scopes invalidate is rejected.
2. **Erase.** `erase` rewrites the JSONL without any line for that
   identity, including tombstones, then `fsync` + rename. The value
   is gone. All-scopes erase is rejected. Another space's same key
   stays.
3. **Atomic.** Write a sibling temp file, replace, reopen the append
   handle. No second store.

## Consequences

- MEM-08 can offer Invalidate vs Erase without a new daemon.
- Rollback: drop `erase`; keep tombstone `forget`.

## Verification

Host: `cargo test -p memory-store` including
`invalidate_keeps_the_value_on_disk`,
`erase_removes_the_value_from_disk`,
`erase_does_not_remove_another_spaces_key`. No panther flash.
