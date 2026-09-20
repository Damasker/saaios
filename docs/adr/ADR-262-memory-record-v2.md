# ADR-262: MemoryRecord v2, legacy JSONL is a view

## Статус

Принято, 2026-09-20. MEM-03 host. Not Visual v1 sign-off. PIN stays
null. No `saai-memoryd`. Memory stays JSONL on Platform Track.
Phone=no this slice.

## Нумерация

После ADR-261 следующий свободный номер — **262**. Не S33.

## Контекст

ADR-125 named the typed record: kind, scope, provenance, state,
sensitivity, validity. The store still wrote flat `MemoryFact`.
`source` was caller-supplied, so a write could claim `"model"`.
Legacy files on disk must still recall.

## Decision

1. **Write v2.** New lines are `MemoryRecord` (`schema: 2`) with
   `ExplicitFact` / `ExplicitPreference`. `LearnedHypothesis` is
   rejected on remember (MEM-09).
2. **Read legacy.** A line without v2 fields parses as `MemoryFact`
   and becomes an ExplicitFact + `LegacyImport` view. `deleted`
   maps to `Invalidated`.
3. **Provenance is assigned.** `MemoryProvenance::assign` sets actor
   `Store` and the request correlation id. Caller `source` is not
   stored as authority.
4. **Fact remains the wire view.** `as_fact()` keeps runtime
   `memory_facts` stable.

## Consequences

- Existing `remember` / `recall` callers keep compiling.
- Rollback: write `MemoryFact` again; leave v2 lines unread.

## Verification

Host: `cargo test -p memory-store` including
`remember_assigns_store_provenance_not_caller_source`,
`legacy_jsonl_still_recalls`,
`learned_hypothesis_cannot_be_remembered`. No panther flash.
