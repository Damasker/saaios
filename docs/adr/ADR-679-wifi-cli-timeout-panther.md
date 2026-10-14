# ADR-679: Wi-Fi `wpa_cli` timeout on panther

## Статус

Принято, 2026-10-14. `/data/saaios/system/saai-shell` is
`60fe8f16…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 15866. Unlock
(540,1200): `unlocked by touch`, process `S`. Do not type
PSK. Unlock stays the proven path (ADR-564). Experimental.
PIN stays null. Leave Сейчас. Not Visual v1 sign-off. Not
VUI-09 Acceptance. Wrap Acceptance stays unchecked.

## Нумерация

После ADR-678 следующий свободный номер — **679**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The user hung after Wi-Fi password send on `36a07194…`.
ADR-678 times out `wpa_cli`/`df` and defers connect one
loop. dest-no-lock stays off. Do not start the shell from
COM13.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr677` (`36a07194…`).
3. Kill 15342 only. displayd 2694 respawned 15866.
4. Unlock works. Do not type PSK on this ADR.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr677 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-677, ADR-678, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
