# ADR-192: VUI-09 — unlocked shell restart on panther HEAD

## Статус

Принято, 2026-09-20. HEAD shell stays `e8865301…`. Unlocked
`saai-shell` was killed once with `/run/saaios/dev-no-lock` present;
native-init respawned the same binary. Сейчас came back without a
lock surface. No Me tap, so no 7-tap, no lock-timeout row, no PIN,
no Orb. `compile()` stays `sui 1`. No new daemon. Leave Сейчас.

## Нумерация

После ADR-191 следующий свободный номер — **192**. Не S33.

## Контекст

The ledger left «service restart» on the ADR-188 flash restart.
`DEV_NO_LOCK_MARKER` exists so repeated unlocked restarts do not
each require a lock cycle. Killing a locked shell without
`session_lock.unlock()` wedges displayd — this slice only kills
while unlocked. Lock cycle, display restart, and cold boot stay
out: marker stays, displayd stays, `/run` is not dropped.

## Decision

1. **Kill only the unlocked shell.** Marker on. PIN stays null.
2. **Do not reflash.** Binary stays ADR-188.
3. **Do not lock, reboot, or kill displayd.**

## Consequences

- Unlocked restart is proven on HEAD chrome. Rollback: do not kill
  a locked shell. Lock cycle, display restart, and cold boot stay
  open.

## Verification

Host: marker path still named. Ledger cites ADR-192. Panther: new
pid, same `e8865301…`, Сейчас without lock; leave Сейчас.
