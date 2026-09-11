# ADR-036: S10 Change 3 -- schedules are a native, space-scoped entity, not a port of `automation-engine`

## Статус

Принято, 2026-09-11.

## Контекст

S10's Definition of Ready called расписания/триггеры "AutomationRule/
TriggerKind's персистентная, привязанная к пространству версия" --
без готового ответа, как именно. По прецеденту S09 Change 1 и S10
Change 1, перед кодом нужен спайк, закрывающий этот вопрос конкретно.

Уже существующий `crates/automation-engine` (Platform Track) выглядит
как готовый кандидат для переиспользования: `TriggerKind`,
`AutomationRule`, `AutomationAction`, cooldown-логика. Прочитан его
реальный код (`crates/automation-engine/src/lib.rs`,
`services/saaios-runtime/src/main.rs`):

- `TriggerKind` -- только `NamedEvent` и `ToolResultThreshold`. Ни
  одного варианта, завязанного на время (`interval`/`cron`/`at`) --
  время как класс триггера там просто не существует.
  `AutomationRule`'s список -- `Vec` в памяти одного процесса
  (`AutomationEngine::default_rules()`), не персистентен и не
  space-scoped ни в каком смысле.
- Единственный существующий "авто-запуск без явного Intent" путь --
  `AutomationAction::AutoDiagnose` -> событие `AutomationAutoDiagnose`
  -> `spawn_auto_diagnose_worker` в `saaios-runtime/src/main.rs`
  вызывает `runtime.handle_user_text(&prompt)` **напрямую, в процессе,
  без единого обращения к `saai-entity-store`**. Ничего не создаётся
  ни как `Intent`, ни как `Task` -- нет `space_id`, нет
  cold-reboot-персистентности, нет видимости в `Сейчас`, нет
  `WorkflowStatus`/confirmation-конвейера. Это ровно то состояние,
  из которого S09 целиком вывело систему (см. ADR-030) -- и ровно то,
  что S10's собственный Acceptance criterion явно запрещает
  ("предложенный planner'ом Action проходит тот же
  WorkflowStatus/confirmation-конвейер, что и explicit-путь S09").
- `automation-engine` зависит от `protocol` и `event-bus`
  (`crates/automation-engine/Cargo.toml`) -- то же ограничение, что
  ADR-030 уже нашло и построило вокруг него весь S09: native OS Track
  и Platform Track сегодня не делят код, и `saai-taskd` не должен
  начинать зависеть от `protocol`-крейта только ради одного enum.

## Решение

Расписание -- новый native `entity_type` (`saaios.schedule`) поверх
уже существующего, уже физически cold-reboot-персистентного
`saai-entity-store` -- **тот же самый прецедент**, что ADR-030 уже
применило к `Intent`/`Task`/`Action`/`Result`, не порт
`automation-engine`'s словаря.

Свойства: `every_secs` (интервал, простейший достаточный для
вертикального среза вид триггера -- ни `cron`, ни `at:HH:MM` не
нужны, чтобы физически доказать сам механизм), `text` (текст, из
которого создаётся `Intent` -- дословно то же поле, что ручной ввод
через клавиатуру уже кладёт в `saaios.intent.properties.text`),
`enabled`, `last_fired_at` (опционально), `fire_count`.

`saai-taskd::run()` получает periodic tick (`tokio::select!` рядом с
уже существующим `next_event().await`, не отдельный сервис -- ADR-030
и сам CLI `saai-taskd`'s текст уже придерживаются "no new service
unless something actually asks for it"). На каждый tick: список
`saaios.schedule` в своём пространстве, для просроченных --
`create_entity(INTENT_TYPE, ...)` с тем же `text`, затем
`update_entity` расписания (`last_fired_at`, `fire_count`).

**Ключевое свойство решения**: созданный так `Intent` неотличим от
введённого вручную через клавиатуру -- `process_intent()`/
`process_planner_intent()` не меняются вообще ни на строку. Весь уже
физически доказанный конвейер (ADR-029/030/031/032/033/034) от
`Intent` до `WorkflowStatus`/`Frame::TaskConfirm` работает
автоматически, без дублирования риск-классификации -- то же
центральное свойство, которое ADR-033 уже установило для planner'а.

## Отклонённые варианты

- **Портировать `TriggerKind`/`AutomationRule` в `saai-taskd`**:
  требует тащить `protocol`/`event-bus` в native OS Track (ADR-030's
  граница) ради enum, у которого к тому же нет временных триггеров --
  пришлось бы всё равно добавлять `TriggerKind::Interval` с нуля,
  просто в чужом крейте.
- **Использовать уже существующий `AutomationAutoDiagnose`-путь как
  есть**: это единственный уже работающий "авто-запуск", но он
  напрямую вызывает модель в обход `saai-entity-store` -- ни
  `space_id`, ни `WorkflowStatus`, ни персистентности. Нарушает
  собственный Acceptance criterion S10.
- **Планировщик внутри `saaios-runtime` (Platform Track), пишущий в
  `saai-entity-store` напрямую**: тот же Кандидат B, отклонённый
  ADR-033 для planner'а, по той же причине -- второй писатель в
  конвейер с чужой стороны границы, дублирующий то, что `saai-taskd`
  уже делает.
- **Отдельный новый native-демон только для расписаний**: не оправдан
  -- `saai-taskd` уже единственный потребитель `saaios.intent`/
  `saaios.task` в своём пространстве и уже держит соединение с
  `saai-entityd`; добавление periodic tick в его существующий цикл
  дешевле нового процесса с собственным жизненным циклом/supervision.

## Последствия

- `cron`/`at:HH:MM`-триггеры, множественные пространства для одного
  расписания и явный UI для создания расписания (сейчас -- только
  прямой `create_entity` через протокол, как и `Intent` в S09 Change 2
  до появления клавиатуры) остаются вне этого Change -- вертикальный
  срез доказывает механизм, не полный UX.
- Реализация и физическая проверка (реальный `saaios.schedule`,
  реальный периодический tick, реальный `Intent` без единого касания
  экрана) -- следующий, отдельный шаг, тем же прецедентом, что S09
  Change 2 и S10 Change 2.

## Ссылки

- ADR-030 -- прецедент "новый entity_type поверх saai-entity-store,
  не порт Platform Track", применённый здесь к расписаниям.
- ADR-033 -- прецедент "не дублировать уже единственный конвейер",
  применённый здесь к отклонению in-process auto-diagnose пути.
- `crates/automation-engine/src/lib.rs`,
  `services/saaios-runtime/src/main.rs` (`spawn_auto_diagnose_worker`)
  -- прочитанный код, на основании которого сделан этот выбор.
