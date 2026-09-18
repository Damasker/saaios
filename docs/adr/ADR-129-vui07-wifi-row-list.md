# ADR-129: VUI-07 Wi-Fi list — WifiRow

## Статус

Принято, 2026-09-18. Host + panther (`c80bb666…`): «Wi-Fi сети»
lists live `wpa_cli scan_results` through `WifiRow`. Password
keyboard, Bluetooth list, and lock restyle are not this slice.

## Нумерация

После ADR-128 следующий свободный номер — **129**. Не S33.

## Контекст

VUI-07 next remaining surface after Inbox and the Spaces *list*.
Система already opens this screen (`SettingRow::open` →
`open_wifi_list`, «Нет адаптера» when the radio is missing). The
destination is still S19 `draw_row_list`: one concatenated label per
BSS (`ssid · защищена/открыта · N dBm`) plus trailing «Обновить» /
«Назад». Empty scan and connected SSID share that one-line shape;
there is no `WifiRow`. Visual Language 9.5 wants connectivity
problems named honestly; it does not ask for invented bars or a
second Wi-Fi-detail frame.

Keep connect behavior: open network associates immediately; secured
opens the existing password keyboard. Do not bind SSID to a Space
here (ideas.md / HIA-02). Do not restyle Bluetooth or trusted
clients in the same slice — they still share `draw_row_list`.

## Decision

1. **`WifiRow` wraps `DataRow`**. Live row: Navigation, primary =
   SSID, value = `защищена`/`открыта` plus the real `signal_dbm` from
   scan, `connected` when `wpa_state=COMPLETED` matches that SSID.
   Empty scan (adapter present, zero SSID rows): Static «Нет сетей».
   Missing adapter never opens this screen (already gated on
   Система). No fabricated RSSI glyph, vendor, or Space binding.
2. **Flatten to `ActionCardView`** (`Подключено` / `Подключить`,
   empty has no button). Hit-test still uses `stacked_row_rect`.
   Empty is not tappable; «Обновить» / «Назад» stay trailing control
   cards after the `WifiRow` list (index shifts by one when empty).
3. **Password input waits.** `WifiPasswordInput` and open-network
   `wifi_connect_open` keep today's handlers.

## Consequences

- Connected vs other networks use the same selected-button pattern
  as `SpaceRow`, not a second status string.
- Bluetooth / trusted clients remain one-line `draw_row_list` until
  their own rows exist.
- Rollback: restore concatenated labels in `Frame::WifiList`.

## Verification

Host: WifiRow constructors; live list order and connected button;
empty vs networks; Refresh/Back after the row list; no invented
bars. Panther `c80bb666…`: «Wi-Fi сети», status `Подключено: Wallbox`,
connected row `защищена · -42 dBm` / `Подключено`. Password keyboard
still later.
