# ADR-151: VUI-07 keyboard — pressed color and haptic tick

## Статус

Принято, 2026-09-19. Intent, Wi-Fi password, and PIN keys use
`ColorRole::Pressed` while a finger is down and request one
`HapticIntent::KeyTick`. Action still fires on release. Do not type
a PSK. Do not send an intent. Do not type PIN digits. Leave with
Отмена. Space detail is not this slice. VUI-08 still owns a later
displayd-centralized motor.

## Нумерация

После ADR-150 следующий свободный номер — **151**. Не S33.

## Контекст

ADR-029 keys were idle `Surface` tiles until release. Tabs already
follow a live finger with `ColorRole::Pressed`. Component library:
pressed feedback begins immediately; haptic policy is requested from
the shell, not from each painter. S04 left `/dev/input/haptic` in
drm-splash; displayd still has no haptic protocol. Keyboard ticks
cannot wait for VUI-08.

## Decision

1. **Pressed fill is `ColorRole::Pressed`** on the same
   `paint_keyboard_keys` rects. Size does not change.
2. **One `KeyTick` on down** (and on slide onto a new key). Release
   clears the fill and runs the existing action. Missing
   `/dev/input/haptic` is a no-op.
3. **Motor stays in `saai-shell`** (`HapticMotor`, drm-splash's 15 ms
   FF_CUSTOM pulse). VUI-08 may move the write to displayd; this
   slice does not flash displayd.

## Consequences

- Host tests the token and the intent. Panther: hold one letter,
  screenshot, Отмена. Do not send.
- Unlock-without-PIN stays silent (not a key).

## Verification

Host: pressed pixel is `Pressed`; idle is not. Panther: letter key
goes cyan on hold and ticks the CS40L26. Leave Отмена.
