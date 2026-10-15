# ADR-736: defer v4l capture off the local-sysfs turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `8ab3cd44…` (ADR-735). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-735 следующий свободный номер — **736**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-735 fills the supervisor tail after `wpa_cli`. The
local-sysfs turn still listed v4l `video*` together with
ifaces / paired / trusted. dest-no-lock stays off. Do not
start the shell from COM13. Do not open modem devices. Do
not type PSK. Section «Связь» stays. Device «Экран» stays.

## Решение

1. `apply_me_local_probes` does not call `capture_nodes`.
   Ifaces / paired / trusted fill without walking v4l.
2. The next event-loop turn lists camera nodes onto held
   facts and stores `me_camera_hold`. Runtime JSON still
   waits. A Wi-Fi connect drops `me_net_hold` only. Recapture
   skips v4l when camera nodes are held.
3. Host test: local probes have no `capture_nodes`; camera
   turn stores the hold; idle blocks on `camera_pending`. Do
   not flash this ADR. dest-no-lock stays off.

## Откат

Call `capture_nodes` from `apply_me_local_probes`. Live
`8ab3cd44…` still lists v4l during ifaces / paired / trusted.

## Ссылки

- ADR-730, ADR-735, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
