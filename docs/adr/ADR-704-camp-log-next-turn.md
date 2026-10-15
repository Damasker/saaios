# ADR-704: defer the camp owner log off the Sistema camp turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `f15c170f…` (ADR-703). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not log the raw camp file.

## Нумерация

После ADR-703 следующий свободный номер — **704**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-703 streams the owner log without a whole-file `String`.
The camp turn still reads that file before Me can paint
owner / CP. dest-no-lock stays off. Do not start the shell
from COM13. Do not open modem devices. Do not type PSK.

## Решение

1. `apply_me_camp_probes` fills `/proc` presence and CP
   sysfs only. Radio tokens stay empty on that turn.
2. `me_camp_log_pending` streams the owner log onto held
   facts before Wi-Fi/`df`. Idle stays blocked while that
   turn is pending. Flatten still uses `camp_fact_log`.
3. Host tests: camp probes have no `read_camp_owner_log`;
   log values keep owner / `ONLINE`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Read the owner log inside `apply_me_camp_probes`. Live
`f15c170f…` still streams the file during the camp turn.

## Ссылки

- ADR-686, ADR-703, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
