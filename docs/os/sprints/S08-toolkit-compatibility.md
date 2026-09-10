# Sprint 08 — GTK и Qt/Kirigami совместимость

## Паспорт

- Состояние: `In progress`.
- Зависит от: S07 (`Done`).
- Архитектурные решения: ADR-021 (источник пакетов -- Alpine musl, не
  from-source; `wl_data_device_manager` обязателен, не опционален).
  Clipboard-через-policy механизм и text-input/OSK-стратегия всё ещё не
  решены -- см. Change 2.
- Рабочий fallback: `saai-shell` и `saai-demo-surface` продолжают быть
  единственными реальными Wayland-клиентами; ни установка, ни запуск
  стороннего приложения через `saai-appd` не меняются, пока это не
  собрано и не проверено физически.

## Goal

Одно настоящее адаптивное GTK-приложение и одно настоящее
Qt/Kirigami-приложение запускаются как обычные клиенты SaaiOS --
устанавливаются через существующий `saai-appd` manifest/lifecycle (S05),
рисуются и принимают touch через `saai-displayd` (S02/S03), работают под
полным ADR-020 sandbox (S07) без единого исключения из capability-модели
для этих двух приложений специально.

## Current state

Проверенные факты (собраны перед этим паспортом, не предположения):

- `saai-displayd` advertises ровно: `wl_compositor`, `wl_shm`, `wl_seat`
  (только touch -- `wl_keyboard` физически не подключается на panther,
  см. ADR-012), `xdg_wm_base`, `ext-session-lock-v1`, `wlr-layer-shell`.
  Нет `wl_data_device_manager` (clipboard/DnD на уровне протокола), нет
  `xdg-decoration`, `wp-viewporter`, `wp-fractional-scale`,
  `zwp-text-input-v3`/`zwp-input-method-v2`, `linux-dmabuf`,
  `wp-presentation-time`, `wp-single-pixel-buffer`.
- ADR-012 закрыл `wl_keyboard`/`libxkbcommon` на этой цели как
  осознанное, не временное решение (`SIGTRAP` в `HandleAliasDef`,
  железо без физической клавиатуры) -- GTK/Qt приложения физически не
  смогут получить `wl_keyboard` capability здесь, только touch.
- `saai-appd`'s manifest schema принимает ровно одно значение `ui`
  ("wayland") -- GTK/Qt-приложение как обычный Wayland-клиент ложится в
  существующую схему без изменений схемы самой по себе.
- ADR-020's sandbox (S07, физически подтверждён 18-пунктным
  `sandbox-probe`) прячет `/dev/dri`, `/proc`, `/sys`, весь rootfs кроме
  явно раскрытых `code_dir`/`data_dir`/сокетов, обнуляет capabilities,
  режет seccomp -- ни один GTK/Qt-специфичный путь (fontconfig,
  GSettings-схемы, icon themes, GTK/Qt modules, локали) никуда на этом
  устройстве не смонтирован: нет `/usr`, нет традиционного
  distro-layout вообще. S07's пакетная модель (S05) уже требует, чтобы
  каждое приложение было самодостаточным под своим `code_dir` -- полный
  GTK/Qt рантайм (библиотеки + fontconfig/icons/schemas/locale) должен
  паковаться целиком внутрь пакета приложения, не ожидать общесистемного
  `/usr`.
- ADR-008 уже установил рабочий, но трудоёмкий путь кросс-компиляции
  C-библиотек под `aarch64-unknown-linux-musl` (Debian source packages +
  meson/autotools патчи под static-only) для четырёх небольших библиотек
  (`libseat`, `eudev`, `mtdev`, `libinput`). GTK4 (glib, pango, cairo,
  gdk-pixbuf, harfbuzz, fontconfig, freetype, graphene, ...) и
  Qt6+Kirigami (Qt Base, Qt Declarative/QML, kirigami2, kcoreaddons,
  ki18n, ...) -- на порядки больше зависимостей, чем прецедент ADR-008
  покрывает; тот же "из исходников по прецеденту Debian" подход в том же
  масштабе -- открытый вопрос по трудозатратам, не подтверждённый факт.
  Alpine Linux уже собирает gtk3/gtk4/qt6/kirigami2 нативно под musl --
  это доказывает принципиальную musl-совместимость апстрима, но не
  отвечает, воспроизводим ли именно этот кросс-путь (zig cc +
  Debian source) на таком масштабе, и не является автоматической
  заменой без собственной оценки версий/ABI (тот же довод, которым
  ADR-008 уже отклонил "готовый Alpine sysroot" для куда меньшего
  случая).

## Scope

**Входит:**

- Change 1 (спайк + ADR): доказать/опровергнуть на R620 (без телефона),
  что минимальное GTK4-приложение и минимальное Qt6+Kirigami-приложение
  вообще кросс-компилируются и статически линкуются под
  `aarch64-unknown-linux-musl` в этом проекте -- через тот же
  zig-тулчейн, что уже используется, оценивая как путь "из исходников по
  прецеденту ADR-008", так и путь "взять musl-собранные бинарники/пакеты
  Alpine и переупаковать" как минимум одной кандидатной альтернативой,
  не предполагая заранее, какой выигрывает. Отдельно вопрос размера:
  статически слинкованный GTK4/Qt6+Kirigami рантайм на приложение --
  сколько мегабайт, укладывается ли в разумный бюджет `/data` (не
  `init_boot`'s жёсткие 8MB -- сторонние приложения живут на `/data`,
  бюджет другой, но не бесконечный). Результат спайка -- ADR-021 (или
  несколько отдельных ADR, если clipboard-через-policy и
  text-input/OSK-стратегия заслуживают раздельного решения, см. ниже) с
  конкретным решением и, если ответ "не воспроизводимо в разумные
  сроки/размер" -- явной пометкой, что S08 требует пересмотра scope, а
  не молчаливой попыткой продолжить.
- Change 2+ (гейтится результатом Change 1): недостающие Wayland-протоколы
  для выбранного минимального набора (`xdg-decoration` как minimum, если
  toolkit'ы не рисуют CSD сами -- нужно перепроверить: GTK4/Kirigami оба
  по умолчанию client-side, вероятно decoration protocol не нужен вообще);
  `zwp-text-input-v3`/`zwp-input-method-v2` в `saai-displayd` плюс
  экранная клавиатура в `saai-shell` (по аналогии с `drm-splash.c`'s
  touch-hittest подходом, но реализующая IME-протокол для сторонних
  клиентов -- не тот же код, что уже есть для собственного UI `saai-shell`);
  settings/theme portal через уже существующий portal-протокол (S07);
  clipboard через policy -- решение о том, перехватывает ли `saai-displayd`
  `wl_data_device_manager` на уровне compositor'а и сверяет с
  `saai-appd`'s grants (новая интеграция, `saai-displayd` сегодня вообще
  не знает о capability-системе), или GTK/Qt патчатся/оборачиваются, чтобы
  использовать S07's portal напрямую вместо нативного Wayland clipboard --
  это отдельное архитектурное решение, не предрешается этим паспортом;
  упаковка runtime в `/data` (per-app bundle, не общесистемный).
- Один демонстрационный GTK-app и один Qt/Kirigami-app, физически
  установленные, запущенные, переключённые и закрытые на Pixel 7 под
  полным sandbox.

**Не входит:**

- Полные KDE Plasma Mobile / GNOME Mobile сессии.
- XWayland / X11-приложения любого рода.
- Аппаратный (GPU/EGL/dmabuf) рендеринг для этих toolkit'ов -- явно
  software-only (`wl_shm`/cairo software / Qt software backend), чтобы не
  открывать заново `/dev/dri`-hidden решение ADR-020 ради двух тестовых
  приложений.
- Физическая (Bluetooth/USB) клавиатура -- ADR-012 остаётся в силе;
  text-input/OSK решает только touch-based сценарий.
- Произвольные сторонние приложения за пределами двух демонстрационных --
  тот же принцип, что уже был в S05's "пока только доверенные приложения".

## Change

1. **Готово (2026-09-10, ADR-021, без кодовых изменений -- host-only
   спайк на R620).** Кросс-компиляция/источник пакетов для GTK4 и
   Qt5+Kirigami2 под aarch64-musl подтверждена воспроизводимой через
   Alpine Linux'а готовые пакеты (не from-source пересборка ADR-008-стиля
   -- решение явно пересматриваемо, не окончательное на весь спринт).
   Размер измерен: `gtk4.0` -- 181 MiB закрытие зависимостей (152
   пакета, включает ненужные X11-fallback/CUPS/GStreamer -- не
   минимальная сборка). Найдено и подтверждено по апстримному исходнику:
   `wl_data_device_manager` -- обязательный, не факультативный global
   для любого GTK4-клиента (`_gdk_wayland_display_open()` требует его
   безусловно) -- меняет приоритет внутри Change 2 (см. ниже).
   Qt5/Kirigami2's ELF-бинарники подтверждены валидными с чистым графом
   зависимостей, но реальный QML-сценарий через `qmlscene-qt5` уперся в
   нерасследованную проблему Qt's `QFactoryLoader` -- отдельная, узкая
   задача, перенесена в Change 2. Clipboard-через-policy МЕХАНИЗМ и
   text-input/OSK-стратегия этим Change'м не решены -- остаются в
   Change 2/3.
2. Недостающие Wayland-протоколы в `saai-displayd`, гейтится Change 1 --
   минимум stub `wl_data_device_manager` (ADR-021) первым, до остального.
3. Text-input-v3/input-method-v2 + экранная клавиатура в `saai-shell` для
   сторонних клиентов.
4. Settings/theme portal поверх существующего portal-протокола (S07).
5. Clipboard через policy -- конкретная реализация выбранного в Change 1
   варианта.
6. Упаковка GTK/Qt runtime в `/data` под манифест приложения (per-app
   bundle).
7. Один GTK-демо и один Qt/Kirigami-демо приложение, физическая приёмка на
   устройстве: install→launch→touch→switch→remove, полный sandbox negative
   test (по аналогии с S07's `sandbox-probe`) специально для этих двух
   приложений.

## Test

- host: кросс-компиляция и статическая линковка обоих toolkit'ов
  проверяется на R620 без телефона (Change 1) -- воспроизводимый скрипт,
  как `build-cross-sysroot.sh`;
- host: headless-смоук (если toolkit это позволяет без реального
  compositor'а) для базовой проверки, что бинарник вообще стартует и не
  падает на инициализации до попытки подключения к Wayland;
- device: оба демо-приложения устанавливаются, рисуют кадр, принимают
  touch, масштабируются, переживают switch и штатное закрытие;
- negative/fault injection: `sandbox-probe`-style проверка для обоих
  демо -- системные разрешения не обходятся через toolkit API
  (в частности clipboard -- приложение без `clipboard.read`/`write` не
  может прочитать/записать через нативный GTK/Qt clipboard путь так же,
  как не может через S07's portal);
- device: намеренно повреждённый/зависший GTK/Qt клиент не валит
  `saai-displayd` (тот же инвариант, что S02 уже доказал для тестового
  клиента).

## Acceptance criteria

- GTK-приложение и Qt/Kirigami-приложение запускаются, масштабируются,
  получают touch, переживают switch и закрываются;
- системные разрешения (capability grants) нельзя обойти через
  toolkit-специфичный API -- clipboard через нативный GTK/Qt путь
  подчиняется тем же grants, что и S07's portal, не открывает
  параллельный небезопасный канал;
- размер и память каждого демо-приложения измерены и явно
  задокументированы (не "разумно", а конкретное число байт/МБ на диске и
  RSS в рантайме);
- намеренно повреждённый Wayland-клиент этих toolkit'ов отключается без
  падения `saai-displayd` -- тот же негативный сценарий, что уже
  доказан для `saai-demo-surface` в S02.

## Threat / privacy impact

Первый раз в системе появляется большой объём стороннего C/C++ кода
(GTK/Qt рантайм целиком) внутри ADR-020 sandbox -- существенно больше
поверхность атаки, чем у первых Rust-сервисов. Существующий mount/seccomp
sandbox должен сдерживать это так же, как любой другой процесс, без
исключений специально под toolkit -- clipboard-через-policy explicitly
проверяется негативным тестом именно потому, что это самый очевидный путь
случайно открыть канал в обход capability-модели (нативный Wayland
`wl_data_device_manager`, если реализован наивно, обходит S07's portal
целиком).

## Rollback

Ничего в уже работающем S05/S07 пути не меняется, пока GTK/Qt демо не
собраны и не проверены физически -- откат тривиален: не устанавливать и
не паковать эти два демо-приложения, `saai-displayd`'s protocol-additions
из Change 2/3 можно оставить смонтированными (новый advertised global,
которым никто не пользуется, не меняет поведение существующих клиентов)
либо убрать тем же cfg-gate паттерном, что уже использован для
`wl_keyboard` в ADR-012.

## Evidence

Заполняется по каждому Change только после зелёных host/device проверок.

Change 1: полный ход спайка и обоснование решений -- ADR-021. Кратко:
`qemu-user-static`+binfmt установлены на R620 (`sudo apt-get install
qemu-user-static`) -- aarch64 ELF исполняются на этом x86_64-хосте без
телефона. Alpine v3.20 `apk-tools-static` собрал реальный aarch64-musl
sysroot (`gtk4.0-demo` closure -- 153 пакета/196 MiB; отдельно голый
`gtk4.0` -- 152 пакета/181 MiB; полный набор с Qt5/Qt6/Kirigami2 -- 194
пакета/387 MiB). `gtk4-demo --version` выполнился под
`qemu-aarch64-static` (код 0). Реальный закоммиченный `saai-displayd`
(headless/host сборка, `cargo build -p saai-displayd`, без
`--features panther-hardware`) поднят на R620, слушает
`WAYLAND_DISPLAY=wayland-1`; `gtk4-demo` под qemu подключился к нему
(лог: `client connected`), `WAYLAND_DEBUG=1` подтвердил полный
registry-обмен всеми 8 текущими globals и `wl_shm_pool`/roundtrip. GTK4
тем не менее отказался открыть дисплей -- подтверждено по исходнику
GTK (`gdk/wayland/gdkdisplay-wayland.c`), что `wl_data_device_manager`
обязателен безусловно, не только `wl_compositor`/`wl_shm`/shell.
Qt5/Kirigami2's бинарники прошли `readelf -d` без недостающих SONAME;
`qmlscene-qt5` против тестовой Kirigami QML-сцены (`kirigami-spike.qml`)
уперся в нерасследованную ENOENT-проблему Qt's `QFactoryLoader`
(`faccessat` на каталог плагинов, тот же путь корректно листается
`busybox ls` под тем же `qemu-aarch64-static -L`) -- не архитектурный
блокер, узкая задача Change 2. Throwaway-артефакты спайка (`apk.static`,
sysroot, `kirigami-spike.qml`) не закоммичены, живут в `/tmp/alpine-spike/`
на R620, воспроизводимы по шагам ADR-021.
