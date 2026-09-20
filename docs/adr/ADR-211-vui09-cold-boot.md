# ADR-211: VUI-09 — cold boot on panther HEAD

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. Operator-approved
cold boot: `reboot -f`. `/run/saaios/dev-no-lock` dropped. Boot lock
painted Canvas + clock `14:07` + tap-unlock hint + `Зарядка 99%`.
`saai-displayd` 401, `saai-shell` 418, `saai-entityd` 155,
`file-recv` 158 came back without intervention. PIN stayed `null`.
Tap-unlock returned Сейчас. Marker restored; unlocked restart
skipped lock (pid 501). `compile()` stays `sui 1`. No new daemon.
Leave Сейчас.

## Нумерация

После ADR-210 следующий свободный номер — **211**. Не S33.

## Контекст

The ledger left cold boot open because `/run` is lost across reboot
and the no-lock marker would drop. ADR-209 proved lock/unlock.
ADR-210 proved display restart without dropping `/run`. Ordinary
`reboot` historically did not complete; `reboot -f` is the proven
cold path. 7-tap gallery and daylight booth stay later cells.

## Decision

1. **`reboot -f` is the cold-boot cell.** Do not invent a PIN.
2. **Unlock by tap** on the post-boot lock surface.
3. **Restore `/run/saaios/dev-no-lock` and restart unlocked.**
4. **Do not reflash.** Binary stays ADR-198.

## Consequences

- Cold boot is proven on HEAD chrome. Rollback: restore the marker
  after unlock. 7-tap gallery and daylight booth stay open. Still
  not Visual v1 sign-off.

## Verification

Host: ledger cites ADR-211. Panther: marker absent at boot; clock
`14:07`; tap-unlock; services up; same `3850427a…`; marker restored;
leave Сейчас.
