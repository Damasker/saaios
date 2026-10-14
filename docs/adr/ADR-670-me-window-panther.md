# ADR-670: visible Sistema window on panther

## Статус

Принято, 2026-10-14. `/data/saaios/system/saai-shell` is
`7c3b9bac…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 10333. Unlock then
Sistema: FramePace `surface=me` `backend=shm` seq=7. Process
settled `S` after the first fill. Unlock stays the proven
path (ADR-564). Experimental. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off. Not VUI-09 Acceptance. Wrap
Acceptance stays unchecked.

## Нумерация

После ADR-669 следующий свободный номер — **670**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The user still hung on Sistema with `1bb214c6…`. Touch-down
wiped the row cache and the first fill compiled every
SettingRow. dest-no-lock stays off. Do not start the shell
from COM13.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr668` (`1bb214c6…`).
3. Kill 9949 only. displayd 2694 respawned 10333.
4. Unlock then Sistema. First fill still takes tens of
   seconds (`me_facts` + flatten). Then process `S`. dest-no-lock
   stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr668 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-668, ADR-669, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
