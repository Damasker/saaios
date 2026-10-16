# ADR-844: recapture skip read_cp_state

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `55211355…` (ADR-843). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not tail the camp owner log.

## Нумерация

После ADR-843 следующий свободный номер — **844**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-842 skips `/proc` camp scan on recapture through
`me_facts()`. Recapture still applied `me_cp_hold` and then
called `apply_me_cp_probes`, so a missing CP state still
called `read_cp_state`. dest-no-lock stays off. Do not start
the shell from COM13. Do not open modem devices. Do not type
PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. `me_facts()` applies `me_cp_hold` and skips
   `apply_me_cp_probes`. The CP turn still reads CP state
   once.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held. Do not tail the camp owner log.
3. Host test: the CP hold arm has no `apply_me_cp_probes`.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Call `apply_me_cp_probes` after applying `me_cp_hold` in
`me_facts()`. Live `55211355…` still calls `read_cp_state` on
recapture when CP state is missing.

## Ссылки

- ADR-842, ADR-843, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
