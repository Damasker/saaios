# ADR-118: Saai Object Model — first-class Relationship

## Статус

Принято, 2026-09-17. Host-verified: `cargo test -p saai-entity-store`,
`cargo test -p saai-entity-protocol`, `cargo test -p saai-entityd`,
`cargo test -p saai-shell`, `cargo test -p saai-taskd`, clippy `-D warnings`
on the affected packages. Pixel 7 (`192.168.168.17`) device-verified:
`saai-entityd`/`saai-shell` installed under `/data/saaios/system`, one
object linked into `home` and `work` via `saaios.in-space`,
`list_relationships` returned both edges after a real `reboot -f`
(`/proc/uptime` 122s). Space UI lists `list_space_members` (physical
partition ∪ `saaios.in-space`); `list_entities` remains the storage
boundary.

## Нумерация

На `origin/feat/pixel7-native-saaios` @ `0797c5a` последний ADR — 116.
Соседний worktree `feat/vui-04-navigation` уже держит незакоммиченный
ADR-117 (`SystemStatus` / pressed / reduced motion). Этот документ
поэтому **118**, чтобы два параллельных среза не делили один номер.

## Контекст

S06 / ADR-019 правильно зафиксировали `Entity.space_id` как единственную
физическую границу первой вертикали. HIA уже требует иное: один объект
может быть релевантен нескольким Space, Intent/Task/Action связаны явно,
а AI-выведенная связь не должна выглядеть как подтверждённый факт.

`saaios.space-relation` (ADR-086) — legacy-граф пространств через Entity.
Workflow links живут в JSON properties (`intent_id`, `task_id`). Это не
ошибка тех ADR; это потолок той модели.

## Решение

Формализовать **Saai Object Model (SOM)** поверх существующего store, без
нового демона и без SQL/ORM:

```text
Object        == Entity (имя не меняем в Rust)
Relationship  == новая first-class versioned запись
Event         == неизменяемый журнал (per-space для Entity, global для Relationship)
ContextFrame  == по-прежнему выбирает релевантность (ADR-087), не владеет графом
```

### Physical vs semantic Space

`Entity.space_id` остаётся **physical storage partition**. Semantic
membership — отношение `saaios.in-space` (`Entity → Space`). Один Object
может иметь несколько таких рёбер. Удаление одного ребра не удаляет Object.
`create_entity` через entityd дополнительно пишет системное
`saaios.in-space` в physical Space (две записи подряд, не транзакция).

### Persistence

Отдельный journal, не втиснутый в случайный Space только потому, что
`Event.space_id` обязателен:

```text
/data/saaios/var/entities/relationships/
  events/<sequence>-<id>.json
  records/<id>.json
```

Те же гарантии, что у entity journal: temp file, fsync, rename, fsync
parent, replay при open, tombstone на delete. Store schema-version
остаётся `1` — каталог аддитивен.

`RelationshipEvent` имеет **глобальную** последовательность. Это physical
journal placement, не semantic membership в пространстве `saaios`.

### Provenance и confidence

`Provenance` tagged: `user` / `system` / `import` / `worker` / `model` /
`derived`. `confidence` допустим только у inferred-видов и строго
`0.0..=1.0`. User-confirmed boolean не подменяет историю: confirmation
пишется mutation'ом (`properties.user_confirmed`), provenance модели
сохраняется.

### Protocol

Новые команды аддитивны при `ENTITYD_WIRE_SCHEMA_V1 = 1`.
`deny_unknown_fields` значит: старый сервер отвергнет новую команду
(ожидаемо); старый клиент продолжает работать с новым сервером. Bump
схемы не нужен.

Подписка получает `EntitydEvent::RelationshipChanged`. Отдельный socket
не создаётся.

### Workflow

`saai-taskd` пишет lineage **рядом** с legacy properties, не вместо них:

```text
Task  ──saaios.realizes──► Intent
Action ──saaios.executes──► Task
Action ──saaios.produces──► Result
```

Две записи подряд — не транзакция. Если relation не записалась, entity
остаётся; ошибка логируется. Нельзя притворяться, что пара atomic.

### Object View

`object_view_content` читает SOM relationships и падает обратно на
`intent_id`, если ребра ещё нет. AI-inferred ребро показывается как
«Возможно связано», не как факт.

### Что не делается

Нет `saai-somd`, нет SQLite, нет Cypher, нет скрытых callback'ов, нет
удаления `Entity.space_id`, нет авто-миграции `saaios.space-relation`,
нет переписывания composed `Сейчас` / OrbHost / BottomNavigation.
`saaios.space-signal` остаётся signal/evidence, не Relationship.

## Последствия

Положительные: один язык связей для Space, workflow и Object View;
AI-ребро отличимо от факта; crash recovery тот же, что у S06.

Ограничения v1: нет derived path-traversal; нет индексов; referential
check строгий для локальных Object; удаление Entity не каскадирует
историю рёбер. Semantic membership на Pixel переживает reboot;
экран «Пространства» читает `list_space_members`.

## Ссылки

- ADR-019, ADR-030, ADR-086, ADR-087, ADR-088
- `crates/saai-entity-store/src/relationship.rs`
- `docs/os/architecture/human-interface-architecture-v2.md` §4.2
