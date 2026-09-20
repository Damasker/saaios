# ADR-210: VUI-09 — display restart on panther HEAD

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. Operator-approved
display restart: unlocked `saai-displayd` 8323 was killed;
native-init respawned 28184, which forked `saai-shell` 28190.
`/run/saaios/dev-no-lock` survived. Сейчас came back without a lock
surface. PIN stayed `null`. `compile()` stays `sui 1`. No new
daemon. Leave Сейчас.

## Нумерация

После ADR-209 следующий свободный номер — **210**. Не S33.

## Контекст

The ledger left display restart open because killing `saai-displayd`
is recovery-only except when operator-approved. ADR-192 killed only
the unlocked shell. ADR-209 proved lock/unlock and restored the
marker. native-init owns the UI slot (budget 5 / 60s) and restarts
displayd; displayd forks the shell. Cold boot and 7-tap gallery stay
later cells.

## Decision

1. **Kill displayd only while unlocked, marker on.** Do not kill a
   locked shell.
2. **Do not reflash.** Binary stays ADR-198.
3. **Do not reboot.** `/run` must survive this cell.

## Consequences

- Display restart is proven on HEAD chrome. Rollback: do not exceed
  the UI restart budget; do not kill locked. Cold boot and 7-tap
  gallery stay open. Still not Visual v1 sign-off.

## Verification

Host: ledger cites ADR-210. Panther: displayd 8323→28184, shell
28125→28190, same `3850427a…`, marker on, PIN null, Сейчас without
lock; leave Сейчас.
