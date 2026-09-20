# ADR-218: VUI-09 — Inbox, Spaces, and list hits come from `compile_v2()`

## Статус

Принято, 2026-09-20. NOW already compiles `now.sui` (ADR-217). Live
Inbox `EventRow`s, Spaces `SpaceRow`s, and Wi-Fi / Bluetooth /
trusted-client lists now generate a `.sui` v2 document at runtime and
hit-test `layout_v2()`. Loc is the live id (entity uuid, space id, or
`wifi.N` / `bluetooth.N` / `trusted.N`). Offline/empty Status rows
invent no actions. Trailing `refresh`/`scan`/`back` stay the nested
rows from ADR-214. Paint stays procedural. Me scroll, the apps grid,
and overlays stay on their formulas. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off.

## Нумерация

После ADR-217 следующий свободный номер — **218**. Не S33.

## Контекст

`now.sui` is static. Inbox and Spaces rows are runtime-sized, so they
cannot live in a build-time file. The public examples already proved
one `EventRow` / `SpaceRow` / `WifiRow` matches `stacked_row_rect`.
Closing those screens means the shell compiles the same grammar with
one component per live row.

## Decision

1. **Generate.** Shell builds `sui 2` text with the proven header,
   stacked vocabulary rows, nested tabs or trailing rows, then
   `compile_v2()` / `layout_v2()`.
2. **Hits.** `inbox_row_at` / `space_row_at` / `wifi_list_action_at` /
   `bluetooth_list_action_at` / `trusted_client_action_at` read
   `hit_test`. Quiet Status rows invent no destination.
3. **Me / apps / overlays.** Me still uses `scrolled_row_rect` because
   `layout_v2` is a linear stack and cannot place a row under the
   header after a drag. Apps stay `now_grid_rect`. Overlays stay
   later.

## Consequences

- Live list destinations are compiled. Me scroll is the remaining
  tab formula. Rollback: restore the `stacked_row_rect` helpers.
  Still not Visual v1 sign-off.

## Verification

Host: Inbox first-row center opens the live uuid; offline invents
none; Spaces `home`; Wi-Fi/Bluetooth/trusted trailing and overflow
dock unchanged. Panther: flash; Сейчас; do not tap Inbox/Spaces
rows, Сопряжь, PSK, or Отозвать.
