# ADR-816: recapture skip cellular_ifaces

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `eb02ae3f…` (ADR-815). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not merge
DNS/IPv4 back into Сотовая сеть.

## Нумерация

После ADR-815 следующий свободный номер — **816**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-814 skips trusted keys on recapture. An empty iface list
is not `me_ifaces_values_held`, so recapture still listed
`/sys/class/net`. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type PSK.
Leave Сейчас. Do not tap Orb.

## Решение

1. Recapture applies `me_ifaces_hold` and skips
   `apply_me_ifaces_probes`. The ifaces turn still lists
   `/sys`.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held. Section «Связь» stays grouped.
3. Host test: the recapture hold arm has no
   `apply_me_ifaces_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_ifaces_probes` after applying `me_ifaces_hold`.
Live `eb02ae3f…` still lists `/sys/class/net` on recapture
when the iface list is empty.

## Ссылки

- ADR-814, ADR-815, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
