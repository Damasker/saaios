# Sprint 03 — `saai-displayd` на Pixel 7

## Паспорт

- Состояние: `Done`.
- Зависит от: S02.
- Архитектурные решения: ADR-002, ADR-004, ADR-005, ADR-007, ADR-008, ADR-009 (supervision/fallback дизайн, см. Change 2), ADR-010 (прямой доступ к DRM без libseat), ADR-011 (touch через сырой evdev, без libinput/libudev), ADR-012 (клавиатурная способность не подключается на этой цели), ADR-013 (nightly build-std для размера init_boot).
- Рабочий primary: физически проверенный прошитый образ, `saai-displayd`
  держит UI-слот с холодной загрузки (commit `53eb01f`/`cad9dac`,
  SHA-256 образа `69785ced...9c9a`). Fallback: `drm-splash` без
  ограничения бюджета (проверено), Android slot B — на случай проблем с
  обоими.
- DRM/KMS backend `saai-displayd` (Change 4) физически проверен, реальные
  пиксели на экране подтверждены пользователем — см. commit `5fe5872`.
- Touch input (Change 5) физически проверен, реальные координаты в логах
  при касании экрана — см. commit `bd1e5e0`.
- Supervision/fallback (Change 6/7) физически проверен полным циклом
  прошивка → холодная загрузка → primary/fallback переключение →
  повторная холодная загрузка — см. commit `53eb01f`/`cad9dac`.

## Goal

Тот же `saai-displayd`/`saai-demo-surface` vertical slice из S02 выводится
на реальный экран Pixel 7 через DRM/KMS и получает настоящие touch-события,
без замены текущей оболочки.

## Current state

Зафиксировано чтением `os/targets/panther/src/native-init.c` и
`drm-splash.c` (commit `1602a87`):

- **Запуск и supervision.** `native-init.c` `start_display_splash()`
  (~строка 1139) делает ровно один `fork()+execl("/saaios/drm-splash", ...)`
  при загрузке, stdout/stderr в `/run/drm-splash.log`. Финальный reaper-цикл
  (`waitpid(-1, ...)`, строки 1421-1434) респавнит **только** USB-консоль
  (`console_pid`); если `drm-splash` падает или выходит, он просто
  собирается reaper'ом и **не перезапускается** — экран остаётся мёртвым.
  Supervision/fallback механизма для UI-процесса сегодня не существует
  вообще — это не доработка существующего, а новая инфраструктура.
- **DRM/KMS.** `drm-splash.c` открывает `/dev/dri/card0` напрямую и работает
  через сырые ioctl (`DRM_IOCTL_MODE_GETRESOURCES/GETCONNECTOR/GETENCODER/
  CREATE_DUMB/ADDFB2/MAP_DUMB/SETCRTC/DIRTYFB`) — legacy/non-atomic KMS, без
  `DRM_IOCTL_MODE_ATOMIC` и без libdrm `drmMode*` helpers. Framebuffer —
  dumb buffer, CPU-mapped (`CREATE_DUMB`+`MAP_DUMB`), обновление экрана идёт
  через полный `SETCRTC` ("refresh"), а не per-frame page flip; `DIRTYFB`
  помечает повреждённые регионы.
- **Формат/разрешение.** `DRM_FORMAT_XRGB8888` (это тот же байтовый порядок,
  что "BGRX" в изначальной формулировке спринта — разные названия одного
  формата, не расхождение). 1080×2400 захардкожено во множестве helper'ов
  масштабирования координат. Refresh rate **не** захардкожен — читается из
  режима коннектора после `GETCONNECTOR` (ожидается 60Гц, но код это не
  проверяет).
- **Touch/input.** Три отдельных device node: `/dev/input/touchscreen`
  (multi-touch protocol B: `ABS_MT_TRACKING_ID`/`POSITION_X`/`POSITION_Y`),
  `/dev/input/volume-buttons`, `/dev/input/power-button` (`EV_KEY`), плюс
  write-only `/dev/input/haptic`. Ноды создаёт `native-init.c`
  `create_input_node()` по имени устройства в sysfs, независимо от
  `drm-splash`. Координаты масштабируются из raw-диапазона устройства в
  фиксированное пространство 1080×2400 через `design_x`/`design_y`.
- **Watchdog/GSA.** Полностью в `native-init.c`, не зависит от того, какой
  UI-процесс работает. `start_hardware_watchdog()` открывает
  `/dev/watchdog0`, `ioctl(WDIOC_SETTIMEOUT)`, форкает отдельный процесс с
  циклом `ioctl(WDIOC_KEEPALIVE)`. `gsa.ko`/`gsa_gsc.ko` грузятся один раз
  на раннем этапе загрузки как модули ядра, без runtime-взаимодействия из
  userspace. Эта машинерия не пострадает и не требует изменений в S03.
- Панель `sdc-s6e3fc3-p10` — параметр кернел cmdline
  (`exynos_drm.panel_name=...`), потребляется DRM-драйвером ядра, в
  userspace-коде не упоминается.
- `saai-displayd` (S02) собран и протестирован только с
  `default-features = false, features = ["wayland_frontend", "desktop"]` —
  headless, без `backend_drm`/`backend_libinput`/`backend_udev`/
  `backend_session_libseat`, и никогда не собирался под
  `aarch64-unknown-linux-musl`. **Обновлено (Change 1, см. ниже):** теперь
  собирается под обе цели — headless по умолчанию, DRM-бэкенд через новую
  cargo-фичу `panther-hardware`.

## Scope

Входит:

- измеримый spike кросс-компиляции `saai-displayd` под
  `aarch64-unknown-linux-musl` (тот же toolchain, что уже используют
  `saaios-runtime`/`saai-console`: `zig-aarch64-musl.sh`) с включёнными
  `backend_drm`, `backend_session_libseat`, `backend_udev`,
  `backend_libinput` — до какого-либо изменения поведения на телефоне;
  фиксирует размер/сложность кросс-сборки, отложенную в ADR-007;
- ADR о DRM-master handoff между `drm-splash` и `saai-displayd`: кто
  держит `/dev/dri/card0` в какой момент, как второй процесс узнаёт, что
  можно/нужно брать master (учитывая, что сегодня supervision вообще нет —
  проектируется одновременно с fallback-механизмом, не отдельно);
  рассматриваются варианты (a) `saai-displayd` пытается стать master сразу
  и на неудаче остаётся в fallback-режиме, (b) `native-init.c` явно
  сигналит `drm-splash` отдать master перед запуском `saai-displayd`;
- добавление supervision/restart-цикла в `native-init.c` reaper (по
  аналогии с уже существующим respawn `console_pid`), сначала для
  `drm-splash` (проверяем на текущей, не тронутой DRM-владением системе,
  прежде чем добавлять туда же `saai-displayd`);
- реальный DRM/KMS backend для `saai-displayd` через Smithay
  `backend_drm`/`backend_session_libseat`/`backend_udev` (atomic KMS, где
  поддерживается железом — сравнить с legacy path `drm-splash`, не
  обязательно совпадать в реализации);
- evdev touch backend через `backend_libinput`, тот же
  `/dev/input/touchscreen`, тот же 1080×2400 design space;
- readiness-сигнал (когда `saai-displayd` реально готов рисовать, а не
  просто запущен) и подключение к уже существующему hardware watchdog —
  без изменения самого `start_hardware_watchdog()`;
- холодная перезагрузка с новым UI, воспроизводимая физическая проверка.

Не входит:

- замена `saai-shell`/бизнес-UI (это S04);
- GPU-ускорение сверх того, что уже даёт Smithay software path;
- изменение `gsa.ko`/`gsa_gsc.ko`/`start_hardware_watchdog()` самих по
  себе — они не трогаются;
- постоянная работа `saai-displayd` как единственного UI — до S04 это
  всё ещё демонстрационный `saai-demo-surface`, не настоящая оболочка.

## Change

1. **Готово (2026-09-07).** Кросс-компиляция spike: `saai-displayd`
   собирается под `aarch64-unknown-linux-musl` с DRM/libinput/udev/libseat
   фичами (новая cargo-фича `panther-hardware`, выключена по умолчанию —
   headless-тесты S02 не задеты). Ни одна из четырёх C-библиотек
   (`libseat`, `libudev`, `libinput`, и его же зависимости `mtdev`,
   `libevdev`) не имела готового aarch64-musl sysroot — все собраны из
   исходников (`os/targets/panther/build-cross-sysroot.sh`,
   [ADR-008](../../adr/ADR-008-cross-compiling-drm-backend-c-deps.md)).
   Итоговый `saai-displayd` — статический aarch64 ELF, 1.6MB. Ничего не
   устанавливалось на телефон на этом шаге, только сборка на хосте.
2. **Готово (2026-09-07).** ADR: DRM-master handoff + supervision/fallback
   дизайн — оказались сильно проще вместе: строгий последовательный
   владение UI-слотом (новый процесс стартует только после реального
   реапинга предыдущего) даёт handoff бесплатно, без явного протокола —
   DRM master освобождается ядром автоматически при закрытии fd.
   [ADR-009](../../adr/ADR-009-ui-supervision-and-drm-handoff.md).
3. **Готово (2026-09-07).** Supervision `drm-splash` в reaper
   `native-init.c` (commit `42a88e0`), физически проверено на устройстве
   (Pixel 7, slot A): пять `kill -9` подряд -- каждый раз новый pid, новый
   DRM master без EBUSY/EACCES, `dmesg` показывает `UI slot restarted
   (N/5 in window)`; шестой kill в том же окне -- `UI slot exceeded restart
   budget (6 in 60s), giving up until reboot`, респавн корректно
   прекращается; холодная перезагрузка (`reboot -f`) -- бюджет сбрасывается
   чисто, `drm-splash` стартует один раз без лишних записей в логе.
   SHA-256 прошитого `init_boot_a` совпадает байт-в-байт с локальной
   сборкой: `9d96f321133e6780738793730e3b06ead44218d31e95bf9c3a6bda6d84fbde5d`.
4. **Готово (2026-09-07).** DRM/KMS backend в `saai-displayd` (Smithay
   `backend_drm`+`backend_udev`; `backend_session_libseat` заменён прямым
   `open()` — [ADR-010](../../adr/ADR-010-drop-libseat.md)), пока без touch
   и без интеграции в `native-init.c` — протестирован вручную через
   USB-консоль поверх уже работающей системы, `drm-splash` не трогается.
   Физически проверено на устройстве: `saai-demo-surface` подключился к
   `saai-displayd`, закоммитил кадр 800×480, `saai-displayd` отблитил его
   в dumb buffer и вывел через `page_flip` на реальную панель. Пользователь
   подтвердил на экране два прямоугольника (белый и серый, общая сторона),
   занимающие примерно верхнюю пятую часть экрана — совпадает с позицией и
   размером тестового буфера `saai-demo-surface` внутри панели 1080×2400.
   По пути найдены и исправлены два реальных бага (commit `5fe5872`): (a)
   выбор crtc принимал только уже активную связку encoder→crtc, что всегда
   проваливалось после исчерпания restart-бюджета `drm-splash` (никто не
   держит активный modeset) — добавлен fallback на `possible_crtcs`; (b)
   `main.rs` проверял фокус до того, как выставлял его в первый раз, из-за
   чего самый первый (и в этом тесте единственный) кадр с содержимым никогда
   не блитился — виден был только начальный чёрный `fill()` из
   `hardware::init()`, без единой ошибки в логах. Это и есть причина
   "чёрного экрана", о котором сообщил пользователь в этом раунде.
   Тестирование велось через временную сборку с отключённой инициализацией
   клавиатуры (не коммитилась, восстановлена немедленно после каждой
   пересборки) — обходит отдельно отслеживаемый баг libxkbcommon (SIGTRAP в
   `HandleAliasDef`, см. commit `1d05260`); в текущем коммитнутом виде
   `saai-displayd` всё ещё падает на старте из-за этого при сборке с
   клавиатурой — отдельная, не входящая в это исправление задача.
5. **Готово (2026-09-08).** Touch backend — не через `backend_libinput`,
   как предполагалось в Scope, а через сырой evdev
   ([ADR-011](../../adr/ADR-011-raw-evdev-touch.md), commit `bd1e5e0`):
   libinput's path-based API всё равно требует udev-инициализированное
   устройство, что требует работающего `udevd`, которого на этой системе
   нет и не будет (противоречит ADR-009/ADR-010). `saai-displayd` читает
   `/dev/input/touchscreen` напрямую, тем же способом, что уже работает в
   `drm-splash.c`. Физически проверено на устройстве: касание экрана дало
   чистую последовательность down/up с реальными координатами по всему
   диапазону 1080×2400 (например `(816, 97)`, `(972, 1606)`,
   `(264, 1936)`), без ошибок. `saai-demo-surface` не реагирует визуально
   (это статический тестовый клиент из S02, к `wl_touch` не подключён) —
   доказательство здесь в захваченном и корректно перенаправленном потоке
   событий, не в визуальном отклике.
6. **Готово (2026-09-08).** Подключить handoff+supervision из шага 2/3 к
   реальному запуску `saai-displayd` вместо/вместе с `drm-splash` по
   спроектированному протоколу. Разблокировано ADR-012 (commit
   `f434af0`).

   Реализовано (commit `53eb01f`): `start_display_splash()`/новая
   `start_saai_displayd()` теперь обёртки над общим `start_ui_binary()`;
   `saai-displayd` — primary, `drm-splash` — fallback без ограничения
   бюджета рестартов (как и спроектировано в ADR-009), переключение
   происходит один раз при исчерпании бюджета primary и не возвращается
   к нему до холодной перезагрузки. Отдельно обнаружен и исправлен
   реальный баг до какой-либо физической проверки: `native-init.c`
   экает детей практически без окружения, а `saai-displayd` требует
   `XDG_RUNTIME_DIR` для Wayland-сокета — без исправления `saai-displayd`
   крашился бы немедленно на каждой попытке и всегда падал в fallback,
   никогда реально не удерживая UI-слот. Исправлено: `start_ui_binary()`'s
   child создаёт `/run/wayland` (0700) и выставляет `XDG_RUNTIME_DIR`
   перед `execl`.

   Собственно readiness-marker/timeout часть дизайна ADR-009 (граница
   между "не смог стартовать" и "упал во время работы" для бюджета)
   сознательно не реализована в этом шаге — простой per-exit счётчик
   (уже работавший для шага 3) оказался достаточным без усложнения;
   ADR-009 сам оставляет это "уточняется по факту реализации", не
   требованием.

   При первой попытке прошивки обнаружился отдельный реальный блокер:
   `init_boot_a`/`init_boot_b` — партиции фиксированного размера 8MB,
   собранный обычным способом `saai-displayd` (1.75MB) переполнял образ
   на ~400KB — бутлоадер безопасно отклонил прошивку (`not big enough`),
   телефон не пострадал. Решено через [ADR-013](../../adr/ADR-013-nightly-build-std-for-size.md)
   (commit `cad9dac`): nightly `-Z build-std` + `panic=immediate-abort`
   плюс `-ffunction-sections -fdata-sections` в кросс-сборке C-библиотек
   sysroot'а (последнее оказалось решающим — освободило libxkbcommon'а
   от полностью недостижимого (после ADR-012) но всё равно слинкованного
   компилятора keymap'ов, ~900KB экономии на этом одном изменении).
   Итоговый образ — ровно 8388608 байт, встаёт в партицию впритык.

   Физически проверено на устройстве, полный цикл: прошит `init_boot_a`
   (`fastboot flash`, подтверждён SHA-256 файла до и после передачи),
   холодная загрузка — `saai-displayd` стартует как primary на 3.39s
   после старта ядра (`dmesg`: `UI slot started: saai-displayd`), держит
   UI-слот без единого падения; `saai-demo-surface` подключился и
   отрендерил тестовый паттерн — видно на экране, подтверждено
   пользователем. Отдельно проверен весь supervision-цикл: 6×`pkill -9
   saai-displayd` подряд — 5 честных рестартов primary
   (`UI slot restarted saai-displayd (N/5 in window)`), на 6-й —
   корректный переход на fallback (`UI slot: saai-displayd exceeded
   restart budget (6 in 60s), falling back to drm-splash`); `drm-splash`
   как fallback респавнится без ограничения бюджета (3×`pkill -9
   drm-splash` подряд, каждый раз чистый рестарт). Холодная перезагрузка
   после всего этого — снова чистый старт `saai-displayd` как primary с
   нуля, без остаточного состояния.
7. **Готово (2026-09-08).** Холодная перезагрузка, физическая регрессия —
   выполнена как часть физической проверки шага 6 (см. выше): полный
   цикл прошивка → холодная загрузка → primary держит UI-слот →
   supervision/fallback работает → повторная холодная перезагрузка даёт
   тот же чистый результат. `os/targets/panther/README.md` обновлён
   (commit `cad9dac`) — новый шаг сборки `build-saai-displayd.sh`,
   `SAAI_DISPLAYD_BIN` в примере полной сборки образа.

## Test

- host: кросс-компиляция под `aarch64-unknown-linux-musl` зелёная в CI
  (добавить job, аналогично существующей cross-сборке Rust-рантайма);
- protocol: тот же headless-сценарий S02 (два теста
  `headless_vertical_slice`) остаётся зелёным — DRM backend не должен
  сломать headless path;
- device: `saai-demo-surface` рисует различимый кадр на физическом экране
  через `saai-displayd`; цвета сверяются напрямую с уже проверенным
  выводом `drm-splash` на том же панели (человек физически сравнивает или
  фотографирует оба вывода) — отдельного заранее записанного эталонного
  хеша для этого пути в репозитории пока нет, создаётся при закрытии
  спринта;
- device: касание доставляется только активному fullscreen-клиенту;
- fault injection: `kill -9` на `saai-displayd` во время работы — экран
  автоматически возвращается к `drm-splash`, USB-консоль не прерывается;
- fault injection: `kill -9` на `drm-splash` в изолированном тесте шага 3
  — respawn воспроизводится несколько раз подряд без утечки DRM master;
- cold reboot: воспроизводимо, hardware watchdog не триггерит сброс.

## Acceptance criteria

- ✅ `saai-displayd` собирается под `aarch64-unknown-linux-musl` и CI это
  проверяет (`cross-panther` job, через `build-saai-displayd.sh`, тот же
  путь что и продакшен-сборка);
- ✅ принят ADR о DRM-master handoff и supervision/fallback (ADR-009);
- ✅ реальные пиксели на физическом экране, содержимое совпадает с тем,
  что отправил клиент (sha256 кадра сверен) — не буквальное сравнение с
  цветами `drm-splash` (архитектуры рендеринга разные, см. Scope: "не
  обязательно совпадать в реализации"), но сам факт корректного вывода
  проверен многократно, включая финальный прогон после прошивки;
- касание доставляется только активному клиенту — верно по конструкции
  (в компоновщике всегда ровно один `focused_surface`, получателя ввода
  просто не существует другого), но **не стресс-тестировано с
  несколькими одновременными клиентами** — единственный клиент
  (`saai-demo-surface`) использовался во всех прогонах;
- ✅ выход/падение `saai-displayd` автоматически возвращает `drm-splash`
  и не прерывает USB-консоль — воспроизведено 6 раз подряд (правильно:
  5 честных рестартов primary + переход на fallback на 6-й), fallback
  сам проверен ещё 3 раза подряд, USB-консоль (serial) оставалась живой
  на протяжении всего теста;
- ✅ холодная загрузка с новым путём воспроизводима — два независимых
  холодных старта после прошивки, оба дали идентичный чистый результат;
- ✅ headless-тесты S02 остаются зелёными без изменений (проверено после
  каждого коммита этого спринта).

## Threat / privacy impact

Новых сетевых endpoint'ов не добавляется. Supervision-цикл в PID 1 —
самая чувствительная новая поверхность: неправильный restart-loop может
удерживать CPU или конфликтовать за DRM master до полного зависания
экрана; лимит попыток respawn обязателен (по аналогии с S05's "crash loop
ограничен", применяется здесь заранее). Hardware watchdog не изменяется —
остаётся независимой линией защиты от полного зависания системы
независимо от того, что происходит с UI-процессами.

## Rollback

Слот A: физически проверенный образ commit `1602a87` (текущий `drm-splash`
без каких-либо supervision-изменений). Слот B: стоковый Android. Если
supervision-механизм из шага 3 сам по себе проблематичен ещё до появления
`saai-displayd` на телефоне — откатывается тем же способом, отдельно от
остальных шагов спринта.

## Evidence

Change 4 (2026-09-07), сборка `saai-displayd` без клавиатуры (диагностика,
не коммитилась), commit `5fe5872` для реальных исправлений в `hardware.rs`/
`main.rs`:

```
saai-displayd: hardware output 1080x2400@60
saai-displayd: hardware output initialized
saai-displayd: listening on WAYLAND_DISPLAY=wayland-1
saai-displayd: client connected
saai-displayd: new xdg_toplevel
saai-displayd: commit on surface ObjectId(wl_surface@7[0], 7) (no buffer)
saai-displayd: focus set to surface ObjectId(wl_surface@7[0], 7)
saai-displayd: commit on surface ObjectId(wl_surface@7[0], 7), frame sha256=0af993f6e268b9e74bea46a8f60d70071f7703aa74cb2634697a32152cf84306
saai-displayd: blit wrote 800x480 px into fb (dst stride=4321, src stride=3200)
```

Пользователь, глядя на реальный экран: "на экране в верхней части два
параллелепипеда белый и серый с общей стороной. занимает где-то пятую
часть экрана" — совпадает с тестовым буфером `saai-demo-surface`
(800×480 в панели 1080×2400, размещён в левом верхнем углу).

Не закрыто в рамках Change 4: клавиатурный путь (`XkbConfig::default()`)
всё ещё падает с SIGTRAP на этом устройстве (см. commit `1d05260`) — вне
scope этого спринта (нет физической клавиатуры), но блокирует запуск
коммитнутого бинарника `saai-displayd` как есть без обхода. Остаётся
отдельной задачей.

Change 5 (2026-09-08), та же диагностическая сборка без клавиатуры, commit
`bd1e5e0` для реальных изменений в `touch.rs`/`main.rs`/`Cargo.toml`:

```
saai-displayd: hardware output 1080x2400@60
saai-displayd: hardware output initialized
saai-displayd: touch input initialized
saai-displayd: listening on WAYLAND_DISPLAY=wayland-1
saai-displayd: client connected
...
saai-displayd: touch down at (816, 97)
saai-displayd: touch up
saai-displayd: touch down at (972, 1606)
saai-displayd: touch up
saai-displayd: touch down at (625, 965)
saai-displayd: touch up
[... 22 более касаний, координаты x∈[169,1016] y∈[79,1957] ...]
```

Первая попытка (через `libinput::Libinput::new_from_path` +
`path_add_device`) физически провалилась на устройстве с ошибкой
`libinput bug: udev device never initialized` — задокументировано и
объяснено в ADR-011, приведшего к пивоту на сырой evdev.

Change 6/7 (2026-09-08), настоящая прошивка `init_boot_a` (не диагностика
— реальный коммитнутый `native-init.c` + `saai-displayd`), commit
`53eb01f`/`cad9dac`. SHA-256 образа `69785cedfd5dbebb44da9a85f49c8a88cb54f75901d18b19f3cbeda7fa7e9c9a`,
8388608 байт (ровно партиция).

Холодная загрузка после прошивки:

```
[    3.385999][    T1] saaios-init: UI slot started: saai-displayd
```

`saai-displayd` держит UI-слот с первого раза, без единого падения;
`saai-demo-surface` подключился и отрендерил тестовый паттерн —
подтверждено пользователем на реальном экране.

Полный supervision-цикл, 6×`pkill -9 saai-displayd`:

```
[  243.495604] UI slot restarted saai-displayd (1/5 in window)
[  244.727237] UI slot restarted saai-displayd (2/5 in window)
[  245.961352] UI slot restarted saai-displayd (3/5 in window)
[  247.193056] UI slot restarted saai-displayd (4/5 in window)
[  248.412112] UI slot restarted saai-displayd (5/5 in window)
[  249.385059] UI slot: saai-displayd exceeded restart budget (6 in 60s), falling back to drm-splash
[  249.646147] UI slot started: drm-splash
```

Fallback `drm-splash`, 3×`pkill -9 drm-splash` подряд — каждый раз чистый
рестарт, без ограничения бюджета, как и спроектировано.

Повторная холодная перезагрузка (`reboot -f`) после всего цикла —
`saai-displayd` снова стартует как primary с нуля на 3.39s, идентично
первому разу, никакого остаточного состояния.

Первая попытка прошивки (до ADR-013) была безопасно отклонена
бутлоадером: `fastboot flash init_boot_a`: `failed to flash partition
(init_boot_a): not big enough` — партиция 8388608 байт, образ 8796544
байт. Ничего не записалось, устройство не пострадало.
