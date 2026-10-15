# ADR-740: defer trusted keys off the local-sysfs turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `a1fb70b4…` (ADR-739). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-739 следующий свободный номер — **740**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-739 fills rmnet ifaces after paired / trusted. The
local-sysfs turn still hashed `authorized_keys` together with
the Bluetooth saved log. dest-no-lock stays off. Do not start
the shell from COM13. Do not open modem devices. Do not type
PSK. Section «Связь» stays. Device «Экран» stays.

## Решение

1. `apply_me_local_probes` does not call `trusted_clients`.
   Paired fills without reading keys or hashing fingerprints.
2. The next event-loop turn counts trusted clients onto held
   facts and stores `me_trusted_hold`. Ifaces still wait. A
   Wi-Fi connect drops `me_net_hold` only. Recapture skips
   authorized_keys when the count is held.
3. Host test: local probes have no `trusted_clients`; trusted
   turn stores the hold; idle blocks on `trusted_pending`. Do
   not flash this ADR. dest-no-lock stays off.

## Откат

Call `trusted_clients` from `apply_me_local_probes`. Live
`a1fb70b4…` still hashes keys during paired.

## Ссылки

- ADR-730, ADR-738, ADR-739, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
