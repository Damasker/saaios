# ADR-673: deferred Sistema observations on panther

## Статус

Принято, 2026-10-14. `/data/saaios/system/saai-shell` is
`828b6ff9…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 2694 stayed. New shell 11339. Unlock then
Sistema: `switched to Me`, chrome frame `55e6c28e…`, then
FramePace `surface=me` `backend=dmabuf` `idle_ok=1` `seq=7`.
First `me_facts()` is still long; the process stayed `R`
through that fill. Unlock stays the proven path (ADR-564).
Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-672 следующий свободный номер — **673**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-672 omits «Наблюдения» from the first flatten. The user
said Sistema works and asked to continue. dest-no-lock stays
off. Do not start the shell from COM13.

## Решение

1. Cross-build without `panic_immediate_abort`. `panic =
   abort` stays.
2. PUT then rename over `/data/saaios/system/saai-shell`.
   Previous kept as `saai-shell.prev-adr670` (`7c3b9bac…`).
3. Kill 10333 only. displayd 2694 respawned 11339.
4. Unlock (540,1200) then Sistema (945,2250). Chrome
   committed, `surface=me` `idle_ok=1`. First facts still
   block the loop. dest-no-lock stayed absent.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr670 /data/saaios/system/saai-shell`
and kill the current shell.

## Ссылки

- ADR-670, ADR-672, ADR-564
- `docs/os/sprints/PIXEL-PATH.md`
