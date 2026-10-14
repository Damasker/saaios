# ADR-659: Me motion hang; shell respawned

## Статус

Принято, 2026-10-14. Live `/data/saaios/system/saai-shell` stays
`eb3197c9…` (ADR-656). dest-no-lock stays absent. displayd pid
2694 stayed. Hung shell 12671 killed. New shell 3916. Lock
surface `surface=lock` `backend=shm` `idle_ok=1` seq=3.
`panel0-backlight` `bl_power` 0, brightness 1023. displayd
`connector DPMS On`. Unlock stays the proven path (ADR-564).
Experimental. PIN stays null. Leave Сейчас. Not Visual v1
sign-off. Not VUI-09 Acceptance. Wrap Acceptance stays
unchecked.

## Нумерация

После ADR-658 следующий свободный номер — **659**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The user reported the shell hung after the ADR-658 four-tab walk.
pid 12671 was `R` (spinning), not sleeping. FramePace:
`reason=motion` `surface=me` `idle_ok=0` pending=2 dropped=16
coalesced=10. An idle-timeout lock started (`session unlocked`
then `idle timeout, locking`) but the process kept burning CPU
on Me motion. Do not start the shell from COM13. Do not tap Orb.
Do not restore dest-no-lock.

## Решение

1. Kill 12671 only. displayd 2694 respawned 3916.
2. dest-no-lock stayed absent. Fresh lock FramePace is quiet.
3. Do not treat this as a leftover wrapper. Me motion + idle
   lock is a live hang to re-check later, not a new flash.

## Откат

No binary change. Previous binary remains
`/data/saaios/system/saai-shell.prev-adr614`.

## Ссылки

- ADR-656, ADR-658, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
