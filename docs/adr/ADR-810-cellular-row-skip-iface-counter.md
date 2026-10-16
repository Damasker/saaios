# ADR-810: cellular row skip iface_counter

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `657ea3bc…` (ADR-809). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not merge
DNS/IPv4 back into Сотовая сеть.

## Нумерация

После ADR-809 следующий свободный номер — **810**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-808 already walks held iface names. The Сотовая сеть
session suffix still read `/sys/class/net/*/statistics` on
every Me row rebuild. dest-no-lock stays off. Do not start
the shell from COM13. Do not open modem devices. Do not type
PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. `cellular_row_with_session` does not call
   `live_traffic_token`. The ifaces probe still filters
   bearers with `iface_counter`.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held. Section «Связь» stays grouped.
3. Host test: `cellular_row_with_session` has no
   `iface_counter(`. Do not flash this ADR. dest-no-lock
   stays off.

## Откат

Call `live_traffic_token` from `cellular_row_with_session`.
Live `657ea3bc…` still reads rx/tx sysfs on every Me row
rebuild.

## Ссылки

- ADR-808, ADR-809, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
