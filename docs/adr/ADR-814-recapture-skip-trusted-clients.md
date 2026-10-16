# ADR-814: recapture skip trusted_clients

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `e39c6efb…` (ADR-813). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь.

## Нумерация

После ADR-813 следующий свободный номер — **814**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-812 skips the local probe on recapture. Zero trusted
clients are not `me_trusted_values_held`, so recapture still
read authorized_keys. dest-no-lock stays off. Do not start
the shell from COM13. Do not open modem devices. Do not type
PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. Recapture applies `me_trusted_hold` and skips
   `apply_me_trusted_probes`. The trusted turn still reads
   authorized_keys.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the recapture hold arm has no
   `apply_me_trusted_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_trusted_probes` after applying
`me_trusted_hold`. Live `e39c6efb…` still rereads
authorized_keys on recapture when the count is zero.

## Ссылки

- ADR-812, ADR-813, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
