# VUI-09 known limitations and Visual v2 backlog

Status: **recorded** (ADR-193). This is not Visual v1 sign-off.
Nothing in the public `.sui` 2 subset is Stable.

Production chrome: `compile()` on `services/saai-shell/ui/root.sui`.
Do not point `build.rs` at `compile_v2()`. Space detail deferred.
MEM-08 omitted until a shell-legal memory read exists.

Current panther shell: `3850427a…` (ADR-198). Lock cycle is proven
(ADR-209). This page does not reboot or kill `saai-displayd`.

The verification ledger stays in
[`vui09-verification.md`](vui09-verification.md).

## Proven live cells this session

| Cell | Evidence |
|---|---|
| lock / unlock cycle | ADR-209; HEAD `3850427a…`; PIN null; tap-unlock; `/run/saaios/dev-no-lock` restored |

## Session-blocked live cells

These stay **open**. They are not proven by this pass.

| Cell | Why this session does not run it |
|---|---|
| display restart | Do not kill `saai-displayd` except recovery. |
| cold boot | `/run` is lost across reboot; the no-lock marker would drop. |
| daylight / indoor / dark | No booth this session. |
| 7-tap gallery | Not a DevSurface-chrome slice. Do not 7-tap. |

Unlocked `saai-shell` restart (ADR-192) is not a display restart.
Lock cycle is ADR-209, not ADR-192.

## Compiler and layout

- `layout_v2()` matches v1 tab hits for public NOW (ADR-194). Tab
  height comes from `EdgeInsets::from_safe` (ADR-195). Nested `tab`
  ids on `BottomNavigation` own the v2 strip (ADR-196); empty
  navigation invents no v1 hits. Nested `row` ids dock the NOW
  footer (ADR-199); empty screens invent no footer hits. Live chrome
  still comes from `layout_v1_root()` over `compile_v1_rollback()`.
  `ObjectSummary` docks the NOW object hit (ADR-200); a screen
  without it invents none. `EventRow` docks Inbox stacked hits
  (ADR-202); Status rows invent no `open_object`.   `SpaceRow` docks
  Spaces stacked hits (ADR-203); Status rows invent no
  `select_space`. `SettingRow` docks Me stacked hits
  (ADR-204); Status rows and `SystemSection` invent no
  interned `loc`. Live Me flatten/scroll stays procedural.
  `WifiRow` docks Wi-Fi list stacked hits (ADR-205); Status rows
  invent no `connect_wifi`. Trailing Обновить/Назад stay procedural.
  `BluetoothRow` docks Bluetooth list stacked hits (ADR-206); Status
  rows invent no `pair_bluetooth`. Trailing Искать/Обновить/Назад
  stay procedural.
  Privileged `TrustedClientRow` docks trusted-client stacked hits
  (ADR-207); Status rows invent no `revoke_trusted_client`.
  `compile_v2_public()` rejects the name. Trailing Назад stays
  procedural.
  Privileged `CapabilityRow` docks Me app stacked hits (ADR-208);
  the row is never actionable. `compile_v2_public()` rejects the
  name.
- `compile_v2_public()` is the third-party gate. It is Experimental,
  not Stable.
- Top `SafeInsets` is the status layer (ADR-112), not tree padding.
  `layout()` stays physical. `EdgeInsets::from_safe` is the conversion.
- `root.sui` stays `sui 1`. Matching tab hits is not permission to
  switch `build.rs` to `compile_v2()`.

## Leftover paint

- ActionCard title/button are `Label`; status and tab labels are
  `Caption` (ADR-197). Status time/battery are `Label`; keys, Orb
  menu, related line, and gallery heading are `Caption` (ADR-198).
  Not Title (72). `TAB_BADGE_PX`, `GALLERY_KICKER_PX`,
  `GALLERY_SWATCH_PX`, and `APP_TILE_LABEL_PX` stay named below
  Caption (ADR-201). Do not 7-tap to see kicker/swatch.
- Gallery fixtures may show privileged rows. Copying those names into
  an app fails `compile_v2_public()`.

## Deferred surfaces

`SpaceDetail`, `MemoryReview`, `ChatThread`, and `Widget` stay outside
the `.sui` v2 vocabulary. Widgets may show time and charge only when a
later sprint names a legal consumer; they are not NOW chrome.

## Visual v2 backlog

Ordered. Items 1–3 and 7–17 are done
(ADR-194/195/197–198/196/199/200/201/202/203/204/205/206/207/208); production still v1.

1. ~~Emit layout and hit-test from `compile_v2()` that match
   `layout_v1_root()` for public NOW tabs.~~ Host: `layout_v2()`
   (ADR-194). Do not attach it to `build.rs` yet.
2. ~~Reconcile `Node` physical pixels with logical `SafeInsets`.~~
   Host: `EdgeInsets::from_safe` (ADR-195). Top inset stays a layer.
3. ~~Keep leftover ActionCard/tab sizes on an explicit named list, or
   map them onto `TextRole` in a paint-normalization slice.~~ Flashed:
   Label/Caption, not Title (ADR-197/198). ADR-201 locks badge, gallery
   kicker/swatch, and app-tile leftovers below Caption. Do not 7-tap.
4. Promote public names from Experimental to Stable only after the
   Pixel 7 promotion checklist in the component library.
5. Add `SpaceDetail` / `MemoryReview` / `ChatThread` / `Widget` to the
   vocabulary only when a shell-legal consumer exists. MEM-08 stays
   omitted until that memory read exists.
6. Run remaining session-blocked v1 cells (display restart, cold
   boot, daylight booth, 7-tap gallery) as operator-approved device
   work. Lock cycle is proven (ADR-209). They are Visual v1 gates,
   not v2 features.
7. ~~Name tabs inside the v2 grammar so `layout_v2()` does not borrow
   `compile_v1_rollback()` ids.~~ Host: nested `tab` (ADR-196). Do not
   attach `layout_v2()` to `build.rs` yet.
8. ~~Name NOW footer destinations so `layout_v2()` matches
   `now_footer_action_rect`.~~ Host: nested `row` (ADR-199). Do not
   attach `layout_v2()` to `build.rs` yet.
9. ~~Dock `ObjectSummary` so `layout_v2()` matches
   `now_object_summary_rect`.~~ Host: `open_object` (ADR-200).
   Lifecycle/trailing stay paint-side. Do not attach `layout_v2()`
   to `build.rs` yet.
10. ~~Keep remaining leftover text sizes named below Caption.~~ Host:
    `leftover_text_sizes_stay_below_caption` (ADR-201). Apps grid on
    panther; do not launch; do not 7-tap.
11. ~~Dock `EventRow` so `layout_v2()` matches Inbox
    `stacked_row_rect`.~~ Host: Button → `open_object` (ADR-202).
    Do not tap live Inbox rows. Do not attach `layout_v2()` to
    `build.rs` yet.
12. ~~Dock `SpaceRow` so `layout_v2()` matches Spaces
    `stacked_row_rect`.~~ Host: Button → `select_space:<loc>`
    (ADR-203). Do not tap live Spaces rows. Do not attach
    `layout_v2()` to `build.rs` yet.
13. ~~Dock `SettingRow` so `layout_v2()` matches Me
    `stacked_row_rect`.~~ Host: Button → interned `loc`
    (`cycle_timezone`, ADR-204). Do not tap live Me rows. Live
    flatten/scroll stays procedural. Do not attach `layout_v2()` to
    `build.rs` yet.
14. ~~Dock `WifiRow` so `layout_v2()` matches Wi-Fi
    `stacked_row_rect`.~~ Host: Button → `connect_wifi` (ADR-205).
    Do not open the live list. Trailing Обновить/Назад stay
    procedural. Do not attach `layout_v2()` to `build.rs` yet.
15. ~~Dock `BluetoothRow` so `layout_v2()` matches Bluetooth
    `stacked_row_rect`.~~ Host: Button → `pair_bluetooth` (ADR-206).
    Do not open the live list. Do not tap Сопряжь. Trailing
    Искать/Обновить/Назад stay procedural. Do not attach
    `layout_v2()` to `build.rs` yet.
16. ~~Dock `TrustedClientRow` so `layout_v2()` matches trusted-client
    `stacked_row_rect`.~~ Host: Button → `revoke_trusted_client`
    (ADR-207). Privileged; `compile_v2_public` rejects. Do not open
    the live list. Do not tap Отозвать. Trailing Назад stays
    procedural. Do not attach `layout_v2()` to `build.rs` yet.
17. ~~Dock `CapabilityRow` so `layout_v2()` matches Me app
    `stacked_row_rect`.~~ Host: never actionable (ADR-208).
    Privileged; `compile_v2_public` rejects. Do not tap Me apps.
    Do not attach `layout_v2()` to `build.rs` yet.

Rollback: delete this page. The ledger and `compile()` path stay.
