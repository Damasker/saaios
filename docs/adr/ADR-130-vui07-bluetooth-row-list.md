# ADR-130: VUI-07 Bluetooth list — BluetoothRow

## Статус

Принято, 2026-09-18. «Bluetooth устройства» lists live `bt-scan` log
rows through `BluetoothRow`. Flashed panther `5b37c5bc…`: title
below the status layer, scan-in-progress with no empty placeholder,
then `Поиск завершён: найдено 2` with `X5` BLE and `OBDII` CLASSIC
(`Сопрячь`), Scan ≠ Refresh ≠ Back. Trusted clients, Wi-Fi password,
and lock restyle are not this slice.

## Нумерация

После ADR-129 следующий свободный номер — **130**. Не S33.

## Контекст

VUI-07 next remaining surface after the Wi-Fi list. Система already
opens this screen (`SettingRow::open` → `open_bluetooth_list`, «Нет
адаптера» when the radio is missing). The destination is still S20
`draw_row_list`: one concatenated label (`name · CLASSIC|BLE`) plus
trailing «Искать устройства (~8 с)» / «Обновить список» / «Назад».
Scan is a real ~8s background `bt-scan`; Refresh only rereads the
log. There is no `BluetoothRow`. HIA-05 / ideas.md: the shell knows
paired-ever (`SAVED` lines), not live connection right now. Do not
invent RSSI, a second BT-detail frame, or a Space binding.

Keep pair behavior: tap still runs `bt-pair <index>`. Do not restyle
trusted clients or the password keyboard in the same slice.

## Decision

1. **`BluetoothRow` wraps `DataRow`**. Live row: Navigation, primary
   = device name, value = `CLASSIC`/`BLE` when the scan printed
   `TRANSPORT` (omit value if still empty). `paired` when the name
   appears in `bluetooth-saved.log`. Empty scan **after** `DONE`:
   Static «Нет устройств». Empty **before** `DONE` (never started or
   still running): no placeholder row — the status line already
   names that. Missing adapter never opens this screen.
2. **Flatten to `ActionCardView`** (`Сопряжено` / `Сопрячь`, empty
   has no button). Hit-test still uses `stacked_row_rect`. Empty is
   not tappable. Scan / Refresh / Back stay trailing control cards
   after the `BluetoothRow` list (index shifts by one only when
   scan-done-and-empty).
3. **Paint through `draw_action_row_list`**, same header inset as
   Wi-Fi. Trusted clients stay on `draw_row_list`.

## Consequences

- Paired vs other devices use the saved-name flag, not a fabricated
  "connected now".
- Rollback: restore concatenated labels in `Frame::BluetoothList`.

## Verification

Host: BluetoothRow constructors; live list order and paired button;
empty only after `DONE`; Scan/Refresh/Back after the row list; no
invented RSSI. Panther `5b37c5bc…`: X5 BLE + OBDII CLASSIC after
DONE; empty placeholder only after DONE (not during `Идёт поиск`).
