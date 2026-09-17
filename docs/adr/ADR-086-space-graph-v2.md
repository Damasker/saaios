# ADR-086: HIA-01 Space Graph v2 (жизненный цикл и связи пространств)

## Статус

Принято, 2026-09-15.

## Контекст

`docs/os/sprints/HIA-ROADMAP.md` (переработка видения
`human-interface-architecture-v2.md` в реалистичный план) называет
HIA-01 первым буildable шагом Фазы 1: пространства должны нести
жизненный цикл (temporary/emerging/stable/archived) и связи друг с
другом, а не быть просто плоским списком из 4 статичных карточек.

`saai-entity-store` уже даёт всё нужное без единого изменения схемы:
`Space` и `Entity` -- РАЗНЫЕ типы (это исправляет более раннее
ошибочное утверждение в первой версии `HIA-ROADMAP.md`, что между
ними есть готовая many-to-many связь -- на деле `Entity.space_id`
это одна строка, одно пространство на сущность), но зарезервированное
системное пространство `"saaios"` (`BUILTIN_SPACE_IDS`,
`SpaceKind::System`) существует именно для кросс-катаной служебной
информации вроде этой. Жизненный цикл и связи смоделированы как
обычные `Entity` в этом системном пространстве -- не как новые поля
`Space`, не как изменение стораджа.

## Решение

Два новых типа сущностей в пространстве `"saaios"`:
`saaios.space-lifecycle` (одна запись на пространство, `properties:
{space_id, lifecycle}`) и `saaios.space-relation` (`properties:
{from_space_id, to_space_id, kind}`, создание пока не имеет UI --
только чтение/отображение, см. "Последствия").

`SpaceLifecycle` enum (`Temporary`/`Emerging`/`Stable`/`Archived`),
`.next()` циклический (Temporary -> Emerging -> Stable -> Archived ->
Temporary), `.label()` возвращает `None` для `Stable` (тихий
дефолт -- нетронутое пространство выглядит как раньше, без всякой
подписи).

Жест: тап по уже выбранной карточке пространства (в отличие от тапа
по невыбранной, который просто переключает выбор) вызывает
`cycle_space_lifecycle()` -- `update_entity` на существующую запись
`saaios.space-lifecycle`, либо `create_entity`, если её ещё нет.
Карточка "Пространства" (`content_card()`, статичная `root.sui`-
разметка, S04) показывает не более ОДНОГО из: метку жизненного цикла
ИЛИ первую цель связи -- никогда оба сразу (см. "Ошибки" ниже: первая
версия попыталась показать оба и переполнила строку).

`saai-shell` уже кэширует сущности выбранного пространства
(`selected_entities`), но НЕ системного -- добавлено отдельное поле
`system_space_entities: Vec<Entity>`, наполняемое в
`apply_entityd_message()` при `space_id == SYSTEM_SPACE_ID`
независимо от того, какое пространство выбрано у пользователя сейчас
(иначе метка на карточке "Дом" пропадала бы, стоит открыть "Работу").

## Ошибки, найденные и исправленные вживую

1. **Переполнение кнопки.** Первая версия текста кнопки --
   `"Выбрано · тап переключит жизненный цикл"` (40 символов) --
   рисуется по центру в кнопке шириной `250.min(rect.width/3)` px,
   шрифтом 25 (`render.rs`, `content_card()`) -- не влезла на
   реальном экране, подтверждено пользователем вживую. Каждая другая
   подпись кнопки в кодовой базе -- одно короткое слово; исправлено
   возвратом к простому `"Выбрано"`/`"Открыть"`, объяснение жеста
   убрано из текста кнопки вовсе (известный грубый край, см. ниже).
   Заодно исправлена и строка статуса -- первая версия пыталась
   показать метку жизненного цикла И список связей одновременно
   (`" · метка · → цель1, цель2"`), рискуя тем же переполнением на
   рисуемой без переноса строке; сведено к "не больше одного".

2. **`invalid entity type`.** `saai-entity-store::is_slug()` разрешает
   в каждом dot-сегменте `entity_type` только строчные буквы, цифры
   и одиночный неповторяющийся дефис -- **подчёркивания запрещены**.
   Константы `SPACE_RELATION_ENTITY_TYPE`/`SPACE_LIFECYCLE_ENTITY_TYPE`
   изначально использовали подчёркивания (`saaios.space_lifecycle`),
   и каждый `create_entity`/`update_entity` тихо отклонялся --
   тихо, потому что `EntityServerMessage::Response { ok: false, .. }`
   до этого ADR просто выставлял `changed = true` и ничего не
   логировал. Существующие типы (`saaios.notification` и т.п.)
   случайно ни разу не использовали подчёркивание, поэтому это
   ограничение не всплывало раньше.

   Диагностировано: добавлены временные `println!` в
   `cycle_space_lifecycle()` и в error-ветку `Response`, пересобрано,
   развёрнуто, пользователь сделал два тапа, лог показал
   `WireError { code: "invalid_record", message: "invalid record:
   invalid entity type" }`. Исправлено переименованием констант на
   дефисы (`saaios.space-relation`/`saaios.space-lifecycle`).
   Временная диагностика в `cycle_space_lifecycle()` убрана после
   находки; строка логирования ошибок в `Response`-ветке оставлена
   насовсем -- реальный слепой участок, который уже стоил времени на
   отладку один раз.

## Test

Host: `cargo test -p saai-shell` -- 49/49 (6 новых:
`space_lifecycle_defaults_to_stable_with_no_matching_record`,
`space_lifecycle_reads_the_matching_record`,
`space_lifecycle_entity_finds_the_real_record_to_update_in_place`,
`space_lifecycle_cycles_through_all_four_states_and_wraps`,
`space_relation_targets_only_matches_the_from_direction`,
`space_display_name_prefers_a_real_space_then_falls_back_to_known_ids`).
`cargo clippy -p saai-shell --all-targets` чист.

Device: подтверждено вживую без пересборки образа (`saai-shell`
живёт на `/data`, ADR-082) -- после исправления обеих находок тап по
уже выбранной карточке пространства "saaios" показал метку
жизненного цикла на карточке. Запись реально сохранилась на диске:

```
/data/saaios/var/entities/spaces/saaios/entities/10ddd9e6-...json
{"entity_type":"saaios.space-lifecycle","title":"Жизненный цикл: saaios",
 "properties":{"lifecycle":"emerging","space_id":"saaios"},"revision":7,...}
```

## Последствия

- Создание связей (`saaios.space-relation`) не имеет UI -- построены
  и юнит-протестированы только чтение (`space_relation_targets()`) и
  отображение первой цели на карточке. Честно задокументированный
  пробел, тот же паттерн, что уже применялся в этом проекте
  ("проводка доказана, часть UI отложена") -- следующий шаг HIA-01,
  если/когда к нему вернёмся.
- Жест "тап по уже выбранной карточке = цикл жизненного цикла" ничем
  не подсказан на самой карточке (кнопка теперь снова просто
  "Выбрано") -- известный грубый край, тот же компромисс, что уже
  принимался для других жестов без подсказки в этом проекте.
- `docs/os/sprints/HIA-ROADMAP.md`: HIA-01 отмечен Done со ссылкой на
  этот ADR.

## Evolution

SOM Relationship (ADR-118) становится canonical future representation
связей, в том числе Space↔Space. Существующие `saaios.space-relation`
Entity остаются readable на переходный период и **не** удаляются
автоматически. Двух независимых графов в продуктовом смысле быть не
должно: на первом этапе читаются оба формата; cleanup — отдельный ADR
после доказанной миграции. Проверенная вертикаль HIA-01 не ломается.

## Ссылки

- `services/saai-shell/src/main.rs` -- `SpaceLifecycle`,
  `space_lifecycle()`, `space_lifecycle_entity()`,
  `space_relation_targets()`, `space_display_name()`,
  `cycle_space_lifecycle()`, `SYSTEM_SPACE_ID`,
  `SPACE_RELATION_ENTITY_TYPE`, `SPACE_LIFECYCLE_ENTITY_TYPE`.
- `crates/saai-entity-store/src/lib.rs` -- `is_dotted_token()`/
  `is_slug()`, источник ограничения на подчёркивания.
- ADR-082 -- почему это разворачивалось и проверялось вживую без
  единой перепрошивки.
- `docs/os/sprints/HIA-ROADMAP.md` -- план, частью которого является
  этот ADR.
