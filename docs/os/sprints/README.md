# Дорожная карта спринтов SaaiOS

Roadmap описывает порядок доказуемых вертикальных результатов. Он не является
обещанием дат: каждый спринт закрывается по критериям, а не по количеству
написанного кода. Процесс и Definition of Done находятся в
[DEVELOPMENT_PROCESS.md](../DEVELOPMENT_PROCESS.md).

После S01 зафиксирован docs-only Sprint 0 mapping для Cognitive Core и Work
Scheduler (без кода):
[cognitive-core-work-scheduler-mapping.md](../architecture/cognitive-core-work-scheduler-mapping.md)
— границы Planner/Scheduler, один workflow store + derived ready set, Task
Ledger = audit-log, UDS/TLS; привязка к S09–S11.

S00–S32 закрыли базовый трек — рабочий телефон под собственной ОС.
Одноустройственная часть
[HIA roadmap](HIA-ROADMAP.md) также реализована с зафиксированными
ограничениями; голос заблокирован аппаратной авторизацией AoC, а PCE ждёт
второго физического runtime-узла.

Текущее направление на устройство — один путь
[PIXEL-PATH.md](PIXEL-PATH.md), не восемь параллельных `*-10` weekend'ов.

**Shell queue:** [Visual Language v1](../architecture/visual-language-v1.md)
([VISUAL-ROADMAP.md](VISUAL-ROADMAP.md)). VUI-00…06 закрыты на host;
VUI-04 remainder и VUI-06 на panther; **VUI-07 закрыт** на panther
(`693e77c7…`; Space detail deferred, MEM-08 omitted). **VUI-08 закрыт**
на panther (`37a8014d…`; ADR-167–179):
ADR-167 MotionClock + keyboard micro hold on panther.
ADR-168 tab Selection hold on panther.
ADR-169 compose Field Focus on panther.
ADR-170 Orb ActivityPulse on panther.
ADR-171 haptic policy on panther.
ADR-172 FramePace commit log on panther.
ADR-173 50 ms scroll p95 on panther.
ADR-174 reduced motion is immediate on panther.
ADR-175 FramePace surface traces on panther.
ADR-176 first visible down commit on panther.
ADR-177 idle main-surface seq on panther.
ADR-178 dma-buf / wl_shm backend tag on panther.
ADR-179 haptic acceptance closeout on panther.
ADR-180 `.sui` v2 vocabulary (host; v1 root chrome unchanged).
ADR-181 `.sui` v2 grammar (host; `compile()` stays v1).
ADR-182 `.sui` v2 component properties (host; `root.sui` unchanged).
ADR-183 `.sui` v1 rollback artifact (`compile()` on `root.sui`).
ADR-184 shared v1 layout/hit-test (`layout_v1_root()`, not `compile_v2()`).
ADR-185 public `.sui` v2 subset (`compile_v2_public()`, privileged gated).
ADR-186 public API docs, NOW example, and stability labels.
ADR-187 close production color literals and leftover NOW chrome.
ADR-188 name leftover `draw_text` sizes (`role_px`).
ADR-189 VUI-09 verification ledger (not Visual v1 sign-off).
ADR-190 increased text 150% on panther HEAD, then restored 100%.
ADR-191 live radio-off on panther HEAD (`wlan0` down → `Нет сети`, then up).
ADR-192 unlocked shell restart on panther HEAD (same `e8865301…`).
ADR-193 VUI-09 known limitations and Visual v2 backlog (not Visual v1 sign-off).
ADR-194 `layout_v2` public NOW tab hits match v1 (host; `compile()` stays v1).
ADR-195 `EdgeInsets::from_safe` converts logical SafeInsets (host; top stays a layer).
ADR-196 nested `tab` ids on `BottomNavigation`; `layout_v2` does not borrow v1 (host).
ADR-197 ActionCard/tab labels use `Label`/`Caption`, not Title (paint; flashes).
ADR-198 status time/battery `Label`, keys `Caption` (paint; flashes).
ADR-199 nested `row` footer ids; `layout_v2` matches `now_footer_action_rect` (host).
ADR-200 `ObjectSummary` docks as the NOW object hit (`open_object`, host).
ADR-201 leftover text sizes stay below Caption; apps grid on panther, no launch.
ADR-202 `EventRow` docks as Inbox stacked hits (`open_object`, host).
ADR-203 `SpaceRow` docks as Spaces stacked hits (`select_space:<loc>`, host).
ADR-204 `SettingRow` docks as Me stacked hits (interned `cycle_timezone`, host).
ADR-205 `WifiRow` docks as Wi-Fi list stacked hits (`connect_wifi`, host).
ADR-206 `BluetoothRow` docks as Bluetooth list stacked hits (`pair_bluetooth`, host).
ADR-207 `TrustedClientRow` docks as trusted-client stacked hits (`revoke_trusted_client`, privileged host).
ADR-208 `CapabilityRow` docks as Me app stacked hits (no action, privileged host).
ADR-209 lock / unlock cycle on panther HEAD (no-PIN tap-unlock; marker restored).
ADR-210 display restart on panther HEAD (`saai-displayd` kill; native-init respawn).
ADR-211 cold boot on panther HEAD (`reboot -f`; marker dropped then restored).
ADR-212 7-tap gallery on panther HEAD (Диагностика then Назад).
ADR-213 thin-tuning sprint parked last (not next Visual work).
ADR-214 list trailing rows in `layout_v2` (`list_refresh`/`list_scan`/`list_back`, host).
ADR-215 Me flatten/scroll in `layout_v2` (`flatten_me_rows` / `scrolled_row_rect`, host).
ADR-216 production `compile_v2()` / `layout_v2` `root_view`.
ADR-217 NOW content hits from compiled `now.sui`.
ADR-218 Inbox/Spaces/list hits from generated `compile_v2()`.
ADR-219 Me scroll hits from `layout_v2_scrolled`.
ADR-220 apps grid hits from generated `compile_v2()` `Button` tiles.
ADR-221 overlay Field/decision hits from `layout_v2`.
ADR-222 privileged `Keyboard` IME bound to Field; USB HID may replace the panel.
ADR-223 OrbHost hits from `layout_v2`; gallery page taps stay a whole-surface formula.
ADR-224 diagnostic Назад hits from `layout_v2` `DataRow` + `row back`.
ADR-225 NOW chrome paint from the same `now_view()` tree as hits.
ADR-226 Inbox/Spaces/list paint from generated `layout_v2` trees.
ADR-227 Me scroll paint from the same `layout_v2_scrolled` tree as hits.
ADR-228 apps grid paint from generated `layout_v2` `Button` tiles.
ADR-229 overlay Field/decision paint from generated `layout_v2`.
ADR-230 OrbHost paint from the same generated `layout_v2` tree as hits.
ADR-231 diagnostic paint from the same generated `layout_v2_scrolled` tree as hits.
ADR-232 leftover formulas: Keyboard keys, gallery page, lock idle/wake.
ADR-233 panther `saai-taskd` supervises live intents from `/data`.
ADR-234 panther `init_boot` starts `saai-taskd` after reboot.
ADR-235 `saai-taskd` follows entityd `SelectionChanged`.
ADR-236 diagnose timeout is Failed + retryable, not Pending.
ADR-237 WORK-02 live dispatch (derived ready, concurrency=1).
ADR-238 IRAB Plan persists a validated DAG, not diagnose.
ADR-239 Intent Object View shows plan → progress from related Tasks.
ADR-240 Primary nav is Сейчас · Пространства · Поиск · Система.
ADR-241 Сейчас leads with attention, current work, next.
ADR-242 Object View shows existing facts, links, OAM.
ADR-243 Space detail lists SOM members, not invented people.
ADR-244 Orb states follow workflow, not voice.
ADR-245 ObservationCache lives in runtime, not a daemon.
ADR-246 runtime status lists only Fresh Observations.
ADR-247 Система paints only live Observations.
ADR-248 product analytics of screens stays off.
ADR-249 PCE-25 laptop identity is x86/computer, never panther.
ADR-250 x86 displayd configures a window, not a panther panel.
ADR-251 x86 displayd seat is pointer+USB HID keyboard, not touch.
ADR-252 x86 shell uses 1280×800 logical layout, panther stays 1080×2400.
ADR-253 panther volume writes tinymix Digital PCM Volume.
ADR-254 power button drives lock/sleep from `/dev/input/power-button`.
ADR-255 cellular row names a live net bearer or «Нет модема».
ADR-256 camera row names a capture node or «Нет узла захвата».
ADR-257 playback row names tinyplay + test-tone, does not play.
ADR-258 Bluetooth adapter presence is hci0, not bt-scan.
ADR-259 WORK-03: Task Done only from Fresh matching Observation.
ADR-260 PolicyEngine adapter keeps the same Allow/AskUser/Deny verdicts.
ADR-261 session grants are scoped records, not a tool-name HashSet.
ADR-262 MemoryRecord v2; legacy JSONL is a view; provenance is assigned.
ADR-264 shell-legal memory read is status `memory_records`, not JSONL.
ADR-265 Visual v1 stays unsigned; `compile_v2_public` stays Experimental.
ADR-266 `saai-displayd` advertises `wp-fractional-scale-v1` and `wp-viewporter` (host).
ADR-267 `saai-displayd` implements `input-method-v2` without a keyboard (host).
ADR-268 Sistema `Записи` are Global `memory_records` from status (host).
ADR-269 first browser candidate is Falkon on QtWebEngine (host; no launch).
ADR-270 OSK is Keyboard keystrokes through IME `commit_string` (host; no wvkbd).
ADR-271 layer-shell keeps the client's height; touch hits the topmost layer (host).
ADR-272 layer blit is clipped to the client's destination, not always (0, 0) (host).
ADR-273 foreign OSK is a bottom Keyboard layer shown on IME activate (host).
ADR-274 Verifying Tasks settle from Fresh runtime `status` observations (host).
ADR-275 OAM/IRAB pass Principal on AuthorityRequest (host).
ADR-276 worker DelegationEnvelope is bound and OneShot (host).
ADR-282 taskd confirm sends execution_id; runtime issues the worker envelope (host).
ADR-283 memory remember/forget go through PolicyEngine (host).
ADR-284 runtime JSON `memory_erase` rewrites one identity off disk (host).
ADR-285 MemoryContextProjection labels kind; Restricted stays off the model (host).
ADR-286 GDK shm size from preferred_scale=120 is identity, not height 1776831 (host).
ADR-287 CPU sampler Health: Fresh→Healthy, Stale→Unknown, no percent threshold (host).
ADR-288 FailureClass: timeout/unreachable retryable; mismatch is not retry (host).
ADR-289 one Planner ReplanRequest after verification mismatch, cap 1 (host).
ADR-290 ObservationThreshold on schedules: Fresh >= gte; Stale not due (host).
ADR-291 Health Attention: Unhealthy lights Orb; Healthy/Unknown omitted (host).
ADR-292 runtime status includes one Health report, not a CPU graph (host).
ADR-293 shell Attention reads status.health; Sistema stays Observation-only (host).
ADR-294 native Wayland clipboard is deny-by-default; portal stays the grant path (host).
ADR-295 shell shows Verifying as in-progress, not worker Result as Done (host).
ADR-296 Failed status names timeout vs mismatch; no Retry button (host).
ADR-297 Intent names a bounded replan from `replan_count` (host).
ADR-298 Sistema names the node phone/panther vs computer/x86 (host).
ADR-299 Sistema «Узел» reads live `status.device`, not USB NCM (host).
ADR-300 Sistema «Это устройство» prefers live `hardware_model` (host).
ADR-301 Result is not Complete while the Task is Running/Verifying (host).
ADR-302 Orb Complete requires the Result's Task to be Done (host).
ADR-303 Orb Failed is the Task's class, not Result.error (host).
ADR-304 Sistema «Записи» omits Restricted even if status leaks it (host).
ADR-305 host GTK4 commits a shm frame on displayd; panther GTK4 4.14.4 still has no frame (ADR-310).
ADR-306 packed Falkon commits a shm hello-frame on host displayd; panther appd is ADR-312.
ADR-307 laptop is a USB client of the panther Space/Entity/Intent/Observation store (panther `c15c551d…` pid 6568).
ADR-308 panther shell carries host F chrome (`4dc19018…` pid 6710; leave Сейчас).
ADR-309 panther displayd advertises fractional-scale + IME v2 (`02c78f9f…` pid 6886).
ADR-310 Alpine gtk4-demo gets `preferred_scale=120` then still asks for height 2337935; no frame.
ADR-311 host `configure_bounds` is the window geometry, not smithay `(0,0)`; no displayd flash.
ADR-312 Falkon installs and launches on panther via appd; software shm hello-frame; no browse.
ADR-313 empty NEWNET brings up loopback (`e9b3c57c…`); Falkon file:// still white shm.
ADR-314 pack mesa swrast/llvmpipe into Falkon; WebEngine still white, renderers die.
ADR-315 sandbox procfs + 128MiB tmp/shm (`2503f4f5…`); renderer still ProcessGone SIGTRAP.
ADR-316 Falkon fonts.conf → /saaios/fonts; panther paints file:// hello.html (renderer lives).
ADR-317 WebEngine file:// field does not Activate IME; saai-shell-osk never mapped.
ADR-318 PCManFM-Qt Filter QLineEdit on panther; tap does not Activate OSK.
ADR-319 displayd advertises zwp_text_input_manager_v2 for Qt (host; no flash).
ADR-320 Alpine GTK 4.14.4 qemu frames when configure_bounds is the window (host; no flash).
ADR-321 Falkon loads hello.html over 127.0.0.1 HTTP (panther; not NetInternet).
ADR-322 host GTK 4.18 Entry sends zwp_text_input_v3::enable (no flash).
ADR-323 empty QT_IM_MODULE blocks Qt text-input-v2; packed PCManFM enables when unset.
ADR-324 IME commit_string reaches that v2 field; Qt 5.15 does not insert without a focused QLineEdit (no flash).
ADR-325 host GTK 4.18 Entry receives OSK hi! through IME (no flash).
ADR-326 host Ctrl+I into packed PCManFM disables text-input-v2 (no flash).
ADR-327 host PathEdit click re-enables v2; IME still does not paint (no flash).
ADR-328 host Qt 5.15 QLineEdit receives OSK hi! through IME (no flash).
ADR-329 packed PCManFM gets OSK commit+delete after PathEdit click; still no shm (no flash).
ADR-330 packed musl Qt 5.15 QLineEdit receives OSK hi! through IME (no flash).
ADR-331 packed PCManFM Filter-band click re-enables v2; still no shm (no flash).
ADR-332 host Ctrl+L into packed PCManFM disables text-input-v2 (no flash).
ADR-333 packed Falkon URL click enables v2; IME commit_string reaches it (no flash).
ADR-334 packed Falkon URL OSK hi! reaches v2 (no flash).
ADR-335 packed Falkon URL OSK still no new shm (no flash).
ADR-336 packed musl Qt 6.6.3 QLineEdit receives OSK hi! through IME (no flash).
ADR-337 packed Falkon URL v2 disables before an update_state settle (no flash).
ADR-338 packed Falkon URL second click re-enables v2; still no shm (no flash).
ADR-339 defer v2 IME commit until after client dispatch (host; no flash).
ADR-340 Falkon URL OSK is silent focusObject null, not discard (no flash).
ADR-341 packed PCManFM v2 OSK is silent focusObject null, not discard (no flash).
ADR-342 packed Falkon URL KEY_A after second click still does not paint (no flash).
ADR-343 packed musl GTK 4.14.4 Entry receives OSK hi! through IME (no flash).
ADR-344 packed gtk4-demo --run=entry binds v3; center click does not enable (no flash).
ADR-345 packed GTK 4.14 Entry types without grab_focus on a keyboard seat (no flash).
ADR-346 packed GTK 4.14 Entry types OSK without wl_keyboard (no flash).
ADR-347 packed Qt 6 QLineEdit binds v2 without wl_keyboard and does not enable (no flash).
ADR-348 packed Qt 5 QLineEdit binds v2 without wl_keyboard and does not enable (no flash).
ADR-349 host Qt 5 QLineEdit binds v2 without wl_keyboard and does not enable (no flash).
ADR-350 host GTK 4.18 Entry types OSK without wl_keyboard (no flash).
ADR-351 host Qt 5 pointer click without wl_keyboard does not enable v2 (no flash).
ADR-352 xdg Activated without wl_keyboard lets Qt QLineEdit type OSK (host; no flash).
ADR-353 packed PCManFM enables v2 without wl_keyboard (no flash).
ADR-354 packed Falkon without wl_keyboard does not enable v2 (no flash).
ADR-355 packed Falkon URL click enables v2 without wl_keyboard (no flash).
ADR-356 packed gtk4-demo click binds v3 without wl_keyboard and does not enable (no flash).
ADR-357 packed PCManFM v2 stays enabled without wl_keyboard (no flash).
ADR-358 packed Falkon URL v2 disables without wl_keyboard (no flash).
ADR-359 host Qt5 QLineEdit without setFocus still types OSK (no flash).
ADR-360 host Qt5 competing pane tap types OSK without wl_keyboard (no flash).
ADR-361 host Qt5 steal-back after tap does not type OSK (no flash).
ADR-362 host Qt5 OSK in the enable window types before steal-back (no flash).
ADR-363 packed PCManFM OSK without wl_keyboard hits FolderView not Filter (no flash).
ADR-364 packed PCManFM Filter click without wl_keyboard stays FolderView (no flash).
ADR-365 packed PCManFM PathEdit click without wl_keyboard stays FolderView (no flash).
ADR-366 packed Qt6 competing pane tap types OSK without wl_keyboard (no flash).
ADR-367 packed Qt6 steal-back after tap does not type OSK (no flash).
ADR-368 packed Qt6 OSK in the enable window types before steal-back (no flash).
ADR-369 packed Qt5 competing pane tap types OSK without wl_keyboard (no flash).
ADR-370 packed Qt5 steal-back after tap does not type OSK (no flash).
ADR-371 packed Qt5 OSK in the enable window types before steal-back (no flash).
ADR-372 packed GTK 4.14 competing pane auto-enables and types OSK without a tap (no flash).
ADR-373 packed gtk4-demo --run=entry is not an example name (no flash).
ADR-374 packed gtk4-demo --run=search_entry enables v3 without a tap (no flash).
ADR-375 packed gtk4-demo --run=search_entry OSK does not change shm (no flash).
ADR-376 packed gtk4-demo search_entry OSK forwards v3 commit_string (no flash).
ADR-377 packed gtk4-demo search_entry has one v3 object, commit equals enable (no flash).
ADR-378 packed gtk4-demo search_entry OSK grows surrounding, not shm (no flash).
ADR-379 packed gtk4-demo search_entry OSK does not commit shm (no flash).
ADR-380 packed gtk4-demo search_entry click then OSK still no toplevel shm (no flash).
ADR-381 packed gtk4-demo search_entry OSK sends a v3 cursor rectangle (no flash).
ADR-382 packed gtk4-demo --run=password_entry enables v3 without a tap (no flash).
ADR-383 packed gtk4-demo password_entry OSK does not commit shm (no flash).
ADR-384 packed Falkon URL OSK immediately after enable still no shm (no flash).
ADR-385 packed Falkon URL OSK grows v2 surrounding, not shm (no flash).
ADR-386 packed Falkon URL OSK extra shm is cursor, not LocationBar (no flash).
ADR-387 packed PCManFM FolderView OSK surrounding stays 0 (no flash).
ADR-388 host frame clock; packed gtk4-demo OSK commits shm (no flash).
ADR-389 packed Falkon URL click flashes then disables before OSK (no flash).
ADR-390 panther idle frame clock when no flip is pending (no flash).
ADR-391 packed PCManFM Filter click OSK grows surrounding (no flash).
ADR-392 packed PCManFM PathEdit click selects path then disables (no flash).
ADR-393 packed PCManFM PathEdit disables without OSK (no flash).
ADR-394 second xdg_toplevel does not steal Activated (no flash).
ADR-395 packed PCManFM PathEdit still disables after no-steal (no flash).
ADR-396 gtk4-demo entry_completion enables v3 without xdg_popup (no flash).
ADR-397 gtk4-demo entry_completion OSK types without xdg_popup (no flash).
ADR-398 packed Falkon URL still flash-disables after no-steal (no flash).
ADR-399 gtk4-demo combobox has no xdg_popup without a click (no flash).
ADR-400 gtk4-demo combobox click 200 90 has no xdg_popup (no flash).
ADR-401 packed GTK 4.14 popover maps configured xdg_popup (no flash).
ADR-402 packed GTK 4.14 popover Entry types OSK hi! (no flash).
ADR-403 host Qt 5.15 QMenu is a second toplevel, not xdg_popup (no flash).
ADR-404 packed GTK 4.14 ComboBox popup maps configured xdg_popup (no flash).
ADR-405 packed GTK 4.14 ComboBox with_entry types OSK hi! (no flash).
ADR-406 packed GTK 4.14 DropDown activate maps xdg_popup (no flash).
ADR-407 packed GTK 4.14 DropDown search types OSK hi! (no flash).
ADR-408 host Qt 5.15 QLineEdit selectAll keeps v2 (no flash).
ADR-409 host Qt 5.15 QLineEdit completer reload keeps v2 (no flash).
ADR-410 host Qt 5.15 competing pane + mouse selectAll keeps v2 (no flash).
ADR-404 packed GTK 4.14 ComboBox popup maps xdg_popup (no flash).
ADR-277 Automation is not the local user (host).
ADR-278 portal capabilities go through PolicyEngine; GrantStore stays (host).
ADR-279 revoke drops GrantStore coverage and live session/envelope grants (host).
ADR-280 Confirm Once is a OneShot grant, not AskUser fallthrough (host).
ADR-281 Falkon package packs QtWebEngineProcess, pak/v8, system ICU (host).
VUI-02 остаётся почти закрытым (шрифты в boot-image — единственный
blocked item). Этот трек не переоткрывает S00–S32.

**Service queue (параллельно shell):** прошивка `saaios-runtime` /
`saai-taskd` не считается вторым DRM-экспериментом. P0 — MEM-01
same-key Home/Work на уже существующем runtime. Attention, Task
visibility и Memory review едут **внутри VUI-05/06**, не отдельными
phone sprints.

Архитектурные ADR остаются справочниками:

- [WORK-ROADMAP.md](WORK-ROADMAP.md) / [ADR-121](../../adr/ADR-121-work-scheduler-v2.md)
- [WORLD-ROADMAP.md](WORLD-ROADMAP.md) / [ADR-122](../../adr/ADR-122-world-model-observation-layer.md)
- [ATTN-ROADMAP.md](ATTN-ROADMAP.md) / [ADR-123](../../adr/ADR-123-attention-projection.md)
- [AUTH-ROADMAP.md](AUTH-ROADMAP.md) / [ADR-124](../../adr/ADR-124-unified-authority-model.md)
- [MEM-ROADMAP.md](MEM-ROADMAP.md) / [ADR-125](../../adr/ADR-125-memory-learning-provenance-v1.md)

Новых фундаментальных одноустройственных моделей не добавлять.

Отдельно, не в Pixel-пути: запуск сторонних Linux ARM64 / Android-приложений —
[APP-COMPAT-ROADMAP.md](APP-COMPAT-ROADMAP.md). Голос и PCE ждут железа.

## Текущее состояние

| ID | Результат | Состояние | Зависит от |
|---|---|---|---|
| S00 | Архитектура и процесс | Done | рабочий DRM UI |
| S01 | Системная идентичность | Done | S00 |
| S02 | Wayland vertical slice на host | Done | S01 |
| S03 | `saai-displayd` на Pixel 7 | Done | S02 |
| S04 | Отдельный `saai-shell` и lock | Done | S03 |
| S05 | Приложения, manifest и lifecycle | Done | S04 |
| S06 | Настоящие пространства и entity store | Done | S05 |
| S07 | Capability, sandbox и portals | Done | S05, S06 |
| S08 | GTK и Qt/Kirigami совместимость | Done (переоцененный объём) | S07 |
| S09 | Intent → Task → Action workflow | Done | S06, S07 |
| S10 | Planner, automation и memory | Done | S09 |
| S11 | Производительность, idle и стабильность | Done (с честными оговорками) | S01–S10 |
| S12 | OTA и release gate | Done (с честными оговорками) | S01–S11 |
| S13 | Завершение пользовательского интерфейса | Done (с честными оговорками) | S01–S12 |
| S14 | О телефоне (номер сборки, версия) | Done | S13 |
| S15 | Хранилище (использовано/свободно) | Done | S13 |
| S16 | Экран: яркость и таймауты | Done | S13 |
| S17 | Дата и время: часовой пояс | Done | S13 |
| S18 | Звук: громкость | Done (звук физически не подтверждён -- честный пробел) | S13 |
| S19 | Wi-Fi: сканирование и подключение | Done (сканирование/статус реальны; пароль ограничен клавиатурой) | S13 |
| S20 | Bluetooth: сопряжение устройств | Done (обнаружение подтверждено; сопряжение не протестировано) | S13 |
| S21 | Уведомления (обобщённая модель) | Done (с честными оговорками -- низкий заряд не понаблюдали живьём) | S06, S09 |
| S22 | OTA в интерфейсе | Done (частично, read-only) | S12 |
| S23 | Сетка/иконки на главном экране | Done (иконки-плейсхолдеры) | S13 |
| S24 | PIN/защита экрана блокировки | Done | S04 |
| S25 | Доступность (размер шрифта, контраст) | Done | S13 |
| S26 | Удалённое выполнение команд по сети | Done (реальный SSH + policy-слой, ADR-074) | `file-recv` (ADR-073) |
| S27 | Заряд устройства не работает под SaaiOS | Done (два пропущенных insmod, ADR-078) | -- |
| S28 | Sandbox-совместимость сторонних приложений | Done (утечка shm-буферов в /tmp, ADR-075) | S07, ADR-072 |
| S29 | Клавиатура: цифры и символы | Done (режим 123/ABC, ADR-076) | ADR-029, ADR-065 |
| S30 | Уведомления от сторонних приложений | Done (портал + capability, ADR-079) | S21 |
| S31 | `file-recv`: скачивание с устройства | Done (GET-глагол, ADR-080) | ADR-073 |
| S32 | Таймер бездействия не видит активность в чужих окнах | Done (глобальный маркер в displayd, ADR-077) | S11, ADR-072 |

## S00 — Архитектура и процесс

**Goal:** зафиксировать собственную Wayland-платформу, продуктовые границы,
процесс, качество и последовательность разработки.

**Acceptance:** приняты ADR-005 и продуктовые принципы; описаны компоненты,
инварианты безопасности, Definition of Ready/Done, тестовые gates, roadmap и
rollback policy; документы связаны из основного индекса.

**Rollback:** удалить документационный commit; рабочий phone image не меняется.

**Evidence:** документация и индексы добавлены commit
`407e5feb90979b1a6c6c02c1b0643d8c75950f29`; GitHub Actions run
[`33984921594`](https://github.com/Damasker/saaios/actions/runs/33984921594)
успешно выполнил format, clippy, workspace tests и e2e. Изменений phone image,
разделов и userdata не было.

## S01 — Системная идентичность

Рабочий паспорт и декомпозиция: [S01-system-identity.md](S01-system-identity.md).

**Goal:** система получает удостоверенный локальный DeviceContext и говорит о
Pixel 7 как работающая SaaiOS, а не как внешний чат-помощник.

**Scope:** `system.identity`, контекст planner, runtime status, target от PID 1,
переименование системного экрана и физический тест двух запросов. Полный
device-state service и новые изменяющие actions не входят.

**Acceptance:** runtime и tool согласованно возвращают
`SaaiOS / native_device / phone / panther`; UI использует модель намерения и
системного результата; модель не отрицает установленную SaaiOS; контекст не
содержит идентификаторов пользователя; image проходит rollback gate.

**Rollback:** combined Evidence image source `be9630d`, SHA-256
`c1ebb5afd0ee276ca7c2774b765084a19e001ad1afc4d3385c57dc7b66aedff8`;
Android slot B.

## S02 — Wayland vertical slice на host

Рабочий паспорт и декомпозиция: [S02-wayland-host-vertical-slice.md](S02-wayland-host-vertical-slice.md).

**Goal:** два независимых процесса показывают первую настоящую поверхность
через прототип `saai-displayd` без телефона.

**Scope:** выбор framework на основе измеримого spike; `wl_shm`; один
`xdg_toplevel`; configure/ack/commit; touch-like pointer injection; закрытие
клиента; headless test. DRM, GPU и toolkit-совместимость не входят.

**Acceptance:** тестовый клиент рисует различимый кадр, получает ввод и может
быть закрыт; повреждённый клиент отключается без падения compositor; принято
ADR о framework; CI воспроизводит сценарий.

**Rollback:** workspace остаётся на текущем C DRM UI; новый сервис не входит в
phone image.

## S03 — `saai-displayd` на Pixel 7

Рабочий паспорт и декомпозиция: [S03-panther-displayd.md](S03-panther-displayd.md).

**Goal:** тот же тестовый Wayland-клиент выводится на реальный экран и получает
касания.

**Scope:** DRM/KMS backend, `1080x2400x60`, BGRX output transform, evdev touch,
focus одного fullscreen-клиента, readiness и аппаратный watchdog. Shell ещё
остаётся старым.

**Acceptance:** цветовая таблица совпадает с проверенной; касание доставляется
только активному клиенту; выход compositor автоматически возвращает текущий
DRM fallback и USB-консоль; холодная загрузка воспроизводима.

**Rollback:** физически проверенный образ, предшествующий S02; слот B не
изменяется.

## S04 — Отдельный `saai-shell` и lock screen

Рабочий паспорт и декомпозиция: [S04-saai-shell.md](S04-saai-shell.md).

**Goal:** текущая мобильная оболочка становится независимым системным клиентом.

**Scope:** lock/status/navigation/system overlay, системный слой, список
окон, перезапуск shell, touch wake и idle screen-off. Бизнес-данные остаются
адаптером к текущему состоянию.

**Acceptance:** визуальная и touch-регрессия четырёх разделов пройдена; lock
не пропускает ввод приложению; намеренное падение shell не валит compositor и
приводит к ограниченному восстановлению.

**Rollback:** `drm-splash` включается как boot/recovery UI.

## S05 — Manifest и жизненный цикл приложений

Рабочий паспорт и декомпозиция:
[S05-application-lifecycle.md](S05-application-lifecycle.md).

**Goal:** SaaiOS устанавливает и управляет первым отдельным приложением.

**Scope:** versioned manifest, `app_id`, каталоги `/data/saaios/apps` и
`/data/saaios/var/apps`, launch/stop/switch, single-instance, crash limit,
события состояния. Пока только доверенные приложения.

**Acceptance:** demo-app устанавливается без изменения `init_boot`, запускается
из оболочки, переключается и удаляется без чужих данных; неизвестная схема и
duplicate id отвергаются; crash loop ограничен.

**Rollback:** отключить `saai-appd` и удалить только каталог demo-app, сохранив
shell и recovery.

**Evidence:** host vertical проверил install→launch→stop→remove и сохранность
app data. На Pixel 7 подтверждены установка после boot, fullscreen Wayland,
touch, штатное закрытие, холодный перескан manifest, сохранность данных,
рестарт appd под PID 1 и ограничение третьего app crash за 60 секунд. S05 image
source `6afb663`, SHA-256
`be22d1af98b73c24fa9272f2575ac6bbc4aa858516de2478e18f417da07db215`;
прошит только `init_boot_a`, slot B не менялся.

## S06 — Пространства и entity store

Рабочий паспорт и декомпозиция:
[S06-space-entity-store.md](S06-space-entity-store.md).

**Goal:** `Дом`, `Работа`, `Личное` и `SaaiOS` становятся настоящими областями
данных, а не только сохранённым UI-переключателем.

**Scope:** versioned модели `Space`, `Entity`, `Event`; атомарное локальное
хранилище; проекции для `Сейчас`; миграции; импорт существующего выбранного
контекста.

**Acceptance:** объект одного пространства не появляется в другом без явного
действия; события append-only; незавершённая запись и холодный reboot не
повреждают store; миграция имеет обратимый backup.

**Rollback:** read-only возврат к предыдущей schema и восстановление backup;
никакой молчаливой downgrade-записи.

**Evidence:** host vertical (store/protocol/daemon/shell) прошёл 28+
unit/process tests с all-target clippy на R620. На Pixel 7 физически
подтверждены: `saai-entityd` ARM64 packaging и persistent supervision из
`native-init.c` (`024ccfe`) тем же паттерном, что и `saai-appd`; `kill -9`
работающего `saai-entityd` дал автоматический restart (`/run/boot.log`) и
независимое переподключение `saai-shell`; выбор пространства через
реальный touch-путь (`Пространства` → `Личное`) пережил полный холодный
`fastboot`-цикл (`/proc/uptime` = 42s), подтверждено на трёх уровнях --
файл `selection.json`, прямой запрос к сокету `entityd` в обход shell,
и визуально на экране; изоляция между пространствами подтверждена
`create_entity`/`list_entities` через тот же прямой протокольный запрос.
Полная пересборка `init_boot` для этой версии сделана не в этой сессии
(бинарник уже был на устройстве от параллельной работы) -- проверка шла
против уже прошитого состояния, содержимое бинаря сверено по хешу с
собранным из того же commit.

## S07 — Capability, sandbox и portals

Рабочий паспорт и декомпозиция:
[S07-capability-sandbox.md](S07-capability-sandbox.md).

**Goal:** приложение получает только явно разрешённые действия и данные.

**Scope:** capability vocabulary, effective grants, process isolation,
filesystem/network mediation, системный permission surface, portal для
выбора объекта и clipboard. Threat-model ADR-020 выбрал mount/network/ipc/uts
namespaces + seccomp-bpf после on-device spike, подтвердившего, что
`CLONE_NEWPID`/`CLONE_NEWUSER` физически недоступны на ядре Pixel 7.

**Acceptance:** негативные тесты доказывают запрет чтения другого app/space,
произвольной сети и подделки системного подтверждения; deny является default;
решения аудируются без пользовательского содержимого.

**Rollback:** сторонние приложения отключаются целиком; системные приложения
продолжают работать с минимальным статическим набором capability.

**Evidence:** capability vocabulary, effective-grants store, consent-экран,
namespace/seccomp isolation и portal-точка входа (clipboard + типизированный
`not_implemented` для `open_file`) реализованы и физически проверены на
Pixel 7 по отдельности (см. полный Evidence в паспорте спринта). Итоговый
device-level negative-test проход через `org.saaios.sandbox-probe` дал чистый
`RESULT PASS failures=0` по 18 инвариантам (скрытые сокеты/файлы/устройства,
read-only root, обнулённые capabilities, seccomp `EPERM` на `kill`/`mount`/
`reboot`, реальное отсутствие сети без `net.internet`). Cold-reboot
acceptance подтвердил, что effective grants (принятые и явно отклонённые)
переживают настоящую холодную перезагрузку побайтово и honoured'ся при
запуске без повторного запроса согласия.

## S08 — GTK и Qt/Kirigami совместимость

Рабочий паспорт и декомпозиция:
[S08-toolkit-compatibility.md](S08-toolkit-compatibility.md).

**Goal:** по одному настоящему адаптивному приложению обоих toolkit работает
как обычный клиент SaaiOS.

**Scope:** необходимые Wayland-протоколы (`wl_data_device_manager`,
`zwp_text_input_manager_v3`), env-var-based theme/font passthrough (не
полноценный settings portal -- у SaaiOS ещё нет источника настроек,
ADR-021's Change 4 сознательно сузил объём), упаковка runtime стороннего
toolkit'а в `/data` через воспроизводимый build-скрипт. Полноценный
input-method-v2/OSK, clipboard-через-policy, GPU-ускорение, полные
KDE/GNOME sessions и XWayland не входят -- каждый закрыт отдельной ADR
как физически/архитектурно нереализуемый в разумном объёме на этом
железе (ADR-022, ADR-023, ADR-024), не просто отложен.

**Acceptance:** GTK-приложение и Qt/Kirigami-приложение запускаются,
масштабируются, получают touch/text input, переживают switch и закрываются;
системные разрешения нельзя обойти toolkit API; размер и память измерены.

**Rollback:** удалить соответствующий runtime bundle без изменения shell.

**Evidence:** закрыт в переоцененном объёме -- GTK-часть Acceptance
снята отдельным решением (ADR-025), Qt/Kirigami принят как достаточное
доказательство Goal. Источник пакетов -- Alpine
musl (ADR-021), не from-source. GPU-ускорение физически исключено --
проприетарный Mali-блоб собран только под Android's HAL, открытый
`panthor` требует ядро >=6.10 против GKI-залоченного 6.1.157 устройства
(ADR-024). GTK4 (Alpine's `gtk4.0` 4.14.4) детерминированно падает на
реальном aarch64 при создании wl_shm-буфера -- дефект апстрима,
локализован через `gdb` на устройстве до точной строки
(`gdk_wayland_display_create_shm_surface`, повреждённый `height`,
вероятно неинициализированный scale из-за отсутствующего
`wp-fractional-scale-v1`), но не устранён; заблокирован для физической
приёмки (ADR-025). Qt5/QtQuick и Kirigami2 тем же методом подтверждены рабочими
на том же железе (ADR-026) -- Change 7 переключил основной toolkit
спринта на Qt/Kirigami. `org.saaios.demo.kirigami` -- первое реальное,
воспроизводимо собираемое приложение стороннего toolkit'а -- прошло
install→launch→stop→remove через настоящий `saai-appd`'s IPC на боевом
устройстве (ADR-027). Touch физически подтверждён отдельно (ADR-028) --
синтетическая evdev-инъекция через `uinput` (безопасно, в изолированном
`unshare -m`-namespace) доставила тап именно на Kirigami-приложения
surface ID, включая корректную маршрутизацию через lock-screen
(ADR-015's инвариант). Негативный sandbox-тест на clipboard для
конкретно этого приложения остаётся непроверенным (общий пробел уже
задокументирован отдельно, ADR-023). GTK-демо не собрано и не принято --
явно заблокировано находкой ADR-025, не входит в критический путь.
Закрытие: `VmRSS` реального Kirigami-процесса -- 61 028 KB; целевой
`kill -9` работающего клиента не затронул `saai-displayd`, `saai-appd`
корректно перезапустил приложение (тот же crash-recovery, что у
остальных S05-приложений); намеренно повреждённый GTK4-клиент (десятки
`SIGSEGV` за время расследования ADR-025) тоже ни разу не уронил
compositor. Устройство возвращено к исходному состоянию.

## S09 — Intent → Task → Action

Рабочий паспорт и декомпозиция:
[S09-intent-task-action.md](S09-intent-task-action.md).

**Goal:** строка намерения создаёт наблюдаемый рабочий процесс, а не только
чатовый запрос.

**Scope:** Change 1 (обязательно первым, спайк + ADR) решает два вопроса
без готового ответа. (1) **Закрыт физически (ADR-029):** как вводится
текст -- `saai-shell` не имел ни одного экрана ввода текста
(`KeyboardInteractivity::None`), а generic `input-method-v2`/OSK-путь
для сторонних клиентов уже физически доказан нерабочим на этом железе
(ADR-022, `libxkbcommon` крашится на любом keymap). Bespoke touch-hit-test
клавиатура внутри `saai-shell` (та же `Node`/`layout`/`hit_test` система,
что уже рисует весь остальной UI, без единого обращения к
`libxkbcommon`) физически собрала реальную строку из синтетических
evdev-тапов (`uinput`, метод ADR-028) -- throwaway-патч отменён,
`saai-shell` пересобран и подтверждён байт-в-байт идентичным боевому.
(2) **Закрыт (ADR-030):** как `Intent`/`Task`/`Action`/`Result`
соотносятся с уже существующей Platform Track'а инфраструктурой
(`crates/policy-engine`, `tool-registry`, `automation-engine`) --
прочитан реальный код: `PendingConfirmation` в `ai-runtime` живёт
локальной переменной в рамках одного tokio-таска одного запроса,
`policy-engine`'s `session_allows` обнуляется при рестарте процесса --
ноль персистентности, S09's "переживает холодную перезагрузку"
физически невыполнимо на этом фундаменте. Ни один native OS Track
сервис не зависит от `protocol`-крейта -- оба рантайма сегодня не
делят код. Решение: `Intent`/`Task`/`Action`/`Result` -- новые
`entity_type` (`saaios.intent`/`saaios.task`/`saaios.action`) поверх
уже существующего, уже физически cold-reboot-персистентного
`saai-entity-store` (S06) -- НЕ портирование `policy-engine`/
`tool-registry`, тот же прецедент, что S06 уже применило к
`memory-store`/`audit-log`. ADR-004's конвергенция остаётся названным,
но не реализуемым в S09 направлением. Change 1 закрыт целиком
(ADR-029 + ADR-030). **Change 2 (минимальный вертикальный срез)
физически подтверждён (ADR-031):** новый native-демон
`services/saai-taskd` (реактивный по `saai-entityd`'s `Subscribe`,
идемпотентный по `intent_id`, 15+3 host-теста, НЕ добавлен в
`native-init.c` до отдельного решения об автозапуске) + реальная
(закоммиченная) on-screen QWERTY-клавиатура в `saai-shell` (19
host-тестов). На реальном Pixel 7: `saai-taskd` против боевого
`saai-entityd` корректно и идемпотентно превратил `Intent` в `Task ->
Action -> Result` (echo-Action, детерминированный); рестарт демона не
создал дублей. Новый инструмент `uinput-touch.c` (protocol-B,
`ABS_MT_*`, сверен с `saai-displayd`'s реальным парсером) + временная
подмена боевого `saai-shell`-бинарника (байт-в-байт восстановлен после
теста) -- пять реальных тапов дважды независимо создали настоящий
`Intent` "hi" через настоящую клавиатуру, `saai-taskd` подобрал оба.
Тестовые сущности удалены, устройство возвращено к исходному
состоянию. **Change 3 (опасный Action + cold reboot) физически
подтверждён (ADR-032):** новый опасный `delete_entity` Action,
классификатор на `saaios.intent`'s собственных properties (не на
свободном тексте клавиатуры), новый экран подтверждения
(`Frame::TaskConfirm` в `saai-shell`, та же система, что consent) --
оба пути (подтвердить/отклонить) проверены реальными тапами; найденный
в процессе пробел (Action не помечался `cancelled` при отклонении)
исправлен до коммита. Главное: выполнен настоящий cold reboot
(`reboot -f`, обычный `reboot` не сработал) с опасным Task в
`waiting_confirmation` -- после ребута (uptime 2 мин, все PID новые)
Task остался нетронутым, свежезапущенный `saai-taskd`'s reconcile
корректно НЕ исполнил его автоматически, только повторный живой тап
ПОСЛЕ ребута довёл дело до конца. Побочно обнаружено и явно
задокументировано: `/saaios/saai-shell`'s "боевой" хэш, который весь
сеанс до этого считался `34653527...`, после настоящего ребута
оказался `87dbe83f...` -- первый жил только в памяти незагружаемого
заново rootfs. S09's Change 1/2/3 все физически закрыты; полный
workflow-каталог/`Входящие`-экран -- будущий, отдельный, более крупный
Change, изначально исключённый из этого спринта ("Не входит").

**Спринт закрыт (`Done`, 2026-09-11).** Полное построчное обоснование
по каждому пункту Acceptance criteria -- в
[S09-intent-task-action.md](S09-intent-task-action.md#acceptance-criteria).
Кратко: ввод текста и подтверждение опасного Action через настоящий
cold reboot подтверждены полностью физически; идемпотентность
подтверждена физически для неопасного пути (рестарт демона) и по коду
плюс host-тестам для опасного (повторное подтверждение уже
исполненного Task не инсценировано отдельно в этом раунде); видимость
в `Сейчас` подтверждена косвенно через корректную маршрутизацию
реальных тапов, не через прямой визуальный контроль экрана.

**Rollback:** не устанавливать/не включать Intent/Task/Action целиком --
`saai-shell` и весь S05-S08 путь работают без единого изменения.

## S10 — Planner, automation и memory

Рабочий паспорт и декомпозиция:
[S10-planner-automation-memory.md](S10-planner-automation-memory.md).

**Goal:** локальный ИИ предлагает планы и автоматизацию внутри видимых границ
-- то же `Intent`/`Task`/`Action`, что S09 уже физически доказало, не новый,
более слабый путь в обход.

**Scope:** Change 1 (спайк + ADR, по прецеденту S09) решает: как свободный
текст Intent превращается в предложенный Action. Ключевая находка Definition
of Ready -- это не гипотетическая интеграция: `saaios-runtime` (Platform
Track) уже реально развёрнут и работает прямо на этом Pixel 7 (`--real-linux
--tcp 172.31.7.1:38127`), с реальной (не mock) локальной моделью
`qwen2.5:3b-instruct` через Ollama на подключённом по USB-NCM хосте --
`automation-engine`'s `system.metrics`-опрос уже реально пишет аудит каждые
~30с. При этом `saai-taskd` и этот работающий `saaios-runtime` физически не
пересекаются нигде -- ни один зарегистрированный инструмент
(`system-tools::install_system_tools`) не касается `saai-entity-store`.
Кандидат A для Change 1 -- `saai-taskd` вызывает уже работающий `saaios-
runtime` через его существующий TCP-протокол (тот же, что `console-tui`)
вместо изобретения нового ИИ-стека -- это и есть ADR-004's конвергенция,
наконец физически возможная поверх уже развёрнутой инфраструктуры.

**Change 1 физически подтверждён (ADR-033):** прямой TCP-запрос с этого же
Windows-хоста к `172.31.7.1:38127` (без захода в консоль устройства) --
неопасный текст ("Сколько свободно места на диске?") заставил модель саму
вызвать `system.disk`, `policy` разрешил, ответ получен без `pending`;
опасный текст ("Останови процесс с pid 999") -- модель предложила
`process.kill_request`, `policy` потребовал подтверждения, вернулся
`pending`, ничего не выполнено; `confirm` с `confirmed:false` отменил,
`confirmed:true` (на безвредном несуществующем pid) реально дошёл до
исполнения. Решение: `pending` в ответе -- единственный сигнал для
`WorkflowStatus` (`WaitingConfirmation` при наличии, `Done` сразу при
отсутствии, через уже существующий `Frame::TaskConfirm`, без изменений UI).
Кандидаты "новый tool внутри saaios-runtime, пишущий в saai-entityd
напрямую" и "свой вызов модели внутри saai-taskd" отклонены -- оба
дублировали бы уже единственный источник риск-классификации.

**Change 2 физически подтверждён (ADR-034):** старый `echo`-плейсхолдер
(S09/ADR-031) заменён реальным мостом (`runtime_bridge.rs`,
`RUNTIME_ACTION_KIND`). В процессе физической проверки найдены и
исправлены до коммита два реальных бага: заголовок `saaios.result` из
ответа модели (длинный/многострочный) ронял `saai-entityd`'s
валидацию и весь демон -- исправлено `safe_title()`; и собственная
serial-тестовая оснастка сессии ломала кириллицу (баг тестовой
инфраструктуры, не проекта). На реальном устройстве: неопасный текст
-> реальный ответ модели, `Result` с корректно усечённым заголовком;
опасный текст -> `pending` -> реальный тап "Подтвердить" на уже
существующем `Frame::TaskConfirm` (ADR-032, **ноль изменений
`saai-shell`**) -> честный результат от `saaios-runtime`; отдельный
запрос -> реальный тап "Отклонить" -> `cancelled`. По пути найдены и
явно задокументированы два серьёзных операционных пробела:
`native-init.c` никогда не поднимает `lo` (таймаут вместо мгновенного
отказа на любых loopback-соединениях native-компонентов), и реально
зашитый `init_boot`-образ устройства устарел -- не содержит on-screen
клавиатуру и экран подтверждения из S09 Change 2/3. **Оба закрыты и
физически подтверждены отдельным раундом в тот же день (ADR-035):**
`configure_loopback()` добавлена в `native-init.c`; `init_boot`
пересобран с текущего `HEAD` (`saai-shell` в нём -- байт-в-байт уже
touch-проверенный `9d4df0ee`) и реально прошит на устройство
(`fastboot flash init_boot_a`). После настоящего cold reboot: `lo`
поднимается сама, без единого ручного шага; мост `saai-taskd` ->
`saaios-runtime` подтверждён напрямую (Intent -> Result за ~31с вместо
прежнего ~110-секундного таймаута).

**Change 3 (расписания/триггеры) физически подтверждён (ADR-036 +
ADR-037):** расписание -- новый native `entity_type`
(`saaios.schedule`), не порт `automation-engine`'s `TriggerKind` (тот
зависит от Platform Track'а и не имеет ни одного временного триггера;
его единственный существующий "авто"-путь вызывает модель в обход
`saai-entity-store`, нарушая S10's собственный Acceptance criterion).
Periodic tick внутри уже существующего `saai-taskd` (не новый сервис)
находит просроченные расписания и создаёт обычный `Intent` --
неотличимый от введённого вручную, `process_intent()` не изменился ни
на строку. На реальном устройстве: расписание с интервалом 20с
сработало немедленно при первом tick'е и затем ровно на этом
интервале ещё дважды (`fire_count` = 3 за ~41с), каждый раз доведя
полный `Intent -> Task -> Action -> Result` до `Done` через уже
существующий planner-мост -- без единого созданного вручную `Intent`.

**Memory, привязанная к пространству, физически подтверждена (ADR-038 +
ADR-039):** `memory-store` остаётся Platform Track'ом (плоский JSONL) --
миграция в `saai-entity-store` потребовала бы, чтобы `saaios-runtime`
впервые стал клиентом native OS Track'а ради простого
партиционирования данных, которому не нужны ни `WorkflowStatus`, ни
confirmation; вместо этого `space_id` явно протянут от `saai-taskd`
(единственного, кто его знает) через wire-протокол в `ToolContext`.
На реальном устройстве: факт, вспомненный моделью в `home`
(`memory.remember`, выбрано моделью самостоятельно), не был виден при
`memory.recall` из `work` -- изоляция подтверждена; тот же запрос из
`home` факт нашёл; `memory.forget` корректно создал tombstone. Два
независимых `saai-taskd` работали одновременно против одного
`saaios-runtime`. Все четыре куска S10 закрыты.

MLP v1 ([ADR-125](../../adr/ADR-125-memory-learning-provenance-v1.md),
[MEM-ROADMAP.md](MEM-ROADMAP.md)) расширяет ADR-038 внутри того же
Platform store: compact key = `(space_id, key)`. S10 не переоткрывается.

**Acceptance:** модель не может обойти capability -- подтверждено, каждый
planner-предложенный Action (включая schedule-порождённые) проходит тот же
`WorkflowStatus`/confirmation-конвейер, что и explicit-путь S09; каждый
Action объясним и отменяем -- подтверждено; budget/loop limits и
устойчивость explicit-пути к отключению модели -- подтверждены
косвенно (независимо проверенный Platform Track механизм и
структурная независимость `process_dangerous_intent` от
`saaios-runtime`, не отдельным целевым тестом в этом спринте).

**Спринт закрыт (`Done`, 2026-09-11).** Полное построчное обоснование --
в [S10-planner-automation-memory.md](S10-planner-automation-memory.md#acceptance-criteria).

**Rollback:** не давать planner'у писать в entity store -- explicit-Action-
путь S09 (`delete_entity`) и весь остальной S05-S09 стек продолжают
работать без единого изменения независимо от состояния planner-моста.

## S11 — Производительность, idle и стабильность ежедневного использования

Рабочий паспорт и декомпозиция:
[S11-performance-idle-daily-use.md](S11-performance-idle-daily-use.md).

**Goal:** SaaiOS становится измеримо стабильной для ежедневного использования
на тестовом Pixel 7 -- реальные бюджеты latency/памяти для уже реализованных
путей S01-S10, физически подтверждённые, не предполагаемые; полная регрессия
по всему стеку одним раундом; там, где ядро уже это позволяет -- реальная
экономия энергии в простое.

**Разделение исходного S11:** черновик объединял performance/idle/GPU с
полноценным OTA/release gate -- два независимых, оба крупных вертикальных
результата (OTA самостоятельно критичен по безопасности, требует подписной
инфраструктуры, которой в репозитории нет вообще). Оба за один спринт не
помещаются (`DEVELOPMENT_PROCESS.md`'s правило деления). OTA вынесен в
отдельный **S12** со своим Definition of Ready. GPU-ускорение уже закрыто
(ADR-024, S08) и не пересматривается здесь.

**Scope:** Change 1 (спайк, обязателен первым) -- реальные замеры
latency/памяти уже существующих daily-use путей (холодная загрузка до
`Сейчас`, launch/switch приложения S05, planner-цикл S10) без нового кода, и
физическая проверка безопасности `mem`-suspend на этом конкретном ядре
(`/sys/power/state` физически подтверждён как поддерживающий `freeze mem
disk`, но никогда не использованный) -- по итогам ADR с конкретными
бюджетами. Change 2 -- один реальный механизм deep idle, вытекающий из
Change 1's находок, либо задокументированный обоснованный отказ, если `mem`
окажется небезопасен. Change 3 -- комбинированный регрессионный проход по
ключевым инвариантам S01-S10 и 24-часовой soak на уже существующей
`system.metrics`-инфраструктуре (S10).

**Acceptance:** latency/memory-бюджеты зафиксированы как конкретные числа и
подтверждены минимум тремя независимыми прогонами; реальное снижение
энергопотребления в простое измерено (не по документации SoC), устройство
просыпается по touch/power-button в 10 из 10 циклов без ручного
вмешательства -- либо обоснованный отказ вместо форсированной реализации;
регрессионный проход не находит отклонений от принятых ADR S01-S10; 24-часовой
soak без незапланированных рестартов и без растущего тренда `mem_used_pct`.

**Change 1 физически подтверждён (ADR-040):** реальные замеры на этом
Pixel 7 -- холодная загрузка ~7.1с, launch/switch и planner-цикл в
рамках бюджета; `mem`-suspend физически проверен безопасным одним
спайк-тестом (~465мс цикл, самопробуждение по Wi-Fi IRQ, USB-NCM и
serial пережили цикл).

**Change 2 физически подтверждён с честной оговоркой (ADR-041 +
ADR-042):** `check_deep_idle()` добавлен в `saai-shell` -- `wlan0
down` -> `echo mem > /sys/power/state` -> `wlan0 up`. Чистый раунд из
3 циклов: пробуждение по power-button 3 из 3, самопробуждение по
Wi-Fi 0 из 3 (не заявлено надёжным, в отличие от единичного успеха в
Change 1's спайке). Найден реальный продуктовый пробел: экран не
гаснет визуально при засыпании, пользователю неочевидно, когда нажимать
кнопку -- открытый UX-вопрос вне рамок S11.

**Change 3 физически подтверждён с честной оговоркой (ADR-043):**
комбинированный регрессионный проход по S05/S07/S09/S10 чист, ноль
регрессий от S11. 24-часовой soak не выполнен буквально -- зачтён
честный частичный результат (~15.9ч непрерывной, необычно тяжёлой
нагрузки, ноль падений core-сервисов). Найден реальный
`saai-displayd` OOM-kill (dmesg, самовосстанавливающийся по ADR-009's
restart budget), не воспроизведённый целенаправленно за три попытки --
задокументирован как открытый, не блокирующий риск. Побочно: реальная
физическая проверка ADR-009's `drm-splash` UI-fallback (сработал как
задумано после случайной ошибки собственного тестирования).

**Спринт закрыт (`Done`, 2026-09-12, с честными оговорками -- см.
[S11-performance-idle-daily-use.md](S11-performance-idle-daily-use.md#acceptance-criteria)).**

**Rollback:** Change 1 не меняет код устройства. Change 2 -- локализованное
изменение существующей idle-проверки `saai-shell`, откат -- не вызывать
`mem`, вернуться к сегодняшнему UI-only `IDLE_TIMEOUT`.

## S12 — OTA и release gate

Рабочий паспорт и декомпозиция:
[S12-ota-release-gate.md](S12-ota-release-gate.md).

**Goal:** новый стек можно безопасно и проверяемо обновить на тестовом
Pixel 7 -- подписанный manifest/artifact, staged verified download,
безопасная запись, ограниченные boot attempts, health confirmation и
автоматический откат. Переход от сохранённого Android в слоте B к
второму SaaiOS-слоту -- явно вне скоупа этого Definition of Ready, не
предрешается здесь.

**Зависит от:** S01-S11 (все `Done`). S04 даёт узкий precedent
(`fastboot flash`/`--set-active`/reboot-цикл уже надёжен), не
precedent для настоящего двух-слотового SaaiOS OTA -- такого в проекте
нет.

**Единственный реально доступный SaaiOS-слот -- A; слот B закрыт owner
policy** до отдельного, специально подтверждённого этапа с внешним
factory recovery, которого не существует -- поэтому запись обновления
(Change 4) честно скоупится как safe in-place update с backup/restore
rollback на единственном слоте, не как cross-slot A/B failover.

**Change 1 физически подтверждён (ADR-044):** спайк (read-only,
ничего не записано) нашёл, что в `native-init.c` уже существует
работающий, каждую загрузку исполняемый механизм записи раздела --
`create_partition_node()` резолвит раздел по `PARTNAME` (ядро уже
парсит GPT, отдаёт имена через `/sys/block/*/*/uevent`), а
`mark_current_slot_successful()` уже реально пишет байт в `devinfo` на
каждом boot. Первоначальная формулировка DoR ("нет ни одного пути
писать раздел") была неполной и честно исправлена тем же ADR. Решение:
OTA's writer переиспользует `create_partition_node()`, без нового
GPT-парсера, ограничен жёстким allow-list `{init_boot_a,
vendor_boot_a}`. Побочная находка для Change 5: `mark_current_slot_
successful()` вызывается до подтверждения здоровья UI -- возможно
глушит аппаратный автооткат для класса отказов, найденного S11.

**Change 2 физически подтверждён (ADR-045):** ed25519-подпись,
versioned `ManifestBody` (крейт `saai-ota-manifest`), anti-downgrade в
две проверки, allow-list как параметр вызова. 15/15 unit-тестов +
end-to-end на реальных файлах, полностью host-only.

**Change 3 физически подтверждён (ADR-046):** staged verified download
на реальном устройстве через USB-NCM (`saai-ota-stage`) -- решение
использовать `reqwest` вместо изначально предполагавшегося `busybox
wget` (уже проверен на этом таргете `saaios-runtime`). Все 5 сценариев
(успех, чужая модель, downgrade, испорченная подпись, усечённая
загрузка) подтверждены физически.

**Change 4 физически подтверждён (ADR-047):** первая в проекте
реальная запись в загрузочный раздел. Self-write-back (контент не
меняется, минимизация риска) на `vendor_boot_a` и `init_boot_a`, с
явным согласием пользователя перед первой попыткой; backup
`vendor_boot_a` независимо совпал с уже задокументированным SHA-256
оригинального образа. Две настоящие перезагрузки подряд, обе чистые,
ноль дрейфа контента.

**Change 5 физически подтверждён (ADR-048), спринт закрыт:** health
confirmation (process-presence) + персистентный boot attempt counter +
автоматический откат. Намеренно нездоровая сборка -- не синтетика:
точная копия реально прошитого `init_boot_a` с `saaios-runtime`,
заменённым на заглушку с `exit 1`, установленная через настоящий
OTA-путь (Changes 2-4). Три реальные плохие перезагрузки, автоматический
откат на третьей, четвёртая восстановительная перезагрузка вернула
полное здоровье -- независимо подтверждено сверкой хеша. Ничего не
вшито в `native-init.c`; тайминг-пробел `mark_current_slot_
successful()` (ADR-046) остаётся открытым, честно не устранён.

**Acceptance:** все пункты подтверждены физически на реальном
устройстве (см. ADR-044..048) -- неподписанный/повреждённый artifact и
запрещённый downgrade отвергаются до записи раздела; прерванная/
повреждённая staged-загрузка не доходит до записи; отказ на любом шаге
не портит активный слот; намеренно нездоровая тестовая сборка
запускает boot attempt limit и автоматический откат; обновлённый образ
переживает настоящий cold reboot (4 подряд в Change 5's drill'е).
Честно зафиксировано: переход слота B и настоящий cross-slot A/B
hardware failover не проверялись и не входят в этот спринт;
health-check -- process-presence, не протокольный; ничего не вшито в
boot-critical путь `native-init.c`.

**Rollback:** до Change 4 ни один Change не меняет установленные
разделы -- откат не требуется, сегодняшний `fastboot`-путь остаётся
рабочим без изменений. После Change 4/5 -- последний физически
подтверждённый рабочий образ слота A, сохранённый как backup; ручной
`fastboot flash` с хоста остаётся рабочим fallback независимо от
состояния нового кода.

## S13 — Завершение пользовательского интерфейса

Рабочий паспорт и декомпозиция:
[S13-user-interface-completion.md](S13-user-interface-completion.md).

**Goal:** `saai-shell` перестаёт быть демонстрационным skeleton'ом --
статус-бар, вкладки "Входящие" и "Я", список приложений на "Сейчас" и
единый визуальный проход показывают реальные данные вместо пустых
placeholder'ов и одной захардкоженной demo-карточки, физически
проверено на реальном Pixel 7.

**Зависит от:** S01-S12 (все `Done`). Найдено во время физической
проверки ADR-051/052 в этой же сессии -- пользователь прямо заявил,
что текущий интерфейс не готов для полноценного использования.

**Scope:** 5 Changes -- (1) статус-бар (часы + Wi-Fi + батарея из
`/sys`), (2) "Входящие" (реальный список задач из entity store), (3)
"Я" (сводка устройства + приложения с грантами, только чтение), (4)
"Сейчас" (реальный список установленных приложений вместо demo-
карточки), (5) единый визуальный проход. Переключение тем и точечный
revoke capability -- явно вне скоупа (см. полный документ). Ни один
backend-протокол не меняется -- весь спринт в `saai-shell`.

**Спринт закрыт (`Done`, с честными оговорками), все 5 Changes
физически подтверждены на реальном Pixel 7 (ADR-053..057).**
Change 2's остаток: тап-по-задаче код-ревьюнут, но не end-to-end
проверен -- `saai-taskd` не развёрнут на тестовом устройстве вообще
(предсуществующая дыра, не баг S13). Change 5's остаток: шрифт
технически подтверждённо заменён на Montserrat (точное совпадение
размера файла в байтах, полная поддержка кириллицы -- в отличие от
отклонённых Space Grotesk/Sora), но пользователь не воспринял
визуальную разницу с Inter на реальном экране; причина не
установлена, закрыто по совместному решению. Запрос на сетку/иконки
в духе home screen сознательно вынесен за пределы этого спринта --
отдельная, существенно большая задача (нет ни одной готовой иконки,
ни сеточной разметки), кандидат на будущий Definition of Ready.

## Управление roadmap

При закрытии спринта его состояние и Evidence обновляются одним commit с
результатом. Следующий спринт получает `Ready` только после Definition of Ready.
Новый крупный запрос сначала помещается в подходящий спринт; если он меняет
архитектурные границы, перед кодом создаётся ADR.

Шаблон новой записи: [SPRINT-TEMPLATE.md](SPRINT-TEMPLATE.md).
