# ADR-675: held Sistema facts on panther

## Статус

Принято, 2026-10-14. `/data/saaios/system/saai-shell` is
`4e18d857…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 12356. Unlock then
Sistema: `switched to Me`, FramePace `surface=me`
`backend=dmabuf` `idle_ok=1` `seq=8`. Process settled `S`.
No mid-fill session-lock error. Unlock stays the proven path
(ADR-564). Experimental. PIN stays null. Leave Сейчас. Not
Visual v1 sign-off. Not VUI-09 Acceptance. Wrap Acceptance
stays unchecked.

## Нумерация

После ADR-674 следующий свободный номер — **675**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-673 stayed `R` after `surface=me` because the observation
turn called `me_facts()` again and idle requested a lock.
ADR-674 holds the first snapshot and skips idle mid-fill.
dest-no-lock stays off. Do not start the shell from COM13.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr673` (`828b6ff9…`).
3. Kill 11339 only. displayd 2694 respawned 12356.
4. Unlock (540,1200) then Sistema (945,2250). `surface=me`
   `idle_ok=1`, then process `S`. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr673 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-673, ADR-674, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
