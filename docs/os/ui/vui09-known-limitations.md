# VUI-09 known limitations and Visual v2 backlog

Status: **recorded** (ADR-193). This is not Visual v1 sign-off.
Nothing in the public `.sui` 2 subset is Stable.

Production chrome: `compile_v2()` on `services/saai-shell/ui/root.sui`
(ADR-216). Frozen v1 stays `compile_v1_rollback()`. Space detail deferred.
MEM-08 omitted until a shell-legal memory read exists. Host chrome is
ADR-268 (`Записи` from status); Restricted omit ADR-304; panther
`4dc19018…` (ADR-308).

Current panther shell: `4dc19018…` (ADR-308). Prior chrome `5ba2ef7b…`
(ADR-254). Lock cycle is proven
(ADR-209). Display restart is proven (ADR-210). Cold boot is proven
(ADR-211). 7-tap gallery is proven (ADR-212). Parked last: thin tuning
(ADR-213). List trailing rows are host (ADR-214). Me flatten/scroll
is host (`layout_v2_scrolled`, ADR-215).
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
Diagnostic paint reads `layout_v2_scrolled` (ADR-231). Leftover
formulas stay Keyboard keys, gallery page, and lock idle/wake
(ADR-232). Empty Object View header reads generated `layout_v2`
(ADR-427). Object View `ObjectSummary` reads that tree (ADR-428).
Fact `DataRow`s read that tree (ADR-429).
Permission `SurfacePattern` and decision facts read that tree
(ADR-430). Consent / pair Status cards read that tree (ADR-431).
Pair fingerprint reads that tree (ADR-432). Overlay
`ContextHeader` paint reads that tree (ADR-433). Compose
`ContextHeader` paint reads that tree (ADR-434). List / apps
`ContextHeader` paint reads that tree (ADR-435). Object View
identity / facts share the `width/22` margin (ADR-436). Apps empty
`SurfacePattern` reads that tree (ADR-437). NOW empty keeps loc
`now.empty` (ADR-438). Wi-Fi password compose uses surface
`wifi-password` (ADR-439). Pair fingerprint wrap
reads the compiled slot (ADR-440). Leftover
`paint_context_header` is gone (ADR-441). Leftover
`task_confirm_view` is gone (ADR-442). Leftover
`now_object_summary_rect` is gone (ADR-443). Leftover
`consent_view` is gone (ADR-444). Leftover
`orb_view` is gone (ADR-445). Leftover live
`Frame::Root` is gone (ADR-446). Leftover
`draw_root` is gone (ADR-447). Leftover
live `content_action_at` dispatch is gone (ADR-448). Leftover
`cycle_space_lifecycle` is gone (ADR-449). Leftover
`content_action_at` table is gone (ADR-450). Leftover
`lock_attention_tap` is gone (ADR-451). Leftover
`present_lock_surface` is gone (ADR-452). Leftover
`now_grid_rect` is gone (ADR-453). Leftover
`scrolled_row_rect` is gone (ADR-454). Leftover
`stacked_trailing_rect` is gone (ADR-455). Leftover
named chrome is closed (ADR-456). Public
Search hits are host-locked (ADR-457). Public
list headers match Inbox stacked top (ADR-458). Public
compose omits `Keyboard` (ADR-459). Public
overlay headers match Inbox stacked top (ADR-460). Public
NOW header is not list stacked top (ADR-461). Public
subset rejects remaining privileged names (ADR-462). Public
subset rejects deferred names (ADR-463). Public
summaries dock stacked rows (ADR-464). Public
primitives dock stacked rows (ADR-465). Public
paint primitives are not stacked rows (ADR-466). Public
examples omit privileged chrome (ADR-467). Public
subset rejects unknown names (ADR-468). Public
NOW empty stays Fill leftover (ADR-469). Concept-board
chrome reopened (ADR-470). Object related is a Button
(ADR-471). Object related is one Button per SOM peer
(ADR-472). Intent plan steps are live Task Buttons
(ADR-473). NOW attention rows are live projection Buttons
(ADR-474). NOW locality weather is a live Button (ADR-476).
Object OAM quick actions and composer mic are host (ADR-477).
NOW current task rows are live running Buttons (ADR-478).
NOW next-action rows are live `now.next.{i}` Buttons (ADR-479).
Space focus is a live `spaces.focus` Button (ADR-480).
Система Android VM / notebook are Status Unavailable (ADR-481).
Space members are live `spaces.member.{i}` Buttons (ADR-482).
Intent plan is unavailable when the store is down (ADR-483).
Search results are live `search.item.{i}` Buttons (ADR-484).
Wi-Fi distinguishes adapter, offline, and unread status (ADR-485).
Object / Intent overflow facts scroll above pinned Buttons
(ADR-486). Object related offline is «Связи недоступны», not
zero peers (ADR-487). NOW overflow rows scroll above the
pinned footer (ADR-488). Space overflow members scroll above
pinned Назад (ADR-489). Object View hides workers/queue
without an Action (ADR-490). Search overflow results scroll
above the tab strip (ADR-491). Inbox overflow EventRows scroll
above the tab strip (ADR-492). Space switcher overflow scrolls
above the tab strip (ADR-493). Система
cycles lock widget disclosure (ADR-494). Wi-Fi overflow
networks scroll above pinned Обновить (ADR-495). Bluetooth
overflow devices scroll above pinned Искать (ADR-496). Trusted-client
overflow scrolls above pinned Назад (ADR-497). Apps grid overflow
tiles scroll above the tab strip (ADR-498). Object View
history stays behind Disclosure (ADR-499). Intent
Confirm only when AskUser exists (ADR-500). Intent
Progress is sourced or indeterminate (ADR-501). Long-RU
fixtures stay in tests (ADR-502). Host chrome
on panther under dest-no-lock (ADR-503). Lock SIGTRAP
class is one-slot lock SHM (ADR-532). Host 515–532 chrome
is on panther (ADR-533; `efca9bd3…`); dest-no-lock stays;
unlock is not proven on panther. Public
`object-public.sui`, `consent-public.sui`,
`remote-pair-public.sui`, `intent-public.sui`, `apps-public.sui`,
`wifi-password-public.sui`, and `pin-setup-public.sui`
stay Experimental.

The verification ledger stays in
[`vui09-verification.md`](vui09-verification.md).

## Proven live cells this session

| Cell | Evidence |
|---|---|
| lock / unlock cycle | ADR-209; HEAD `3850427a…`; PIN null; tap-unlock; `/run/saaios/dev-no-lock` restored |
| dest-no-lock leftover lock | ADR-510; host; dead lock client kept `locked=true` after ADR-503 swap; live recovery was displayd restart; no displayd flash |
| dest-no-lock 515–532 shell | ADR-533; `efca9bd3…`; PUT+rename; displayd 26159 stayed; shell 2320; `surface=now`; marker stayed; unlock not proven |
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
  invent no `connect_wifi`. Trailing `row refresh`/`back` dock via
  compiler `v2_trailing_top` (ADR-214/455).
  `BluetoothRow` docks Bluetooth list stacked hits (ADR-206); Status
  rows invent no `pair_bluetooth`. Trailing `row scan`/`refresh`/`back`
  dock via compiler `v2_trailing_top` (ADR-214/455).
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
the `.sui` v2 vocabulary. `compile_v2_public` rejects them (ADR-463).
`WidgetCard` and surface `home` stay unknown, not deferred
(ADR-468).
Widgets may show time and charge only when a later sprint names a
legal consumer; they are not NOW chrome.

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
34. ~~Leftover formulas recorded.~~ Host (ADR-232). Keyboard keys,
    gallery page, and lock idle/wake stay formulas. Do not 7-tap.
35. Thin tuning is last (ADR-213). Physical lighting, leftover visual
    nits, and gallery-fixture completeness stay in that sprint. Do
    not schedule as next work.

Rollback: delete this page. The ledger and frozen v1 path stay.
