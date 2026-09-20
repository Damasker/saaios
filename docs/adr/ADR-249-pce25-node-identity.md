# ADR-249: PCE-25 node identity — laptop ≠ panther

## Статус

Принято, 2026-09-20. Host identity. Not Visual v1 sign-off.
PIN stays null. Panther chrome unchanged this slice.

## Нумерация

После ADR-248 следующий свободный номер — **249**. Не S33.

## Контекст

Wave D starts PCE-25 Physical Multi-node Test Lab. Pixel 7 remains
the phone-gate. The laptop is a second x86 surface. Resource
Scheduler and PCE-01..24 stay closed. Node identity must not invent
a PIN, PSK, or space UUID. `system.identity` already carries
`target` / `device_class` / `architecture`; the hole was that an
x86 host could still claim `panther`/`phone`.

USB NCM `172.31.7.1` (ADR-073/074) is panther reachability, not an
identity field. Identity snapshots still omit IP, MAC, serial, and
hostname.

## Decision

1. **Schema 1 unchanged.** Same keys. Laptop facts:
   `device_class=computer`, `target=x86`, `architecture=x86_64`.
   Panther facts stay `phone` / `panther` / `aarch64`.
2. **x86 cannot impersonate the phone-gate.** If architecture is
   x86/x86_64/i686, `target=panther` and `device_class=phone` are
   coerced to `x86` / `computer`.
3. **Distinct nodes** = different `(target, architecture, device_class)`.
   No new UUID.
4. **Not PCE-01..24.** No Resource Scheduler, replica, or placement.

## Consequences

Host tests prove panther and laptop keys differ, including a spoofed
panther env on x86. Panther runtime binary is unchanged this slice:
aarch64 + `androidboot.hardware=panther` still classifies as phone.
Next D slice is windowed displayd on the x86 surface, not a panther
chrome change. Rollback: revert this crate.

## Verification

Host: `cargo test -p system-tools panther_stays_phone_gate
x86_laptop_is_computer_surface_not_panther
x86_host_cannot_impersonate_panther_phone_gate
mock_identity_stays_virtual_fixture`.
Panther: no flash. Marker on. Leave Сейчас.
