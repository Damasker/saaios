# ADR-537: Bluetooth transport is BLE or omitted

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-536 следующий свободный номер — **537**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`bt-scan` writes `TRANSPORT\t<name>\t<CLASSIC|BLE>`. The list row
painted that token as the status. `CLASSIC` is an English scan
word, not a board label. BLE is the same class of radio acronym
`radio_suffix` already keeps.

## Решение

1. `BLE` / `LE` stays `BLE`.
2. `CLASSIC` and any other transport stay off the row. No new BT
   taxonomy.
3. The log parser still stores the raw token.

## Откат

Restore `device.transport.clone()` on `BluetoothRow::open`.

## Ссылки

- ADR-530, ADR-531
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-07 / VUI-09
