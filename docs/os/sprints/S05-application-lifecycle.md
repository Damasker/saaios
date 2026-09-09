# Sprint 05 — Manifest и жизненный цикл приложений

## Паспорт

- Состояние: `Done`.
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
  app layout, storage, supervisor, daemon IPC, demo-package и интеграция с
  shell уже реализованы и прошли первый аппаратный lifecycle;
- PID 1 запускает `saai-appd` из `/data/saaios/system` и возвращает сервис
  после его аварийного завершения; холодный старт проверен на Pixel 7;
- три последовательных падения приложения на Pixel 7 физически подтвердили
  restart budget 3/60 и возврат фокуса оболочке.

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
4. **Готово (2026-09-09).** Versioned JSON-lines Unix IPC и host integration
   test с отдельными daemon, app и observer (`9321d0f`, `f583bd3`).
5. **Готово, включая Pixel 7 (2026-09-09).** Demo-app package,
   SUI-карточка, установка, запуск, touch, штатное закрытие и возврат фокуса
   в `saai-shell` (`3a5f3f7`, `45d65bb`, `fb5c845`, `8498348`, `1f85cd7`,
   `351b483`).
6. **Готово (2026-09-09).** ARM64 packaging `saai-appd`, постоянный device
   install в `/data`, супервизия сервиса, fault injection 3/60 и cold
   regression (`254c4de`, `6afb663`).

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

Change 4: `saai-appd` обслуживает Unix socket `/run/saaios/appd.sock` с
правами `0660`, строгой schema 1 и ограничением сообщения 64 KiB. Команды
`list/install/launch/stop/remove` возвращают typed responses; подписавшиеся
клиенты получают broadcast-события `installed/running/stopped/crashed/`
`crash_limited/removed`. Integration test запускает отдельный daemon,
настоящий child process и observer, проходит полный lifecycle, подтверждает
single-instance по одинаковому PID и сохранение app data после удаления кода.
Общий прогон после Change 4: 27 unit tests и 1 integration test; all-target
clippy с `-D warnings`; ARM64 musl check успешен.

Change 5: wire types вынесены в общий минимальный `saai-app-protocol`; shell
имеет неблокирующий reconnecting client и получает registry responses и
lifecycle events без запуска процессов напрямую. Карточка `Saai Demo`, её
геометрия и действие объявлены в `root.sui`; renderer и touch hit-test
используют скомпилированное описание. Состояния `не установлено / готово /
работает / crash-limited / ошибка` наблюдаемы в shell. Устанавливаемый пакет
содержит manifest v1 и отдельный ARM64 Wayland binary под `/data`; приложение
рисует полноэкранный мобильный экран с Inter и физически проверенной упаковкой
цвета, принимает touch и имеет явное штатное закрытие.

`application-wayland-host.sh` на R620 прошёл реальную цепочку из отдельных
`saai-displayd`, `saai-appd` и двух Wayland clients: install, launch нового
окна, переход фокуса, stop и возврат фокуса предыдущему окну. Readiness
первого клиента проверяется до launch, поэтому тест не зависит от порядка
планирования процессов. Relevant unit/integration tests и all-target clippy
с `-D warnings` успешны. ARM64 musl demo-package собран скриптом
`build-demo-package.sh`; manifest SHA-256
`2e6cd4f6da7d7620f866dcb52e2a35441b4082758e638e45f7808a4d34f9647e`,
binary SHA-256
`0e0bafde28a48a3750acb1200489cf527973c7b103a255b240daa47ea454d124`.

На физическом Pixel 7 пакет установлен из `/data/saaios/packages` в
`/data/saaios/apps` уже после загрузки, без перепрошивки `init_boot`.
Пользователь через SUI-карточку запустил приложение, подтвердил корректный
полноэкранный вывод и touch, затем штатно закрыл его. Журнал `saai-displayd`
зафиксировал переключение фокуса с shell surface на отдельный app surface,
рендер `1080x2400` и маршрутизацию реальных touch down/up. После закрытия
процесс demo отсутствовал, а PID `saai-appd`, `saai-displayd` и `saai-shell`
остались живы; idle-lock оболочки продолжил работать. Хэши package manifest
и binary на устройстве совпали с воспроизводимой ARM64-сборкой выше.

Change 6: отдельный `build-saai-appd.sh` и cross-panther CI собирают
статический ARM64 service для `/data/saaios/system/saai-appd`; бинарник не
расходует фиксированный ramdisk budget. PID 1 проверяет regular executable,
запускает appd после монтирования `/data` и перезапускает после выхода.
Новый образ с source commit `6afb663` собран из локально проверенных приватных
артефактов; SHA-256 образа
`be22d1af98b73c24fa9272f2575ac6bbc4aa858516de2478e18f417da07db215`,
compressed ramdisk 7,428,206 bytes при лимите раздела 8 MiB. Прошит только
`init_boot_a`; slot B не менялся.

После холодной загрузки на Pixel 7 PID 1 сам запустил appd PID 155, displayd
PID 381 и shell PID 389. Registry заново обнаружил установленный manifest;
контрольный app-data marker сохранил SHA-256
`4019af35219b1c77788069fff3b2f91e1cc9b10741ca514af21b63a5bf8af56b`.
Отдельная инъекция отказа завершила только appd: PID 1 поднял новый PID 436,
тогда как PID displayd и shell не изменились; последовательность
`exited → started → restarted` записана в `/run/boot.log`.

Device fault injection запустил demo из shell и трижды отправил процессу
`SIGKILL` в пределах 60 секунд. Первые два падения дали ожидаемые новые PID
`818 → 824 → 829`; после третьего demo больше не запускался. На всём сценарии
PID appd 436, displayd 381 и shell 389 не менялись. `saai-displayd` каждый раз
фиксировал новый toplevel, передачу фокуса приложению и возврат фокуса shell;
после третьего lifecycle event shell закоммитил новый кадр состояния.
