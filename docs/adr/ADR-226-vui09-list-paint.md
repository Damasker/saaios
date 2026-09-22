# ADR-226: VUI-09 — list paint reads `layout_v2()`

## Статус

Принято, 2026-09-20. NOW chrome already paints from `now_view()`
(ADR-225). Live Inbox, Spaces, Wi-Fi, Bluetooth, and trusted-client
row cards now take rects from the same generated `compile_v2()` /
`layout_v2()` trees hit-test already uses (ADR-218). Trailing
Обновить / Искать / Назад come from named `row` nodes, not
`stacked_trailing_rect`. Me scroll, apps grid, overlays, OrbHost, and
diagnostic stay later slices. PIN stays null. Do not tap Inbox/Spaces
rows, Сопряжь, or Отозвать. Leave Сейчас. Not Visual v1 sign-off.

## Нумерация

После ADR-225 следующий свободный номер — **226**. Не S33.

## Контекст

Hits for these lists already generate documents. Paint still placed
cards on `stacked_row_rect` / `stacked_trailing_rect`, a second
formula. Inbox/Spaces tabs also came from `root_view` even though the
live document names `BottomNavigation`.

## Decision

1. **One tree per list.** Paint builds `layout_v2` from
   `inbox_v2_source` / `spaces_v2_source` / `wifi_v2_source` /
   `bluetooth_v2_source` / `trusted_v2_source`.
2. **Row ids.** Each card looks up its loc (`entity` uuid, `space.id`,
   `wifi.{i}`, `bluetooth.{i}` / `bluetooth.status.{i}`,
   `trusted.{i}`, or the empty/offline Status loc). Trailing controls
   use `refresh` / `scan` / `back`.
3. **Tabs.** Inbox and Spaces tab rects come from that document's
   `BottomNavigation`, same helper NOW uses.
4. **No invented Buttons.** Empty and offline stay Status rows.

## Consequences

- List paint and hit-test share the generated tree. Me/apps/overlay
  paint stay procedural until their slices. Rollback: restore
  `stacked_row_rect` in the card builders. Still not Visual v1
  sign-off.

## Verification

Host: empty Inbox `inbox.empty` rect matches `layout_v1_find`; Wi-Fi
trailing `back` matches the compiled node. Panther: flash; Сейчас; do
not open lists or tap rows; leave Сейчас.
