# ADR-772: entityd TCP skip `/data/saaios` exists

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `b4be60b2…` (ADR-771). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64.

## Нумерация

После ADR-771 следующий свободный номер — **772**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-770 already skips `phone_gate_surface` in the runtime JSON
helper. `entityd_tcp_addr` still called it at shell construct,
which stats `/data/saaios` before the first Sistema turn.
dest-no-lock stays off. Do not start the shell from COM13. Do
not open modem devices. Do not type PSK.

## Решение

1. `entityd_tcp_addr` uses `cfg!(target_arch = "aarch64")` for
   the phone gate. It does not call `phone_gate_surface`.
2. Unit tests still see no TCP address. A Wi-Fi connect drops
   `me_net_hold` only.
3. Host test: the helper has no `phone_gate_surface()` and no
   `/data/saaios`. Do not flash this ADR. dest-no-lock stays
   off.

## Откат

Call `phone_gate_surface` from `entityd_tcp_addr`. Live
`b4be60b2…` still stats `/data/saaios` at shell construct.

## Ссылки

- ADR-770, ADR-771, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
