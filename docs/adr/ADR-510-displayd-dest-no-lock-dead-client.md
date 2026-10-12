# ADR-510: dest-no-lock unlocks a dead lock client

## Статус

Принято, 2026-10-12. Host. No displayd flash. Lock SIGTRAP stays
open. Live panther recovered by restarting displayd; leftover
`locked=true` was the stuck lock surface.

## Нумерация

После ADR-509 следующий свободный номер — **510**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-503 swapped shell under dest-no-lock and left displayd running.
The previous shell had taken `ext-session-lock-v1` and died without
`unlock()`. displayd stayed `locked=true` and routed every touch to
the dead lock surface. The new dest-no-lock shell skipped `lock()`,
so it never unlocked. The user could not dismiss lock.

Without dest-no-lock the session must stay locked when the client
dies. This is not a PIN/idle/wake formula change (ADR-232).

## Решение

1. `should_unlock_dead_lock_client` is true only when the compositor
   is locked **and** `/run/saaios/dev-no-lock` is present.
2. On privileged-shell SIGCHLD, unlock before relaunch so the next
   dest-no-lock shell receives touches.
3. A real lock (marker absent) stays locked.

Host proof: `rustc --test services/saai-displayd/src/leftover_lock.rs`
(4 passed). `cargo test -p saai-displayd` needs libxkbcommon on this
WSL and is not this slice.

## Откат

Drop the SIGCHLD unlock; dest-no-lock leftovers need a displayd
restart again.

## Ссылки

- ADR-109, ADR-503
- `docs/os/sprints/PIXEL-PATH.md`
