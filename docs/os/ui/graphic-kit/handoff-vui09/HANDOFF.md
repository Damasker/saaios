# SaaiOS VUI-09 — Pixel 7 chrome handoff (ревизия)

**Статус:** графические сценарии для реализации и проверки; **не Pixel-снимки, не утверждение Visual v1, не доказательство наличия данных**. 25 отдельных SVG с `viewBox=1080×2400`, плюс машинный манифест. Данные вроде «Объект SOM» и «Шаг плана» — **подписи контрактных сценариев, не фиктивные live-поля**.

## Важные инварианты

- Четыре пункта навигации: Сейчас / Пространства / Поиск / Система. Object и Intent — переходы, не вкладки.
- Палитра взята из `v1-tokens.json`; poster HEX никогда не используются как shipping tokens. Цветовые статусы подписаны.
- Основная геометрия — 4-unit rhythm, типографические роли Inter, небольшие радиусы; opaque-first.
- Минимальная **логическая** hit-area 48×48; SVG показывает визуальную поверхность, а не hit-testing. Реальные области касания должны задаваться общим layout/hit-test движком и проверяться на Pixel.
- Orb idle повторяет SVG kit и остаётся локусом; microphone выключен, listen не заявляется доступным.
- Никаких вымышленных дедлайнов, процентов, прогнозов, персон-агентов, runtimes Ready или новой палитры.

## Component contracts

| Component | Данные | Тап / статус | Нет данных | Иконка |
|---|---|---|---|---|
| ObjectSummary | SOM identity/state | Навигация только если есть target, status не кнопка | Название/статус не выдумывать | object-device/file kit по типу |
| DataRow | SOM/DeviceState fact | Статус | скрыть поле | не требуется |
| RelatedRow | разрешённый peer SOM | переход по реальному peer ID, шеврон | **скрыть строку** | object-file/device kit или нужен SVG |
| QuickActionButton | action descriptor OAM + policy | вызов action только enabled | блок скрыть при 0 actions | **нужен SVG конкретного action ID** |
| PlanStepRow | Task DAG state | детали Task по реальному ID | скрыть | статус-маркер |
| ConfirmBlock | pending AskUser | confirm/cancel по policy | скрыть | action-confirm/action-cancel из kit |
| AttentionRow | real event/decision | перейти к источнику | текст «Ничего не требует внимания» | нужен SVG для event type |
| TaskRow / Progress | реальный task state, verified progress | открыть задачу | «Нет текущих задач» | маркер состояния |
| Composer / OrbHost | текст / search / idle Orb | text + search; mic **disabled** | доступный ручной ввод | orb-idle + nav-search kit |
| DeviceRow | live DeviceState/settings | переход в поддерживаемую настройку | Unavailable с причиной | object-device kit |
| RuntimeRow | обнаруженный capability | status, не обещать запуск | Android VM / notebook: Unavailable | нужен SVG |
| BottomNavigation | 4 домена | активный экран + переходы | никогда не добавлять Object/Inbox tab | 4 nav SVG из kit |

## Борда → live поле / отсутствие

| Surface | Секция борда | Live source | Когда не существует |
|---|---|---|---|
| Object | Related 1/2/4 | SOM relation + resolvable peer | Нет peer → row hidden; 0 → весь блок скрыть |
| Object | Quick actions 0/1/2 | OAM action descriptors + policy | 0 → блок скрыть; denied/offline → disabled с текстом причины |
| Intent | Steps 1/5/12 | IRAB + WSV2 Task DAG | 0 → «Нет задачи» |
| Intent | Confirm | pending AskUser only | Без AskUser — **нет кнопки** |
| Сейчас | Attention 0/1/3 | actionable events/decisions | 0 → «Ничего не требует внимания» |
| Сейчас | Current tasks 0/3 | Workflow current tasks | 0 → «Нет текущих задач» |
| Сейчас | Composer | text + search | voice remains disabled |
| Пространство | Focus/related | SOM membership, workflow | no map, no fake members |
| Система | Device / Wi-Fi / brightness | DeviceState/settings | no observation → unavailable, not zero |
| Система | Android VM / notebook | runtime availability | Unavailable until proven |

## Дельта относительно `mockups/surface-*.jpg`

- `surface-object.jpg`: real RelatedRow list with chevrons and peer resolution; OAM-only QuickActionButton group, never hardcode Open Settings / Check Identity; 0/1/2 action cases; explicit Disabled/Unavailable reason.
- `surface-intent.jpg`: textual related/activity plan becomes vertical Task DAG steps; 1/5/12 density and scroll; confirm only while AskUser exists. Worker labels only when executions exist.
- `surface-now.jpg`: attention and current work are projections of real state; mic visually disabled; Orb idle; no weather, fictitious due dates, status percentages.
- `surface-space.jpg`: SOM members, conditional focus task; no map without map data; no invented avatars.
- `surface-system.jpg`: live device/Wi-Fi/brightness, unavailable future runtimes; no fabricated telemetry.

## QA / acceptance

1. Compare with physical Pixel 7 for colors, Cyrillic, optical density and safe insets.
2. Validate all 25 scenarios against real serialized fixtures from the app. Artwork alone cannot assert data integration.
3. Confirm component SVG availability; if none, mark «нужен SVG» rather than invent a production icon.
4. Ensure 48×48 **logical** tap target and pointer/focus bounds match actual layout; press only OAM and AskUser real actions.
5. Verify overflow scroll does not overlap footer, and disabled/offline content carries readable reason.
6. This pack is **not** a replacement for source boards, HIA, or Visual v1 approval.
