# Sprint 05 — Manifest и жизненный цикл приложений

## Паспорт

- Состояние: `In progress`.
- Зависит от: S04 (`Done`).
- Архитектурные решения: ADR-005, ADR-018.
- Рабочий fallback: отключить `saai-appd`; `saai-shell`, `saai-displayd`,
  `drm-splash` и USB recovery остаются без изменений.

## Goal

Установить первое отдельное доверенное Wayland-приложение в `/data`, запустить
его из оболочки и безопасно остановить/удалить без изменения `init_boot`.

## Current state

- `saai-displayd` принимает обычные `xdg_toplevel` clients и не выдаёт им
  системные session-lock/layer-shell globals;
- `saai-shell` — отдельный supervised процесс, но не имеет списка приложений;
- `saai-demo-surface` умеет быть Wayland-клиентом, но запускается вручную;
- `/data/saaios` монтируется PID 1;
- Change 1 уже добавил `saai-appd` crate и строгую typed-модель manifest v1;
  app layout, storage, daemon IPC и supervisor остаются следующими шагами.

## Scope

Входит: manifest v1, строгая валидация путей/id, атомарная установка,
list/launch/stop/remove, single-instance, crash limit 3/60, события состояния,
demo-app и подключение оболочки.

Не входит: недоверенные пакеты, подписи, sandbox/cgroups, capability grants,
магазин/update, GTK/Qt compatibility и пользовательские entity data.

## Change

1. **Готово (2026-09-09).** Manifest v1 как typed model + негативные
   unit/schema tests (`137c159`, `5f00f91`, `97d6397`).
2. **Готово (2026-09-09).** Файловое хранилище с staging+rename,
   duplicate-id, startup scan и path-boundary tests (`5efe10e`, `24f1d62`,
   `b872e86`).
3. **Готово (2026-09-09).** Supervisor процессов: launch/stop,
   single-instance и crash budget (`6be3fca`).
4. Versioned JSON-lines Unix IPC и host integration test двух процессов.
5. Demo-app package и запуск/переключение из `saai-shell`.
6. ARM64 packaging `saai-appd`, device install в `/data`, fault injection и
   cold regression без изменения `init_boot` для самого demo-app.

## Test

- unit/schema: valid v1; unknown schema/field; invalid id/exec/capability;
- filesystem: duplicate id и traversal не меняют apps/var другого app;
- host integration: appd+demo проходят install→launch→stop→remove;
- fault injection: третий app crash за 60s даёт `crash_limited`;
- device: demo устанавливается после boot, получает обычный toplevel и touch;
- cold reboot: appd пересканирует manifest, данные и stopped policy целы.

## Acceptance criteria

- demo-app устанавливается и удаляется без новой прошивки `init_boot`;
- неизвестная schema, поле, duplicate id и выход из package root отвергаются;
- single-instance не создаёт второй PID;
- падение app не меняет PID appd/shell/displayd; 3/60 ограничивает рестарт;
- удаление одного app не меняет хэши дерева кода/данных другого;
- shell показывает наблюдаемое `installed/running/stopped/crash_limited`.

## Threat / privacy impact

S05 создаёт запуск исполняемого кода и новые постоянные каталоги. До S07
источник пакета обязан быть доверенным и локальным; effective capabilities
пусты. Ни содержимое пользовательских файлов, ни manifest-секреты не пишутся
в telemetry. Все filesystem tests включают соседнее app как canary.

## Rollback

Остановить `saai-appd`, удалить только demo code directory и вернуть shell к
экрану без app actions. S04 image
`52a6b774b8f6c7e64a35efc6cc45bbdeffe5e9529dcc4638e4c54c0d5712dfbc`
остаётся известным рабочим boot artifact; слот B не меняется.

## Evidence

Change 1: `services/saai-appd` принимает только schema 1, DNS-like lowercase
`app_id`, SemVer, нормализованный относительный `bin/...` exec, Wayland UI и
валидные capability tokens; unknown fields/schema и duplicates отвергаются.
Восемь unit/schema tests прошли локально и на R620 с `--locked`; clippy прошёл
с `-D warnings`.

Change 2: `AppStore` создаёт отдельные code/data roots, копирует доверенный
пакет в sibling staging без следования symlink, повторно проверяет manifest и
executable и только затем делает rename в конечный `app_id`. Duplicate не
перезаписывает код; ошибка удаляет staging и не создаёт app data. Отдельная
canary-проверка подтверждает неизменность данных соседнего приложения; source
внутри управляемых code/data trees и несовпадение directory id с manifest
отвергаются. Startup scan заново строит сортированный registry только из
валидных каталогов, игнорирует незавершённый staging и не доверяет отдельной
изменяемой базе. Общий прогон: 16 tests, `cargo test --locked`; all-target
clippy с `-D warnings`.

Change 3: `AppSupervisor` запускает реальный executable с рабочим каталогом
app и явными `SAAIOS_APP_ID`, `SAAIOS_DATA_DIR`, `XDG_RUNTIME_DIR`,
`WAYLAND_DISPLAY`; inherited environment очищается. `single_instance=true`
возвращает существующий PID, `false` допускает отдельные процессы. Явный
`stop` завершает и reap-ит children без записи crash. Неуспешные exits
автоматически перезапускаются и учитываются в окне 60 секунд; третий переводит
в `crash_limited`, после чего только явный `launch` очищает budget. Drop и
ошибочные process operations не оставляют намеренно забытых children/state.
Общий прогон после Change 3: 21 test; all-target clippy с `-D warnings`.

Аппаратный результат пока не заявляется по host-тестам.
