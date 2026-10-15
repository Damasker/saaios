# ADR-733: deferred Sistema df on panther

## Статус

Принято, 2026-10-15. `/data/saaios/system/saai-shell` is
`c52583ad…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 29666. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=12`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Section «Связь» stays. Device «Экран»
stays. «Хранилище» stays the S15 name.

## Нумерация

После ADR-732 следующий свободный номер — **733**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-732 fills «Хранилище» after `wpa_cli`, not on the net
turn. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr731` (`6ff452f7…`).
3. displayd 2694 stayed. Live shell 29666.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr731 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-731, ADR-732, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
