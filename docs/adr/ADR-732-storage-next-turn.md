# ADR-732: defer `df` off the wpa_cli turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `6ff452f7…` (ADR-731). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather.

## Нумерация

После ADR-731 следующий свободный номер — **732**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-731 fills v4l / ifaces after `wpa_cli` / `df`. The net
turn still ran `df` during `wpa_cli status`. dest-no-lock
stays off. Do not start the shell from COM13. Do not open
modem devices. Do not type PSK. Section «Связь» stays.
Device «Экран» stays. «Хранилище» stays the S15 name.

## Решение

1. `apply_me_net_probes` does not call `storage_string`. Wi-Fi
   / supervisor fill without `df`. «Хранилище» stays
   «неизвестно» for one more frame.
2. The next event-loop turn runs `df` onto held facts and
   stores `me_storage_hold`. Local sysfs still waits. A Wi-Fi
   connect drops `me_net_hold` only. Recapture skips `df`
   when the string is held.
3. Host test: net probes have no `storage_string`; storage
   turn stores the hold; idle blocks on `storage_pending`.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Call `storage_string` from `apply_me_net_probes`. Live
`6ff452f7…` still runs `df` during `wpa_cli`.

## Ссылки

- ADR-686, ADR-710, ADR-730, ADR-731, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
