# ADR-209: VUI-09 — lock / unlock cycle on panther HEAD

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. Operator-approved
lock cycle: `/run/saaios/dev-no-lock` removed, unlocked `saai-shell`
killed, boot lock painted Canvas + clock `13:58` + tap-unlock hint
`Коснитесь, чтобы разблокировать` + `Заряд 97%`. PIN stayed `null`
— no keypad, no invented PIN. Tap-unlock returned Сейчас. Marker
restored; unlocked restart skipped lock. Do not kill a locked
shell. `compile()` stays `sui 1`. No new daemon. Leave Сейчас.

## Нумерация

После ADR-208 следующий свободный номер — **209**. Не S33.

## Контекст

The VUI-09 ledger left lock / unlock open because
`DEV_NO_LOCK_MARKER` skips boot lock and idle re-lock, and killing a
locked shell without `session_lock.unlock()` wedges displayd. ADR-192
proved unlocked restart only. Operator carte blanche now allows the
real cycle. PIN is still null (ADR-134 tap-unlock). Do not save a
PIN. Display restart, cold boot, and 7-tap gallery stay later cells.

## Decision

1. **Remove the marker, then kill only while unlocked.** Boot lock
   is the proof, not idle timeout.
2. **Unlock by tap on the lock surface.** No PIN digits.
3. **Restore `/run/saaios/dev-no-lock` and restart unlocked** so later
   cells do not idle-lock.
4. **Do not reflash.** Binary stays ADR-198. Do not kill displayd.

## Consequences

- Lock cycle is proven on HEAD chrome. Rollback: restore the marker
  before any unlocked kill. Display restart, cold boot, and 7-tap
  gallery stay open. Still not Visual v1 sign-off.

## Verification

Host: ledger cites ADR-209 and tap-unlock. Panther: pid 27000→28091
locked, then tap-unlock; marker restored; pid 28125 Сейчас without
lock; `pin_code` still null; leave Сейчас.
