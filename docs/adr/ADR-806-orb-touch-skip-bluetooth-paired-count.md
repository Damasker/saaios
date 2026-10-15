# ADR-806: orb touch skip bluetooth_paired_count

## Статус

Принято, 2026-10-16. Host. Live `/data/saaios/system/saai-shell`
is `9f54aa29…` (ADR-805). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Camera names stay `video*`. Node
identity stays panther · aarch64. Do not split runtime into
two socket calls. Do not tap Orb.

## Нумерация

После ADR-805 следующий свободный номер — **806**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-804 already skips `bluetooth_paired_count` in
`build_orb_frame`. The orb touch hit-test still read the
saved-log when the menu was open. dest-no-lock stays off. Do
not start the shell from COM13. Do not open modem devices. Do
not type PSK. Leave Сейчас. Do not tap Orb.

## Решение

1. The orb touch path uses `me_local_hold` / held facts, not
   `bluetooth_paired_count`. The local probe still reads the
   saved-log.
2. A Wi-Fi connect drops `me_net_hold` only. Recapture skips
   JSON when runtime is held.
3. Host test: the orb touch hit-test has no
   `bluetooth_paired_count()`. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `bluetooth_paired_count` from the orb touch hit-test.
Live `9f54aa29…` still reads the saved-log on orb touch.

## Ссылки

- ADR-804, ADR-805, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
