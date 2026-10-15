# ADR-766: settings skips `/data/saaios` exists

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `0364c405…` (ADR-765). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64.

## Нумерация

После ADR-765 следующий свободный номер — **766**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The settings turn (`me_facts_chrome`) still called
`phone_gate_surface()`, which stats `/data/saaios`. First open
claimed no sysfs on that turn. dest-no-lock stays off. Do not
start the shell from COM13. Do not open modem devices. Do not
type PSK. Section «Связь» stays. Device «Экран» stays.

## Решение

1. `me_facts_chrome` derives the phone gate from
   `cfg!(target_arch = "aarch64")`. It does not call
   `phone_gate_surface`.
2. A Wi-Fi connect drops `me_net_hold` only. Runtime still
   fills identity from the held model. Recapture skips when
   facts are held.
3. Host test: chrome has no `phone_gate_surface()` and no
   `/data/saaios`. Do not flash this ADR. dest-no-lock stays
   off.

## Откат

Call `phone_gate_surface` from `me_facts_chrome`. Live
`0364c405…` still stats `/data/saaios` on first Sistema paint.

## Ссылки

- ADR-756, ADR-765, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
