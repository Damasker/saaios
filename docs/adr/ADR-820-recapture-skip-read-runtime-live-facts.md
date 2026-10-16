# ADR-820: recapture skip read_runtime_live_facts

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `e48a8a1c…` (ADR-819). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь.

## Нумерация

После ADR-819 следующий свободный номер — **820**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-818 skips the camera list on recapture. Recapture still
called `apply_me_runtime_probes` after applying
`me_runtime_hold`, so a later Me rebuild still opened the
runtime socket. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type PSK.
Leave Сейчас. Do not tap Orb.

## Решение

1. Recapture applies `me_runtime_hold` and skips
   `apply_me_runtime_probes`. The runtime turn still reads
   identity and observations on one socket.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the recapture hold arm has no
   `apply_me_runtime_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_runtime_probes` after applying
`me_runtime_hold`. Live `e48a8a1c…` still opens the runtime
socket on recapture when the hold is set.

## Ссылки

- ADR-818, ADR-819, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
