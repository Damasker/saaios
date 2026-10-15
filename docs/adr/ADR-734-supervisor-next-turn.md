# ADR-734: defer supervisor tail off the wpa_cli turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `c52583ad…` (ADR-733). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather.

## Нумерация

После ADR-733 следующий свободный номер — **734**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-733 fills «Хранилище» after `wpa_cli`. The net turn still
tailed the boot log during `wpa_cli status`. dest-no-lock
stays off. Do not start the shell from COM13. Do not open
modem devices. Do not type PSK. Section «Связь» stays.
Device «Экран» stays.

## Решение

1. `apply_me_net_probes` does not call `read_supervisor_token`.
   Wi-Fi fills without the 8 KiB tail.
2. The next event-loop turn reads the supervisor token onto
   held facts and stores `me_supervisor_hold`. `df` still
   waits. A Wi-Fi connect drops `me_net_hold` only. Recapture
   skips the tail when the token is held.
3. Host test: net probes have no `read_supervisor_token`;
   supervisor turn stores the hold; idle blocks on
   `supervisor_pending`. Do not flash this ADR. dest-no-lock
   stays off.

## Откат

Call `read_supervisor_token` from `apply_me_net_probes`. Live
`c52583ad…` still tails the boot log during `wpa_cli`.

## Ссылки

- ADR-696, ADR-732, ADR-733, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
