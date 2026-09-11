# ADR-038: S10's last piece -- memory scoped to space stays in Platform Track, threaded by `space_id`, not migrated to `saai-entity-store`

## Статус

Принято, 2026-09-11.

## Контекст

Последний нерешённый кусок S10 (Definition of Ready): `memory-store`'s
`MemoryFact` не имеет `space_id` -- тот же пробел, что S06 уже
отметило. Сам Definition of Ready прямо не предрешал ответ: "по
прецеденту, вероятно новый `entity_type`, а не расширение
`memory-store`, но это отдельное решение". Прочитан реальный код перед
выбором.

`crates/memory-store` -- целиком Platform Track: `MemoryFact` --
плоский append-only JSONL (`/data/saaios/var/runtime/memory.jsonl`,
уже переживает cold reboot -- `/data` персистентен с S05/S06, вопрос
не в персистентности как таковой). Инструменты `memory.remember`/
`memory.recall`/`memory.forget` регистрируются в `ToolRegistry` и
вызываются моделью прямо посреди `diagnose()`, а не через
`saai-taskd`'s отдельный `Action`/подтверждение-конвейер -- в отличие
от Intent/Task/Action/Schedule, memory физически не проходит через
`saai-taskd` вообще, когда до неё доходит модель.

Ключевая находка: **"пространство" как понятие сегодня не существует
нигде в Platform Track** -- ноль упоминаний `space_id` в `ai-runtime`,
`saaios-runtime`, `tool-registry`, `model-provider`. `ToolContext`
(`crates/tool-registry/src/lib.rs`) несёт только `correlation_id`/
`call_id`. Единственное место, которое СЕГОДНЯ знает пространство --
`saai-taskd` (свой `--space`, ADR-030), и оно уже передаёт запрос
`saaios-runtime` через `runtime_bridge::diagnose()` (ADR-033/034).

## Отклонённый вариант: `saaios.memory_fact` как native entity_type

Прямой прецедент Change 1/2/3 (Intent/Task/Action/Schedule) --
напрашивается тот же ответ: новая сущность в `saai-entity-store`.
Прочитаны зависимости: `saai-entity-protocol` (владеет wire-типами
`ClientRequest`/`ResponseResult`, которые использует `saai-entityd`)
зависит только от `saai-entity-store`, который сам по себе лёгкий
(`chrono`/`serde`/`uuid`, без `protocol`/`event-bus`) -- зависимость
сама по себе не тяжёлая.

Но архитектурно это не тот же случай: Intent/Task/Action/Schedule --
всегда `saai-taskd`-опосредованный путь (ADR-030/033/036 каждый раз
проводили один и тот же конвейер через один и тот же процесс). Memory
вызывается моделью **внутри** `saaios-runtime`, посреди `diagnose()`
-- чтобы записать `saaios.memory_fact` напрямую в `saai-entity-store`,
`saaios-runtime` должен был бы сам стать клиентом `saai-entityd`'s
Unix-сокета. Это первый случай, когда Platform Track зависел бы от
native OS Track вообще в какую-либо сторону -- инверсия факта,
установленного ADR-030 и опиравшегося на него каждый следующий ADR
этого спринта ("оба рантайма сегодня не делят код"). Ничего в
требовании ("memory не должна течь между Домом/Работой/Личным") не
требует ни `WorkflowStatus`, ни видимости в `Сейчас`, ни
confirmation-конвейера -- то, ради чего entity-store вообще
существует для Intent/Task/Action. Заводить новую, ранее не
существовавшую межпроцессную зависимость ради простого
партиционирования данных -- заявка на архитектурный сдвиг больше, чем
эта задача заслуживает.

## Решение

Memory остаётся полностью в Platform Track, как плоский JSONL,
партиционированный полем `space_id`, переданным явно от единственного
источника, который его знает -- `saai-taskd`.

- **`MemoryFact`**: `+space_id: Option<String>`, `+origin_correlation_id:
  Option<Uuid>` ("происхождение" -- какой именно запрос создал факт,
  переиспользует уже существующий `correlation_id`, который весь
  остальной audit trail и так использует, не новая система
  провенанса).
- **`ToolContext`**: `+space_id: Option<String>` -- единственное поле,
  через которое любой инструмент (не только memory) сегодня или в
  будущем может узнать пространство вызова.
- **Wire**: `ClientRequest::Diagnose` получает `space_id:
  Option<String>` (`#[serde(default)]` -- существующие клиенты без
  поля продолжают работать, `console-tui` шлёт `None`). `saai-taskd`'s
  `runtime_bridge::diagnose()` передаёт свой уже известный
  `self.space_id`.
- **Политика видимости**: запрос с `Some(space)` видит факты этого
  `space_id` и факты без пространства (`None` -- глобальные/системные,
  тот же прецедент, что пространство `SaaiOS` в S06 -- общее для
  всех). Запрос без пространства (`console-tui`, прямая консоль без
  выбранного контекста) видит ВСЁ -- точно текущее поведение, нулевая
  регрессия для уже работающего пути.

## Последствия

- `space_id` доходит до `ToolContext` только на пути немедленного
  выполнения (`PolicyVerdict::Allow` внутри `diagnose()`) -- ни один
  сегодняшний memory-инструмент не требует подтверждения
  (`requires_confirmation: false`), поэтому путь `confirm()`
  (`PendingConfirmation`, живёт в `policy-engine`, отдельный крейт) в
  этом срезе `space_id` не несёт. Если когда-нибудь появится
  space-зависимый инструмент, требующий подтверждения, это отдельное,
  не предрешаемое здесь расширение `PendingConfirmation`.
- Memory НЕ видна в `Сейчас`/не проходит через `saai-entityd` --
  инспектируется только через уже существующие `memory.recall`/
  `{"op":"memory_tail"}` пути. Если это когда-нибудь понадобится,
  потребует отдельного решения (см. отклонённый вариант выше) -- не
  предрешается здесь.
- `ToolContext` -- новое обязательное поле, затрагивает все места
  конструирования (`ai-runtime`, `tool-registry`, `system-tools`,
  `telemetry`, `memory-store`'s собственные тесты) -- механическая, не
  архитектурная правка.
- Прямые консольные memory-операции (`ClientRequest::MemoryRemember`/
  `MemoryRecall`/`MemoryForget`/`MemoryTail`) пространство пока не
  несут -- `console-tui` сегодня не имеет понятия пространства вообще;
  остаётся вне этого среза.

## Ссылки

- ADR-030 -- источник "оба рантайма не делят код", на который здесь
  опирается отклонение native-entity-варианта.
- ADR-033 -- прецедент явного отклонения кандидата, дублирующего
  границу между Track'ами, применённый здесь по аналогии.
