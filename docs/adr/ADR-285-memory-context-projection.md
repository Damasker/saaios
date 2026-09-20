# ADR-285: MemoryContextProjection labels kind and drops Restricted

## Статус

Принято, 2026-09-21. MEM-04 host. Not Visual v1 sign-off. PIN stays
null. Do not flash runtime or shell.

## Нумерация

После ADR-284 следующий свободный номер — **285**. Не S33.

## Контекст

ADR-125 named `MemoryContextProjection` with kind/scope labels.
`format_context` still printed `[space] key: value` and treated
Restricted like Normal. Restricted must not go to a remote model
without policy. Status chrome already dropped Restricted (ADR-264);
recall and the model prompt did not.

## Decision

1. **Projection.** `MemoryContextProjection` / `MemoryContextRow`
   (`space`, `kind`, `key`, `value`). `format_context` renders
   `[space kind] key: value` inside the same `<memory_records>`
   wrapper. Still data, not "Known facts".
2. **Sensitivity.** Compact views (`latest_visible_records`, recall,
   list_recent, projection) omit `Restricted`. JSONL and
   `read_all_records` keep them for erase.
3. **No flash.** Host tests. Shell «Записи» already shows `kind`;
   panther paint still waits.

## Consequences

- A Restricted pin/secret is not in the model prompt or `memory.recall`.
- Preference is labelled `explicit_preference`.
- Rollback: restore the unlabelled `format_context` loop.

## Verification

Host: `cargo test -p memory-store --offline` including
`format_context_labels_kind`,
`restricted_records_are_not_projected_or_recalled`. No panther flash.
