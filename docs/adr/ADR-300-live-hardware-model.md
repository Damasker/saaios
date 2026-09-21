# ADR-300: Sistema device summary uses live hardware_model

## Статус

Принято, 2026-09-21. PCE-25 live fact. Not Visual v1 sign-off.
PIN stays null. Do not flash shell or runtime.

## Нумерация

После ADR-299 следующий свободный номер — **300**. Не S33.

## Контекст

ADR-299 named the node from `status.device` class/target/arch.
Система «Это устройство» still used `/proc/device-tree/model`, which
is `неизвестно` on x86. The same identity snapshot already carries
`hardware_model` (DMI on laptop, device-tree on panther). Wave D
needs that live fact on the device row, not a second probe and not
the USB address.

## Decision

1. **Same object.** `hardware_model` is optional on the parsed
   `status.device` row. Empty, null, or address-shaped values are
   omitted; class/target/arch still stand.
2. **Summary.** «Это устройство» prefers the live model, then the
   local `hardware_model()` fallback. Storage stays local `df`.
3. **No IP.** `172.` / `192.168` / `:` never become the model.
4. **Host only.** Panther paint waits the next shell experiment.

## Consequences

- Laptop Sistema can name the DMI product instead of `неизвестно`.
- Rollback: drop `model` on `LiveNodeIdentity` and always use local.

## Verification

Host: `cargo test -p saai-shell --offline live_node_hardware`. No
panther flash. Leave Сейчас.
