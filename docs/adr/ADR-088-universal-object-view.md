# ADR-088: HIA-07 Universal Object View

## Статус

Принято, 2026-09-15.

## Контекст

До этого ADR у каждого показанного типа сущности был свой отдельный
экран: `saaios.task` открывал `task_confirm_view`/`draw_task_confirm`
(S09 Change 3), `saaios.notification` вообще не открывал ничего --
тап по строке в "Входящие" сразу её скрывал (S21). `HIA-ROADMAP.md`'s
HIA-07 просит один переиспользуемый "экран объекта" (шапка +
состояние + related + действия) для любой сущности вместо этого
разнобоя, подтверждённый минимум на двух разных `entity_type`, плюс
честный негативный сценарий для незнакомого типа.

## Решение

`Frame::ObjectView { title, status, related: Option<String>, header,
actions: Vec<(Rect, &'static str)> }` заменяет старый `Frame::
TaskConfirm` целиком -- не добавлен рядом, а именно заменил
единственный экран, который раньше существовал для одного типа.
Геометрия -- та же `Node`/`layout()`-схема, что и везде в этом
проекте (`object_view(width, height, action_count)`, header-leaf
плюс опциональный button-row, 0-2 кнопки вместо всегда-двух), рисует
её новая `draw_object_view` -- прямое обобщение старой `draw_task_
confirm` (теперь удалённой как мёртвый код -- единственный вызов
заменён) на переменное число кнопок и необязательную третью строку
"related".

`ObjectViewContent`/`object_view_content(entity, selected_entities)`
-- единственное место, которое знает про конкретные `entity_type`:

- `saaios.task`: статус "Ждёт подтверждения", related -- реальный
  поиск сущности `saaios.intent` по `intent_id` в уже загруженном
  списке (не отдельный round-trip к `saai-entityd`) -- показывается,
  только если такая сущность реально нашлась. Кнопки "Подтвердить"/
  "Отклонить" -- тот же текст, что был на старом экране.
- `saaios.notification`: статус -- `body`-свойство, кнопка "Скрыть".
- Любой другой `entity_type` (негативный сценарий
  `HIA-ROADMAP.md`): заголовок из `entity.title`, статус -- список
  `ключ: значение` по всем properties (или "Нет дополнительных
  данных", если их нет), кнопок нет. Никогда не пусто, никогда не
  падает.

Оба реальных типа теперь открывают Object View по тапу на строке
"Входящие" (`self.viewing_entity_id = Some(id)`) -- у уведомления
раньше тап СРАЗУ скрывал его без экрана; теперь тап открывает
экран, а "Скрыть" -- отдельное действие внутри него (то же "меньше
неожиданных действий от одного тапа", что уже применялось в других
местах проекта). `handle_object_view_action(index)` -- единственное
место, которое решает, что означает нажатая кнопка, тем же
паттерном, что `task_confirm_action_at`'s `bool` уже использовал:
геометрия отдаёт только позицию, смысл ей придаёт вызывающий код.

`confirming_task_id`/`confirming_task()`/`confirm_pending_task()`
переименованы/обобщены в `viewing_entity_id`/`viewing_entity()`/
`handle_object_view_action()` -- "актуальность" сущности при показе
теперь для каждого `entity_type` своя (задача всё ещё
`waiting_confirmation`, уведомление всё ещё не `dismissed`),
незнакомый тип не имеет своего понятия устаревания и всегда
"актуален", пока id существует.

`task_confirm_view`/`task_confirm_action_at` НЕ тронуты --
`Frame::RemotePairing` (SSH-пейринг, ADR-074) по-прежнему переиспользует
именно эту геометрию напрямую, у неё нет отношения к сущностям вообще.

## Test

Host: `cargo test -p saai-shell` -- 67/67 (8 новых:
`object_view_action_at_finds_two_buttons_by_position`,
`object_view_action_at_finds_a_single_button_spanning_the_full_row`,
`object_view_action_at_finds_nothing_with_zero_actions`,
`object_view_content_for_a_task_has_no_related_line_without_a_
matching_intent`, `object_view_content_for_a_task_shows_the_
originating_intent_when_present`, `object_view_content_for_a_
notification_shows_its_body_and_a_dismiss_action`,
`object_view_content_for_an_unknown_entity_type_is_never_empty_and_
has_no_actions` -- прямая проверка негативного сценария,
`object_view_content_for_an_unknown_entity_type_with_no_properties_
still_has_a_status_line`). Существующие `task_confirm_screen_*` тесты
(3 шт.) не тронуты и всё ещё проходят -- геометрия под RemotePairing
не менялась. `cargo clippy -p saai-shell --all-targets` чист.

Device: подтверждено вживую без пересборки образа (`saai-shell`
живёт на `/data`, ADR-082) -- две тестовые сущности (`saaios.task` в
`waiting_confirmation`, `saaios.notification`) созданы напрямую по
протоколу `saai-entityd` тем же throwaway-однострочным Rust-
бинарём, что и в ADR-087 (собран и запущен только для этого теста,
удалён из дерева и с устройства после, никогда не коммитился).
Задача: тап на строке -- открылся экран с заголовком/статусом/двумя
кнопками, тап "Отклонить" -- экран закрылся, вернул на "Входящие".
Уведомление: тап на строке -- открылся экран с одной кнопкой
"Скрыть" (не сразу исчезло, как было раньше), тап "Скрыть" -- экран
закрылся, строка пропала из списка.

## Последствия

- `docs/os/ideas.md`: уведомление больше не скрывается прямым тапом
  по строке в "Входящие" -- нужен один лишний тап (открыть экран,
  потом "Скрыть"). Осознанный компромисс ради единообразия с
  задачами, не проверялось, насколько это раздражает в реальном
  использовании.
- Related показывает только `saaios.task -> saaios.intent` -- других
  реальных related-связей в системе пока нет (в отличие от ADR-086's
  `saaios.space-relation`, у которого пока нет ни одного реального
  экземпляра вообще). Если появятся новые типы с осмысленной
  связью -- `object_view_content`'s `_` ветка остаётся местом,
  куда их добавлять.
- `draw_task_confirm` удалён как мёртвый код (единственный вызов
  заменён на `draw_object_view`) -- `task_confirm_view`/`task_
  confirm_action_at` остались, RemotePairing их по-прежнему
  использует напрямую.

## Evolution

Universal Object View получает related objects из SOM (ADR-118).
До миграции fallback на `intent_id` и уже загруженный список сущностей
остаётся. Незнакомый `entity_type` по-прежнему честный: title + status,
related только если SOM (или legacy) реально нашёл связанный объект,
иначе пустая related-строка, не выдуманная. Действия Object View в
следующем срезе читаются из OAM (ADR-119); hardcoded
Подтвердить/Отклонить/Скрыть пока остаются parallel path.

## Ссылки

- `services/saai-shell/src/main.rs` -- `object_view()`,
  `object_view_action_at()`, `ObjectViewContent`,
  `object_view_content()`, `viewing_entity_id`, `viewing_entity()`,
  `handle_object_view_action()`, `Frame::ObjectView`.
- `services/saai-shell/src/render.rs` -- `draw_object_view()`.
- S09 Change 3 / ADR-031 -- исходный task-only confirm-экран, теперь
  заменённый этим ADR.
- S21 -- исходные уведомления и их прежний тап-сразу-скрывает жест.
- ADR-082 -- почему это разворачивалось и проверялось вживую без
  единой перепрошивки.
- `docs/os/sprints/HIA-ROADMAP.md` -- план, частью которого является
  этот ADR.
