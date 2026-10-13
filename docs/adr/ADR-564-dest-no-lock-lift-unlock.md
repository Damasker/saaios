# ADR-564: dest-no-lock lifted; unlock proven

## Статус

Принято, 2026-10-13. `/data/saaios/system/saai-shell` is
`dce4a158…` (pixel7 musl, panic messages on). dest-no-lock is
gone. displayd pid 24060 stayed. Unlock is proven. Experimental.
PIN stays null. Not Visual v1 sign-off. Not VUI-09 Acceptance.

## Нумерация

После ADR-563 следующий свободный номер — **564**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-563 fixed the leftover compose `ContextHeader` abort on no-PIN
lock. Live panther still had `035db4fa…`. The user asked to lift
`/run/saaios/dev-no-lock`. A first lift on the old binary requested
session lock without SIGTRAP, then aborted. dest-no-lock was restored
until this binary landed.

## Решение

1. Cross-build `saai-shell` without `panic_immediate_abort`.
   `panic = abort` stays.
2. PUT `saai-shell` under `/data/saaios/incoming/`, then rename over
   `/data/saaios/system/saai-shell`. Previous binary kept as
   `saai-shell.prev-adr563` (`035db4fa…`).
3. Kill the dest-no-lock shell only. displayd pid 24060 stayed and
   respawned shell 25898 under the marker.
4. Remove `/run/saaios/dev-no-lock`. Kill 25898. New shell 26048:
   `session lock requested`, lock surface 1080×2400, `surface=lock`
   `backend=shm`. No ContextHeader abort. No SIGTRAP.
5. Tap on the lock surface: `unlocked by touch`. displayd:
   `session unlocked`. Idle timeout then re-locked. dest-no-lock
   stays off.

## Откат

`mv /data/saaios/system/saai-shell.prev-adr563 /data/saaios/system/saai-shell`
and `touch /run/saaios/dev-no-lock`, then kill the current shell.

## Ссылки

- ADR-512, ADR-532, ADR-563
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
