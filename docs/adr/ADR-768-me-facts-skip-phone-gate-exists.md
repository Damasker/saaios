# ADR-768: recapture skips `/data/saaios` exists

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `356e19c9…` (ADR-767). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64.

## Нумерация

После ADR-767 следующий свободный номер — **768**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-766 stopped settings from stating `/data/saaios`. The full
`me_facts` rebuild still called `phone_gate_surface()`. dest-no-lock
stays off. Do not start the shell from COM13. Do not open
modem devices. Do not type PSK. Recapture must skip via holds.
Section «Связь» stays. Device «Экран» stays.

## Решение

1. `me_facts` derives the phone gate from
   `cfg!(target_arch = "aarch64")`. It does not call
   `phone_gate_surface`.
2. A Wi-Fi connect drops `me_net_hold` only. Held facts still
   skip the rebuild. Runtime still fills identity from the
   held model.
3. Host test: `me_facts` has no `phone_gate_surface()` and no
   `/data/saaios`. Do not flash this ADR. dest-no-lock stays
   off.

## Откат

Call `phone_gate_surface` from `me_facts`. Live `356e19c9…`
still stats `/data/saaios` on a full Me rebuild.

## Ссылки

- ADR-766, ADR-767, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
