# ADR-710: reuse Sistema net probes; skip recapture wpa_cli/`df`

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `412c67bf…` (ADR-709). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-709 следующий свободный номер — **710**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-709 reuses `/proc` presence on recapture. After the
staircase dropped the hold, recapture still ran `wpa_cli`
status, `df`, local sysfs, and the supervisor token.
dest-no-lock stays off. Do not start the shell from COM13.
Do not open modem devices. Do not type PSK.

## Решение

1. `apply_me_net_probes` returns when net values are already
   held. It does not call `wpa_cli` or `df`.
2. The net turn stores those fields on `me_net_hold`.
   Recapture seeds facts from that cache before apply.
   A Wi-Fi connect drops the hold so the next paint re-probes.
3. Host test: held Wi-Fi / storage survive apply. Do not
   flash this ADR. dest-no-lock stays off.

## Откат

Always call `wifi_status_from_cli` from net probes. Live
`412c67bf…` still shells out on recapture.

## Ссылки

- ADR-686, ADR-709, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
