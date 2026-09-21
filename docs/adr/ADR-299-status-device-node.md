# ADR-299: Sistema Узел reads live status.device

## Статус

Принято, 2026-09-21. PCE-25 live fact. Not Visual v1 sign-off.
PIN stays null. Do not flash shell or runtime.

## Нумерация

После ADR-298 следующий свободный номер — **299**. Не S33.

## Контекст

ADR-249 already puts `device_class` / `target` / `architecture` on
runtime `status.device` (same object as `system.identity`). ADR-298
painted Система «Узел» from a local phone-gate guess. Wave D needs
the same live-fact channel as Observation, Memory, and Health: the
status blob, not USB NCM and not a second identity store.

## Decision

1. **Same keys.** Shell parses `status.device` for class/target/arch.
   Russian labels stay ADR-298 (`Телефон` / `Компьютер`).
2. **Fallback.** Missing, empty, or address-shaped values
   (`172.`, `192.168`, `:`) keep the local `phone_gate_surface()`
   guess. No IP, MAC, serial, hostname.
3. **Live wins.** A laptop `computer`/`x86` row is not painted as
   panther even if the local guess would say phone.
4. **Host only.** Panther paint waits the next shell experiment.
   Runtime binary unchanged: `device` is already on the DTO.

## Consequences

- Sistema «Узел» is a live fact when runtime status is reachable.
- Rollback: drop `live_node_from_status_json` and resolve only locally.

## Verification

Host: `cargo test -p saai-shell --offline live_node`. No panther flash.
Leave Сейчас.
