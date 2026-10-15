# ADR-808: cellular row skip cellular_ifaces

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `2ca6088c…` (ADR-807). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not merge
DNS/IPv4 back into Сотовая сеть.

## Нумерация

После ADR-807 следующий свободный номер — **808**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The ifaces turn already holds `facts.cellular_ifaces`. The
Сотовая сеть session suffix still listed `/sys/class/net` and
ran the bearer filter on every Me row rebuild. dest-no-lock
stays off. Do not start the shell from COM13. Do not open
modem devices. Do not type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. `live_traffic_token` walks the held iface names, not
   `cellular_ifaces()`. The ifaces probe still lists `/sys`.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held. Section «Связь» stays grouped.
3. Host test: `cellular_row_with_session` has no
   `cellular_ifaces()`. Do not flash this ADR. dest-no-lock
   stays off.

## Откат

Call `cellular_ifaces` from `live_traffic_token`. Live
`2ca6088c…` still lists `/sys/class/net` on every Me row
rebuild.

## Ссылки

- ADR-807, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
