# ADR-158: VUI-07 permission as blocked, recovery mark on Failed

## Статус

Принято, 2026-09-19. Permission is `SurfacePattern::blocked` on Object
View, not a Caption dump. Recovery is the Failed mark on a
`SurfacePattern` card plus the existing trailing recovery control
(Bluetooth «Искать»). Consent chrome and Сопрячь stay untouched.
Do not tap Разрешить or Сопрячь.

## Нумерация

После ADR-157 следующий свободный номер — **158**. Не S33.

## Контекст

Object View already computes an OAM permission line and then flattens
it into `details` next to activity. Failed Bluetooth occupies slot 0
without the Failed mark — `BluetoothRow::from_pattern` dropped
`paints_mark()`. Visual Language 9.5 wants cause plus a direct
recovery action; «Искать» is already that action, but the failed
card itself has no non-color cue.

Confirmation stays `DecisionOverlay` (ADR-157). App-consent stays
ADR-142.

## Decision

1. **Object View `permission` is `SurfacePattern::blocked`**. Omit it
   from `details`. Paint mark + Body. No invented button.
2. **Pattern cards keep `StatusIndicator` when `paints_mark()`**.
   Empty still has no mark. Loading / offline / blocked / failed do.
3. **Do not tap Сопрячь / Разрешить.** Panther proof writes
   `PAIR-ERROR`, opens Bluetooth, screenshots the Failed mark above
   «Искать», leaves with Назад.

## Consequences

- Permission and recovery are named states, not Caption leftovers.
- Rollback: restore permission into `details` and drop `indicator`.

## Verification

Host: OAM unavailable is blocked and not a detail line; failed
Bluetooth row has a mark and is not Сопрячь; «Искать» still follows.
Panther: Failed mark + «Искать», Назад. Do not tap Сопрячь.
