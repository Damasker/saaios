# Sprint 06 — Пространства и entity store

## Паспорт

- Состояние: `In progress`.
- Зависит от: S05 (`Done`).
- Архитектурные решения: ADR-005, ADR-006, ADR-019.
- Рабочий fallback: S05 image и read-only legacy active-space.

## Goal

Сделать `Дом`, `Работа`, `Личное` и `SaaiOS` настоящими изолированными
областями локальных объектов, события которых переживают перезапуск и холодную
загрузку.

## Current state

- S05 image source `6afb663` физически принят на Pixel 7;
- legacy UI хранит только индекс `/data/saaios/var/ui/active-space`;
- новый Wayland-shell не владеет entity data;
- memory-store и audit-log существуют отдельно, но не имеют `space_id` и не
  являются entity store.

## Scope

Входит: schema v1 `Space/Entity/Event`, строгая валидация, атомарная запись,
append-only журнал, восстановление проекций, bootstrap четырёх пространств,
миграция legacy selection, локальный daemon IPC и проекции shell.

Не входит: sandbox/capability enforcement, недоверенные приложения,
межпространственное перемещение, синхронизация, шифрование, вложения и
полнотекстовый поиск.

## Change

1. **Готово (2026-09-09).** ADR-019 и исполняемый план S06.
2. **Готово (2026-09-09).** Typed schema и негативные validation tests
   (`40c689a`, `948da26`).
3. Event-first atomic store, projection replay и crash-recovery tests.
4. Идемпотентный bootstrap и миграция legacy active-space.
5. `saai-entityd`: versioned local IPC, create/update/delete/list/select.
6. `saai-shell`: реальные пространства и scoped проекции `Сейчас`.
7. ARM64 packaging, persistent service supervision, device isolation/fault/
   cold-reboot acceptance.

## Test

- schema: unknown version/field, invalid id/type, oversized payload;
- isolation: list одного space никогда не возвращает entity другого;
- recovery: остановка после event rename, но до projection rename;
- corruption: опубликованный invalid event блокирует запись, temp игнорируется;
- migration: `0..3`, missing, invalid и повторный запуск;
- daemon integration: два клиента и подписчик проходят CRUD/select;
- device: выбор пространства и объект переживают shell/entityd restart и cold
  reboot.

## Acceptance criteria

- четыре пространства имеют стабильные id, не зависящие от перевода UI;
- все entity-запросы требуют space id и не пересекают границу выборки;
- journal append-only, а проекции полностью восстанавливаются;
- незавершённая запись не повреждает предыдущую принятую версию;
- legacy selection мигрирует один раз и остаётся доступным для rollback;
- shell показывает данные выбранного пространства, а не меняет только заголовок.

## Threat / privacy impact

Store впервые содержит пользовательские entity data. Содержимое не пишется в
обычные системные журналы и telemetry; допустимы только id, schema, размер,
счётчик и код ошибки. До S07 доступ разрешён только доверенным системным
клиентам, что явно не считается process sandbox.

## Rollback

Остановить `saai-entityd`, вернуть S05 shell/image и читать прежний
`/data/saaios/var/ui/active-space`. Новый store не удалять и не понижать его
schema; повторное включение S06 должно продолжить тот же журнал.

## Evidence

Добавляется по каждому Change только после зелёных автоматических или
физических проверок.

Change 2: новый независимый crate `saai-entity-store` задаёт строгие schema 1
для `Space`, `Entity`, `Event` и tagged event payload. Space id и entity type
не могут содержать path components; UUID, revision, sequence, timestamp order,
bounded title/name и 64-KiB properties проверяются до записи. Event не может
вложить entity другого пространства. Unknown field/schema/event kind
отвергаются. Семь unit tests и clippy с `-D warnings` прошли на R620.
