# SaaiOS `.sui` v2 public API

Status: **Experimental** (ADR-186). Nothing here is Stable.

Product source: [Visual Language v1](../architecture/visual-language-v1.md)

Contract source: [Component Library v1](component-library-v1.md)

Compiler: `compile_v2_public()` in `saai-ui-compiler`. Production
chrome still compiles through `compile()` on
`services/saai-shell/ui/root.sui`.

## Stability labels

| Label | Meaning | Compile path |
|---|---|---|
| Experimental | Public name. API may still change. | `compile_v2_public()` |
| Privileged | Shell chrome. Not an app API. | `compile_v2()` only |
| Deferred | Not in the vocabulary. | fails |
| Stable | Promotion checklist passed on Pixel 7. | none yet |

`sui_v2_stability(name)` returns the first three. Unknown names return
`None`.

## Public example

[`examples/now-public.sui`](examples/now-public.sui) is the labelled NOW
sample: `ContextHeader`, `ObjectSummary`, `SurfacePattern`,
`BottomNavigation` with nested `tab now/inbox/spaces/me` (ADR-196),
and nested `row apps` / `row intent` (ADR-199). It matches the live
Сейчас composition (header, object, empty pattern, footer, tabs)
without leftover v1 NOW cards and without `OrbHost`. Empty
`BottomNavigation` invents no v1 hits. Empty screens invent no
footer hits. A screen without `ObjectSummary` invents no object
hit (ADR-200).

[`examples/inbox-public.sui`](examples/inbox-public.sui) is the
labelled Inbox sample: `ContextHeader`, `EventRow` with
`a11y = Button` (ADR-202), and the same nested tabs. Status
`EventRow` invents no `open_object`.

[`examples/spaces-public.sui`](examples/spaces-public.sui) is the
labelled Spaces sample: `ContextHeader`, `SpaceRow` with
`a11y = Button` (ADR-203), and the same nested tabs. Status
`SpaceRow` invents no `select_space`.

[`examples/me-public.sui`](examples/me-public.sui) is the
labelled Me sample: `ContextHeader`, `SettingRow` with
`a11y = Button` and interned `loc = cycle_timezone` (ADR-204),
and the same nested tabs. Status `SettingRow` and `SystemSection`
invent no interned action.

[`examples/wifi-public.sui`](examples/wifi-public.sui) is the
labelled Wi-Fi sample: `ContextHeader`, `WifiRow` with
`a11y = Button` (ADR-205), and nested `row refresh` / `row back`
(ADR-214). Status `WifiRow` invents no `connect_wifi`. No nested
tabs: live «Wi-Fi сети» has trailing controls, not
`BottomNavigation`.

[`examples/bluetooth-public.sui`](examples/bluetooth-public.sui) is the
labelled Bluetooth sample: `ContextHeader`, `BluetoothRow` with
`a11y = Button` (ADR-206), and nested `row scan` / `row refresh` /
`row back` (ADR-214). Status `BluetoothRow` invents no
`pair_bluetooth`. No nested tabs.

[`examples/trusted-privileged.sui`](examples/trusted-privileged.sui) is
the labelled shell sample: `TrustedClientRow` with `a11y = Button`
(ADR-207) and nested `row back` (ADR-214). `compile_v2_public()`
rejects it. Status rows invent no `revoke_trusted_client`.

[`examples/capability-privileged.sui`](examples/capability-privileged.sui)
is the labelled shell sample: `CapabilityRow` with `a11y = Status`
(ADR-208). `compile_v2_public()` rejects it. The row occupies
`stacked_row_rect` and invents no action.

```
compile_v2_public(include_str!("…/now-public.sui"))
```

Spec sheets for those types remain in the component library. This page
does not invent a second palette or a second layout engine.

## Gallery

`saai-ui-core::public_gallery_type_names()` lists public fixture types.
`privileged_gallery_type_names()` is `DecisionOverlay` and
`TrustedClientRow`. The device gallery may show privileged rows for
review. Copying them into an app document fails `compile_v2_public()`.

## Deprecation

- `root.sui` has no leftover NOW actions (ADR-187). Live NOW uses
  `ObjectSummary` and the footer. `inspect_selected_entity` is not a
  public name.
- Privileged names that appear in gallery fixtures are not promoted by
  that appearance.
- Removing a public name requires an ADR and a compile error, not a
  silent skip.
- `compile_v2()` on `root.sui` stays forbidden. `layout_v2()` matches
  v1 tab hits for this example (ADR-194/196) and live footer hits
  (ADR-199) but is not wired into `build.rs`. `inset = safe` uses
  `EdgeInsets::from_safe` (ADR-195); the top inset is the status
  layer, not a content pad. Nested `tab` ids own the v2 strip.
  Nested `row` ids own the NOW footer. `ObjectSummary` owns the
  NOW object hit (`open_object`, ADR-200). `EventRow` owns Inbox
  stacked hits (ADR-202). `SpaceRow` owns Spaces stacked hits
  (ADR-203). `SettingRow` owns Me stacked hits (interned `loc`,
  ADR-204). `WifiRow` owns Wi-Fi stacked hits (`connect_wifi`,
  ADR-205). `BluetoothRow` owns Bluetooth stacked hits
  (`pair_bluetooth`, ADR-206). Privileged `TrustedClientRow` owns
  trusted-client stacked hits (`revoke_trusted_client`, ADR-207);
  `compile_v2_public()` rejects the name. Privileged `CapabilityRow`
  occupies Me app stacked hits with no action (ADR-208);
  `compile_v2_public()` rejects the name. Nested trailing `row`
  ids own list Обновить/Искать/Назад (`list_refresh` / `list_scan` /
  `list_back`, ADR-214).

## Migration

| From | To |
|---|---|
| Copying `saai-shell` paint | `saai-ui-core` types + this example |
| `sui 1` leftover NOW cards | removed in ADR-187; public NOW sample above |
| `OrbHost` / lock / gallery in an app | omit; those stay privileged |
| Switching `build.rs` to `compile_v2()` | do not; keep `compile()` |
| Wanting tab hit-test from a v2 NOW | nested `tab` + `layout_v2(compile_v2_public(…))`; not `root_view` |
| Logical `SafeInsets` on a `Node` | `EdgeInsets::from_safe`; do not pad top (status layer) |
| Empty `BottomNavigation {}` | no tab hits; do not borrow v1 ids |
| Wanting footer hit-test from a v2 NOW | nested `row apps` / `row intent` + `layout_v2`; not v1 `content_actions` |
| Empty screen without `row` | no footer hits; do not invent `open_apps` |
| Wanting object hit-test from a v2 NOW | `component ObjectSummary` + `layout_v2`; not `inspect_selected_entity` |
| Screen without `ObjectSummary` | no object hit; do not invent `open_object` |
| Wanting Inbox hit-test from a v2 screen | `component EventRow` + `a11y = Button` + `layout_v2` |
| Inbox `EventRow` with `a11y = Status` | stacked rect, no `open_object` |
| Wanting Spaces hit-test from a v2 screen | `component SpaceRow` + `a11y = Button` + `layout_v2` |
| Spaces `SpaceRow` with `a11y = Status` | stacked rect, no `select_space` |
| Wanting Me hit-test from a v2 screen | `component SettingRow` + `a11y = Button` + interned `loc` + `layout_v2` |
| Me `SettingRow` with `a11y = Status` | stacked rect, no interned action |
| Wanting Wi-Fi trailing hits from a v2 screen | nested `row refresh` / `row back` + `layout_v2` |
| Screen without trailing `row` | no `list_back`; do not invent Назад |
| Wanting Bluetooth trailing hits from a v2 screen | nested `row scan` / `row refresh` / `row back` + `layout_v2` |
| Wi-Fi `WifiRow` with `a11y = Status` | stacked rect, no `connect_wifi` |
| Wanting Bluetooth hit-test from a v2 screen | `component BluetoothRow` + `a11y = Button` + `layout_v2` |
| Bluetooth `BluetoothRow` with `a11y = Status` | stacked rect, no `pair_bluetooth` |
| Wanting trusted-client hit-test from a v2 screen | `compile_v2()` + `TrustedClientRow`; not `compile_v2_public` |
| `TrustedClientRow` in a public document | compile error; privileged |
| Wanting Me app hit-test from a v2 screen | `compile_v2()` + `CapabilityRow`; stacked rect, no action |
| `CapabilityRow` in a public document | compile error; privileged |

Space detail, Memory review, chat, and widgets stay deferred.
Known limitations: [`vui09-known-limitations.md`](vui09-known-limitations.md).
