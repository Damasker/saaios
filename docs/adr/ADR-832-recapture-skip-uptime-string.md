# ADR-832: recapture skip uptime_string

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `61af5b84…` (ADR-831). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not 7-tap.

## Нумерация

После ADR-831 следующий свободный номер — **832**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-830 skips device-tree on recapture through `me_facts()`.
Recapture still applied `me_uptime_hold` and then called
`apply_me_uptime_probes`, so an empty uptime string still
read `/proc/uptime`. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type PSK.
Leave Сейчас. Do not tap Orb. Do not 7-tap.

## Решение

1. `me_facts()` applies `me_uptime_hold` and skips
   `apply_me_uptime_probes`. The uptime turn still reads
   `/proc/uptime`.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the uptime hold arm has no
   `apply_me_uptime_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_uptime_probes` after applying `me_uptime_hold`
in `me_facts()`. Live `61af5b84…` still reads `/proc/uptime`
on recapture when the uptime string is empty.

## Ссылки

- ADR-830, ADR-831, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
