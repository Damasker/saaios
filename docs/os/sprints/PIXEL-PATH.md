# Путь на Pixel 7 — один очередь, не шесть выходных

Status: **active sequence after ADR-118…186.**
Visual Language remains the only hardware-changing *shell* experiment.
Service binaries (`saaios-runtime`, `saai-taskd`, `saai-entityd`) могут
прошиваться в ту же неделю — это не второй DRM/GPU эксперимент.

Новых фундаментальных одноустройственных моделей не добавлять. Дальше —
vertical slices в существующие процессы и VUI-04…09.

## Зачем этот файл

Восемь независимых треков с `*-10` на телефоне означают, что ни один
слой не доезжает до panther, пока Visual держит «единственный
эксперимент». Это слишком медленно. Ниже — один продуктовый путь.

Детальные ADR и таблицы задач остаются в своих roadmap. Этот файл
отвечает только: **что следующим касается телефона и в каком порядке.**

## Что уже есть (host, в дереве)

| Слой | ADR | Host сейчас | На телефоне |
|---|---|---|---|
| SOM / OAM / IRAB | 118–120 | crates + entityd | не прошито как продукт |
| Work Scheduler v2 | 121 | DAG + derived ready set в `saai-taskd` | WORK-02 host; не прошито |
| World Model | 122 | `saai-observation` types | нет `saai-deviced`, не нужно для P0 |
| Attention | 123 | `saai-attention` projection | Orb + NOW + Inbox **на panther** |
| UAM | 124 | `saai-authority` + live `decide_named` | `TaskConfirm` живой; AUTH-04 host |
| Memory MLP | 125 | `(space,key)` + labelled context | **P0 runtime прошит** (MEM-01/02/05) |
| Visual | 111–116, 126–186 | VUI-00…06 on panther; VUI-07 EventRow Inbox **на panther** (`c1547c02…`); VUI-07 Spaces list **на panther** (`9bb75db5…`); VUI-07 Wi-Fi list **на panther** (`c80bb666…`); VUI-07 Bluetooth list **на panther** (`5b37c5bc…`); VUI-07 trusted clients **на panther** (`cd207b18…`); VUI-07 Wi-Fi password **на panther** (`0eeb36d3…`); VUI-07 PIN setup **на panther** (`eb4cb508…`); VUI-07 lock idle **на panther** (`a19327cb…`); VUI-07 intent input **на panther** (`6239bebd…`); VUI-07 developer surface **на panther** (`bbc11a30…`); VUI-07 Object View **на panther** (`e2081d84…`); VUI-07 apps grid **на panther** (`a57da14a…`); VUI-07 Inbox header **на panther** (`75f1f054…`); VUI-07 Spaces header **на panther** (`464c0e92…`); VUI-07 Система header **на panther** (`15fc3487…`); VUI-07 consent header **на panther** (`cff22339…`); VUI-07 PIN setup header **на panther** (`e8b215aa…`); VUI-07 remote pairing header **на panther** (`4a1f7553…`); VUI-07 Bluetooth header **на panther** (`efbab13a…`); VUI-07 Wi-Fi header **на panther** (`d562249b…`); VUI-07 trusted-clients header **на panther** (`b73f9433…`); VUI-07 lock attention **на panther** (`55f7bd25…`); VUI-07 lock/PIN keyboard **на panther** (`32d50a67…`); VUI-07 compact QWERTY **на panther** (`874ad0e2…`); VUI-07 keyboard press **на panther** (`3a97f0e8…`); VUI-07 DevSurface header **на panther** (`c9f52227…`); VUI-07 lock device state **на panther** (`88121ad2…`); VUI-07 lock sleep AOD **на panther** (`c3fd2fcf…`); VUI-07 empty/loading/offline **на panther** (`e18f3d9d…`); VUI-07 blocked/failed **на panther** (`eba9d216…`); VUI-07 confirmation **на panther** (`c7cbee14…`); VUI-07 permission/recovery **на panther** (`812f1684…`); VUI-07 overflow Назад **на panther** (`b06c0bb1…`); VUI-07 DevSurface scroll **на panther** (`bbde1ab2…`); VUI-07 keyboard avoidance **на panther** (`3d289b66…`); VUI-07 focus order **на panther** (`6d465506…`); VUI-07 interrupted key **на panther** (`c095cc5d…`); VUI-07 PIN setup avoidance **на panther** (`1372267b…`); VUI-07 list Назад closeout **на panther** (`693e77c7…`); VUI-07 gallery SurfacePattern host goldens (ADR-166); VUI-08 motion clock **на panther** (ADR-167; leave Отмена); VUI-08 tab Selection **на panther** (ADR-168; leave Сейчас); VUI-08 compose Context **на panther** (ADR-169; leave Отмена); VUI-08 Orb ActivityPulse **на panther** (ADR-170; leave Сейчас); VUI-08 haptic policy **на panther** (ADR-171; leave Сейчас); VUI-08 FramePace **на panther** (ADR-172; leave Сейчас); VUI-08 scroll p95 **на panther** (ADR-173; leave Сейчас); VUI-08 reduced motion **на panther** (`28c91505…`; ADR-174; leave Сейчас); VUI-08 FramePace traces **на panther** (`9df60b2a…`; ADR-175; leave Сейчас); VUI-08 first visible **на panther** (`3bc3e374…`; ADR-176; leave Сейчас); VUI-08 idle redraw **на panther** (`b00a13cf…`; ADR-177; leave Сейчас); VUI-08 frame backend **на panther** (`37a8014d…`; ADR-178; leave Сейчас); VUI-08 haptic acceptance **на panther** (`37a8014d…`; ADR-179; leave Сейчас); VUI-09 `.sui` v2 vocabulary host (ADR-180; chrome unchanged `37a8014d…`; leave Сейчас); VUI-09 `.sui` v2 grammar host (ADR-181; chrome unchanged `37a8014d…`; leave Сейчас); VUI-09 `.sui` v2 properties host (ADR-182; chrome unchanged `37a8014d…`; leave Сейчас); VUI-09 `.sui` v1 rollback host (ADR-183; chrome unchanged `37a8014d…`; leave Сейчас) | **saai-ui-compiler ADR-186** |

## Две очереди, не девять

```text
SHELL QUEUE (один hardware-changing эксперимент)
  VUI-04 navigation / status / Orb
       ↓
  VUI-05 Object / Intent / Task  ← сюда же ATTN-02 и WORK-08
       ↓
  VUI-06 Система                 ← сюда же ручной Memory review, не MEM-08 отдельно
       ↓
  VUI-07…09

SERVICE QUEUE (можно шить параллельно shell)
  P0  saaios-runtime     MEM-01 same-key + MEM-02 None≠All + MEM-05 no model write
  P1  runtime + taskd    MEM-02/05, AUTH-04 decide_named, WORK ready-set
  P2  runtime            MEM-03/06 typed record + erase
  P3  later              WORLD cache; daemon только по доказанному IPC
```

APP-COMPAT, Learning (MEM-09), голос, PCE, `saai-deviced` — **не в этом
пути**. Их не планировать, пока P0–P2 не пощупаны на panther.

## P0 — закрыто на panther (2026-09-18)

1. VUI-04 на shell — remainder (`Я`→`Система` **на panther**,
   `1d191d7a…`). SafeInsets + rotation/keyboard/rapid-tab на host.
2. Прошит `saaios-runtime` MEM-01/02/05 (`cc5f9913…`, reboot `-f`).
3. Pixel: `p0_same_key` Home=`home-value` / Work=`work-value`; default
   recall пуст; `all=true` не смешивает; модель без `memory.remember`.

Это закрывает дыру ADR-038, уже живущую на устройстве. Не ждать MEM-10.

## P1 — сервисы, без нового UI

Пока VUI-04 идёт, на host добить и сразу шить runtime/taskd:

- MEM-02: `None` ≠ All
- MEM-05: модель не пишет ExplicitFact/Preference (`memory.remember` снять
  или понизить до hypothesis/proposal)
- AUTH-04: `decide_named` видит live grants (реальный баг, не новый экран)
- WORK-02: derived ready set, concurrency=1

Pixel smoke: remember из Work не течёт в Home; модель не создаёт
authoritative fact; опасный Action по-прежнему через `TaskConfirm`.
AUTH-04 runtime прошит 2026-09-18 (`af90a6b3…`): confirm без pending —
`no pending confirmation`. `saai-taskd` не в native-init, WORK-02 пока
host-only derived view.

## P2 — едет на VUI, не отдельным треком

Не открывать ATTN-02 / WORK-08 / MEM-08 как самостоятельные phone sprints.

| VUI slice | Берёт с собой |
|---|---|
| VUI-04 remainder | `Я`→`Система` **на panther** (`1d191d7a…`). SafeInsets, rotation, keyboard, rapid-tab on host |
| VUI-05 | NOW ATTN-02, Inbox ATTN-03, WORK-08 **на panther** (`63b8b64` then current shell); Object View + offline + `AgentSummary` + gallery **host** |
| VUI-06 `Система` | inventory + domain grouping + honest missing/offline + scroll-cache + tab rename **на panther** (`1d191d7a…`); MEM-08 still blocked |
| VUI-07 Inbox | `EventRow` + empty vs offline **на panther** (`c1547c02…`) |
| VUI-07 Spaces | live `SpaceRow` list **на panther** (`9bb75db5…`); Space detail later |
| VUI-07 Wi-Fi | live `WifiRow` list **на panther** (`c80bb666…`); password keyboard unchanged |
| VUI-07 Bluetooth | live `BluetoothRow` list **на panther** (`5b37c5bc…`) |
| VUI-07 trusted clients | live `TrustedClientRow` list **на panther** (`cd207b18…`) |
| VUI-07 Wi-Fi password | `Field` masked preview **на panther** (`0eeb36d3…`) |
| VUI-07 PIN setup | `Field` masked preview **на panther** (`eb4cb508…`) |
| VUI-07 lock idle | Canvas + clock + tap hint **на panther** (`a19327cb…`) |
| VUI-07 intent input | `Field` text preview **на panther** (`6239bebd…`) |
| VUI-07 developer surface | live Static `DataRow` list **на panther** (`bbc11a30…`) |
| VUI-07 Object View | `ObjectSummary` + NOW tap **на panther** (`e2081d84…`) |
| VUI-07 apps grid | `ContextHeader` + live apps only **на panther** (`a57da14a…`) |
| VUI-07 Inbox header | `ContextHeader` + live EventRow **на panther** (`75f1f054…`) |
| VUI-07 Spaces header | `ContextHeader` + live SpaceRow **на panther** (`464c0e92…`) |
| VUI-07 Система header | `ContextHeader` + live SettingRow list **на panther** (`15fc3487…`) |
| VUI-07 consent header | `ContextHeader` + requested DataRow **на panther** (`cff22339…`) |
| VUI-07 PIN setup header | `ContextHeader` + Password `Field` **на panther** (`e8b215aa…`) |
| VUI-07 remote pairing header | `ContextHeader` + live client DataRow **на panther** (`4a1f7553…`) |
| VUI-07 Bluetooth header | `ContextHeader` + live BluetoothRow **на panther** (`efbab13a…`) |
| VUI-07 Wi-Fi header | `ContextHeader` + live WifiRow **на panther** (`d562249b…`) |
| VUI-07 trusted-clients header | `ContextHeader` + live TrustedClientRow **на panther** (`b73f9433…`) |
| VUI-07 lock attention | compact lock-state `StatusIndicator` **на panther** (`55f7bd25…`) |
| VUI-07 lock/PIN keyboard | ADR-029 `Node`/`hit_test()` keys + Password occupancy **на panther** (`32d50a67…`; PIN unset, left via Отмена) |
| VUI-07 compact QWERTY | bottom-docked staggered ADR-029 keys **на panther** (`874ad0e2…`; left via Отмена) |
| VUI-07 keyboard press | `ColorRole::Pressed` + `KeyTick` **на panther** (`3a97f0e8…`; left via Отмена) |
| VUI-07 DevSurface header | `ContextHeader` + live diagnostic DataRow **на panther** (`c9f52227…`; 7-tap, Назад below fold) |
| VUI-07 lock device state | live Caption `Заряд N%` **на panther** (`88121ad2…`; PIN unset, tap-unlock) |
| VUI-07 lock sleep AOD | clock-only deep-idle, wake is not unlock **на panther** (`c3fd2fcf…`) |
| VUI-07 empty/loading/offline | `SurfacePattern` + Bluetooth «Сканирование…» **на panther** (`e18f3d9d…`) |
| VUI-07 blocked/failed | `SurfacePattern` Failed + Bluetooth `PAIR-ERROR` **на panther** (`eba9d216…`) |
| VUI-07 confirmation | `DecisionOverlay` Body facts on Object View **на panther** (`c7cbee14…`; NOW empty, no waiting task) |
| VUI-07 permission/recovery | OAM blocked + Failed mark on Bluetooth `PAIR-ERROR` **на panther** (`812f1684…`) |
| VUI-07 overflow Назад | `stacked_control_rect` docks DevSurface back on-screen **на panther** (`b06c0bb1…`; 7-tap, leave Назад) |
| VUI-07 DevSurface scroll | data rows scroll above docked Назад **на panther** (`bbde1ab2…`; 7-tap, swipe, leave Назад) |
| VUI-07 keyboard avoidance | Intent `Field` sits on docked QWERTY **на panther** (`3d289b66…`; leave Отмена) |
| VUI-07 focus order | Intent Field is focus stop 0 in the layout tree **на panther** (`6d465506…`; leave Отмена) |
| VUI-07 interrupted key | down+up must share one keyboard action **на panther** (`c095cc5d…`; leave Отмена) |
| VUI-07 PIN setup avoidance | PIN Field sits on docked dialer **на panther** (`1372267b…`; leave Отмена) |
| VUI-07 list Назад closeout | Wi-Fi/BT/trusted Назад docks on-screen **на panther** (`693e77c7…`; leave Назад) |
| VUI-07 gallery patterns | `SurfacePattern` fixtures + host goldens (ADR-166; no gallery tap) |
| VUI-08 motion clock | `MotionClock` + `wl_surface.frame` only while `needs_frame`; keyboard MicroFeedback hold **на panther** (ADR-167; leave Отмена) |
| VUI-08 tab Selection | tab `Pressed` holds `Selection` 180 ms after release **на panther** (ADR-168; leave Сейчас) |
| VUI-08 compose Context | intent Field `Focus` outline for 240 ms on open **на panther** (ADR-169; leave Отмена) |
| VUI-08 Orb ActivityPulse | Running inset loops 240 ms on / 240 ms off; Idle Orb still **на panther** (ADR-170; leave Сейчас) |
| VUI-08 haptic policy | `HapticEvent` map + rate-limit + Звук «Виброотклик» **на panther** (ADR-171; leave Сейчас) |
| VUI-08 FramePace | 32-sample commit log + `/run/saaios/shell-frame.last` **на panther** (ADR-172; leave Сейчас) |
| VUI-08 scroll p95 | scroll-only p95 ≤ 50 ms on host; `p95_scroll`/`p95_ok` **на panther** (ADR-173; leave Сейчас) |
| VUI-08 reduced motion | «Меньше движения» drops clocks on the same tap **на panther** (`28c91505…`; ADR-174; leave Сейчас) |
| VUI-08 FramePace traces | `FrameSurface` ring dump `/run/saaios/shell-frame.trace` **на panther** (`9df60b2a…`; ADR-175; leave Сейчас) |
| VUI-08 first visible | non-scroll `input_to_commit` ≤ 50 ms; `input_ok` **на panther** (`3bc3e374…`; ADR-176; leave Сейчас) |
| VUI-08 idle redraw | quiet Сейчас keeps `seq` still; `idle_ok` **на panther** (`b00a13cf…`; ADR-177; leave Сейчас) |
| VUI-08 frame backend | main commit names `backend=dmabuf`/`shm` **на panther** (`37a8014d…`; ADR-178; leave Сейчас) |
| VUI-08 haptic acceptance | KeyPress-only, 15 ms rate-limit, tabs/Orb silent **на panther** (`37a8014d…`; ADR-179; leave Сейчас) |
| VUI-09 `.sui` v2 vocabulary | proven component/surface names; `sui 1` only **host** (ADR-180; chrome unchanged `37a8014d…`; leave Сейчас) |
| VUI-09 `.sui` v2 grammar | `compile_v2()` names ADR-180 components; `compile()` stays v1 **host** (ADR-181; chrome unchanged `37a8014d…`; leave Сейчас) |
| VUI-09 `.sui` v2 properties | `text`/`color`/`spacing`/`inset`/`scroll`/`loc`/`focus`/`a11y` **host** (ADR-182; chrome unchanged `37a8014d…`; leave Сейчас) |
| VUI-09 `.sui` v1 rollback | `compile()` on `root.sui`; `compile_v2` stays off the build **host** (ADR-183; chrome unchanged `37a8014d…`; leave Сейчас) |
| VUI-09 v1 shared layout | `layout_v1_root()` from `compile_v1_rollback()`; leftover NOW cards stay no-tap **на panther** (`1079a5db…`; ADR-184; leave Сейчас) |
| VUI-09 public subset | `compile_v2_public()` rejects privileged names; `compile()` stays v1 **host** (ADR-185; chrome unchanged `1079a5db…`; leave Сейчас) |
| VUI-09 public API docs | NOW example + stability labels + migration page **host** (ADR-186; chrome unchanged `1079a5db…`; leave Сейчас) |
| VUI-09 allowlist leftover NOW | empty `root.sui` content, no diagnostic RGB, `draw_action_card` reuse **на panther** (`36bcc8eb…`; ADR-187; leave Сейчас) |
| VUI-09 named text sizes | `role_px` + leftover size names; paint unchanged **на panther** (`e8865301…`; ADR-188; leave Сейчас) |
| VUI-09 verification ledger | matrix cells cited; not Visual v1 sign-off **host** (ADR-189; chrome unchanged `e8865301…`; leave Сейчас) |
| VUI-09 increased text | `text_scale_pct` 150 then 100 on HEAD **на panther** (`e8865301…`; ADR-190; leave Сейчас) |
| VUI-09 radio-off | `wlan0` down → status `Нет сети` then restored **на panther** (`e8865301…`; ADR-191; leave Сейчас) |
| VUI-09 shell restart | unlocked `saai-shell` kill + respawn **на panther** (`e8865301…`; ADR-192; leave Сейчас) |
| VUI-09 known limitations | ledger gaps + Visual v2 backlog **host** (ADR-193; chrome unchanged `e8865301…`; leave Сейчас) |
| VUI-09 v2 tab hits | `layout_v2` public NOW ≡ v1 tabs **host** (ADR-194; chrome unchanged `e8865301…`; leave Сейчас) |
| VUI-09 safe insets | `EdgeInsets::from_safe`; top is status layer **host** (ADR-195; chrome unchanged `e8865301…`; leave Сейчас) |
| VUI-09 v2 named tabs | nested `tab` on `BottomNavigation`; empty nav invents none **host** (ADR-196; chrome unchanged `e8865301…`; leave Сейчас) |
| VUI-09 ActionCard/tab roles | `Label`/`Caption` not Title **на panther** (`5eb6a27f…`; ADR-197; leave Сейчас) |
| VUI-09 status/key roles | status `Label`, keys `Caption` **на panther** (`3850427a…`; ADR-198; leave Сейчас) |
| VUI-09 v2 footer rows | nested `row apps`/`intent`; `layout_v2` ≡ live footer **host** (ADR-199; chrome unchanged `3850427a…`; leave Сейчас) |
| VUI-09 v2 object hit | `ObjectSummary` → `open_object`; `layout_v2` ≡ live object **host** (ADR-200; chrome unchanged `3850427a…`; leave Сейчас) |
| VUI-09 leftover text budget | badge/kicker/swatch/app-tile stay named below Caption **на panther** (`3850427a…`; ADR-201; Приложения, no launch; leave Сейчас) |
| VUI-09 v2 Inbox rows | `EventRow` → `open_object` on `stacked_row_rect` **host** (ADR-202; chrome unchanged `3850427a…`; Inbox tab, no row tap; leave Сейчас) |
| VUI-09 v2 Spaces rows | `SpaceRow` → `select_space:<loc>` on `stacked_row_rect` **host** (ADR-203; chrome unchanged `3850427a…`; Spaces tab, no row tap; leave Сейчас) |
| VUI-09 v2 Me rows | `SettingRow` → interned `loc` on `stacked_row_rect` **host** (ADR-204; chrome unchanged `3850427a…`; Система tab, no row tap; leave Сейчас) |
| VUI-09 v2 Wi-Fi rows | `WifiRow` → `connect_wifi` on `stacked_row_rect` **host** (ADR-205; chrome unchanged `3850427a…`; list not opened; leave Сейчас) |
| VUI-09 v2 Bluetooth rows | `BluetoothRow` → `pair_bluetooth` on `stacked_row_rect` **host** (ADR-206; chrome unchanged `3850427a…`; list not opened; leave Сейчас) |
| VUI-09 v2 trusted rows | `TrustedClientRow` → `revoke_trusted_client` on `stacked_row_rect` **host** (ADR-207; privileged; `compile_v2_public` rejects; list not opened; leave Сейчас) |
| VUI-09 v2 capability rows | `CapabilityRow` occupies `stacked_row_rect` with no action **host** (ADR-208; privileged; `compile_v2_public` rejects; no Me app tap; leave Сейчас) |
| VUI-09 lock cycle | no-PIN tap-unlock **на panther** (`3850427a…`; ADR-209; marker restored; leave Сейчас) |
| VUI-09 display restart | `saai-displayd` kill + native-init respawn **на panther** (`3850427a…`; ADR-210; marker on; leave Сейчас) |
| VUI-09 cold boot | `reboot -f` lock then tap-unlock **на panther** (`3850427a…`; ADR-211; marker restored; leave Сейчас) |
| VUI-09 7-tap gallery | DevSurface Диагностика then Назад **на panther** (`3850427a…`; ADR-212; leave Сейчас) |
| VUI-09 thin tuning | physical lighting, leftover visual nits, gallery fixtures **parked last** (ADR-213; not next) |
| VUI-09 v2 trailing rows | `row refresh`/`scan`/`back` → `list_*` on `stacked_trailing_rect` **host** (ADR-214; chrome unchanged `3850427a…`; leave Сейчас) |
| VUI-09 v2 Me flatten | `flatten_me_rows` + `scrolled_row_rect` offset 0 **host** (ADR-215; chrome unchanged `3850427a…`; leave Сейчас) |
| VUI-09 v2 production | `compile_v2()` + `layout_v2` `root_view` **на panther** (ADR-216; `c05eedf9…`; leave Сейчас) |
| VUI-09 v2 NOW hits | compiled `now.sui` footer/object **на panther** (ADR-217; `d9faae38…`; leave Сейчас) |
| VUI-09 v2 live lists | generated Inbox/Spaces/Wi-Fi/Bluetooth/trusted hits **на panther** (ADR-218; `c8870c10…`; leave Сейчас) |
| VUI-09 v2 Me scroll | `layout_v2_scrolled` **на panther** (ADR-219; `44afa590…`; leave Сейчас) |
| VUI-09 v2 apps grid | generated `Button` tiles match `now_grid_rect` **на panther** (ADR-220; `9c146600…`; Приложения without launch; leave Сейчас) |
| VUI-09 v2 overlay hits | `Field` + decision `Button`s from `layout_v2` **host** (ADR-221) |
| VUI-09 Keyboard IME | privileged `Keyboard` bound to Field; USB HID replaces OSK **host** (ADR-222; no volume/power; leave Сейчас) |
| VUI-09 v2 Orb hits | generated `OrbHost` / `orb-menu:` match `orb_zone_rect` **на panther** (ADR-223; `a49dfadf…`; do not tap Изменить; leave Сейчас) |
| VUI-09 v2 diagnostic hits | generated `DataRow` + `row back` match `stacked_control_rect` **host** (ADR-224; do not 7-tap; leave Сейчас) |
| VUI-09 NOW paint | `draw_now` chrome slots from `now_view()` **на panther** (ADR-225; `94613f7a…`; leave Сейчас) |
| VUI-09 list paint | Inbox/Spaces/Wi-Fi/Bluetooth/trusted cards from generated `layout_v2` **на panther** (ADR-226; `f91144ab…`; leave Сейчас) |
| VUI-09 Me paint | «Я» cards from `layout_v2_scrolled` **на panther** (ADR-227; `65fb3473…`; do not open «Я»; leave Сейчас) |
| VUI-09 apps paint | Приложения tiles from generated `layout_v2` **на panther** (ADR-228; `40250fc4…`; do not open Приложения; leave Сейчас) |
| VUI-09 overlay paint | Field/decision Buttons from generated `layout_v2` **host** (ADR-229; do not tap Разрешить/Сопряжь; leave Сейчас) |

Если VUI-05 ещё не готов, Attention **не** рисуем отдельным экраном.

## P3 — после того как P0 виден на panther

- MEM-03 `MemoryRecord` v2 + provenance
- MEM-06 physical erase
- WORLD-02 ObservationCache внутри runtime (без daemon)
- WORLD-05 Verification только когда WORK-03 жив

`saai-deviced` (WORLD-03) — только если cache + несколько consumers
реально требуют IPC. Не заранее.

## Что сознательно не делать сейчас

- новые ADR «ещё одна фундаментальная модель»
- Learning / personality / vector DB
- отдельный `saai-memoryd` / `saai-learningd` / `policyd`
- ждать AUTH-10, чтобы трогать confirmation (он уже есть)
- шесть Pixel-weekend'ов `WORK-09 WORLD-08 ATTN-07 AUTH-10 MEM-10`

Один device pass в неделю покрывает то, что реально прошито в P0–P2.

## Definition of Done для «лежит на телефоне»

Host-тесты зелёные **и** соответствующий бинарник прошит на panther **и**
записан reboot/recall/confirm. Пока прошивки нет — статус host-only,
даже если crate полный.
