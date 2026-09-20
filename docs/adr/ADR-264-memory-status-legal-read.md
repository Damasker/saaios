# ADR-264: Shell-legal memory read is status `memory_records`

## Статус

Принято, 2026-09-20. MEM-08 legal read on host. Chrome not painted.
Not Visual v1 sign-off. PIN stays null. Phone chrome waits for a
shell experiment. Do not open JSONL from `saai-shell`.

## Нумерация

После ADR-263 следующий свободный номер — **264**. Не S33.

## Контекст

ADR-126 omitted Memory on Система: shell must not become a second
JSONL client and must not invent a probe. Console already has
`MemoryRecall` on the runtime socket. ADR-247 already lets the shell
read `status` for Fresh Observation when the runtime binary exists.
Memory is not Observation (ADR-125).

## Decision

1. **Same channel, other field.** `RuntimeStatusDto.memory_records`
   lists live Global records (`MemoryAccessScope::Global`). Status has
   no space, so this is not All (MEM-02).
2. **Not Observation.** Separate DTO: key, value, `space=global`,
   kind. Restricted records are omitted.
3. **Chrome later.** ADR-268 paints `Записи` from this field on host.
   Panther paint still waits for a shell experiment.
4. **Still forbidden.** Shell must not `open()` the JSONL. Shell must
   not call `MemoryRecall` as a second client.

## Consequences

- A legal read exists. The review surface does not.
- Rollback: drop `memory_records` from status.

## Verification

Host: `cargo test -p memory-store -p saaios-runtime` including
`global_review_omits_space_records` and
`status_lists_only_global_memory_records`. No panther flash. Leave
Сейчас. Do not tap Система.
