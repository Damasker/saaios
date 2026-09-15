# ADR-087: HIA-02 ContextFrame Engine (Wi-Fi присутствие)

## Статус

Принято, 2026-09-15.

## Контекст

`docs/os/sprints/HIA-ROADMAP.md`'s HIA-02: одно активное пространство
(`selected_space_id`) сейчас переключается только вручную (тап).
Видение хочет несколько одновременных источников контекста, каждый
со своей уверенностью, и автоматическое переключение по физическому
присутствию -- начиная с Wi-Fi (S19, реально сканируется уже давно)
как первого, простейшего источника: `wpa_supplicant` уже непрерывно
поддерживает состояние ассоциации, не нужно ничего нового опрашивать
активным сканированием (в отличие от Bluetooth, у которого нет
фонового демона -- `bt-scan` сам блокируется на ~8с, что делает его
не пригодным для тихого фонового опроса каждые несколько секунд;
честно оставлен будущим источником, не построен здесь).

## Решение

Новый внутренний тип `ContextFrameEntry { space_id, confidence: u8,
source: ContextSource }` (`Manual` | `Wifi`), хранится в
`Shell::context_frame: Vec<ContextFrameEntry>` -- не сущность, не
меняет `saai-entity-store`. Максимум одна запись на источник
(`upsert_context_entry` заменяет, не копит историю).
`effective_context_space()` -- чистая функция, берёт запись с
наибольшей `confidence`.

Источники и их вес: `MANUAL_CONFIDENCE = 50` (тап), `WIFI_CONFIDENCE
= 80` (подключение к сети, сопоставленной с пространством через
новую `saaios.space-signal`-сущность в системном пространстве,
`properties: {space_id, signal_type: "wifi_ssid", value: <ssid>}` --
как и `saaios.space-relation` в ADR-086, создание не имеет UI,
только чтение; см. "Последствия"). Wi-Fi весит больше -- реальное
физическое присутствие должно обычно побеждать то, что было выбрано
раньше. `refresh_context_signals_if_due()` (throttled как
`refresh_apps_if_due`/`refresh_statusbar_if_due`, раз в
`CONTEXT_SIGNAL_REFRESH_INTERVAL` = 5с, чтобы не дёргать `wpa_cli
status` на каждый кадр) пересчитывает Wi-Fi-запись с нуля каждый раз
и, если топ-запись расходится с `selected_space_id`, вызывает тот же
`select_space()`, что и ручной тап -- отдельного "auto-selected"
состояния нет, `selected_space_id` остаётся единственным полем, как
и предполагал Rollback в `HIA-ROADMAP.md`.

**Ключевое решение по негативному сценарию** ("потеря сигнала не
оставляет ContextFrame пустым, откатывается на manual/последнее
известное"): `upsert_manual_context()` вызывается ТОЛЬКО из
`invoke_content_action`'s тап-ветки `select_space:` -- НЕ из
`apply_entityd_message`'s общих `Selection`/`SelectionChanged`
обработчиков, хотя они тоже меняют `selected_space_id`. Если бы
Wi-Fi-триггернутое переключение тоже перезаписывало Manual-запись,
потеря Wi-Fi-сигнала откатывалась бы туда же, куда сам Wi-Fi только
что переключил -- т.е. никуда не откатывалась бы видимо. Так Manual
остаётся именно "последний РУЧНОЙ выбор", а не "последнее любое
состояние" -- разница, которую тестовый сценарий ниже проверяет
напрямую.

## Test

Host: `cargo test -p saai-shell` -- 54/54 (5 новых:
`effective_context_space_falls_back_with_an_empty_frame`,
`effective_context_space_prefers_the_highest_confidence_entry`,
`upsert_context_entry_replaces_the_same_source_instead_of_accumulating`,
`losing_the_wifi_signal_falls_back_to_the_manual_entry` -- ровно тот
негативный сценарий, что называет `HIA-ROADMAP.md`,
`space_for_wifi_ssid_matches_only_the_right_type_and_value`).
`cargo clippy -p saai-shell --all-targets` чист.

Device: подтверждено вживую без пересборки образа (`saai-shell`
живёт на `/data`, ADR-082). Поскольку никакого UI для создания
`saaios.space-signal` не существует (как и для
`saaios.space-relation` в ADR-086), сущность для теста создана и
удалена напрямую по протоколу `saai-entityd` -- двумя временными
однострочными Rust-бинарями (`seed-signal`/`delete-signal`,
собраны и запущены только для этого теста, удалены из дерева и с
устройства после, никогда не коммитились):

1. Вручную выбрано "Личное" (Manual-запись = personal).
2. `seed-signal work Wallbox "Wi-Fi: Wallbox"` создал
   `saaios.space-signal`, связав реально подключённую сеть Wallbox
   с пространством "Работа". Через ~5с (`CONTEXT_SIGNAL_REFRESH_
   INTERVAL`) `/data/saaios/var/entities/selection.json` сам сменился
   на `work` -- пользователь подтвердил вживую: "Сам переключился на
   «Работа»".
3. `delete-signal <id> <revision>` удалил связь. Через ~5с
   `selection.json` сам вернулся на `personal` -- пользователь
   подтвердил вживую: "Сам вернулся на «Личное»".

Оба перехода -- без единого тапа пользователя, оба видны и на экране,
и в персистентном файле выбора.

## Последствия

- Создание `saaios.space-signal` не имеет UI -- тот же честный
  пробел, что ADR-086 уже оставила для `saaios.space-relation`.
  Из коробки, без единой вручную созданной записи, HIA-02 не меняет
  никакое наблюдаемое поведение вообще -- вся логика активна, но
  молчит, пока такой связи не существует.
- Bluetooth как второй физический источник -- не построен (нет
  фонового демона для тихого опроса, `bt-scan` сам блокируется на
  ~8с) -- честно отложено, `SPACE_SIGNAL_TYPE_WIFI_SSID`'s
  `signal_type`-поле уже оставляет для этого место в схеме без
  изменений на будущее.
- Manual-запись это "последний РУЧНОЙ выбор", не "последнее любое
  состояние" -- см. "Решение" выше. Значит: один раз оказавшись в
  зоне действия известной сети, устройство останется в
  сопоставленном пространстве даже после выхода из этой сети, если
  пользователь ни разу не тапнул руками с тех пор -- ожидаемое
  поведение этой версии, не баг, но стоит иметь в виду при
  дальнейшей работе над HIA-02/HIA-03.

## Ссылки

- `services/saai-shell/src/main.rs` -- `ContextFrameEntry`,
  `ContextSource`, `effective_context_space()`,
  `upsert_context_entry()`, `remove_context_source()`,
  `space_for_wifi_ssid()`, `upsert_manual_context()`,
  `refresh_context_signals_if_due()`, `SPACE_SIGNAL_ENTITY_TYPE`,
  `MANUAL_CONFIDENCE`, `WIFI_CONFIDENCE`.
- ADR-086 -- `saaios.space-relation`, тот же прецедент "чтение
  построено, создание отложено".
- ADR-082 -- почему это разворачивалось и проверялось вживую без
  единой перепрошивки.
- `docs/os/sprints/HIA-ROADMAP.md` -- план, частью которого является
  этот ADR.
