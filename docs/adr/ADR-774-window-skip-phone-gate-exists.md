# ADR-774: window setup skip `/data/saaios` exists

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `b350cf13…` (ADR-773). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64.

## Нумерация

После ADR-773 следующий свободный номер — **774**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-772 already skips `phone_gate_surface` in entityd TCP
lookup. Window construct still called it to pick 1080×2400
fullscreen, which stats `/data/saaios` before the first
Sistema turn. dest-no-lock stays off. Do not start the shell
from COM13. Do not open modem devices. Do not type PSK.

## Решение

1. Window setup uses `cfg!(target_arch = "aarch64")` for the
   phone gate. It does not call `phone_gate_surface`.
2. Panther still gets 1080×2400 fullscreen. A Wi-Fi connect
   drops `me_net_hold` only.
3. Host test: the window block has no `phone_gate_surface()`
   and no `/data/saaios`. Do not flash this ADR. dest-no-lock
   stays off.

## Откат

Call `phone_gate_surface` from window setup. Live `b350cf13…`
still stats `/data/saaios` before the first frame.

## Ссылки

- ADR-772, ADR-773, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
