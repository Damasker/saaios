# ADR-780: runtime skip `/tmp/saaios.sock` exists on aarch64

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `10129e45…` (ADR-779). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls.

## Нумерация

После ADR-779 следующий свободный номер — **780**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-770 already skips `phone_gate_surface` on the USB-NCM
fallback. The runtime turn still stats `/tmp/saaios.sock`
before `/run/saaios/saaios.sock`. dest-no-lock stays off. Do
not start the shell from COM13. Do not open modem devices.
Do not type PSK. One status JSON read stays.

## Решение

1. `runtime_sock_candidates` pushes `/tmp/saaios.sock` only
   when `cfg!(not(target_arch = "aarch64"))`. Panther stats
   `/run/saaios/saaios.sock`.
2. Host still uses `/tmp`. A Wi-Fi connect drops
   `me_net_hold` only. Recapture skips JSON when runtime is
   held.
3. Host test: the helper gates `/tmp` on arch; x86 still
   lists it. Do not flash this ADR. dest-no-lock stays off.

## Откат

Always push `/tmp/saaios.sock`. Live `10129e45…` still stats
tmp on every runtime turn.

## Ссылки

- ADR-770, ADR-779, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
