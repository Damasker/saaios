# ADR-697: supervisor token on the Wi-Fi turn, on panther

## Статус

Принято, 2026-10-15. `/data/saaios/system/saai-shell` is
`64c3fc50…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 31880. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=8`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-696 следующий свободный номер — **697**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-696 fills the supervisor token on the Wi-Fi/`df` turn.
dest-no-lock stays off. Do not start the shell from COM13.
Do not type PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr695` (`0f93f0d9…`).
3. Kill 31146. First respawn 31775 opened Object View;
   kill 31775 only. displayd 2694 respawned 31880.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr695 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-695, ADR-696, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
