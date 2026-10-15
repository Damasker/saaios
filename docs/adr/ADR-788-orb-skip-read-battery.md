# ADR-788: Me orb skip read_battery

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `d12e70a6…` (ADR-787). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb.

## Нумерация

После ADR-787 следующий свободный номер — **788**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-784 already skips `read_runtime_live_facts` in
`build_orb_frame`. First-open Sistema still called
`read_battery` on every Me paint, which stats `maxfg` capacity
and status. dest-no-lock stays off. Do not start the shell
from COM13. Do not open modem devices. Do not type PSK.

## Решение

1. `build_orb_frame` does not call `read_battery`. Orb quantity
   stays unset on that paint.
2. Status-bar and lock still read the gauge. A Wi-Fi connect
   drops `me_net_hold` only. Recapture skips JSON when runtime
   is held.
3. Host test: the orb painter has no `read_battery()`. Do not
   flash this ADR. dest-no-lock stays off.

## Откат

Call `read_battery` from `build_orb_frame`. Live `d12e70a6…`
still stats `maxfg` on every Me paint.

## Ссылки

- ADR-786, ADR-787, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
