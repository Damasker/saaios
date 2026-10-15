# ADR-776: boot skip `/data/saaios/system/boot-attempts`

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `ceb55563…` (ADR-775). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. «Обновления» stays the
slot. DevSurface still reads attempts.

## Нумерация

После ADR-775 следующий свободный номер — **776**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-774 already skips `/data/saaios` exists on window setup.
The boot turn still read `/data/saaios/system/boot-attempts`
after `boot_slot`. Sistema paints the slot, not attempts.
dest-no-lock stays off. Do not start the shell from COM13.
Do not open modem devices. Do not type PSK.

## Решение

1. `apply_me_boot_probes` calls `boot_slot`. It does not call
   `boot_attempts`.
2. DevSurface still reads attempts live. A Wi-Fi connect
   drops `me_net_hold` only. Recapture skips bootconfig when
   the slot is held.
3. Host test: boot probes have no `boot_attempts()`. Do not
   flash this ADR. dest-no-lock stays off.

## Откат

Call `boot_attempts` from `apply_me_boot_probes`. Live
`ceb55563…` still stats boot-attempts on the boot turn.

## Ссылки

- ADR-742, ADR-775, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
