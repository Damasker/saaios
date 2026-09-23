# ADR-117: `SystemStatus` composite, tab press, reduced motion

## Status

Accepted, 2026-09-17. Host-verified: `cargo test -p saai-ui-core` 46/46,
`cargo test -p saai-shell` 97/97, clippy `-D warnings` clean on both.
Physical Pixel 7 review of the pressed fill and the new `Я` row is not
claimed here.

## Context

VUI-04's remaining honest gaps after ADR-116:

- `draw_status_bar` still took a loose S13 argument list instead of a
  `saai-ui-core` composite.
- `NavigationItem.pressed` existed but nothing in `saai-shell` set it.
- `OrbHost.reduced_motion` existed but no `ShellSettings` field could
  set it.

## Decision

**`SystemStatus`** (component-library-v1.md section 7.6): clock text, a
compact `StatusIndicator` for network (`Active` = Wi-Fi up, `Offline` =
no network), optional `Metric` for battery. Space/context color stays a
surface argument so this composite cannot confuse it with severity.
`present_status_bar` still snapshots the same primitive fields
(`time_text` / `wifi_up` / `battery` / `dot_color`) for ADR-093's skip-
unchanged-commit path; it builds a `SystemStatus` only at paint time.

**Tab `pressed`**: `Shell.pressed_tab` tracks `tab_at` while a toplevel
touch is down and no modal is open. `root_navigation_items` sets
`NavigationItem.pressed` from that field. `draw_tab_bar` fills
`ColorRole::Pressed` on an unselected pressed tab without changing
icon/label sizes (section 4: press does not shift layout). Release still
switches pages; press is feedback only.

**`reduced_motion`**: persisted `ShellSettings` field, default `false`.
A real `Я` row ("Уменьшить движение") toggles it. `build_orb_frame`
passes it to `OrbHost::with_reduced_motion`. No sixth Orb state.

## Verification

Host tests and clippy for the two crates. Not claimed: a device
screenshot of the pressed fill, or exercising Orb `Running` with
reduced motion on Pixel 7.

## Consequences

- Status bar data has one contract; pixel layout is unchanged.
- Pressed tabs are a real trigger, not an unused field.
- Reduced motion is settable; motion/arc/ring Context Light axes remain
  later VUI-04 work.
- `Я → Система` is still not renamed.

## Rollback

No protocol or storage schema change besides an additive JSON key
ignored by older shells. Previous shell binary remains the rollback
artifact.

## Links

- ADR-116 -- `BottomNavigation` / `OrbHost` contracts this ADR wires.
- ADR-053 -- original status-bar content.
- ADR-093 -- status-bar snapshot dedup, kept as primitive fields.
