# ADR-030: Intent/Task/Action -- новые entity_type в native saai-entity-store, не Platform Track's policy-engine/tool-registry

## Статус

Принято, 2026-09-11.

## Контекст

S09's Definition of Ready назвала второй, отдельный от текста (ADR-029)
вопрос Change 1: где живёт модель данных `Intent`/`Task`/`Action`/
`Result`, и участвует ли в этом Platform Track's уже существующая
инфраструктура (`crates/policy-engine`, `crates/tool-registry`,
`crates/ai-runtime`) -- через переиспользование как библиотеки -- или
native OS Track строит независимый эквивалент, как S06 уже сделал для
`memory-store`/`audit-log` (построил `saai-entityd` заново вместо их
расширения).

S09's Acceptance criteria требует: Task, ожидающий подтверждения
опасного Action, переживает холодную перезагрузку и НЕ исполняется
автоматически при старте -- подтверждение обязано запрашиваться заново,
не браться из кэша. Это единственный критерий, который решает вопрос:
он проверяем только чтением реального кода обоих кандидатов, не
теоретически.

## Находка

Прочитан реальный код обоих кандидатов, не только их публичное API:

**`crates/policy-engine/src/lib.rs`**: `PolicyEngine::session_allows` --
`Mutex<HashSet<String>>` в памяти процесса, обнуляется при рестарте.
`PendingConfirmation` (`policy-engine/src/lib.rs:158`) -- обычная
`struct` без какой-либо персистентности.

**`crates/ai-runtime/src/lib.rs`**: `PendingConfirmation` создаётся как
локальная переменная (`let mut pending: Option<PendingConfirmation> =
None;`, строка 336) внутри `handle_user_text_inner` -- живёт ровно в
рамках одного асинхронного tokio-таска, обрабатывающего один
пользовательский запрос. Ничего не пишется на диск; следующий запрос
(даже подтверждение) -- это новый вызов с нуля.

**`crates/tool-registry/src/lib.rs`**: `ToolRegistry` -- `HashMap<String,
Arc<dyn ToolExecutor>>`, строится один раз при старте процесса
(`ToolExecutor: Send + Sync`, `async_trait`), полностью в памяти,
рассчитан на tokio-рантайм.

Итого: у Platform Track сегодня нет ни одного байта персистентности для
pending-confirmation/session-grant -- ни между запросами внутри одного
процесса дольше одного tool-iteration цикла, ни тем более через
перезапуск процесса. Требование S09 ("переживает холодную
перезагрузку") физически невыполнимо на этом фундаменте без того,
чтобы переписать `policy-engine`/`ai-runtime` с нуля под персистентную
модель -- то есть переиспользование не экономит работу, оно её
создаёт.

Отдельно подтверждено: ни один native OS Track сервис (`services/saai-
*`) не зависит от `protocol`-крейта (`grep` по всем `Cargo.toml` --
только Platform Track крейты и `saaios-runtime` его используют). Два
рантайма сегодня физически не делят ни строчки кода -- ADR-004's
"Platform later consumes OS services through the same tool/policy/event
APIs" остаётся названным направлением, не фактом.

**`crates/saai-entity-store/src/lib.rs`+`store.rs`** (S06, физически
доказан переживающим cold-reboot -- `docs/os/sprints/S06-space-entity-
store.md:166-185`, реальный `fastboot reboot`, не hot-swap): `Entity`
уже универсальна -- `entity_type: String` (dotted-token, до 96 байт),
`properties: Map<String, Value>` (JSON, до 64 КБ), `revision: u64`
(optimistic concurrency), полный event-sourcing
(`EventPayload::EntityCreated/Updated/Deleted`, монотонный `sequence`).
`EntityStore::list_entities(space_id)` уже даёт способ на старте
перечислить все сущности пространства и отфильтровать по
`entity_type`/`properties.status` -- ровно то, что нужно для
resume-after-reboot без auto-execute.

Схема не требует ни новой персистентности, ни новой инфраструктуры:
`Intent`/`Task`/`Action`/`Result` укладываются как новые `entity_type`
значения (`saaios.intent`, `saaios.task`, `saaios.action`) поверх уже
работающего и уже физически проверенного на реальном железе store.

## Решение

`Intent`/`Task`/`Action`/`Result` -- новые `entity_type` в native
`saai-entity-store` (S06), НЕ портирование/переиспользование Platform
Track's `policy-engine`/`tool-registry`/`ai-runtime`. Повторяется
ровно тот же прецедент, что S06 уже установило для `memory-store`/
`audit-log`.

Конкретно для Change 2/3 (не архитектурное решение этого ADR, но
рамка, которую он задаёт):

- `Task`/`Action` хранят собственное поле состояния прямо в
  `properties` (`pending`/`running`/`waiting_confirmation`/`done`/
  `failed`/`cancelled`) -- подтверждение опасного `Action` это
  персистентная запись в entity store, а не структура в памяти
  процесса. На старте новый native компонент (Change 2/3, не этот ADR)
  читает `list_entities` и для всего в `waiting_confirmation`
  ПОВТОРНО запрашивает подтверждение -- никогда не исполняет
  автоматически, что напрямую закрывает S09's Threat impact
  требование.
- Идемпотентность -- через `Action`'s собственный `revision`/`status`:
  исполнитель проверяет текущее состояние `Action`-сущности перед
  выполнением; уже `done` не выполняется повторно, конфликт по
  `revision` (уже встроен в `EntityStore::update_entity`) не даёт двум
  параллельным попыткам исполнить одно действие дважды.
- Словарь риска (`RiskLevel`-подобный: low/medium/high/critical) не
  импортируется из `tool-registry` как код (это создало бы
  единственную межрантаймовую зависимость, которой нет больше нигде) --
  переопределяется тем же набором значений native OS Track'ом
  самостоятельно, как отдельный, не связанный типами enum. Совпадение
  словаря -- сознательное, для будущей конвергенции ADR-004, но не
  общий код сегодня.

ADR-004's конвергенция (Platform Track потребляет OS-сервисы через
общий tool/policy/event API) остаётся в силе как направление, но
осознанно НЕ реализуется в S09 -- ничего в этом решении ей не мешает
(native `Intent`/`Task`/`Action` как entity store данные может быть
позже прочитан/записан `saaios-runtime`, если конвергенция когда-либо
случится, тем же способом, каким сегодня к нему обращается
`saai-shell`), но это отдельная, будущая задача, не Change этого
спринта.

## Последствия

- S09's Change 1 закрыт полностью: текст (ADR-029) и модель данных
  (этот ADR) оба физически/кодово обоснованы.
- Change 2 (минимальный вертикальный срез -- одна Intent-строка -> один
  Task с одним неопасным Action -> Result в `Сейчас`) может начинаться
  без дальнейших архитектурных развилок: новый native daemon или
  расширение `saai-appd` читает/пишет `saai-entity-store` напрямую
  через уже существующий `saai-entityd`-протокол (`saai-entity-
  protocol`), без нового IPC-механизма.
- Native OS Track и Platform Track остаются двумя независимыми
  рантаймами без общего кода -- как и было явно зафиксировано ADR-004,
  теперь дополнительно подтверждено на уровне `Cargo.toml`-зависимостей
  и не нарушено этим решением.
- Риск: словарь risk-level дублируется (native enum отдельно от
  `tool-registry::RiskLevel`) -- сознательный компромисс в пользу
  отсутствия кросс-рантайм зависимости; если ADR-004's конвергенция
  когда-либо случится, унификация словаря -- часть той будущей задачи,
  не долг, накопленный сейчас.

## Ссылки

- ADR-004 -- Platform Track vs OS Track разделение и названное (не
  реализованное) направление конвергенции.
- ADR-019 -- `Space`/`Entity`/`Event` схема, которую это решение
  расширяет новыми `entity_type`, не новой инфраструктурой.
- ADR-029 -- первый вопрос Change 1 (текстовый ввод), закрытый отдельно.
- `docs/os/sprints/S06-space-entity-store.md` -- физическое
  доказательство cold-reboot персистентности entity store, на которое
  опирается это решение.
- `docs/os/sprints/S09-intent-task-action.md` -- оба вопроса Change 1,
  из которых этот ADR закрывает второй.
