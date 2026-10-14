# ADR-687: deferred Wi-Fi/`df` on panther

## Статус

Принято, 2026-10-15. `/data/saaios/system/saai-shell` is
`04308074…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 22923. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=10`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-686 следующий свободный номер — **687**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-686 fills Wi-Fi and storage on the turn after camp facts.
dest-no-lock stays off. Do not start the shell from COM13.
Do not type PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr685` (`f29eb476…`).
3. Kill 21032 only. displayd 2694 respawned 22923.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr685 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-685, ADR-686, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
