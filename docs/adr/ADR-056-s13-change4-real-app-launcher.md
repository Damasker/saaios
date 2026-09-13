# ADR-056: S13 Change 4 -- реальный список приложений на "Сейчас", убран DemoAppState

## Статус

Принято, 2026-09-13.

## Контекст

"Сейчас" показывала единственную захардкоженную карточку demo-
приложения (`manage_app:org.saaios.demo-surface`), управляемую
отдельным клиентским состоянием `DemoAppState` (Missing/Installed/
Running/Stopped/CrashLimited/Pending/Error) -- вся логика запуска,
установки и отслеживания жизненного цикла была вручную завязана на
один-единственный `app_id`. `saai-appd`'s протокол давно поддерживал
`List` для произвольного числа приложений, но клиент никогда не
показывал их.

## Решение

- `root.sui`'s `demo-app` запись удалена. "Сейчас" теперь строит
  контент через `now_content_cards()`: одна карточка на каждое
  `self.installed_apps` (S13 Change 3), затем два оставшихся
  статических действия ("Объект пространства", "Новое намерение") --
  тем же `stacked_row_rect`-подходом, что уже используют "Входящие" и
  "Я".
- `now_action_at()` -- аналог `content_action_at` для рантайм-
  контента: возвращает `"manage_app:<id>"` для строки приложения или
  action-строку статической карточки.
- `invoke_app_launch()` заменяет весь `DemoAppState`-специфичный путь
  запуска -- обобщён на любой `app_id`: не трогает уже запущенное,
  иначе вызывает `self.appd.launch(app_id)`. Установка "с нуля"
  (бывший `DemoAppState::Missing` → `appd.install()`) сознательно не
  перенесена -- по решению пользователя, список показывает только уже
  установленные приложения, экран установки нового приложения вне
  скоупа S13.
- `apply_appd_message()` обобщён: раньше вся его логика была matching
  по `app_id == DEMO_APP_ID`; теперь только `ConsentRequired`/
  `ConsentDecided` (единственное, что реально требовало реакции
  здесь -- остальное уже брал на себя `update_app_caches`). Имя
  приложения для экрана согласия теперь берётся из `installed_apps`
  вместо захардкоженной строки `"Saai Demo"` -- `PendingConsent.
  app_name`/`Frame::Consent.app_name` соответственно сменили тип с
  `&'static str` на `String`.
- Удалены полностью: `enum DemoAppState` и его `impl`, константы
  `DEMO_APP_ID`/`DEMO_APP_ACTION`/`DEMO_PACKAGE_PATH`, поле
  `demo_app_state`, а также ставший недостижимым
  `AppdClient::install()` (в протоколе `ClientRequest::Install`
  остаётся -- удалён только клиентский вызов без вызывающего кода).

## Test

Host: `cargo test -p saai-shell` -- 22/22 зелёные. Тест
`demo_action_geometry_comes_from_sui_markup` переписан в
`now_page_static_actions_come_from_sui_markup` (проверяет 6 записей в
`ROOT_CONTENT_ACTIONS`, позиции `selected-entity`/`new-intent` вместо
удалённой `demo-app`). `cargo clippy -p saai-shell` чист (в том числе
предупреждение о неиспользуемом `AppdClient::install` устранено
удалением, не подавлением). Кросс-компиляция чиста, hash
`408c014f5c2b29af251ba68635004a881da12c17475bff7afe8468f53c3afb10`.

Device (`cp && mv` + `chmod`, ADR-042): физически подтверждено --
"Сейчас" показывает реальный список установленных приложений; тап по
карточке demo-приложения его запускает (`appd.launch`), приложение
подтверждённо появилось на экране.

## Threat / privacy impact

Экран согласия теперь показывает РЕАЛЬНОЕ имя любого приложения
(из `installed_apps`), а не жёстко заданную строку -- честнее для
пользователя, не расширяет полномочия.

## Rollback

Предыдущий физически подтверждённый `saai-shell` (`cp && mv`).

## Ссылки

- `docs/os/sprints/S13-user-interface-completion.md` -- Change 4 из 5
  (последний функциональный Change; Change 5 -- визуальный проход).
- ADR-054/055 -- `stacked_row_rect` и `installed_apps`, оба
  переиспользованы здесь без изменений.
- `services/saai-shell/src/main.rs` -- `now_content_cards`,
  `now_action_at`, `invoke_app_launch`, `apply_appd_message`.
