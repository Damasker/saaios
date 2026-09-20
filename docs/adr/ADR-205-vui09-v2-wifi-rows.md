# ADR-205: VUI-09 — dock `WifiRow` as Wi-Fi list stacked hits

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. `layout_v2()`
places `WifiRow` on `stacked_row_rect` (same 430 + n×220 as Inbox).
`a11y = Button` emits the live `connect_wifi` action from
`WifiRow::open`. `a11y = Status` is `WifiRow::empty` «Нет сетей» and
is not actionable. A screen without `WifiRow` invents no Wi-Fi hits.
No root tabs: live «Wi-Fi сети» has trailing Обновить/Назад, not
`BottomNavigation`. `compile()` stays v1. No new daemon. Do not open
the live list this slice (Me scroll would pass Блокировка). Leave
Сейчас.

## Нумерация

После ADR-204 следующий свободный номер — **205**. Не S33.

## Контекст

ADR-204 named Me `SettingRow` hits. Live Wi-Fi cards still come from
`wifi_list_action_at` over `stacked_row_rect`. `WifiRow` is already a
public v2 name. Live network taps are `WifiListTap::Network(index)`
with action `connect_wifi`. This slice does not invent an SSID and
does not model trailing Обновить/Назад (`stacked_trailing_rect`).
Tapping a live network would open the PSK field; this slice does not
open the list. Bluetooth stays procedural.

## Decision

1. **Grammar.** `component WifiRow` on `screen wifi`. Button rows get
   `connect_wifi`. Status rows occupy the stacked rect without an
   action. Ids come from `loc` when present.
2. **Layout.** Same stacked formula as Inbox. Header slot fills
   0…first row so 540,250 misses. No `BottomNavigation` in the public
   sample, so 135,2250 misses.
3. **Example.** `docs/os/ui/examples/wifi-public.sui` names one Button
   row at `wifi.item`.
4. **Do not switch `build.rs` or reflash.** Live chrome stays
   `wifi_list_action_at` + `layout_v1_root()`. Trailing controls stay
   procedural.

## Consequences

- v2 layout owns the public Wi-Fi row destination. Обновить/Назад,
  PSK, and Bluetooth stay procedural. Rollback: treat `WifiRow` as a
  Fill leaf. Next: Experimental→Stable after Visual v1, or
  operator-approved lock/display/cold-boot. Still not Visual v1
  sign-off.

## Verification

Host: public Wi-Fi 540,525 hits `connect_wifi`; Status row misses;
no tab hits. Panther: chrome unchanged on HEAD; do not open the
list; leave Сейчас.
