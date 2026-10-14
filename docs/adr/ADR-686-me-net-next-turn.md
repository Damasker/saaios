# ADR-686: defer Wi-Fi/`df` off the Sistema camp turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `f29eb476…` (ADR-685). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-685 следующий свободный номер — **686**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-685 settled `S` after `surface=me`. The live-probe turn
still blocked on two `wpa_cli status` calls and `df /data`
(400 ms each if stuck) before camp rows could replace chrome.
dest-no-lock stays off. Do not start the shell from COM13.
Do not type PSK.

## Решение

1. `me_facts()` keeps storage «неизвестно» and Wi-Fi «Нет
   статуса». Camp, `/proc`, supervisor, and runtime still run
   on the full-facts turn.
2. The next loop sets `me_net_ready` and applies one
   `wpa_cli status` plus `df` to the held facts. Idle stays
   blocked while `me_net_pending`. Observations still wait
   one more turn (ADR-672).
3. Host test: stripping net probes keeps `cellular_cp`;
   applying values fills storage and Wi-Fi. Do not flash
   this ADR. dest-no-lock stays off.

## Откат

Revert `me_net_pending` / `apply_me_net_probes`. Live
`f29eb476…` still waits on `wpa_cli`/`df` during the camp
turn.

## Ссылки

- ADR-676, ADR-678, ADR-685, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
