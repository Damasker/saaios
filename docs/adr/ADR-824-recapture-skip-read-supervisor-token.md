# ADR-824: recapture skip read_supervisor_token

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `b0b2023d…` (ADR-823). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not tail the camp
owner log.

## Нумерация

После ADR-823 следующий свободный номер — **824**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-822 skips `wpa_cli` on recapture. Recapture still called
`apply_me_supervisor_probes` after applying
`me_supervisor_hold`, so a later Me rebuild still tailed the
8KiB boot log when the supervisor token was empty.
dest-no-lock stays off. Do not start the shell from COM13.
Do not open modem devices. Do not type PSK. Leave Сейчас. Do
not tap Orb.

## Решение

1. Recapture applies `me_supervisor_hold` and skips
   `apply_me_supervisor_probes`. The supervisor turn still
   tails 8KiB. Do not tail the camp owner log.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the recapture hold arm has no
   `apply_me_supervisor_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_supervisor_probes` after applying
`me_supervisor_hold`. Live `b0b2023d…` still tails the boot
log on recapture when the supervisor token is empty.

## Ссылки

- ADR-822, ADR-823, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
