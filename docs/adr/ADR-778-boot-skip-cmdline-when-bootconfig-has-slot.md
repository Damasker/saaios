# ADR-778: boot skip `/proc/cmdline` when bootconfig has the slot

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `5f02c7cc…` (ADR-777). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. «Обновления» stays the
slot.

## Нумерация

После ADR-777 следующий свободный номер — **778**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-776 already skips `boot-attempts` on the boot turn.
`boot_slot` still opened `/proc/bootconfig` and `/proc/cmdline`
together before searching. dest-no-lock stays off. Do not
start the shell from COM13. Do not open modem devices. Do
not type PSK.

## Решение

1. `boot_slot` reads `/proc/bootconfig` first. When that text
   has `androidboot.slot_suffix=`, it does not open
   `/proc/cmdline`.
2. Cmdline stays the fallback. A Wi-Fi connect drops
   `me_net_hold` only. Recapture skips bootconfig when the
   slot is held.
3. Host test: bootconfig is searched before cmdline; a slot
   token in bootconfig does not need cmdline. Do not flash
   this ADR. dest-no-lock stays off.

## Откат

Read both `/proc/bootconfig` and `/proc/cmdline` before
searching. Live `5f02c7cc…` still opens cmdline on every
boot turn.

## Ссылки

- ADR-742, ADR-776, ADR-777, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
