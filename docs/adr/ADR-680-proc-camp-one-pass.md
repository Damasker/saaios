# ADR-680: one `/proc` walk for Sistema camp presence

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `60fe8f16…` (ADR-679). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-679 следующий свободный номер — **680**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-667 stopped walking `/proc` per cellular field. `me_facts()`
still called `camp_owner_running`, `read_endpoint_holder`, and
`status_query_running` separately — three full `/proc` cmdline
scans on the live-probe turn. dest-no-lock stays off. Do not
start the shell from COM13. Do not open modem devices.

## Решение

1. `proc_camp_presence_from_rows` classifies owner, status-query,
   and endpoint from one row list. The live walk reads each
   `/proc/<pid>/cmdline` once.
2. `me_facts` uses that snapshot. Endpoint still comes from fd
   symlink names only.
3. Host test: owner + `sit-sim-status` + endpoint in one pass.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Revert `proc_camp_presence`. Live `60fe8f16…` still walks
`/proc` three times on the live-probe turn.

## Ссылки

- ADR-667, ADR-676, ADR-679, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
