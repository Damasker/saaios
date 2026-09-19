# ADR-156: VUI-07 SurfacePattern — blocked and failed

## Статус

Принято, 2026-09-19. `SurfacePattern` grows `blocked` / `failed`.
First live consumer: Bluetooth `PAIR-ERROR` paints Failed instead of
disappearing after ADR-145 removed the Surface subtitle. Permission,
confirmation, and recovery are not this slice. Do not tap Сопрячь.

## Нумерация

После ADR-155 следующий свободный номер — **156**. Не S33.

## Контекст

ADR-155 named empty / loading / offline. Blocked and Failed still
have constructors missing. Pairing failure is already parsed
(`PAIR-ERROR\t` in `/run/saai-shell-bt-pair.log`) but
`bluetooth_status_summary` is dead after the Bluetooth header
migration — the list can look idle or loading while the log says the
pair failed.

Confirmation is already `DecisionOverlay`. Permission is already the
Object View OAM line. Those stay. This slice does not invent a
recovery button.

## Decision

1. **`SurfacePattern::blocked` / `failed`**. Same caller-supplied
   message. Marks follow `UniversalState`. Idle still paints none.
2. **`bluetooth_pair_error_from`** returns the `PAIR-ERROR` reason
   only when that line is the first result. `PAIRED` first is not a
   pattern — success stays on the device row.
3. **Failed occupies slot 0**, like loading/empty. Devices shift down.
   Slot 0 is not Сопрячь. Controls stay trailing.
4. **Do not tap Сопрячь.** Panther proof writes a `PAIR-ERROR` log
   line, then opens the list.

## Consequences

- Pairing failure is a named Failed state, not a vanished subtitle.
- Rollback: restore the dead summary and drop the constructors.

## Verification

Host: failed is `UniversalState::Failed` and not busy; PAIR-ERROR
overrides loading; slot 0 with an error is not Device(0). Panther:
write `PAIR-ERROR`, open Bluetooth, screenshot the failed card, leave
with Назад. Do not tap Сопрячь.
