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
is on panther (ADR-533; `efca9bd3…`). OSK and status-bar
SHM use two slots (ADR-534). Main window SHM uses two
slots (ADR-535). Host 534–535 SHM is on panther
(ADR-536; `0bc48f1a…`). Bluetooth transport is BLE
or omitted (ADR-537). DevSurface ContextFrame
source is «вручную» / «Wi-Fi» (ADR-538). ObjectSummary
meta uses Search kind labels (ADR-539). Host 537–539 chrome
is on panther (ADR-540; `39453be9…`). Sistema playback
is «есть» / «нет» (ADR-541). ObjectSummary names
«Уведомление» / «Расписание» (ADR-542). Descriptor
owner is «процесс» (ADR-543). Duty `cp-left` is
«модем ушёл» (ADR-544). Host 541–544 chrome is on
panther (ADR-545; `abd38350…`). `pref` / `Pref` stay
off chrome (ADR-546). `gpsnfw` / `NFW` stay off
chrome (ADR-547). Host 546–547 chrome is on
panther (ADR-548; `425fbafc…`). `sgc` / `SGC` stay
off chrome (ADR-549). `tty` / `TTY` stay off
chrome (ADR-550). Host 549–550 chrome is on
panther (ADR-551; `e1205a3e…`). Empty forecast is
«нет данных», not `current` (ADR-552). Transport
fail is «сеть» (ADR-553). Host 552–553 locality
copy is on panther (ADR-554; `181cffbf…`). Locality
source is Open-Meteo, not `open-meteo.forecast`
(ADR-555). Записи space id is the space name
(ADR-556). dest-no-lock stays;
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
| dest-no-lock 534–535 shell | ADR-536; `0bc48f1a…`; PUT+rename; displayd 26159 stayed; shell 28902; `surface=now` `backend=shm`; marker stayed; unlock not proven |
| dest-no-lock 537–539 shell | ADR-540; `39453be9…`; PUT+rename; displayd 26159 stayed; shell 1169; `surface=now` `backend=shm`; marker stayed; unlock not proven |
| dest-no-lock 541–544 shell | ADR-545; `abd38350…`; PUT+rename; displayd 26159 stayed; shell 5233; `surface=now` `backend=shm`; marker stayed; unlock not proven |
| dest-no-lock 546–547 shell | ADR-548; `425fbafc…`; PUT+rename; displayd 26159 stayed; shell 7651; `surface=now` `backend=shm`; marker stayed; unlock not proven |
| dest-no-lock 549–550 shell | ADR-551; `e1205a3e…`; PUT+rename; displayd 26159 stayed; shell 8910; `surface=now` `backend=shm`; marker stayed; unlock not proven |
| dest-no-lock 552–553 shell | ADR-554; `181cffbf…`; PUT+rename; displayd 26159 stayed; shell 10132; `surface=now` `backend=shm`; marker stayed; unlock not proven |
| dest-no-lock 555–556 shell | ADR-557; `126b72c8…`; PUT+rename; displayd 26159 stayed; shell 12687; `surface=now` `backend=shm`; marker stayed; unlock not proven |
| dest-no-lock 558–559 shell | ADR-560; `72cba91c…`; PUT+rename; displayd 26159 stayed; shell 14170; `surface=now` `backend=shm`; marker stayed; unlock not proven |
| dest-no-lock 561 shell | ADR-562; `035db4fa…`; PUT+rename; displayd 26159 stayed; shell 15129; `surface=now` `backend=shm`; marker stayed; unlock not proven |
| lock lift ContextHeader abort | ADR-563; host; session lock requested; no SIGTRAP; `missing ContextHeader`; dest-no-lock restored; displayd 24060; shell 24062; `surface=now`; unlock not proven |
| dest-no-lock lift unlock | ADR-564; `dce4a158…`; PUT+rename; displayd 24060 stayed; shell 26048; dest-no-lock removed; `surface=lock`; tap unlock; idle re-lock |
| displayd connector DPMS | ADR-565; `2462a9e9…`; PUT+rename; displayd 2694; shell 2696; `connector DPMS On`; `surface=lock`; dest-no-lock off; sysfs `dpms` still Off |
| scroll viewport hit_test | ADR-566; host; `Node::scroll` clips hits; screens not migrated; dest-no-lock stays off |
| elevation token | ADR-567; host; Flat/Rest/Raised/Overlay at 0/4/8/12; no drop shadow; dest-no-lock stays off |
| Button label wrap | ADR-568; host; two lines; full name stays; dest-no-lock stays off |
| gallery Button wrap paint | ADR-594; host; `label_lines`; dest-no-lock stays off |
| gallery Field wrap paint | ADR-595; host; label/support wrap; editor one line; dest-no-lock stays off |
| gallery DataRow wrap paint | ADR-596; host; primary/secondary reflow; value one line; dest-no-lock stays off |
| gallery Metric wrap paint | ADR-597; host; label/verified wrap; value one line; dest-no-lock stays off |
| gallery Disclosure wrap paint | ADR-598; host; `label_lines`; chevron stays; dest-no-lock stays off |
| gallery StatusIndicator wrap paint | ADR-599; host; reason below label; compact hides reason; dest-no-lock stays off |
| gallery ContextHeader wrap paint | ADR-600; host; Space name wraps; section title one line; dest-no-lock stays off |
| gallery TaskSummary wrap paint | ADR-601; host; title wraps; related stays separate; dest-no-lock stays off |
| gallery IntentSummary wrap paint | ADR-602; host; title wraps; «Нет задачи» stays; dest-no-lock stays off |
| gallery ObjectSummary wrap paint | ADR-603; host; title wraps; meta one line; dest-no-lock stays off |
| gallery AgentSummary wrap paint | ADR-604; host; execution title wraps; «Нет исполнения» stays; dest-no-lock stays off |
| gallery DecisionOverlay wrap paint | ADR-605; host; object name wraps; facts stay separate; dest-no-lock stays off |
| live ObjectSummary wrap paint | ADR-606; host; title wraps; meta one line; dest-no-lock stays off |
| live ContextHeader wrap paint | ADR-607; host; Space name wraps; section title one line; dest-no-lock stays off |
| live TaskSummary wrap paint | ADR-608; host; title wraps; related stays separate; dest-no-lock stays off |
| live IntentSummary wrap paint | ADR-609; host; title wraps; «Нет задачи» stays; dest-no-lock stays off |
| live Metric wrap paint | ADR-610; host; label wraps; value one line; dest-no-lock stays off |
| live DecisionOverlay button wrap paint | ADR-611; host; accept/decline wrap; heading unpainted; dest-no-lock stays off |
| live DecisionOverlay facts Body | ADR-612; host; overlay facts Body; other facts Caption; dest-no-lock stays off |
| live Sistema DataRow wrap height | ADR-613; host; cursor uses wrap paint height; dest-no-lock stays off |
| shell 594–613 wrap paint | ADR-614; `f09b83e0…` on `/data`; dest-no-lock stays off; lock surface |
| live technical IDs MonoBody | ADR-615; host; IPv4/IPv6/DNS/CID MonoBody; no copy; dest-no-lock stays off |
| live bearer/endpoint MonoBody | ADR-616; host; Канал/Дескриптор stay; no copy; dest-no-lock stays off |
| diagnostic build/kernel MonoBody | ADR-617; host; Сборка/Ядро MonoBody; space Body; dest-no-lock stays off |
| live camera nodes MonoBody | ADR-618; host; `video*` MonoBody; empty stays Body; dest-no-lock stays off |
| leftover overlay_decision_paint | ADR-619; host; helper gone; hits stay; dest-no-lock stays off |
| leftover now_weather_paint_row | ADR-620; host; wrapper gone; scrolled paint stays; dest-no-lock stays off |
| leftover now_weather_hit | ADR-621; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover now_action_at | ADR-622; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover now_attention_paint_rows | ADR-623; host; wrapper gone; scrolled paint stays; dest-no-lock stays off |
| leftover now_attention_hit | ADR-624; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover now_task_paint_rows | ADR-625; host; wrapper gone; scrolled paint stays; dest-no-lock stays off |
| leftover now_task_hit | ADR-626; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover now_next_paint_rows | ADR-627; host; wrapper gone; scrolled paint stays; dest-no-lock stays off |
| leftover now_next_hit | ADR-628; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover wifi_list_action_at | ADR-629; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover bluetooth_list_action_at | ADR-630; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover trusted_client_action_at | ADR-631; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover object_view_hit | ADR-632; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover search_row_at | ADR-633; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover search_paint_rows | ADR-634; host; wrapper gone; scrolled paint stays; dest-no-lock stays off |
| leftover inbox_row_at | ADR-635; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover space_row_at | ADR-636; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover space_detail_action_at | ADR-637; host; wrapper gone; scrolled hits stay; dest-no-lock stays off |
| leftover space_focus_paint_row | ADR-638; host; wrapper gone; scrolled paint stays; dest-no-lock stays off |
| leftover space_member_paint_rows | ADR-639; host; wrapper gone; scrolled paint stays; dest-no-lock stays off |
| leftover object_overlay_paint | ADR-640; host; wrapper gone; scrolled paint stays; dest-no-lock stays off |
| leftover object_view_action_at | ADR-641; host; helper gone; scrolled hits stay; dest-no-lock stays off |
| leftover object_view_content | ADR-642; host; wrapper gone; content_with stays; dest-no-lock stays off |
| leftover overlay_field_v2_source | ADR-643; host; wrapper gone; source_with stays; dest-no-lock stays off |
| leftover overlay_field_rect | ADR-644; host; wrapper gone; rect_with stays; dest-no-lock stays off |
| leftover now_attention_section | ADR-645; host; wrapper gone; section_with stays; dest-no-lock stays off |
| leftover now_workflow_sections | ADR-646; host; wrapper gone; sections_with stays; dest-no-lock stays off |
| leftover orb_visual_state | ADR-647; host; wrapper gone; state_with stays; dest-no-lock stays off |
| leftover orb_attention_from_entities | ADR-648; host; wrapper gone; orb_attention stays; dest-no-lock stays off |
| leftover now_attention_v2_source | ADR-649; host; alias gone; body_v2_source stays; dest-no-lock stays off |
| leftover now_footer_action_views | ADR-650; host; wrapper gone; views_from stays; dest-no-lock stays off |
| leftover now_node_rect | ADR-651; host; alias gone; v2_named_rect stays; dest-no-lock stays off |
| leftover space_member_rows | ADR-652; host; alias gone; search_rows stays; dest-no-lock stays off |
| leftover diagnostic_space_copy | ADR-653; host; alias gone; space_display_name stays; dest-no-lock stays off |
| leftover scroll_surface_damage | ADR-654; host; alias gone; root_content_rect stays; dest-no-lock stays off |
| leftover related_object_first_peer | ADR-655; host; helper gone; peer_ids stays; dest-no-lock stays off |
| shell 615–655 leftover helpers | ADR-656; `eb3197c9…` on `/data`; dest-no-lock stays off; lock surface |
| shell 656 panther frames | ADR-657; unlock + Now/Spaces/Search commits; Me log pending; dest-no-lock stays off |
| shell 656 four tabs | ADR-658; Now/Spaces/Search/Me; Object View ate tabs until respawn; dest-no-lock stays off |
| Me motion hang | ADR-659; pid 12671 `R` motion/me idle_ok=0; kill+respawn 3916 lock; dest-no-lock stays off |
| Me row cache | ADR-660; host; `me_all_rows` reuses cache across Selection/`tick_motion`; dest-no-lock stays off |
| Me cache flash | ADR-661; `4a03a41b…` on `/data`; dest-no-lock stays off; lock surface |
| Me paint cache | ADR-662; host; compiled Sistema layout reused across Selection/`tick_motion`; dest-no-lock stays off |
| Me motion skip | ADR-663; host; cached Sistema is not full-painted on motion ticks; dest-no-lock stays off |
| Me facts next turn | ADR-664; host; first Sistema paint is chrome; `me_facts` waits one loop; dest-no-lock stays off |
| empty Me header | ADR-665; host; empty `me_v2_source` keeps `ContextHeader` via `me.pending`; dest-no-lock stays off |
| Sistema hang fix | ADR-666; `ed2bc0d1…` on `/data`; first Me paint chrome then facts; dest-no-lock stays off |
| camp snapshot | ADR-667; host; one `/proc` camp walk per Me facts; dest-no-lock stays off |
| camp flash | ADR-668; `1bb214c6…` on `/data`; first Me fill still paint-heavy; dest-no-lock stays off |
| Me window | ADR-669; host; compile only visible Sistema rows; dest-no-lock stays off |
| Me window flash | ADR-670; `7c3b9bac…` on `/data`; first Me fill still fact-heavy; dest-no-lock stays off |
| Sistema works | ADR-671; `7c3b9bac…`; user-confirmed; Search p95_ok; warm Me switch slept; dest-no-lock stays off |
| Me obs next turn | ADR-672; host; first flatten omits «Наблюдения»; full list next loop; dest-no-lock stays off |
| Me obs flash | ADR-673; `828b6ff9…` on `/data`; `surface=me` idle_ok; first facts still long; dest-no-lock stays off |
| Me facts hold | ADR-674; host; hold first `me_facts`; skip idle mid-fill; dest-no-lock stays off |
| Me facts hold flash | ADR-675; `4e18d857…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| Me chrome facts | ADR-676; host; first flatten skips `wpa_cli`/`df`/`/proc`; dest-no-lock stays off |
| Me chrome flash | ADR-677; `36a07194…` on `/data`; chrome list before live probes; dest-no-lock stays off |
| Wi-Fi cli hang | ADR-678; host; `wpa_cli` 400 ms timeout; connect next loop; do not type PSK; dest-no-lock stays off |
| Wi-Fi cli flash | ADR-679; `60fe8f16…` on `/data`; unlock slept; do not type PSK; dest-no-lock stays off |
| proc camp one pass | ADR-680; host; one `/proc` walk for owner/endpoint/status; dest-no-lock stays off |
| proc camp flash | ADR-681; `ce4c374b…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| camp log one read | ADR-682; host; one owner-log read for all Sistema camp fields; dest-no-lock stays off |
| camp log flash | ADR-683; `bb350461…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| supervisor tail | ADR-684; host; 8 KiB `supervise=` tail instead of 256 KiB; dest-no-lock stays off |
| supervisor flash | ADR-685; `f29eb476…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| Me net next turn | ADR-686; host; camp facts skip `wpa_cli`/`df`; do not type PSK; dest-no-lock stays off |
| Me net flash | ADR-687; `04308074…` on `/data`; `surface=me` then process `S`; do not type PSK; dest-no-lock stays off |
| proc camp inline | ADR-688; host; classify `/proc` camp presence without a cmdline `Vec`; dest-no-lock stays off |
| proc camp inline flash | ADR-689; `0e6ea3d9…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| Me runtime next turn | ADR-690; host; camp facts skip runtime JSON; dest-no-lock stays off |
| Me runtime flash | ADR-691; `8f60e55c…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| Me local with net | ADR-692; host; camp facts skip Bluetooth/v4l/ifaces/keys; dest-no-lock stays off |
| Me local flash | ADR-693; `f25649f7…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| proc camp early exit | ADR-694; host; stop `/proc` after owner, `sit-sim-status`, and `saai-modemd`; dest-no-lock stays off |
| proc camp early-exit flash | ADR-695; `0f93f0d9…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| supervisor with net | ADR-696; host; camp facts skip supervisor tail; dest-no-lock stays off |
| supervisor net flash | ADR-697; `64c3fc50…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| camp on chrome hold | ADR-698; host; apply `/proc` + camp log onto held chrome facts; dest-no-lock stays off |
| camp chrome flash | ADR-699; `f9e2d372…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| proc comm filter | ADR-700; host; skip `/proc` cmdline unless `comm` is a camp binary; dest-no-lock stays off |
| proc comm flash | ADR-701; `64d39dde…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| camp log stream | ADR-702; host; stream owner log; flatten uses held fact lines; dest-no-lock stays off |
| camp log stream flash | ADR-703; `f15c170f…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| camp log next turn | ADR-704; host; camp facts skip owner log; fill on the next turn; dest-no-lock stays off |
| camp log next flash | ADR-705; `d7afc85b…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| camp log reuse | ADR-706; host; recapture uses held owner-log lines; skip re-open; dest-no-lock stays off |
| camp log reuse flash | ADR-707; `a74c3dc4…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| camp presence reuse | ADR-708; host; recapture uses held `/proc` presence; skip the walk; dest-no-lock stays off |
| camp presence flash | ADR-709; `412c67bf…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| net reuse | ADR-710; host; recapture uses held Wi-Fi/`df`; skip shell-out; dest-no-lock stays off |
| net reuse flash | ADR-711; `3be0c132…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| Me cellular groups | ADR-712; host; camp tokens under Связь; `Сотовая сеть` stays the summary; dest-no-lock stays off |
| Me cellular groups flash | ADR-713; `daef927d…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| runtime reuse | ADR-714; host; recapture uses held identity / «Наблюдения»; skip JSON; dest-no-lock stays off |
| runtime reuse flash | ADR-715; `b77c623c…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| chrome reuse | ADR-716; host; recapture uses held kernel / uptime / boot; skip sysfs; dest-no-lock stays off |
| chrome reuse flash | ADR-717; `95b17773…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| adapter reuse | ADR-718; host; recapture uses held Wi-Fi / Bluetooth presence; skip sysfs; dest-no-lock stays off |
| adapter reuse flash | ADR-719; `90a5c1a6…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| Me storage group | ADR-720; host; `Хранилище` under Устройство; `Это устройство` stays the model; dest-no-lock stays off |
| Me storage group flash | ADR-721; `7a127ccc…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| locality reuse | ADR-722; host; recapture uses held «Местность»; skip widget JSON; dest-no-lock stays off |
| locality reuse flash | ADR-723; `cfe15b55…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| locality next turn | ADR-724; host; chrome skip widget JSON; fill on the camp turn; dest-no-lock stays off |
| locality next flash | ADR-725; `e6af707f…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| chrome next turn | ADR-726; host; first paint skip kernel / uptime / boot / adapters; fill next loop; dest-no-lock stays off |
| chrome next flash | ADR-727; `7fe7ab4b…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| locality off camp | ADR-728; host; camp skip widget JSON; fill on the next loop; dest-no-lock stays off |
| locality off camp flash | ADR-729; `4499003c…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| local sysfs next turn | ADR-730; host; net skip v4l / ifaces / paired / trusted; fill next loop; dest-no-lock stays off |
| local sysfs next flash | ADR-731; `6ff452f7…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| storage next turn | ADR-732; host; net skip `df`; fill «Хранилище» next loop; dest-no-lock stays off |
| storage next flash | ADR-733; `c52583ad…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| supervisor next turn | ADR-734; host; net skip boot-log tail; fill next loop; dest-no-lock stays off |
| supervisor next flash | ADR-735; `8ab3cd44…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| camera next turn | ADR-736; host; local skip v4l; fill `video*` next loop; dest-no-lock stays off |
| camera next flash | ADR-737; `57db0be8…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| ifaces next turn | ADR-738; host; local skip net class / `ip`; fill rmnet next loop; dest-no-lock stays off |
| ifaces next flash | ADR-739; `a1fb70b4…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| trusted next turn | ADR-740; host; local skip authorized_keys; fill count next loop; dest-no-lock stays off |
| trusted next flash | ADR-741; `f4ae808e…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| boot next turn | ADR-742; host; chrome skip bootconfig; fill slot / attempts next loop; dest-no-lock stays off |
| boot next flash | ADR-743; `7e2b6bbd…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| adapters next turn | ADR-744; host; chrome skip wlan0 / hci0; fill next loop; dest-no-lock stays off |
| adapters next flash | ADR-745; `95821052…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| CP next turn | ADR-746; host; camp skip CP sysfs; fill next loop; dest-no-lock stays off |
| CP next flash | ADR-747; `3692690a…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| playback next turn | ADR-748; host; chrome skip tinyplay / tone; fill next loop; dest-no-lock stays off |
| playback next flash | ADR-749; `738b3d91…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| uptime next turn | ADR-750; host; chrome skip `/proc/uptime`; fill next loop; dest-no-lock stays off |
| uptime next flash | ADR-751; `b523e71c…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| model next turn | ADR-752; host; chrome skip device-tree model; fill next loop; dest-no-lock stays off |
| model next flash | ADR-753; `79e1c0fb…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| kernel next turn | ADR-754; host; chrome skip `/proc/sys/kernel/osrelease`; fill next loop; dest-no-lock stays off |
| kernel next flash | ADR-755; `03012fd5…` on `/data`; `surface=me` then process `S`; dest-no-lock stays off |
| Field label wrap | ADR-569; host; label and support two lines; editor one line; dest-no-lock stays off |
| DataRow wrap | ADR-570; host; primary/secondary two lines; value one line; dest-no-lock stays off |
| Metric wrap | ADR-571; host; label two lines; value one line; dest-no-lock stays off |
| Disclosure wrap | ADR-572; host; two lines; hit 48; dest-no-lock stays off |
| StatusIndicator wrap | ADR-573; host; reason below label; compact hides reason; dest-no-lock stays off |
| ContextHeader wrap | ADR-574; host; Space name two lines; heading stays full; dest-no-lock stays off |
| TaskSummary wrap | ADR-575; host; title two lines; related stays separate; dest-no-lock stays off |
| IntentSummary wrap | ADR-576; host; title two lines; «Нет задачи» stays; dest-no-lock stays off |
| ObjectSummary wrap | ADR-577; host; title two lines; meta one line; dest-no-lock stays off |
| AgentSummary wrap | ADR-578; host; execution two lines; unassigned one caption; dest-no-lock stays off |
| DecisionOverlay wrap | ADR-579; host; object two lines; facts stay separate; dest-no-lock stays off |
| DataRow technical | ADR-580; host; MonoBody value; copy is not navigation; dest-no-lock stays off |
| RAT label | ADR-581; host; `cellular.rat` is RAT; radio stays «Радио»; dest-no-lock stays off |
| unsolff off chrome | ADR-582; host; no row; fact still read; dest-no-lock stays off |
| unsol off chrome | ADR-583; host; no row; fact still read; dest-no-lock stays off |
| screen off chrome | ADR-584; host; no cellular.screen row; device «Экран» stays; dest-no-lock stays off |
| cellinfo off chrome | ADR-585; host; no row; fact still read; dest-no-lock stays off |
| aptime off chrome | ADR-586; host; no row; fact still read; dest-no-lock stays off |
| slot off chrome | ADR-587; host; no row; fact still read; dest-no-lock stays off |
| sigcrit off chrome | ADR-588; host; no row; fact still read; dest-no-lock stays off |
| smsact off chrome | ADR-589; host; no row; fact still read; dest-no-lock stays off |
| linkcrit off chrome | ADR-590; host; no cellular.linkcrit row; device «Связь» stays; dest-no-lock stays off |
| smscb off chrome | ADR-591; host; no row; fact still read; dest-no-lock stays off |
| calllist off chrome | ADR-592; host; no row; fact still read; dest-no-lock stays off |
| shell 566–592 on panther | ADR-593; `74a69097…`; dest-no-lock stays off; lock surface; prev `dce4a158…` |
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
