# ADR-752: defer device-tree model off the chrome turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `b523e71c…` (ADR-751). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`.

## Нумерация

После ADR-751 следующий свободный номер — **752**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-751 fills `/proc/uptime` after kernel / model. The chrome
turn still read `/proc/device-tree/model` together with
kernel. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. Section
«Связь» stays. Device «Экран» stays. Node identity line
`panther · aarch64` stays.

## Решение

1. `apply_me_chrome_probes` does not call `hardware_model` or
   `node_model_resolved`. Kernel fills without the device-tree
   model.
2. The next event-loop turn reads the model onto held facts
   and stores `me_model_hold`. Uptime still waits. A Wi-Fi
   connect drops `me_net_hold` only. Recapture skips model
   when the string is held.
3. Host test: chrome probes have no `hardware_model()`; model
   turn stores the hold; idle blocks on `model_pending`. Do
   not flash this ADR. dest-no-lock stays off.

## Откат

Call `hardware_model` from `apply_me_chrome_probes`. Live
`b523e71c…` still reads the device-tree model during chrome.

## Ссылки

- ADR-726, ADR-750, ADR-751, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
