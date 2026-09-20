# ADR-268: Sistema reviews Global memory from status, not JSONL

## Статус

Принято, 2026-09-21. MEM-08 chrome on host. Not a panther flash.
Not Visual v1 sign-off. PIN stays null. Do not open JSONL. Do not
call `MemoryRecall` as a second client.

## Нумерация

После ADR-267 следующий свободный номер — **268**. Не S33.

## Контекст

ADR-264 put Global live records on `status.memory_records`. ADR-126
omitted Memory on Система until a legal read existed. The same
`{"op":"status"}` channel already feeds Observation rows (ADR-247).
This week's shell flash is already spent; chrome can still be
host-tested.

## Decision

1. **Same request.** `read_runtime_live_facts()` parses `observations`
   and `memory_records` from one status blob.
2. **Section `Записи`.** Only Global rows (`space=global`). Readout,
   no forget/erase tap. Empty list omits the section. Not Observation
   (`Наблюдения` stays a different section). Title is not `Память`
   because that label is RAM usage.
3. **Still forbidden.** No `open()` of the JSONL. No `MemoryRecall`.
   Restricted records stay off this channel (runtime already omits them).
4. **Panther later.** Do not flash `saai-shell` this week.

## Consequences

- Manual review exists in host tests. Phone paint waits for the next
  shell experiment.
- Rollback: drop `memory_records` from `MeFacts` and the `Записи`
  insert.

## Verification

Host: `live_memory_records_from_status_json_keep_only_global` and
`me_system_sections_show_global_memory_records`. No panther flash.
Leave Сейчас. Do not tap Система.
