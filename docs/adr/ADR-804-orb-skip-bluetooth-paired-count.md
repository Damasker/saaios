# ADR-804: orb skip bluetooth_paired_count

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `f876ebc3…` (ADR-803). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb.

## Нумерация

После ADR-803 следующий свободный номер — **804**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The local turn already holds `bluetooth_paired`. Orb paint still
called `bluetooth_paired_count` when the menu was open. dest-no-lock
stays off. Do not start the shell from COM13. Do not open modem
devices. Do not type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. `build_orb_frame` uses `me_local_hold` / held facts, not
   `bluetooth_paired_count`. The local probe still reads the
   saved-log.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the orb painter has no `bluetooth_paired_count()`.
   Do not flash this ADR. dest-no-lock stays off.

## Откат

Call `bluetooth_paired_count` from `build_orb_frame`. Live
`f876ebc3…` still reads the saved-log on orb paint.

## Ссылки

- ADR-784, ADR-803, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
