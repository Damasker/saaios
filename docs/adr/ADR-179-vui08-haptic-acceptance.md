# ADR-179: VUI-08 — haptic acceptance closeout

## Статус

Принято, 2026-09-20. VUI-08 haptic acceptance is the ADR-171
contract: one `HapticEvent` map, 15 ms rate-limit, no tick for
tabs, Orb, unlock, or other decoration. The Звук «Виброотклик»
switch stays independent of «Меньше движения». No new waveform,
no new daemon, no displayd protocol. Do not type a PIN. Do not
send an intent. Do not 7-tap. Do not toggle the switch. Leave
Сейчас.

## Нумерация

После ADR-178 следующий свободный номер — **179**. Не S33.

## Контекст

ADR-171 already mapped `KeyPress` → `KeyTick`, dropped `TabPress`
and `OrbActivity`, rate-limited the uploaded FF replay, and put
the disable in Система → Звук. That binary is still on panther.
The last VUI-08 acceptance box was documentary: name those three
facts as the closed contract so motion/pacing work is not left
half-checked.

## Decision

1. **Consistent** means every live tick goes through
   `haptic_intent_for`. Painters do not name a waveform.
2. **Rate-limited** means `KEY_TICK_REPLAY_MS` (15). A second play
   inside that window is dropped.
3. **Absent for passive decoration** means tabs, Orb activity, and
   unlock stay `None` even when the switch is on. The switch lives
   in Звук, not Интерфейс.

## Consequences

- VUI-08 motion, haptics, and frame pacing are closed on panther.
  Rollback: reopen the acceptance box; keep ADR-171.

## Verification

Host: KeyPress+enabled is `KeyTick`; disabled/Tab/Orb are `None`;
14 ms is dropped; «Виброотклик» is under Звук and independent of
reduced motion. Panther: `haptics_enabled` stays true, Система →
Звук shows «Виброотклик» Вкл, leave Сейчас. Do not toggle.
