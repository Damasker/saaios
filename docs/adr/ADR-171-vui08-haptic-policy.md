# ADR-171: VUI-08 — haptic policy, rate-limit, disable switch

## Статус

Принято, 2026-09-20. Components name a `HapticEvent`; only policy
returns a `HapticIntent`, and only `HapticMotor` talks to
`/dev/input/haptic`. Tabs and Orb stay silent. Keyboard `KeyPress`
is still one `KeyTick` when haptics are on. Rate-limit is the 15 ms
FF replay so pulses do not overlap. `haptics_enabled` is an
independent Система → Звук switch, not `reduced_motion`. Do not
type a PIN. Do not send an intent. Do not 7-tap. Leave Сейчас.

## Нумерация

После ADR-170 следующий свободный номер — **171**. Не S33.

## Контекст

ADR-151 put `HapticIntent::KeyTick` on keyboard down. Painters were
not supposed to own the motor, but the only caller still asked for
a key tick by name. VUI-08 wants a map from component events to
policy, a rate-limit, and a runtime disable that is not mixed with
reduced motion. Displayd still has no haptic protocol; this slice
does not flash it. Unlock stays silent (S04). Tab and Orb events
map to `None` (ADR-168 / ADR-170).

## Decision

1. **`HapticEvent`** is the product meaning (`KeyPress`, `TabPress`,
   `OrbActivity`). `haptic_intent_for(event, enabled)` is the only
   map. Disabled, tabs, and Orb return `None`.
2. **Rate-limit** `KEY_TICK_REPLAY_MS` (15), the uploaded FF replay.
   A second play inside that window is dropped.
3. **`ShellSettings.haptics_enabled`** defaults true; missing JSON
   key stays true. Система → Звук «Виброотклик» toggles it. Motor
   stays in `saai-shell`. No new daemon, no new waveform.

## Consequences

- Keyboard still ticks when the switch is on. Quiet decoration and
  tab/Orb motion do not. Rollback: drop the event map and the
  setting; restore the ADR-151 `play(KeyTick)` call.

## Verification

Host: KeyPress+enabled is `KeyTick`; disabled/Tab/Orb are `None`;
rate-limit rejects 14 ms and allows 15 ms. Panther: Система → Звук
shows «Виброотклик», toggle Выкл then back to Вкл, leave Сейчас.
Do not type a PIN. Do not send.
