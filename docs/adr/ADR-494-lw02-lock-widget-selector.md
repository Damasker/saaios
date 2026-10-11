# ADR-494: LW-02 — owner cycles lock widget disclosure on Система

## Статус

Принято, 2026-10-11. Unlocked Система now has a cycle row for
`lock_widgets`. Persist + immediate re-evaluation use the existing
settings save and lock projection. Physical acceptance stays pending.
No PIN enrollment. No flash.

## Нумерация

После ADR-493 следующий свободный номер — **494**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

LW-01 persisted `hidden` / `device` / `summary` with no owner-facing
control. The default stayed `summary` so appearance did not change.
LW-02 asks for an unlocked System selector and persistence tests.
Pixel typography, wake/unlock, and cold reboot stay a device window.

## Decision

1. **Host-lock** `cycle_lock_widgets` on Система → Приватность.
   Labels: Сводка → Скрыты → Только устройство → Сводка.
2. **Persist** through `ShellSettings::save`. The next lock paint
   reads `settings.lock_widgets` (already the projection input).
3. **Not this slice.** Do not flash. Do not enroll a PIN. Do not
   add a live settings file watcher. Do not convert leftover
   keyboard / gallery / lock formulas.

## Consequences

- The owner can hide lock widgets without editing JSON. Rollback:
  drop the row; the key still loads. LW-02 physical acceptance
  remains open.

## Verification

Host: cycle visits every policy; Me row names the current label;
malformed settings still hide. Panther: do not flash.
