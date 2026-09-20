# VUI-09 verification ledger

Status: **in progress** (ADR-189). This is not Visual v1 sign-off.
Nothing in the public `.sui` 2 subset is Stable.

Production chrome: `compile_v2()` on `services/saai-shell/ui/root.sui`
(ADR-216). Frozen v1 stays `compile_v1_rollback()`. Space detail
deferred. MEM-08 omitted.

Current panther shell: `c05eedf9…` (ADR-216). Prior chrome `3850427a…` (ADR-198). ActionCard/tabs were
`5eb6a27f…` (ADR-197). Named sizes were `e8865301…` (ADR-188).
`from_safe` (ADR-195), nested tab ids (ADR-196), nested footer
rows (ADR-199), and the object hit (ADR-200) stay host. Leftover
text budget (ADR-201) is host-locked; apps grid is the device cell.
Inbox `EventRow` hits (ADR-202) stay host. Spaces `SpaceRow` hits
(ADR-203) stay host. Me `SettingRow` hits (ADR-204) stay host.
Wi-Fi `WifiRow` hits (ADR-205) stay host.
Bluetooth `BluetoothRow` hits (ADR-206) stay host.
Privileged `TrustedClientRow` hits (ADR-207) stay host.
Privileged `CapabilityRow` hits (ADR-208) stay host.
Lock / unlock cycle (ADR-209) is proven on HEAD; tap-unlock; PIN null.
Display restart (ADR-210) is proven on HEAD; native-init respawned `saai-displayd`.
Cold boot (ADR-211) is proven on HEAD; `reboot -f`; marker dropped then restored.
7-tap gallery (ADR-212) is proven on HEAD; Диагностика then Назад.
Thin tuning (ADR-213) is parked last; not next work.
List trailing rows (ADR-214) are host; `list_back` / `list_refresh` / `list_scan`.
Me flatten/scroll (ADR-215) is host; `flatten_me_rows` + `scrolled_row_rect` at offset 0.
Production `compile_v2()` (ADR-216); `layout_v2` is live `root_view`.
NOW content hits are compiled `now.sui` (ADR-217).
Inbox/Spaces/list hits are generated `compile_v2()` documents (ADR-218).
Me scroll hits are `layout_v2_scrolled` (ADR-219).
Privileged `Keyboard` is the Field-bound IME (ADR-222).

## Matrix

| Area | Cell | Status | Evidence |
|---|---|---|---|
| Build | host tests | proven | overlay `cargo test -p saai-shell` 250; `saai-ui-compiler` 37 |
| Build | pixel7 cross-build | proven | ADR-198 `3850427a…` |
| Render | Сейчас composition | proven | ObjectSummary + footer; leftover NOW cards gone (ADR-187) |
| Render | four tabs | proven | hits 135/405/675/945 y=2250 (ADR-184); Inbox tap ADR-188 |
| Render | semantic color | proven | Theme + `panel_pixel`; no production RGB (ADR-187) |
| Render | named text sizes | proven | `role_px` leftovers (ADR-188); ActionCard/tabs (ADR-197); status/keys `Label`/`Caption` (ADR-198); leftover PX below Caption (ADR-201) |
| Input | tab switch | proven | Inbox then back to Сейчас on HEAD |
| Input | keyboard | proven | ADR-029/150/161; not re-typed this slice |
| Input | interrupted key | proven | ADR-163 |
| State | empty / offline patterns | host | SurfacePattern tests; live NOW empty is «Ничего срочного» |
| A11y | reduced motion | proven | ADR-174; not toggled this slice |
| A11y | focus order | proven | ADR-162 |
| A11y | increased text on panther | proven | ADR-190; HEAD `e8865301…` 150% then restored 100; no Me tap |
| Perf | frame pace / p95 / idle | proven | ADR-172–177; not re-read `/run/saaios/shell-frame.last` this slice |
| Perf | haptic policy | proven | ADR-171/179 KeyPress-only |
| Safety | public subset gate | host | `compile_v2_public()` (ADR-185/186) |
| Render | v2 layout ≡ v1 hits | host | ADR-194, ADR-195, ADR-196, ADR-199, ADR-200, ADR-202, ADR-203, ADR-204, ADR-205, ADR-206, ADR-207, ADR-208, ADR-214, ADR-215, ADR-216, ADR-217, ADR-218, ADR-219, ADR-220, ADR-221, ADR-222; `layout_v2` nested tab ids; nested `row` footer matches `now_footer_action_rect`; `ObjectSummary` hits `open_object`; live NOW hits `now.sui`; live Inbox/Spaces/Wi-Fi/Bluetooth/trusted hits generated `compile_v2()`; live Me scroll `layout_v2_scrolled`; live apps grid `Button` matches `now_grid_rect` / `manage_app`; overlay `Field` docks above keyboard reserve; overlay decision `Button` matches `consent:accept`; privileged `Keyboard` binds Field, USB HID may replace the panel; `EventRow`/`SpaceRow`/`SettingRow`/`WifiRow`/`BluetoothRow`/`TrustedClientRow`/`CapabilityRow` match `stacked_row_rect`; trailing `row refresh`/`scan`/`back` match `stacked_trailing_rect`; Me flatten matches `flatten_me_rows` / `scrolled_row_rect`; `select_space`; `cycle_timezone`; `connect_wifi`; `pair_bluetooth`; `revoke_trusted_client`; `list_back`; empty nav/rows/object/Status invent none; `compile_v2_public` rejects privileged; production `compile_v2()` |
| Device | lock / unlock cycle | proven | ADR-209; HEAD `3850427a…`; PIN null; clock `13:58`; tap-unlock; marker restored; pid 27000→28091→28125 |
| Device | display restart | proven | ADR-210; HEAD `3850427a…`; `saai-displayd` 8323→28184; shell 28125→28190; marker on; Сейчас without lock |
| Device | cold boot | proven | ADR-211; `reboot -f`; marker dropped; `saai-displayd` 401; shell 418; clock `14:07`; tap-unlock; `saai-entityd`/`file-recv` up; marker restored; pid 501 |
| Device | daylight / indoor / dark | parked | thin tuning (ADR-213); last, not next |
| A11y | 7-tap gallery | proven | ADR-212; HEAD `3850427a…`; 7-tap `SaaiOS · сборка`; Диагностика; Назад; leave Сейчас |
| Resilience | service restart | proven | ADR-192; unlocked kill, marker on, same `e8865301…`, Сейчас without lock |
| Resilience | network / AI offline | proven | ADR-191; `wlan0` down → status `Нет сети`; Сейчас still live ObjectSummary; restored up |

## Still not Visual v1

- NOW footer and object hits come from compiled `now.sui` (ADR-217).
  Inbox, Spaces, and list hits come from generated `compile_v2()`
  (ADR-218). Me scroll hits `layout_v2_scrolled` (ADR-219). Apps grid
  hits generated `Button` tiles (ADR-220). Overlay Field and decision
  `Button`s hit `layout_v2` (ADR-221). Privileged `Keyboard` binds the
  focused Field; USB HID may replace the panel (ADR-222).
- Gallery covers fixtures; copying privileged names into an app fails
  `compile_v2_public()`, which is the gate, not a Stable API.
- ActionCard title/button use `Label`; status and tab labels use
  `Caption` (ADR-197). Status time/battery are `Label`; keys are
  `Caption` (ADR-198). They are not Title/Body. Badge, gallery
  kicker/swatch, and app-tile leftovers stay named below Caption
  (ADR-201). 7-tap gallery is proven (ADR-212). Leftover visual nits
  and panel lighting belong to thin tuning (ADR-213), not next work.
- Space detail, Memory review, chat, widgets stay deferred.
- Known limitations and the Visual v2 backlog: [`vui09-known-limitations.md`](vui09-known-limitations.md) (ADR-193).
