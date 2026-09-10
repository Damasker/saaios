# ADR-027: org.saaios.demo.kirigami -- первое реальное сторонне-рантаймовое приложение проходит полную физическую приёмку

## Статус

Принято, 2026-09-10.

## Контекст

ADR-026 физически подтвердил, что Qt5/QtQuick и Kirigami2 рендерят
корректно на этом железе -- ручным, throwaway воспроизведением вне
`saai-appd` (`unshare -m`/`mount --bind`). Change 7's реальная задача --
не спайк, а настоящее, устанавливаемое через `saai-appd` приложение с
полной физической приёмкой: install→launch→switch→stop→remove.

## Находка

Собран `org.saaios.demo.kirigami` -- первое приложение в проекте, чей
рантайм не собирается из исходников этого репозитория (в отличие от
`demo-surface`'s `services/saai-demo-surface`), а целиком приходит из
Alpine's прекомпилированного aarch64-пакета `kirigami2` (ADR-021's
решение). Два новых артефакта, оба закоммичены:

- `apps/kirigami-demo/{manifest.toml,app.qml,README.md,launch.c}` --
  `AppManifest`'а (ADR-020, `MANIFEST_SCHEMA_V1`) единственное поле
  `exec` -- один путь, без argv/env полей. `qmlscene-qt5` требует путь
  к `.qml`-файлу аргументом и три env-переменные сверх того, что
  `AppSupervisor::spawn()` уже безусловно выставляет
  (`QML2_IMPORT_PATH`/`QT_PLUGIN_PATH`/`XKB_CONFIG_ROOT` -- физически
  установлены ADR-026). `launch.c` -- статический aarch64-musl
  бинарник (собран тем же `zig cc`, что уже использует
  `build-cross-sysroot.sh`), сам manifest's `exec`-цель: выставляет эти
  три переменные через `getcwd()` (супервизор уже делает `current_dir`
  = `code_dir`) и делает `execv()` в реальный `qmlscene-qt5` с
  `app.qml` единственным аргументом. Без единого shell -- sandbox не
  открывает `/bin/sh` приложениям.
- `os/targets/panther/build-kirigami-demo-package.sh` -- тот же паттерн,
  что `build-demo-package.sh`, но вместо `cargo build` -p
  saai-demo-surface скрипт сам разворачивает `apk-tools-static`
  (пиннутая версия и URL) и `apk add kirigami2` в свежий aarch64
  sysroot (Alpine v3.20, тот же релиз-branch, что весь спринт), затем
  извлекает `bin/qmlscene-qt5`, весь `lib/`-closure (дереференс через
  `cp -L` -- `store.rs`'s `copy_tree` отклоняет symlink'и, найдено ещё
  в Change 6), `qml/`, `plugins/`, `share/X11/xkb` целиком в
  `dist/panther/packages/org.saaios.demo.kirigami/` (не закоммичено,
  `dist/` в `.gitignore`, тот же паттерн, что demo-surface). Trust-модель
  явно задокументирована в комментарии скрипта: pinned HTTPS +
  `--allow-untrusted`, та же граница доверия, что уже принята для
  других build-time зависимостей проекта (`build-cross-sysroot.sh`'s
  `git clone --branch v3.2.14` eudev по HTTPS, ADR-008) -- не новая,
  более слабая политика, применение уже существующей.
  `apk add`'s не-нулевой exit (`ERROR: N errors updating directory
  permissions`, каждый post-install trigger падает на `chroot:
  Operation not permitted`) -- подтверждено в ADR-021 некритичным
  (нет root/chroot на этом хосте, файлы всё равно корректно
  извлекаются) -- `|| true` с explicit-проверкой, что
  `usr/bin/qmlscene-qt5` реально появился, вместо слепого игнора любой
  ошибки. Скрипт физически прогнан на R620 дважды подряд (идемпотентен,
  кэширует `apk-tools-static` под `artifacts/toolchain/`) -- готовый
  пакет 249 MiB на диске, 89 MiB сжатый для переноса (заметно меньше
  507 MiB ручных throwaway-сборок этого дня -- те тащили Qt6/GTK4/
  mesa-dri-gallium/LLVM транзитивно из общего sysroot'а, этот пакует
  ровно `kirigami2`'s собственное закрытие зависимостей).

Физически на устройстве, через настоящий `saai-appd`'s IPC (не
`unshare -m`-обход):

- `install` -- успешно, пакет со скрипта, ноль ручных правок.
- `launch` -- реальный процесс `qmlscene-qt5 .../app.qml` (подтверждён
  через `ps`, `execv()` внутри `launch` корректно заменил образ
  процесса), состояние `running`, тот же pid спустя выдержку --
  не крашится, не попадает в `crash_limited`.
- Параллельно запущен `org.saaios.demo-surface` -- оба приложения
  сосуществуют (`list` показывает оба `running`), подтверждает
  compositor корректно работает с несколькими реальными клиентами
  одновременно (аналог "switch" на уровне протокола -- физического
  touch-переключения через UI в этой сессии не выполнено, см.
  "Последствия").
- `stop` -- чисто останавливает `org.saaios.demo.kirigami`,
  `org.saaios.demo-surface` не затронут.
- `remove` -- пакет удалён, `list` подтверждает возврат к исходному
  состоянию (только `org.saaios.demo-surface`).

## Решение

`org.saaios.demo.kirigami` -- первое реальное, воспроизводимое,
закоммиченное приложение стороннего toolkit'а в проекте. Паттерн
(`apps/<name>/` с минимальными закоммиченными артефактами +
`os/targets/panther/build-<name>-package.sh`, тянущий тяжёлый рантайм
из внешнего, но пиннутого источника при сборке) устанавливает прецедент
для будущих toolkit-приложений -- не нужно вендорить Alpine's закрытие
зависимостей в git, достаточно закоммитить рецепт.

## Последствия

- **Touch не проверен физически в рамках этой сессии.** `panther-
  hardware`-сборка `saai-displayd` не имеет синтетического touch-
  инжектора (в отличие от headless-сборки's `inject-key` debug
  триггера, ADR-022/ADR-025) -- читает реальное evdev-устройство.
  Serial-консоль не может физически коснуться экрана. Acceptance
  criteria's "touch" пункт остаётся неподтверждённым физическим тапом
  -- достоверно только: приложение объявляет и принимает `wl_seat`'s
  touch capability так же, как уже проверенный `demo-surface`
  (ADR не переоткрывает этот вопрос, наследует существующее
  S02/S03-покрытие для touch-протокола как такового).
- **Негативный sandbox-тест на clipboard не переприменён к этому
  приложению.** `app.qml` не реализует clipboard-операции вовсе (не их
  функция), и известный пробел (нативный `wl_data_device_manager` не
  проверяет `Capability::ClipboardRead/Write`) уже задокументирован
  архитектурно, не специфично для этого приложения -- ADR-023. Повторное
  демонстрирование того же пробела через другой toolkit не добавило бы
  новой информации.
- Остальная часть ADR-020's sandbox (mount/seccomp/`/dev`/`/proc`
  маскирование) не переверифицирована заново для этого конкретного
  приложения специальным негативным тестом -- покрыта уже существующим
  S07's 18-пунктным `sandbox-probe`, единым для всех приложений; Change
  6 отдельно физически подтвердил именно новый кусок (`/lib`-reveal).
- `apps/kirigami-demo/README.md` явно документирует, почему у этого
  пакета нет "исходников" в обычном смысле -- будущий читатель не должен
  искать Qt-код в этом репозитории.

## Ссылки

- ADR-020 -- `AppManifest`'а single-`exec`-field схема, откуда
  необходимость `launch.c`.
- ADR-021 -- источник Alpine-пакетов, откуда `kirigami2`.
- ADR-026 -- физическое подтверждение, что Qt5/Kirigami2 рендерят
  корректно на этом железе, без чего этот ADR не имел бы смысла
  пытаться.
- ADR-008 -- прецедент pinned-HTTPS доверия к build-time зависимостям,
  на который ссылается `build-kirigami-demo-package.sh`.
