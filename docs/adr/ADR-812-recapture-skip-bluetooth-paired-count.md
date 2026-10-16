# ADR-812: recapture skip bluetooth_paired_count

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `a2c39b78…` (ADR-811). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь.

## Нумерация

После ADR-811 следующий свободный номер — **812**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

A Wi-Fi connect drops `me_net_hold` only and recaptures Me
rows. Zero paired devices are not `me_local_values_held`, so
recapture still read the saved-log. dest-no-lock stays off.
Do not start the shell from COM13. Do not open modem devices.
Do not type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. Recapture applies `me_local_hold` and skips
   `apply_me_local_probes`. The local turn still reads the
   saved-log.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the recapture hold arm has no
   `apply_me_local_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_local_probes` after applying `me_local_hold`.
Live `a2c39b78…` still rereads the paired-device log on
recapture when the count is zero.

## Ссылки

- ADR-811, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
