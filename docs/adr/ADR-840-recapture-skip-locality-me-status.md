# ADR-840: recapture skip locality_me_status

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `c9066c4e…` (ADR-839). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not invent a «Цвет пространства»
split label. `me_chrome_hold` still owns «Местность».

## Нумерация

После ADR-839 следующий свободный номер — **840**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-838 skips `/proc/bootconfig` on recapture through
`me_facts()`. Recapture still applied `me_chrome_hold` and
then called `apply_me_locality_probes`, so empty locality
still read the locality JSON. dest-no-lock stays off. Do not
start the shell from COM13. Do not open modem devices. Do
not type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. `me_facts()` skips `apply_me_locality_probes` when
   `me_chrome_hold` is set. The locality turn still reads
   JSON once. Chrome probes stay a no-op.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the chrome-hold skip arm has no
   `apply_me_locality_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_locality_probes` after boot in `me_facts()`
even when `me_chrome_hold` is set. Live `c9066c4e…` still
reads locality JSON on recapture when locality is empty.

## Ссылки

- ADR-838, ADR-839, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
