# ADR-818: recapture skip capture_nodes

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `44fca9d7…` (ADR-817). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь.

## Нумерация

После ADR-817 следующий свободный номер — **818**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-816 skips the iface list on recapture. An empty camera
list is not `me_camera_values_held`, so recapture still listed
`video*`. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. Leave
Сейчас. Do not tap Orb.

## Решение

1. Recapture applies `me_camera_hold` and skips
   `apply_me_camera_probes`. The camera turn still lists
   `video*`.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the recapture hold arm has no
   `apply_me_camera_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_camera_probes` after applying `me_camera_hold`.
Live `44fca9d7…` still lists `video*` on recapture when the
camera list is empty.

## Ссылки

- ADR-816, ADR-817, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
