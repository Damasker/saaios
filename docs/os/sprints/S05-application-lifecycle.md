# Sprint 05 — Manifest и жизненный цикл приложений

## Паспорт

- Состояние: `Frozen until S01 closes`.
- Зависит от: S04 (`Provisional` until S01 Evidence closes the gate).
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
- `/data/saaios` монтируется PID 1, однако app layout/manifest/daemon ещё нет.

## Scope

Входит: manifest v1, строгая валидация путей/id, атомарная установка,
list/launch/stop/remove, single-instance, crash limit 3/60, события состояния,
demo-app и подключение оболочки.

Не входит: недоверенные пакеты, подписи, sandbox/cgroups, capability grants,
магазин/update, GTK/Qt compatibility и пользовательские entity data.

## Change

1. Manifest v1 как typed model + негативные unit/schema tests.
2. Файловое хранилище с staging+rename, duplicate-id и path-boundary tests.
3. Supervisor процессов: launch/stop/single-instance/crash budget.
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

Заполняется по мере закрытия Change 1–6. Аппаратный результат не заявляется
по host-тестам.
