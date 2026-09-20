# ADR-254: power button drives lock/sleep from live gpio

## Статус

Принято, 2026-09-20. Shell power. Not Visual v1 sign-off.
PIN stays null. Voice stays off (ADR-092). Display CRTC blank stays
displayd (S04 known limitation).

## Нумерация

После ADR-253 следующий свободный номер — **254**. Не S33.

## Контекст

Wave E continues phone hardware. native-init already exposes
`s2mpg12-power-keys` as `/dev/input/power-button`. drm-splash used
KEY_POWER=116 to lock and `disable_display()`. saai-shell idle-locks
and AOD-sleeps from timers, but `hardware_keyboard.rs` and displayd
HID both exclude volume/power/touch/haptic. A Система toggle is not
a fact source.

## Decision

1. **Same node as native-init.** Shell reads `/dev/input/power-button`
   non-blocking. KEY_POWER press (EV_KEY value=1) is the only event.
2. **Existing lock/AOD, not a new UI.** Unlocked → session lock.
   Locked → `sleeping` (lock sleep view). Sleeping → wake to lock,
   never unlock. Matches drm-splash's lock half, not CRTC blank.
3. **`dev-no-lock` wins.** Same as idle timeout: the marker keeps the
   shell from locking so recovery stays `kill saai-displayd`, not a
   PIN path. Press is logged and ignored.
4. **Host no-op.** Without the node, poll returns. Tests cover parse
   and the four-way action table.

## Consequences

Physical power on panther can lock/sleep/wake without tapping
Блокировка/Гашение. Display stays painted until displayd has a power
protocol. Modem/cameras/BT stay later E slices. Rollback: revert
`power_button.rs` and the poll.

## Verification

Host: `cargo test -p saai-shell --offline -- power_press`.
Panther: flash shell; marker stays on so a physical press must log
`power button ignored (dev-no-lock)` and leave Сейчас unlocked.
Do not tap Блокировка/Гашение/PIN. Do not kill a locked shell.
