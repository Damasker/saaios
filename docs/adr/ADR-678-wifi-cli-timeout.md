# ADR-678: timeout and defer Wi-Fi `wpa_cli`

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
is `36a07194…` (ADR-677). dest-no-lock stays absent. Hung
14277 killed (`R` on `surface=me` after Wi-Fi password send).
displayd 2694 respawned 15342. Unlock stays the proven path
(ADR-564). Experimental. PIN stays null. Leave Сейчас. Not
Visual v1 sign-off. Not VUI-09 Acceptance. Wrap Acceptance
stays unchecked. Do not type PSK.

## Нумерация

После ADR-677 следующий свободный номер — **678**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The user entered a Wi-Fi password and the shell hung. OSK send
called `wifi_connect_psk` on the same turn: six blocking
`wpa_cli` invocations with no timeout, then `draw()`. pid
14277 stayed `R` at `surface=me` seq=713. dest-no-lock stays
off. Do not start the shell from COM13. Do not log the PSK.

## Решение

1. `command_output_timeout` kills `wpa_cli` and `df` after
   400 ms. Host tests: `sleep 2` returns none; `true` returns
   some.
2. OSK send and open-network tap only set
   `wifi_connect_pending`. The next event-loop turn runs
   connect so the keyboard can close first.
3. Idle waits while connect is pending. Do not flash this ADR.

## Откат

Revert the timeout helper and `wifi_connect_pending`. Live
`36a07194…` still hangs on password send.

## Ссылки

- ADR-676, ADR-677, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
