# VUI-09 known limitations and Visual v2 backlog

Status: **recorded** (ADR-193). This is not Visual v1 sign-off.
Nothing in the public `.sui` 2 subset is Stable.

Production chrome: `compile_v2()` on `services/saai-shell/ui/root.sui`
(ADR-216). Frozen v1 stays `compile_v1_rollback()`. Space detail deferred.
MEM-08 omitted until a shell-legal memory read exists.

Current panther shell: `c05eedf9…` (ADR-216). Prior chrome `3850427a…`
(ADR-198). Lock cycle is proven
(ADR-209). Display restart is proven (ADR-210). Cold boot is proven
(ADR-211). 7-tap gallery is proven (ADR-212). Parked last: thin tuning
(ADR-213). List trailing rows are host (ADR-214). Me flatten/scroll
is host (`flatten_me_rows` / `scrolled_row_rect`, ADR-215).
Production is `compile_v2()` (ADR-216). NOW content hits use
compiled `now.sui` (ADR-217). Inbox/Spaces/list hits generate
`compile_v2()` documents (ADR-218). Me scroll uses
`layout_v2_scrolled` (ADR-219). Apps grid hits use generated
`compile_v2()` `Button` tiles (ADR-220). Overlay Field and decision
`Button` hits use `layout_v2` (ADR-221). Privileged `Keyboard` is the
Field-bound IME; USB HID may replace the panel (ADR-222). OrbHost
hits use generated `layout_v2` (ADR-223). Diagnostic Назад hits
use generated `layout_v2` (ADR-224). NOW chrome paint reads the
same `now_view()` tree (ADR-225). Inbox/Spaces/list paint reads the
generated list trees (ADR-226). Me scroll paint reads
`layout_v2_scrolled` (ADR-227). Apps grid paint reads generated
`layout_v2` (ADR-228). Overlay paint reads generated `layout_v2`
(ADR-229). OrbHost paint reads generated `layout_v2` (ADR-230).
Diagnostic paint reads `layout_v2_scrolled` (ADR-231). Keyboard
keys, gallery page, and lock idle/wake stay formulas.

The verification ledger stays in
[`vui09-verification.md`](vui09-verification.md).

## Proven live cells this session

| Cell | Evidence |
|---|---|
| lock / unlock cycle | ADR-209; HEAD `3850427a…`; PIN null; tap-unlock; `/run/saaios/dev-no-lock` restored |
| display restart | ADR-210; `saai-displayd` 8323→28184; shell 28125→28190; marker survived |
| cold boot | ADR-211; `reboot -f`; marker dropped; displayd 401; shell 418; tap-unlock; marker restored |
| 7-tap gallery | ADR-212; 7-tap `SaaiOS · сборка`; Диагностика; Назад; leave Сейчас |

Unlocked `saai-shell` restart (ADR-192) is not a display restart.
Lock cycle is ADR-209. Display restart is ADR-210. Cold boot is
ADR-211. 7-tap gallery is ADR-212.

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
  invent no `connect_wifi`. Trailing `row refresh`/`back` dock as
  `stacked_trailing_rect` (ADR-214).
  `BluetoothRow` docks Bluetooth list stacked hits (ADR-206); Status
  rows invent no `pair_bluetooth`. Trailing `row scan`/`refresh`/`back`
  dock as `stacked_trailing_rect` (ADR-214).
  Privileged `TrustedClientRow` docks trusted-client stacked hits
  (ADR-207); Status rows invent no `revoke_trusted_client`.
  `compile_v2_public()` rejects the name. Trailing `row back` docks
  (ADR-214).
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
  Caption (ADR-201). 7-tap gallery is proven (ADR-212).
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
   kicker/swatch, and app-tile leftovers below Caption. 7-tap gallery is
   proven (ADR-212).
4. Promote public names from Experimental to Stable only after the
   Pixel 7 promotion checklist in the component library.
5. Add `SpaceDetail` / `MemoryReview` / `ChatThread` / `Widget` to the
   vocabulary only when a shell-legal consumer exists. MEM-08 stays
   omitted until that memory read exists.
6. ~~Run remaining session-blocked v1 cells.~~ Lock cycle, display
   restart, cold boot, and 7-tap gallery are proven
   (ADR-209–212).
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
    panther; do not launch. 7-tap gallery is proven (ADR-212).
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
    (`cycle_timezone`, ADR-204). Flatten/scroll is ADR-215. Do not
    attach `layout_v2()` to `build.rs` yet.
14. ~~Dock `WifiRow` so `layout_v2()` matches Wi-Fi
    `stacked_row_rect`.~~ Host: Button → `connect_wifi` (ADR-205).
    Trailing `row refresh`/`back` (ADR-214). Do not attach
    `layout_v2()` to `build.rs` yet.
15. ~~Dock `BluetoothRow` so `layout_v2()` matches Bluetooth
    `stacked_row_rect`.~~ Host: Button → `pair_bluetooth` (ADR-206).
    Trailing `row scan`/`refresh`/`back` (ADR-214). Do not attach
    `layout_v2()` to `build.rs` yet.
16. ~~Dock `TrustedClientRow` so `layout_v2()` matches trusted-client
    `stacked_row_rect`.~~ Host: Button → `revoke_trusted_client`
    (ADR-207). Privileged; `compile_v2_public` rejects. Trailing
    `row back` (ADR-214). Do not attach `layout_v2()` to `build.rs`
    yet.
17. ~~Dock `CapabilityRow` so `layout_v2()` matches Me app
    `stacked_row_rect`.~~ Host: never actionable (ADR-208).
    Privileged; `compile_v2_public` rejects. Do not tap Me apps.
    Do not attach `layout_v2()` to `build.rs` yet.
18. ~~Dock list trailing rows so `layout_v2()` matches
    `stacked_trailing_rect`.~~ Host: `list_refresh` / `list_scan` /
    `list_back` (ADR-214). Do not attach `layout_v2()` to `build.rs`
    yet.
19. ~~Dock Me flatten/scroll so `layout_v2()` matches
    `flatten_me_rows` / `scrolled_row_rect` at offset 0.~~ Host
    (ADR-215). Do not invent Me hits from empty `root.sui`.
20. ~~Switch production `build.rs` to `compile_v2()`.~~ Live
    `layout_v2` tabs (ADR-216). NOW chrome paint reads `now_view()`
    (ADR-225). Other content paint stays procedural.
21. ~~NOW footer and object hits from compiled `now.sui`.~~ Live
    (ADR-217). NOW paint uses those nodes (ADR-225).
    Inbox/Spaces/Me/lists stay procedural.
22. ~~Inbox, Spaces, and list hits from generated `compile_v2()`.~~
    Host (ADR-218). Me scroll, apps grid, and overlays stay formulas.
23. ~~Me scroll hits from `layout_v2_scrolled`.~~ Host (ADR-219).
    Apps grid and overlays stay formulas.
24. ~~Overlay Field/decision hits from `layout_v2`.~~ Host (ADR-221).
    Privileged `Keyboard` is the Field-bound IME (ADR-222). USB HID
    with `KEY_A` replaces the on-screen panel. Volume/power are not
    a keyboard.
25. ~~OrbHost hits from `layout_v2`.~~ Host (ADR-223). Gallery page
    taps stay a whole-surface formula; they are not named
    `Button`/`OrbHost` locs.
26. ~~Diagnostic Назад hits from `layout_v2`.~~ Host (ADR-224).
    `DataRow`s stay read-only. Gallery page taps and Keyboard keys
    stay their formulas.
27. ~~NOW chrome paint from `layout_v2`.~~ Live (ADR-225).
28. ~~Inbox/Spaces/list paint from `layout_v2`.~~ Host (ADR-226).
    Apps/overlay paint stay procedural.
29. ~~Me scroll paint from `layout_v2_scrolled`.~~ Host (ADR-227).
    Apps/overlay paint stay procedural.
30. ~~Apps grid paint from `layout_v2`.~~ Host (ADR-228). Overlay
    paint stays procedural.
31. ~~Overlay paint from `layout_v2`.~~ Host (ADR-229). Keyboard keys
    stay their formula.
32. ~~OrbHost paint from `layout_v2`.~~ Host (ADR-230). Do not tap
    Изменить.
33. ~~Diagnostic paint from `layout_v2_scrolled`.~~ Host (ADR-231).
    Do not 7-tap gallery.
34. Thin tuning is last (ADR-213). Physical lighting, leftover visual
    nits, and gallery-fixture completeness stay in that sprint. Do
    not schedule as next work.

Rollback: delete this page. The ledger and frozen v1 path stay.
