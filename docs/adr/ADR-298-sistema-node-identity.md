# ADR-298: Sistema names the node — laptop ≠ panther

## Статус

Принято, 2026-09-21. PCE-25 chrome. Not Visual v1 sign-off.
PIN stays null. Do not flash shell.

## Нумерация

После ADR-297 следующий свободный номер — **298**. Не S33.

## Контекст

ADR-249 fixed `system.identity` so x86 cannot claim `panther`/`phone`.
Система still showed device-tree model and storage without the node
class. Wave D needs the same Space/Entity/Intent on two nodes; the
device page must say which node this is. USB NCM IP is reachability,
not identity.

## Decision

1. **Same keys.** `node_class` / `node_target` match ADR-249:
   phone/panther vs computer/x86 from `phone_gate_surface()`.
2. **Readout.** Система «Узел» is `Телефон · panther · aarch64` or
   `Компьютер · x86 · <ARCH>`. Same three keys as ADR-249. Static.
   No IP, MAC, serial, hostname.
3. **Not PCE-01..24.** No Resource Scheduler. No new UUID.
4. **Host only.** Panther paint waits the next shell experiment.

## Consequences

- Laptop shell shows computer/x86 without spoofing the phone-gate.
- Rollback: drop the «Узел» row and the two fields.

## Verification

Host: `cargo test -p saai-shell --offline node_identity`. No panther flash.
Leave Сейчас.
