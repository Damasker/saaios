# ADR-770: runtime JSON skip `/data/saaios` exists

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `09c2e79d…` (ADR-769). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls.

## Нумерация

После ADR-769 следующий свободный номер — **770**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-764 already skips `phone_gate_surface` in
`apply_me_runtime_probes`. `read_runtime_live_facts` still
called it on the USB-NCM fallback, which stats `/data/saaios`.
dest-no-lock stays off. Do not start the shell from COM13. Do
not open modem devices. Do not type PSK. One status JSON read
stays.

## Решение

1. `read_runtime_live_facts` uses `cfg!(not(target_arch =
   "aarch64"))` for the USB-NCM fallback. It does not call
   `phone_gate_surface`.
2. Unix sockets still run first, then the on-device runtime
   binary, then host USB-NCM. One `read_runtime_live_facts`
   from the runtime turn.
3. A Wi-Fi connect drops `me_net_hold` only. Host test: the
   helper has no `phone_gate_surface()`. Do not flash this
   ADR. dest-no-lock stays off.

## Откат

Call `phone_gate_surface` from `read_runtime_live_facts`. Live
`09c2e79d…` still stats `/data/saaios` on the JSON fallback.

## Ссылки

- ADR-764, ADR-769, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
