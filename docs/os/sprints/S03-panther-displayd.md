# Sprint 03 — `saai-displayd` на Pixel 7

## Паспорт

- Состояние: `Ready`.
- Зависит от: S02.
- Архитектурные решения: ADR-002, ADR-004, ADR-005, ADR-007. Новое решение
  о supervision/fallback механизме фиксируется отдельным ADR перед стадией
  "Change" ниже (см. Scope).
- Рабочий fallback: физически проверенный образ commit `1602a87`
  (S02 закрыт), `drm-splash` как единственный владелец `/dev/dri/card0`;
  Android slot B.

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
  `aarch64-unknown-linux-musl`.

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

1. Кросс-компиляция spike: собрать `saai-displayd` под
   `aarch64-unknown-linux-musl` с DRM/libinput/udev/libseat фичами,
   зафиксировать зависимости, размер бинарника, реальные системные
   библиотеки, нужные на телефоне (glibc/musl совместимость udev/libseat).
   Ничего не устанавливается на телефон на этом шаге.
2. ADR: DRM-master handoff + supervision/fallback дизайн (объединяет два
   архитектурных вопроса, поскольку они взаимозависимы — fallback без
   handoff-протокола не имеет смысла).
3. Добавить supervision `drm-splash` в reaper `native-init.c` изолированно
   (без `saai-displayd`), физически проверить, что респавн работает и не
   ломает существующее поведение — минимальный обратимый шаг.
4. Реализовать DRM/KMS backend в `saai-displayd` (Smithay
   `backend_drm`+`backend_session_libseat`+`backend_udev`), пока без
   touch и без интеграции в `native-init.c` — тестируется вручную через
   USB-консоль поверх уже работающей системы, `drm-splash` не трогается.
5. Реализовать evdev touch backend (`backend_libinput`).
6. Подключить handoff+supervision из шага 2/3 к реальному запуску
   `saai-displayd` вместо/вместе с `drm-splash` по спроектированному
   протоколу.
7. Холодная перезагрузка, физическая регрессия, обновить
   `os/targets/panther/README.md` при необходимости.

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

- `saai-displayd` собирается под `aarch64-unknown-linux-musl` и CI это
  проверяет;
- принят ADR о DRM-master handoff и supervision/fallback;
- цветовая таблица на физическом экране совпадает с проверенной;
- касание доставляется только активному клиенту;
- выход/падение `saai-displayd` автоматически возвращает `drm-splash` и
  не прерывает USB-консоль — воспроизведено минимум 3 раза подряд;
- холодная загрузка с новым путём воспроизводима;
- headless-тесты S02 остаются зелёными без изменений.

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

Заполняется при закрытии.
