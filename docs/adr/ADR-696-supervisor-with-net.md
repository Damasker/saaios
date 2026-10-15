# ADR-696: fill the Sistema supervisor token on the Wi-Fi turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `0f93f0d9…` (ADR-695). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-695 следующий свободный номер — **696**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-695 stops `/proc` once owner, status, and modemd are
known. The camp turn still tails 8 KiB of the supervisor boot
log. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK.

## Решение

1. `me_facts()` leaves `cellular_supervisor` empty. Camp log
   and `/proc` still run on the full-facts turn.
2. `apply_me_net_values` also writes the supervisor token on
   the existing Wi-Fi/`df` turn. Tail size stays 8 KiB.
3. Host test: stripping net probes drops `hold`; applying it
   restores the token and keeps `cellular_cp`. Do not flash
   this ADR. dest-no-lock stays off.

## Откат

Read the supervisor token from `me_facts()`. Live `0f93f0d9…`
still tails the boot log during the camp turn.

## Ссылки

- ADR-684, ADR-695, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
