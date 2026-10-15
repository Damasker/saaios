# ADR-731: deferred Sistema local sysfs on panther

## Статус

Принято, 2026-10-15. `/data/saaios/system/saai-shell` is
`6ff452f7…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 24397. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=12`. Process settled `S`.
Unlock stays the proven path (ADR-564). Experimental. PIN
stays null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather. Section «Связь» stays. Device «Экран»
stays. Camera names stay `video*`.

## Нумерация

После ADR-730 следующий свободный номер — **731**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-730 fills v4l / ifaces / paired / trusted after
`wpa_cli` / `df`, not on the net turn. dest-no-lock stays
off. Do not start the shell from COM13. Do not open modem
devices. Do not type PSK.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr729` (`4499003c…`).
3. Kill 23069 only. displayd 2694 respawned 24397.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr729 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-729, ADR-730, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
