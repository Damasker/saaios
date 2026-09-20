# ADR-258: Bluetooth adapter presence is hci0, not bt-scan

## Статус

Принято, 2026-09-20. Shell Система. Not Visual v1 sign-off.
PIN stays null. Do not tap Сопряжь.

## Нумерация

После ADR-257 следующий свободный номер — **258**. Не S33.

## Контекст

S20 already pairs through `/saaios/bt-scan` / `/saaios/bt-pair`.
`bluetooth_adapter_present` treated the scan binary as the adapter.
Panther has a live `/sys/class/bluetooth/hci0`. A host without hci0
but with a copied `bt-scan` would have shown a fake adapter.

## Decision

1. **hci0 is the adapter.** Present iff
   `/sys/class/bluetooth/hci0` exists. `hci0:65` is not a second
   adapter.
2. **List stays gated.** No hci0 → «Нет адаптера», no dispatch, so
   Сопряжь is not offered.
3. **Pairing tools stay.** Scan/pair still use bt-scan/bt-pair when
   the list is open. This slice does not tap them.

## Consequences

Система follows the live controller, not the ramdisk binary. Rollback:
revert `bluetooth_adapter_present`.

## Verification

Host: `cargo test -p saai-shell --offline -- bluetooth_adapter_is_hci0`.
Leave Сейчас. Do not tap Сопряжь.
