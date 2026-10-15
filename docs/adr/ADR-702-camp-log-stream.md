# ADR-702: stream the camp owner log; flatten uses held fact lines

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `64d39dde…` (ADR-701). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not log the raw camp file.

## Нумерация

После ADR-701 следующий свободный номер — **702**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The live owner log is several megabytes. `read_camp_owner_log`
loaded it into one `String`, then `cellular_row_with_session`
opened it again on flatten. A tail would drop early one-shot
lines. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices.

## Решение

1. `owner_fact_lines_from_lines` keeps the last matching line
   per field while streaming. `owner_fact_lines` still wraps
   a `&str`.
2. `read_camp_owner_log` uses `BufReader`. `apply_me_camp_values`
   stores that filtered text on `MeFacts.camp_fact_log`.
3. Flatten reads `camp_fact_log`, not the file. Host tests:
   held log survives apply; `cellular_row_with_session` has no
   `read_camp_owner_log` call. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Read the owner log with `read_to_string` and open it again
from flatten. Live `64d39dde…` still loads the whole file
twice on the camp turn.

## Ссылки

- ADR-682, ADR-701, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
