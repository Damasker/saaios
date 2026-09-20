# ADR-206: VUI-09 — dock `BluetoothRow` as Bluetooth list stacked hits

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. `layout_v2()`
places `BluetoothRow` on `stacked_row_rect` (same 430 + n×220 as
Wi-Fi). `a11y = Button` emits the live `pair_bluetooth` action from
`BluetoothRow::open`. `a11y = Status` is empty/loading/failed
(`BluetoothRow::from_pattern`) and is not actionable. A screen
without `BluetoothRow` invents no Bluetooth hits. No root tabs: live
«Bluetooth устройства» has trailing Искать/Обновить/Назад, not
`BottomNavigation`. `compile()` stays v1. No new daemon. Do not open
the live list this slice (Me scroll would pass Блокировка; Сопряжь
is forbidden). Leave Сейчас.

## Нумерация

После ADR-205 следующий свободный номер — **206**. Не S33.

## Контекст

ADR-205 named Wi-Fi `WifiRow` hits. Live Bluetooth cards still come
from `bluetooth_list_action_at` over `stacked_row_rect`. `BluetoothRow`
is already a public v2 name. Live device taps are
`BluetoothListTap::Device(index)` with action `pair_bluetooth`. This
slice does not invent a device address and does not model trailing
Искать/Обновить/Назад. Tapping a live device would start pairing;
this slice does not open the list. `TrustedClientRow` stays
privileged and procedural.

## Decision

1. **Grammar.** `component BluetoothRow` on `screen bluetooth`. Button
   rows get `pair_bluetooth`. Status rows occupy the stacked rect
   without an action. Ids come from `loc` when present.
2. **Layout.** Same stacked formula as Wi-Fi. Header slot fills
   0…first row so 540,250 misses. No `BottomNavigation` in the public
   sample, so 135,2250 misses.
3. **Example.** `docs/os/ui/examples/bluetooth-public.sui` names one
   Button row at `bluetooth.item`.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `bluetooth_list_action_at` + `layout_v1_root()`. Trailing controls
   stay procedural.

## Consequences

- v2 layout owns the public Bluetooth row destination. Искать /
  Обновить / Назад, Сопряжь, and trusted clients stay procedural.
  Rollback: treat `BluetoothRow` as a Fill leaf. Next:
  Experimental→Stable after Visual v1, or operator-approved
  lock/display/cold-boot. Still not Visual v1 sign-off.

## Verification

Host: public Bluetooth 540,525 hits `pair_bluetooth`; Status row
misses; no tab hits. Panther: chrome unchanged on HEAD; do not open
the list; leave Сейчас.
