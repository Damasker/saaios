# ADR-304: Sistema Записи omit Restricted even if status leaks it

## Статус

Принято, 2026-09-21. MEM-08 chrome. Not Visual v1 sign-off.
PIN stays null. Do not flash shell or runtime.

## Нумерация

После ADR-303 следующий свободный номер — **304**. Не S33.

## Контекст

ADR-264/268: shell-legal memory is `status.memory_records`, Global
only. Runtime already drops `MemorySensitivity::Restricted` and does
not serialize `sensitivity`. Sistema still painted every Global row
the blob contained. A leaked Restricted pin/secret would show on
«Записи». MEM-04 says Restricted stays off compact views.

## Decision

1. **Same channel.** No JSONL. No `MemoryRecall`.
2. **Filter.** `live_memory_records_from_status_json` omits
   `sensitivity=restricted` (case-insensitive). Missing sensitivity
   stays Normal, matching the current DTO.
3. **No tap.** Forget/erase still absent. Empty list still omits
   «Записи».
4. **Host only.** Panther paint waits the next shell experiment.

## Consequences

- Restricted cannot appear on Система even if a future status row
  includes it.
- Rollback: drop the sensitivity check.

## Verification

Host: `cargo test -p saai-shell --offline live_memory_records`. No
panther flash. Leave Сейчас.
