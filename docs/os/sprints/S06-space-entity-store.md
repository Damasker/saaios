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
3. **Готово (2026-09-09).** Event-first atomic store, projection replay и
   crash-recovery tests (`b127bad`, `d3f59b5`).
4. **Готово (2026-09-09).** Идемпотентный bootstrap и миграция legacy
   active-space (`9d8ca71`).
5. **Готово на host (2026-09-09).** `saai-entityd`: versioned local IPC,
   create/update/delete/list/select (`67d295d`, `0029908`, `b1fa88f`,
   `f6ff2e6`, `3d0183a`).
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

Change 3: store публикует каждый immutable event отдельным mode-0600 файлом
через `fsync → rename → directory fsync`, а затем атомарно обновляет
восстанавливаемую entity projection. Space создаётся целиком в sibling staging
directory. Replay требует непрерывную sequence, matching filename/id, первый
`space_created` и корректные revision transitions. Инъекция отказа после
durable event, но до projection write успешно восстановила entity при reopen;
temporary event проигнорирован, опубликованный invalid JSON заблокировал open.
CRUD сохранил байты предыдущих событий, а два space вернули только собственные
entity.

Change 4: bootstrap создаёт стабильные `home/work/personal/saaios`, маркирует
`saaios` как system space и переводит legacy indices `0..3` в эти id.
Missing/invalid legacy даёт `home`; повторный bootstrap не перечитывает
изменившийся legacy-файл и не меняет уже принятую `selection.json`. Legacy
остаётся нетронутым. После Change 4 общий результат — 16 unit tests и clippy с
`-D warnings` на R620.

Change 5: `saai-entity-protocol` задаёт ограниченный 128-KiB JSON-lines wire
с явными schema/request id и обязательным `space_id` для каждой entity-команды.
Большие response variants boxed без изменения JSON. `saai-entityd` один
владеет store, bootstrap и mode-0660 Unix socket; поддерживает list spaces,
get/select space, scoped list, create/update/delete и явную event subscription.
Daemon назначает UUID/timestamps/revision, а optimistic update/delete требуют
текущую revision.

Host integration использовал controller, отдельный work-space client и
subscriber: migrated selection=`work`, Home/Work получили разные entity,
home list не увидел work id, stale revision была отвергнута, update/delete и
selection дали typed events. После убийства и нового запуска daemon выбранное
`personal`, обновлённая Home entity и удалённая Work entity восстановились.
Итог: 20 unit/protocol tests, 1 process integration и all-target clippy с
`-D warnings` на R620.
