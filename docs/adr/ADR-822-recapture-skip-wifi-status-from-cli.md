# ADR-822: recapture skip wifi_status_from_cli

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `08cef1c9…` (ADR-821). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь.

## Нумерация

После ADR-821 следующий свободный номер — **822**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-820 skips runtime JSON on recapture. Recapture still
called `apply_me_net_probes` after applying `me_net_hold`, so
a later Me rebuild still ran `wpa_cli` when wifi status was
empty. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. Leave
Сейчас. Do not tap Orb.

## Решение

1. Recapture applies `me_net_hold` and skips
   `apply_me_net_probes`. The net turn still runs `wpa_cli`.
   A Wi-Fi connect drops `me_net_hold` only, so recapture
   still probes net after connect.
2. Recapture skips JSON when runtime is held. Section
   «Связь» stays grouped.
3. Host test: the recapture hold arm has no
   `apply_me_net_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_net_probes` after applying `me_net_hold`. Live
`08cef1c9…` still runs `wpa_cli` on recapture when wifi
status is empty.

## Ссылки

- ADR-820, ADR-821, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
