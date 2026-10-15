# ADR-713: grouped Sistema cellular facts on panther

## Статус

Принято, 2026-10-15. `/data/saaios/system/saai-shell` is
`daef927d…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 6599. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=11`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Section «Связь» stays. Device «Экран» stays.

## Нумерация

После ADR-712 следующий свободный номер — **713**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-712 keeps `Сотовая сеть` as the healthy summary and
puts DNS / IPv4 / VoNR / Трасса on their own rows under
Связь. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr711` (`3be0c132…`).
3. Kill 5302 only. displayd 2694 respawned 6599.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr711 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-711, ADR-712, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
