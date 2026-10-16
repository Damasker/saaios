# ADR-830: recapture skip hardware_model

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `ceee30e9…` (ADR-829). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not un-group Связь. Do not leftover-hunt
`apply_me_chrome_probes`. Do not 7-tap.

## Нумерация

После ADR-829 следующий свободный номер — **830**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-828 skips osrelease on recapture through `me_facts()`.
Recapture still applied `me_model_hold` and then called
`apply_me_model_probes`, so an empty model string still read
`/proc/device-tree/model`. dest-no-lock stays off. Do not
start the shell from COM13. Do not open modem devices. Do not
type PSK. Leave Сейчас. Do not tap Orb. Do not 7-tap.

## Решение

1. `me_facts()` applies `me_model_hold` and skips
   `apply_me_model_probes`. The model turn still reads
   device-tree. Runtime JSON still uses `facts.model`, not
   `hardware_model()`.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the model hold arm has no
   `apply_me_model_probes`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `apply_me_model_probes` after applying `me_model_hold`
in `me_facts()`. Live `ceee30e9…` still reads device-tree on
recapture when the model string is empty.

## Ссылки

- ADR-828, ADR-829, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
