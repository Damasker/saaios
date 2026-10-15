# VUI-09 verification ledger

Status: **in progress** (ADR-189). This is not Visual v1 sign-off.
Nothing in the public `.sui` 2 subset is Stable.

Production chrome: `compile_v2()` on `services/saai-shell/ui/root.sui`
(ADR-216). Frozen v1 stays `compile_v1_rollback()`. Space detail
deferred. MEM-08 host chrome ADR-268; panther paint waits.

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
Me flatten/scroll (ADR-215) is host; `flatten_me_rows` + `layout_v2_scrolled`.
Production `compile_v2()` (ADR-216); `layout_v2` is live `root_view`.
NOW content hits are compiled `now.sui` (ADR-217).
Inbox/Spaces/list hits are generated `compile_v2()` documents (ADR-218).
Me scroll hits are `layout_v2_scrolled` (ADR-219).
Privileged `Keyboard` is the Field-bound IME (ADR-222).
OrbHost hits are generated `layout_v2` (ADR-223). Diagnostic Назад hits are generated `layout_v2` (ADR-224). NOW chrome paint reads `now_view()` (ADR-225). List paint reads generated `layout_v2` (ADR-226). Me scroll paint reads `layout_v2_scrolled` (ADR-227). Apps grid paint reads generated `layout_v2` (ADR-228). Overlay paint reads generated `layout_v2` (ADR-229). OrbHost paint reads generated `layout_v2` (ADR-230). Diagnostic paint reads `layout_v2_scrolled` (ADR-231). Leftover formulas stay Keyboard keys, gallery page, and lock idle/wake (ADR-232). Empty Object View header reads the same generated overlay tree; `object-public.sui` lays out through `compile_v2_public` (ADR-427). Object View `ObjectSummary` docks on that tree (ADR-428). Fact `DataRow`s dock below the summary (ADR-429). Permission `SurfacePattern` and decision facts dock on that tree (ADR-430). Consent / pair Status cards dock on `stacked_row_rect` (ADR-431). Pair fingerprint docks on that tree (ADR-432). Overlay `ContextHeader` paint reads that tree (ADR-433). Compose `ContextHeader` paint reads that tree (ADR-434). List / apps `ContextHeader` paint reads that tree (ADR-435). Object View identity / facts share the `width/22` margin (ADR-436). Apps empty `SurfacePattern` reads that tree (ADR-437). NOW empty keeps loc `now.empty` (ADR-438). Wi-Fi password compose uses surface `wifi-password` (ADR-439). Pair fingerprint wrap reads the compiled slot (ADR-440). Leftover `paint_context_header` is gone (ADR-441). Leftover `task_confirm_view` is gone (ADR-442). Leftover `now_object_summary_rect` is gone (ADR-443). Leftover `consent_view` is gone (ADR-444). Leftover `orb_view` is gone (ADR-445). Leftover live `Frame::Root` is gone (ADR-446). Leftover `draw_root` is gone (ADR-447). Leftover live `content_action_at` dispatch is gone (ADR-448). Leftover `cycle_space_lifecycle` is gone (ADR-449). Leftover `content_action_at` table is gone (ADR-450). Leftover `lock_attention_tap` is gone (ADR-451). Leftover `present_lock_surface` is gone (ADR-452). Leftover `now_grid_rect` is gone (ADR-453). Leftover `scrolled_row_rect` is gone (ADR-454). Leftover `stacked_trailing_rect` is gone (ADR-455). Leftover named chrome is closed (ADR-456). Public Search hits are host-locked (ADR-457). Public list headers match Inbox stacked top (ADR-458). Public compose omits `Keyboard` (ADR-459). Public overlay headers match Inbox stacked top (ADR-460). Public NOW header is not list stacked top (ADR-461). Public subset rejects remaining privileged names (ADR-462). Public subset rejects deferred names (ADR-463). Public summaries dock stacked rows (ADR-464). Public primitives dock stacked rows (ADR-465). Public paint primitives are not stacked rows (ADR-466). Public examples omit privileged chrome (ADR-467). Public subset rejects unknown names (ADR-468). Public NOW empty stays Fill leftover (ADR-469). Concept-board chrome reopened (ADR-470). Object related is a Button (ADR-471). Object related is one Button per SOM peer (ADR-472).

## Matrix

| Area | Cell | Status | Evidence |
|---|---|---|---|
| Build | host tests | proven | overlay `cargo test -p saai-shell`; `saai-ui-compiler` (ADR-427 empty object + public examples) |
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
| Safety | public subset gate | host | `compile_v2_public()` (ADR-185/186/427); all public examples lay out without shell internals; still Experimental |
| Render | v2 layout ≡ v1 hits | host | ADR-194, ADR-195, ADR-196, ADR-199, ADR-200, ADR-202, ADR-203, ADR-204, ADR-205, ADR-206, ADR-207, ADR-208, ADR-214, ADR-215, ADR-216, ADR-217, ADR-218, ADR-219, ADR-220, ADR-221, ADR-222, ADR-223, ADR-224, ADR-225; `layout_v2` nested tab ids; nested `row` footer matches `now_footer_action_rect`; `ObjectSummary` hits `open_object`; live NOW hits `now.sui`; live NOW paint uses `now_view()` chrome slots; live Inbox/Spaces/Wi-Fi/Bluetooth/trusted hits generated `compile_v2()`; live Me scroll `layout_v2_scrolled`; live apps grid `Button` matches `layout_v2` / `manage_app`; overlay `Field` docks above keyboard reserve; overlay decision `Button` matches `consent:accept`; privileged `Keyboard` binds Field, USB HID may replace the panel; live Orb `OrbHost` matches `orb_zone_rect`; live diagnostic `DataRow` + `row back` match `stacked_control_rect`; `EventRow`/`SpaceRow`/`SettingRow`/`WifiRow`/`BluetoothRow`/`TrustedClientRow`/`CapabilityRow` match `stacked_row_rect`; trailing `row refresh`/`scan`/`back` match `layout_v2`; Me flatten matches `layout_v2_scrolled`; `select_space`; `cycle_timezone`; `connect_wifi`; `pair_bluetooth`; `revoke_trusted_client`; `list_back`; empty nav/rows/object/Status invent none; `compile_v2_public` rejects privileged; production `compile_v2()` |
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
  focused Field; USB HID may replace the panel (ADR-222). OrbHost
  hits generated `layout_v2` (ADR-223). Diagnostic Назад hits
  generated `layout_v2` (ADR-224). NOW chrome paint reads the same
  `now_view()` nodes (ADR-225). Inbox/Spaces/Wi-Fi/Bluetooth/trusted
  paint reads the generated list trees (ADR-226). Me scroll paint
  reads `layout_v2_scrolled` (ADR-227). Apps grid paint reads
  generated `layout_v2` (ADR-228). Overlay paint reads generated
  `layout_v2` (ADR-229). OrbHost paint reads generated `layout_v2`
  (ADR-230). Diagnostic paint reads `layout_v2_scrolled` (ADR-231).
  Leftover formulas stay Keyboard keys, gallery page, and lock
  idle/wake (ADR-232). Empty Object View header reads generated
  `layout_v2` (ADR-427). Object View `ObjectSummary` reads that
  tree (ADR-428).   Fact `DataRow`s read that tree (ADR-429). Permission
  `SurfacePattern` and decision facts read that tree (ADR-430).
  Consent / pair Status cards read that tree (ADR-431).
  Pair fingerprint reads that tree (ADR-432).
  Overlay `ContextHeader` paint reads that tree (ADR-433).
  Compose `ContextHeader` paint reads that tree (ADR-434).
  List / apps `ContextHeader` paint reads that tree (ADR-435).
  Object View identity / facts share the `width/22` margin
  (ADR-436). Apps empty `SurfacePattern` reads that tree
  (ADR-437). NOW empty keeps loc `now.empty` (ADR-438).
  Wi-Fi password compose uses surface `wifi-password` (ADR-439).
  Pair fingerprint wrap reads the compiled slot (ADR-440).
  Leftover `paint_context_header` is gone (ADR-441).
  Leftover `task_confirm_view` is gone (ADR-442).
  Leftover `now_object_summary_rect` is gone (ADR-443).
  Leftover `consent_view` is gone (ADR-444).
  Leftover `orb_view` is gone (ADR-445).
  Leftover live `Frame::Root` is gone (ADR-446).
  Leftover `draw_root` is gone (ADR-447).
  Leftover live `content_action_at` dispatch is gone (ADR-448).
  Leftover `cycle_space_lifecycle` is gone (ADR-449).
  Leftover `content_action_at` table is gone (ADR-450).
  Leftover `lock_attention_tap` is gone (ADR-451).
  Leftover `present_lock_surface` is gone (ADR-452).
  Leftover `now_grid_rect` is gone (ADR-453).
  Leftover `scrolled_row_rect` is gone (ADR-454).
  Leftover `stacked_trailing_rect` is gone (ADR-455).
  Leftover named chrome is closed (ADR-456).
  Public Search hits are host-locked (ADR-457).
  Public list headers match Inbox stacked top (ADR-458).
  Public compose omits `Keyboard` (ADR-459).
  Public overlay headers match Inbox stacked top (ADR-460).
  Public NOW header is not list stacked top (ADR-461).
  Public subset rejects remaining privileged names (ADR-462).
  Public subset rejects deferred names (ADR-463).
  Public summaries dock stacked rows (ADR-464).
  Public primitives dock stacked rows (ADR-465).
  Public paint primitives are not stacked rows (ADR-466).
  Public examples omit privileged chrome (ADR-467).
  Public subset rejects unknown names (ADR-468).
  Public NOW empty stays Fill leftover (ADR-469).
  Concept-board chrome reopened (ADR-470).
  Object related is a Button (ADR-471).
  Object related is one Button per SOM peer (ADR-472).
  Intent plan steps are live Task Buttons (ADR-473).
  NOW attention rows are live projection Buttons (ADR-474).
  Locality weather is Open-Meteo telemetry (ADR-475).
  NOW weather is a live `now.weather` Button (ADR-476).
  Object OAM quick actions and composer mic stay host (ADR-477).
  NOW current task rows are live running Buttons (ADR-478).
  NOW next-action rows are live `now.next.{i}` Buttons (ADR-479).
  Space focus is a live `spaces.focus` Button (ADR-480).
  Система Android VM / notebook are Status Unavailable (ADR-481).
  Space members are live `spaces.member.{i}` Buttons (ADR-482).
  Intent plan is unavailable when the store is down (ADR-483).
  Search results are live `search.item.{i}` Buttons (ADR-484).
  Wi-Fi distinguishes adapter, offline, and unread status (ADR-485).
  Object / Intent overflow facts scroll above pinned Buttons
  (ADR-486).
  Object related offline is «Связи недоступны», not zero peers
  (ADR-487).
  NOW overflow rows scroll above the pinned footer (ADR-488).
  Space overflow members scroll above pinned Назад (ADR-489).
  Object View hides workers/queue without an Action (ADR-490).
  Search overflow results scroll above the tab strip (ADR-491).
  Inbox overflow EventRows scroll above the tab strip (ADR-492).
  Space switcher overflow scrolls above the tab strip (ADR-493).
  Система cycles lock widget disclosure (ADR-494).
  Wi-Fi overflow networks scroll above pinned Обновить (ADR-495).
  Bluetooth overflow devices scroll above pinned Искать (ADR-496).
  Trusted-client overflow scrolls above pinned Назад (ADR-497).
  Apps grid overflow tiles scroll above the tab strip (ADR-498).
  Object View history stays behind Disclosure (ADR-499).
  Intent Confirm only when AskUser exists (ADR-500).
  Intent Progress is sourced or indeterminate (ADR-501).
  Long-RU fixtures stay in tests (ADR-502).
  Host chrome on panther under dest-no-lock (ADR-503).
  Lock SIGTRAP class is one-slot lock SHM (ADR-532).
  Host 515–532 chrome is on panther (ADR-533; `efca9bd3…`).
  OSK and status-bar SHM use two slots (ADR-534).
  Main window SHM uses two slots (ADR-535).
  Host 534–535 SHM is on panther (ADR-536; `0bc48f1a…`);
  dest-no-lock stays; unlock is not proven on panther.
  Bluetooth transport is BLE or omitted (ADR-537).
  DevSurface ContextFrame source is «вручную» / «Wi-Fi»
  (ADR-538). ObjectSummary meta uses Search kind labels
  (ADR-539). Host 537–539 chrome is on panther
  (ADR-540; `39453be9…`); dest-no-lock stays; unlock is
  not proven on panther. Sistema playback is «есть» / «нет»
  (ADR-541). ObjectSummary names «Уведомление» /
  «Расписание» (ADR-542). Descriptor owner is «процесс»
  (ADR-543). Duty `cp-left` is «модем ушёл» (ADR-544).
  Host 541–544 chrome is on panther
  (ADR-545; `abd38350…`); dest-no-lock stays; unlock is
  not proven on panther. `pref` / `Pref` stay off chrome
  (ADR-546). `gpsnfw` / `NFW` stay off chrome (ADR-547).
  Host 546–547 chrome is on panther
  (ADR-548; `425fbafc…`); dest-no-lock stays; unlock is
  not proven on panther. `sgc` / `SGC` stay off chrome
  (ADR-549). `tty` / `TTY` stay off chrome (ADR-550).
  Host 549–550 chrome is on panther
  (ADR-551; `e1205a3e…`); dest-no-lock stays; unlock is
  not proven on panther. Empty forecast is «нет данных»,
  not `current` (ADR-552). Transport fail is «сеть»
  (ADR-553). Host 552–553 locality copy is on
  panther (ADR-554; `181cffbf…`); dest-no-lock stays;
  unlock is not proven on panther. Locality source is
  Open-Meteo, not `open-meteo.forecast` (ADR-555).
  Записи space id is the space name (ADR-556).
  Host 555–556 chrome is on panther
  (ADR-557; `126b72c8…`); dest-no-lock stays;
  unlock is not proven on panther.
  Диагностика space is the name, not `home`
  (ADR-558). Observation source is named
  once (ADR-559). Host 558–559 chrome is on
  panther (ADR-560; `72cba91c…`); dest-no-lock
  stays; unlock is not proven on panther.
  Sistema drops the numeric weather code
  (ADR-561). Host 561 chrome is on panther
  (ADR-562; `035db4fa…`); dest-no-lock stays;
  unlock is not proven on panther.
  No-PIN lock skips compose `ContextHeader`
  (ADR-563). Lift aborted; dest-no-lock
  restored; unlock is not proven on panther.
  Host 563 lock field is on panther
  (ADR-564; `dce4a158…`); dest-no-lock lifted;
  tap unlock and idle re-lock proven.
  displayd writes connector DPMS On after present
  (ADR-565; `2462a9e9…`); dest-no-lock stays off;
  sysfs `dpms` still reads Off.
  Scroll viewport clips `hit_test` (ADR-566).
  dest-no-lock stays off.
  Elevation token is 0 / 4 / 8 / 12 (ADR-567).
  dest-no-lock stays off.
  Button long labels wrap to two lines (ADR-568).
  dest-no-lock stays off.
  Field label and support wrap (ADR-569).
  dest-no-lock stays off.
  DataRow long content reflows (ADR-570).
  dest-no-lock stays off.
  Metric labels wrap (ADR-571).
  dest-no-lock stays off.
  Disclosure long labels wrap (ADR-572).
  dest-no-lock stays off.
  StatusIndicator reason wraps (ADR-573).
  dest-no-lock stays off.
  ContextHeader Space name wraps (ADR-574).
  dest-no-lock stays off.
  TaskSummary long titles wrap (ADR-575).
  dest-no-lock stays off.
  IntentSummary long titles wrap (ADR-576).
  dest-no-lock stays off.
  ObjectSummary long titles wrap (ADR-577).
  dest-no-lock stays off.
  AgentSummary execution titles wrap (ADR-578).
  dest-no-lock stays off.
  DecisionOverlay object names wrap (ADR-579).
  dest-no-lock stays off.
  DataRow technical values are mono and copyable (ADR-580).
  dest-no-lock stays off.
  RAT label is RAT (ADR-581).
  dest-no-lock stays off.
  unsolff stays off chrome (ADR-582).
  dest-no-lock stays off.
  unsol stays off chrome (ADR-583).
  dest-no-lock stays off.
  cellular.screen stays off chrome (ADR-584).
  dest-no-lock stays off.
  cellinfo stays off chrome (ADR-585).
  dest-no-lock stays off.
  aptime stays off chrome (ADR-586).
  dest-no-lock stays off.
  slot stays off chrome (ADR-587).
  dest-no-lock stays off.
  sigcrit stays off chrome (ADR-588).
  dest-no-lock stays off.
  smsact stays off chrome (ADR-589).
  dest-no-lock stays off.
  linkcrit stays off chrome (ADR-590).
  dest-no-lock stays off.
  smscb stays off chrome (ADR-591).
  dest-no-lock stays off.
  calllist stays off chrome (ADR-592).
  dest-no-lock stays off.
  host 566–592 chrome on panther (ADR-593;
  `74a69097…`). dest-no-lock stays off.
  gallery Button paint wraps (ADR-594).
  dest-no-lock stays off.
  gallery Field paint wraps (ADR-595).
  dest-no-lock stays off.
  gallery DataRow paint reflows (ADR-596).
  dest-no-lock stays off.
  gallery Metric paint wraps (ADR-597).
  dest-no-lock stays off.
  gallery Disclosure paint wraps (ADR-598).
  dest-no-lock stays off.
  gallery StatusIndicator paint wraps (ADR-599).
  dest-no-lock stays off.
  gallery ContextHeader paint wraps (ADR-600).
  dest-no-lock stays off.
  gallery TaskSummary paint wraps (ADR-601).
  dest-no-lock stays off.
  gallery IntentSummary paint wraps (ADR-602).
  dest-no-lock stays off.
  gallery ObjectSummary paint wraps (ADR-603).
  dest-no-lock stays off.
  gallery AgentSummary paint wraps (ADR-604).
  dest-no-lock stays off.
  gallery DecisionOverlay paint wraps (ADR-605).
  dest-no-lock stays off.
  live ObjectSummary paint wraps (ADR-606).
  dest-no-lock stays off.
  live ContextHeader paint wraps (ADR-607).
  dest-no-lock stays off.
  live Sistema Task paint wraps (ADR-608).
  dest-no-lock stays off.
  live Sistema Intent paint wraps (ADR-609).
  dest-no-lock stays off.
  live Sistema Metric paint wraps (ADR-610).
  dest-no-lock stays off.
  live Object View overlay buttons wrap (ADR-611).
  dest-no-lock stays off.
  live Object View overlay facts paint as Body (ADR-612).
  dest-no-lock stays off.
  live Sistema DataRow advances by wrap height (ADR-613).
  dest-no-lock stays off.
  host 594–613 wrap paint on `/data` (ADR-614; `f09b83e0…`).
  dest-no-lock stays off.
  live observation IPv4/IPv6/DNS/CID paint MonoBody (ADR-615).
  dest-no-lock stays off.
  live observation bearer/endpoint paint MonoBody (ADR-616).
  dest-no-lock stays off.
  DevSurface build/kernel paint MonoBody (ADR-617).
  dest-no-lock stays off.
  live Sistema `video*` nodes paint MonoBody (ADR-618).
  dest-no-lock stays off.
  leftover `overlay_decision_paint` is gone (ADR-619).
  dest-no-lock stays off.
  leftover `now_weather_paint_row` is gone (ADR-620).
  dest-no-lock stays off.
  leftover `now_weather_hit` is gone (ADR-621).
  dest-no-lock stays off.
  leftover `now_action_at` is gone (ADR-622).
  dest-no-lock stays off.
  leftover `now_attention_paint_rows` is gone (ADR-623).
  dest-no-lock stays off.
  leftover `now_attention_hit` is gone (ADR-624).
  dest-no-lock stays off.
  leftover `now_task_paint_rows` is gone (ADR-625).
  dest-no-lock stays off.
  leftover `now_task_hit` is gone (ADR-626).
  dest-no-lock stays off.
  leftover `now_next_paint_rows` is gone (ADR-627).
  dest-no-lock stays off.
  leftover `now_next_hit` is gone (ADR-628).
  dest-no-lock stays off.
  leftover `wifi_list_action_at` is gone (ADR-629).
  dest-no-lock stays off.
  leftover `bluetooth_list_action_at` is gone (ADR-630).
  dest-no-lock stays off.
  leftover `trusted_client_action_at` is gone (ADR-631).
  dest-no-lock stays off.
  leftover `object_view_hit` is gone (ADR-632).
  dest-no-lock stays off.
  leftover `search_row_at` is gone (ADR-633).
  dest-no-lock stays off.
  leftover `search_paint_rows` is gone (ADR-634).
  dest-no-lock stays off.
  leftover `inbox_row_at` is gone (ADR-635).
  dest-no-lock stays off.
  leftover `space_row_at` is gone (ADR-636).
  dest-no-lock stays off.
  leftover `space_detail_action_at` is gone (ADR-637).
  dest-no-lock stays off.
  leftover `space_focus_paint_row` is gone (ADR-638).
  dest-no-lock stays off.
  leftover `space_member_paint_rows` is gone (ADR-639).
  dest-no-lock stays off.
  leftover `object_overlay_paint` is gone (ADR-640).
  dest-no-lock stays off.
  leftover `object_view_action_at` is gone (ADR-641).
  dest-no-lock stays off.
  leftover `object_view_content` is gone (ADR-642).
  dest-no-lock stays off.
  leftover `overlay_field_v2_source` is gone (ADR-643).
  dest-no-lock stays off.
  leftover `overlay_field_rect` is gone (ADR-644).
  dest-no-lock stays off.
  leftover `now_attention_section` is gone (ADR-645).
  dest-no-lock stays off.
  leftover `now_workflow_sections` is gone (ADR-646).
  dest-no-lock stays off.
  leftover `orb_visual_state` is gone (ADR-647).
  dest-no-lock stays off.
  leftover `orb_attention_from_entities` is gone (ADR-648).
  dest-no-lock stays off.
  leftover `now_attention_v2_source` is gone (ADR-649).
  dest-no-lock stays off.
  leftover `now_footer_action_views` is gone (ADR-650).
  dest-no-lock stays off.
  leftover `now_node_rect` is gone (ADR-651).
  dest-no-lock stays off.
  leftover `space_member_rows` is gone (ADR-652).
  dest-no-lock stays off.
  leftover `diagnostic_space_copy` is gone (ADR-653).
  dest-no-lock stays off.
  leftover `scroll_surface_damage` is gone (ADR-654).
  dest-no-lock stays off.
  leftover `related_object_first_peer` is gone (ADR-655).
  dest-no-lock stays off.
  host 615–655 leftover helpers on `/data` (ADR-656; `eb3197c9…`).
  dest-no-lock stays off.
  panther frames after ADR-656 (ADR-657): unlock + Now/Spaces/Search.
  dest-no-lock stays off.
  four root tabs after respawn (ADR-658): `switched to Me`.
  dest-no-lock stays off.
  Me motion hang respawned (ADR-659). dest-no-lock stays off.
  Me row cache across motion ticks (ADR-660; host). dest-no-lock
  stays off.
  Me row cache on `/data` (ADR-661; `4a03a41b…`). dest-no-lock
  stays off.
  Me paint cache across motion ticks (ADR-662; host). dest-no-lock
  stays off.
  Me motion skip full paint (ADR-663; host). dest-no-lock stays
  off.
  Me facts on the next event-loop turn (ADR-664; host). dest-no-lock
  stays off.
  Empty Me header dock (ADR-665; host). dest-no-lock stays off.
  Sistema hang fix on `/data` (ADR-666; `ed2bc0d1…`). dest-no-lock
  stays off. `surface=me` `idle_ok=1`.
  One camp snapshot per Me facts (ADR-667; host). dest-no-lock
  stays off.
  Camp snapshot on `/data` (ADR-668; `1bb214c6…`). dest-no-lock
  stays off.
  Visible Sistema window (ADR-669; host). dest-no-lock stays off.
  Visible Sistema window on `/data` (ADR-670; `7c3b9bac…`). dest-no-lock
  stays off.
  User-confirmed Sistema (ADR-671; `7c3b9bac…`). dest-no-lock stays
  off. Search `p95_ok=1`.
  First flatten omits «Наблюдения» (ADR-672; host). dest-no-lock
  stays off.
  Deferred observations on `/data` (ADR-673; `828b6ff9…`). dest-no-lock
  stays off. `surface=me` `idle_ok=1`.
  Hold first `me_facts` (ADR-674; host). dest-no-lock stays off.
  Held facts on `/data` (ADR-675; `4e18d857…`). dest-no-lock stays
  off. `surface=me` then process `S`.
  First flatten skips live probes (ADR-676; host). dest-no-lock
  stays off.
  Chrome list on `/data` (ADR-677; `36a07194…`). dest-no-lock stays
  off. Chrome list before live probes.
  Timeout Wi-Fi `wpa_cli` (ADR-678; host). dest-no-lock stays off.
  Do not type PSK.
  Timeout on `/data` (ADR-679; `60fe8f16…`). dest-no-lock stays off.
  One `/proc` camp walk (ADR-680; host). dest-no-lock stays off.
  One `/proc` walk on `/data` (ADR-681; `ce4c374b…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  One camp-log read (ADR-682; host). dest-no-lock stays off.
  One camp-log read on `/data` (ADR-683; `bb350461…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  8 KiB supervisor tail (ADR-684; host). dest-no-lock stays off.
  8 KiB supervisor tail on `/data` (ADR-685; `f29eb476…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Defer Wi-Fi/`df` (ADR-686; host). dest-no-lock stays off.
  Do not type PSK.
  Deferred Wi-Fi/`df` on `/data` (ADR-687; `04308074…`). dest-no-lock
  stays off. `surface=me` then process `S`. Do not type PSK.
  Inline `/proc` camp classify (ADR-688; host). dest-no-lock stays
  off.
  Inline classify on `/data` (ADR-689; `0e6ea3d9…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Defer runtime JSON (ADR-690; host). dest-no-lock stays off.
  Deferred runtime JSON on `/data` (ADR-691; `8f60e55c…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Local probes on the Wi-Fi turn (ADR-692; host). dest-no-lock stays
  off.
  Local probes on `/data` (ADR-693; `f25649f7…`). dest-no-lock stays
  off. `surface=me` then process `S`.
  Stop `/proc` after owner, status, and modemd (ADR-694; host).
  dest-no-lock stays off.
  `/proc` stop on `/data` (ADR-695; `0f93f0d9…`). dest-no-lock stays
  off. `surface=me` then process `S`.
  Supervisor token on the Wi-Fi turn (ADR-696; host). dest-no-lock
  stays off.
  Supervisor token on `/data` (ADR-697; `64c3fc50…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Camp probes onto held chrome (ADR-698; host). dest-no-lock stays
  off.
  Camp onto chrome hold on `/data` (ADR-699; `f9e2d372…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Skip `/proc` cmdline unless `comm` is a camp binary (ADR-700; host).
  dest-no-lock stays off.
  `comm` filter on `/data` (ADR-701; `64d39dde…`). dest-no-lock stays
  off. `surface=me` then process `S`.
  Stream the camp owner log (ADR-702; host). dest-no-lock stays off.
  Streamed owner log on `/data` (ADR-703; `f15c170f…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Owner log on the next turn (ADR-704; host). dest-no-lock stays off.
  Owner log next turn on `/data` (ADR-705; `d7afc85b…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Reuse streamed owner-log lines (ADR-706; host). dest-no-lock stays off.
  Held owner-log lines on `/data` (ADR-707; `a74c3dc4…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Reuse camp `/proc` presence (ADR-708; host). dest-no-lock stays off.
  Held `/proc` presence on `/data` (ADR-709; `412c67bf…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Reuse net probes (ADR-710; host). dest-no-lock stays off.
  Held Wi-Fi/`df` on `/data` (ADR-711; `3be0c132…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Cellular facts grouped under Связь (ADR-712; host). dest-no-lock
  stays off.
  Grouped Связь on `/data` (ADR-713; `daef927d…`). dest-no-lock stays
  off. `surface=me` then process `S`.
  Reuse runtime JSON (ADR-714; host). dest-no-lock stays off.
  Held identity / «Наблюдения» on `/data` (ADR-715; `b77c623c…`).
  dest-no-lock stays off. `surface=me` then process `S`.
  Reuse chrome sysfs (ADR-716; host). dest-no-lock stays off.
  Held kernel / uptime / boot on `/data` (ADR-717; `95b17773…`).
  dest-no-lock stays off. `surface=me` then process `S`.
  Reuse adapter presence (ADR-718; host). dest-no-lock stays off.
  Held Wi-Fi / Bluetooth presence on `/data` (ADR-719; `90a5c1a6…`).
  dest-no-lock stays off. `surface=me` then process `S`.
  Storage grouped under Устройство (ADR-720; host). dest-no-lock stays
  off.
  Grouped `Хранилище` on `/data` (ADR-721; `7a127ccc…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Reuse locality status (ADR-722; host). dest-no-lock stays off.
  Held «Местность» on `/data` (ADR-723; `cfe15b55…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Locality on the camp turn (ADR-724; host). dest-no-lock stays off.
  Locality camp turn on `/data` (ADR-725; `e6af707f…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Chrome sysfs next turn (ADR-726; host). dest-no-lock stays off.
  Chrome sysfs next turn on `/data` (ADR-727; `7fe7ab4b…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Locality off camp (ADR-728; host). dest-no-lock stays off.
  Locality after camp on `/data` (ADR-729; `4499003c…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Local sysfs next turn (ADR-730; host). dest-no-lock stays off.
  Local sysfs next turn on `/data` (ADR-731; `6ff452f7…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Storage next turn (ADR-732; host). dest-no-lock stays off.
  Storage next turn on `/data` (ADR-733; `c52583ad…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Supervisor next turn (ADR-734; host). dest-no-lock stays off.
  Supervisor next turn on `/data` (ADR-735; `8ab3cd44…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Camera next turn (ADR-736; host). dest-no-lock stays off.
  Camera next turn on `/data` (ADR-737; `57db0be8…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Ifaces next turn (ADR-738; host). dest-no-lock stays off.
  Ifaces next turn on `/data` (ADR-739; `a1fb70b4…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Trusted next turn (ADR-740; host). dest-no-lock stays off.
  Trusted next turn on `/data` (ADR-741; `f4ae808e…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Boot next turn (ADR-742; host). dest-no-lock stays off.
  Boot next turn on `/data` (ADR-743; `7e2b6bbd…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Adapters next turn (ADR-744; host). dest-no-lock stays off.
  Adapters next turn on `/data` (ADR-745; `95821052…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  CP next turn (ADR-746; host). dest-no-lock stays off.
  CP next turn on `/data` (ADR-747; `3692690a…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Playback next turn (ADR-748; host). dest-no-lock stays off.
  Playback next turn on `/data` (ADR-749; `738b3d91…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Uptime next turn (ADR-750; host). dest-no-lock stays off.
  Uptime next turn on `/data` (ADR-751; `b523e71c…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Model next turn (ADR-752; host). dest-no-lock stays off.
  Model next turn on `/data` (ADR-753; `79e1c0fb…`). dest-no-lock
  stays off. `surface=me` then process `S`.
  Gallery page taps stay a
  whole-surface formula.
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
