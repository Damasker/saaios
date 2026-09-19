# Путь на Pixel 7 — один очередь, не шесть выходных

Status: **active sequence after ADR-118…161.**
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
| Visual | 111–116, 126–161 | VUI-00…06 on panther; VUI-07 EventRow Inbox **на panther** (`c1547c02…`); VUI-07 Spaces list **на panther** (`9bb75db5…`); VUI-07 Wi-Fi list **на panther** (`c80bb666…`); VUI-07 Bluetooth list **на panther** (`5b37c5bc…`); VUI-07 trusted clients **на panther** (`cd207b18…`); VUI-07 Wi-Fi password **на panther** (`0eeb36d3…`); VUI-07 PIN setup **на panther** (`eb4cb508…`); VUI-07 lock idle **на panther** (`a19327cb…`); VUI-07 intent input **на panther** (`6239bebd…`); VUI-07 developer surface **на panther** (`bbc11a30…`); VUI-07 Object View **на panther** (`e2081d84…`); VUI-07 apps grid **на panther** (`a57da14a…`); VUI-07 Inbox header **на panther** (`75f1f054…`); VUI-07 Spaces header **на panther** (`464c0e92…`); VUI-07 Система header **на panther** (`15fc3487…`); VUI-07 consent header **на panther** (`cff22339…`); VUI-07 PIN setup header **на panther** (`e8b215aa…`); VUI-07 remote pairing header **на panther** (`4a1f7553…`); VUI-07 Bluetooth header **на panther** (`efbab13a…`); VUI-07 Wi-Fi header **на panther** (`d562249b…`); VUI-07 trusted-clients header **на panther** (`b73f9433…`); VUI-07 lock attention **на panther** (`55f7bd25…`); VUI-07 lock/PIN keyboard **на panther** (`32d50a67…`); VUI-07 compact QWERTY **на panther** (`874ad0e2…`); VUI-07 keyboard press **на panther** (`3a97f0e8…`); VUI-07 DevSurface header **на panther** (`c9f52227…`); VUI-07 lock device state **на panther** (`88121ad2…`); VUI-07 lock sleep AOD **на panther** (`c3fd2fcf…`); VUI-07 empty/loading/offline **на panther** (`e18f3d9d…`); VUI-07 blocked/failed **на panther** (`eba9d216…`); VUI-07 confirmation **на panther** (`c7cbee14…`); VUI-07 permission/recovery **на panther** (`812f1684…`); VUI-07 overflow Назад **на panther** (`b06c0bb1…`); VUI-07 DevSurface scroll **на panther** (`bbde1ab2…`); VUI-07 keyboard avoidance **на panther** (`3d289b66…`) | **saai-shell `3d289b66…` прошит** |

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
