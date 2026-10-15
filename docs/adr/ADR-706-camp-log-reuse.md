# ADR-706: reuse streamed camp owner log; skip re-open

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `d7afc85b…` (ADR-705). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not log the raw camp file.

## Нумерация

После ADR-705 следующий свободный номер — **706**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-705 streams the owner log on the turn after `/proc`.
Recapture after that staircase dropped the hold and opened
the file again. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type PSK.

## Решение

1. `apply_me_camp_log_probes` fills from `camp_fact_log` when
   that string is already held. It does not open the file.
2. The log turn stores those lines on `me_camp_fact_log`.
   Recapture seeds facts from that cache before apply.
3. Host test: held `registration_raw` survives apply without
   a file. Do not flash this ADR. dest-no-lock stays off.

## Откат

Always call `read_camp_owner_log` from log probes. Live
`d7afc85b…` still re-opens the file on recapture.

## Ссылки

- ADR-704, ADR-705, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
