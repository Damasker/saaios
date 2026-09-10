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
2. **Готово (2026-09-10, `a6d9804`).** `wl_data_device_manager` в
   `saai-displayd` -- через smithay's собственный `data_device` модуль
   (`DataDeviceState` + `ClientDndGrabHandler`/`ServerDndGrabHandler`/
   `SelectionHandler`/`DataDeviceHandler` с дефолтными no-op методами,
   `delegate_data_device!`), тот же паттерн, что уже у compositor/shm/
   seat/xdg_shell/session_lock/layer_shell. `set_data_device_focus()`
   вызывается из `activate_toplevel()` -- независимо от наличия
   клавиатуры (ADR-012), т.к. фокус clipboard/DnD следит за тем, какой
   клиент владеет выделением, не за тем, какой получает key events.
   Даёт реальный, рабочий client-to-client clipboard/DnD через
   встроенный smithay-брокеринг offer/fd между клиентами -- не
   протокольную заглушку -- но **намеренно не связано с S07's grants**:
   `saai-appd`'s sandbox физически не видит этот трафик (Wayland wire
   между двумя уже запущенными клиентами, вне mount/seccomp границы) --
   явно задокументировано в коде как временный, ограниченный текущим
   набором доверенных клиентов (`saai-shell` + demo-приложения) пробел,
   не закрытая задача; привязка к `Capability::ClipboardRead/Write` --
   отдельная будущая работа.
3. **Частично готово (2026-09-10, `5ded5dc`, ADR-022; input-method-v2/OSK
   отложены, не сделаны).** `zwp_text_input_manager_v3` в `saai-displayd`
   -- через smithay's `TextInputManagerState`, тот же паттерн, что и
   `wl_data_device_manager` (ADR-021): фокус ведётся из
   `activate_toplevel()`, независимо от клавиатуры. `zwp_input_method_
   manager_v2` (сторона `saai-shell` как экранной клавиатуры) **не
   реализован** -- физический спайк (7 вариантов компиляции keymap на
   реальном Pixel 7, каждый в отдельном forked-процессе) доказал, что
   smithay's `GetInputMethod`-обработчик безусловно требует рабочую
   клавиатуру (`seat.get_keyboard().unwrap()`), а на этом железе не
   компилируется НИ ОДИН keymap -- даже полностью самодостаточный,
   написанный вручную, без единого обращения к файлам. Это глубже, чем
   диагноз ADR-012 ("неполный xkb-data") -- вероятно баг в самой сборке
   `libxkbcommon.a`. Три пути вперёд (расследовать xkbcommon-баг,
   написать input-method-v2 вручную в обход smithay, патчить smithay) --
   ни один не выбран, каждый требует отдельного решения.
4. **Готово в переоцененном объёме (2026-09-10, `fa61b10`).** Не
   отдельный протокол/портал -- перед кодом физически проверено
   (реальный Alpine `gtk4-demo` под qemu против текущего
   `saai-displayd`), что GTK4 НЕ виснет и НЕ падает при полном
   отсутствии settings-портала/D-Bus (которого в этой ОС нет вообще) --
   открывает дисплей и рисует кадр, просто откатываясь на дефолты
   toolkit'а. Это не тот же класс блокера, что `wl_data_device_manager`
   (Change 2), так что полноценный портал/протокол не нужен для базовой
   работоспособности. Вместо этого `AppSupervisor::spawn()` (`saai-appd`)
   теперь передаёт механические env-переменные, которые GTK4/Qt уже
   сами читают: `QT_QPA_PLATFORM=wayland`, `GTK_A11Y=none`,
   `NO_AT_BRIDGE=1` (безусловно -- убирают реальный шум логов от
   неудачных попыток достучаться до a11y-bus/определить XCB на каждом
   запуске) и `FONTCONFIG_PATH`, если у приложения есть собственный
   `<code_dir>/etc/fonts` (ADR-021's self-contained per-app bundle).
   Сознательно НЕ выставляется конкретное имя темы (`GTK_THEME` и т.п.)
   -- у SaaiOS ещё нет настоящего источника настроек/темы (это задача
   S09/S10, не `saai-appd`), и выдумывать значение самому означало бы
   принимать продуктовое решение не по адресу; toolkit'ы используют
   собственные дефолты, пока не появится реальная система настроек --
   явная, не молчаливая граница scope.
5. **Заблокировано архитектурой smithay, задокументировано, не
   реализовано (2026-09-10, ADR-023).** Прочитан исходник smithay
   0.7.0's `wayland/selection/data_device/device.rs`/`offer.rs`: ни
   запись (`SetSelection`), ни чтение (`wl_data_offer.receive`) не дают
   точки перехвата, через которую compositor мог бы реально ОТКАЗАТЬ в
   операции против S07's grants -- `new_selection()` вызывается уже
   ПОСЛЕ применения selection (чистое уведомление, не гейт), а чтение
   обрабатывается через внутренний `ObjectData`, минующий
   `Dispatch`/handler-трейты совсем. Третий раз за спринт (после
   ADR-022) упирается в тот же класс стены -- API smithay не даёт
   нужной детализации для capability-модели SaaiOS. На `panther-hardware`
   запись уже случайно заблокирована (нет `wl_keyboard` -- ADR-012), но
   это хрупкая, не преднамеренная защита, не решение. Два пути вперёд
   (написать `wl_data_device_manager` вручную, или патчить smithay) --
   ни один не выбран, решается отдельно при появлении бюджета.
6. **Готово (2026-09-10, механизм подтверждён физически).** Упаковка
   GTK/Qt runtime в `/data` под манифест приложения (per-app bundle).
   Перед кодом подтверждено на реальном устройстве: `/lib` на этом
   rootfs содержит только `firmware/` и `modules/` -- ни `ld-musl-
   aarch64.so.1`, ни какого-либо `libc.so` нигде на устройстве нет
   (`find / -maxdepth 2 -iname 'ld-musl*' -o -iname 'libc.so*'` -- пусто).
   Именно поэтому все первосортные бинарники в проекте статические --
   динамически слинкованный Alpine-бинарник (ADR-021) физически не может
   выполниться без собственного loader'а. Добавлено новое поле
   `SandboxPaths::lib_dir: Option<PathBuf>` -- тот же mask-and-reveal
   паттерн, что уже применяется к `code_dir`/`data_dir` (S07): `/lib`
   безусловно маскируется в пустой tmpfs для каждого приложения, и
   раскрывается bind-mount'ом из `<code_dir>/lib` только если оно у
   приложения есть -- ни статические Rust-бинарники (у которых
   `lib_dir: None`), ни хост, ни другие приложения этот `/lib` не видят.
   `AppSupervisor::spawn()` дополнительно выставляет
   `LD_LIBRARY_PATH=/lib`, когда `lib_dir` присутствует -- не полагаясь
   молча на musl's дефолтный search path. Механизм физически
   воспроизведён напрямую (`unshare -m` + `mount --bind
   <package>/lib /lib`, тот же примитив, что `sandbox.rs`'s
   `reveal_directory` использует внутри) с реальным Alpine `gtk4-demo`
   (753 файла, `cp -L`-развёрнутый от символических ссылок, т.к.
   `store.rs`'s `copy_tree` отклоняет символьные ссылки как security
   boundary) -- бинарник запускается, линкуется, доходит до
   GLib/GTK-инициализации и до попытки подключения к Wayland. Реальный
   тестовый пакет `org.saaios.test.gtk4-lib-probe`, установленный и
   запущенный через полный `saai-appd`-путь (не в обход sandbox), тоже
   подтвердил, что линковка отрабатывает -- `crash_limited` наступил
   НЕ из-за `/lib`-механизма, а из-за двух отдельных, узких упаковочных
   пробелов, диагностированных через `Stdio::inherit()`-патч
   (`/run/saai-appd.log`) и прямое воспроизведение вне sandbox: (1)
   тестовый пакет не включал `/usr/share/X11/xkb` -- клиентский
   `libxkbcommon.so` (не тот же вендоренный статический крейт, что
   упёрся в ADR-022) падал в SIGSEGV сразу после `xkbcommon: ERROR:
   failed to add default include path`; подтверждено, что
   `XKB_CONFIG_ROOT=<bundle>/share/X11/xkb` целиком убирает эту ошибку и
   падение -- решаемо упаковочной конвенцией, не архитектурный блокер;
   (2) после этого GTK4 доходит до реального рендеринга и падает там же,
   где и должно быть ожидаемо для минимального Alpine-пакета без
   GPU-драйвера: `Ngl`/`GL`/`Vulkan`-рендереры все отказывают
   инициализироваться (нет Mesa/Mali-драйвера в пакете), откат на
   Cairo software-рендерер падает на `Unable to create Cairo image
   surface: invalid value ... for the size of the input` и завершается
   `Segmentation fault`. Обе находки -- предметная область Change 7
   (реальное упаковывание демо-приложения и его рендеринг-путь), не
   этого Change: цель Change 6 -- доказать, что sandbox МОЖЕТ пустить
   динамически слинкованный рантайм внутрь себя -- физически доказана.
   Throwaway-приложение удалено (`remove`), устройство подтверждено
   возвращённым к исходному состоянию (`list` -- только
   `org.saaios.demo-surface`).
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

Change 2: host -- `cargo build`/`cargo test -p saai-displayd` зелёные в
обеих конфигурациях (default headless и `--features panther-hardware`),
включая новый `data_device_global.rs` (спавнит настоящий бинарь
`saai-displayd`, подключается plain `wayland-client`, проверяет
`wl_data_device_manager` в списке globals). `fmt --check` и
`clippy --all-targets -D warnings` чисты в обеих конфигурациях.
`cargo test --workspace` -- 55 test-result блоков, все зелёные. Пересобран
ADR-008's cross-sysroot (`build-cross-sysroot.sh`, не существовал на этом
чекауте) и кросс-компилирован `saai-displayd` под aarch64-musl
(`build-saai-displayd.sh`) -- 563 KB, `ARM aarch64, statically linked,
stripped`.

Физически на устройстве (Pixel 7, настоящий DRM/touch backend, не
headless): бинарь развёрнут hot-swap'ом (`/saaios/saai-displayd`), хэш
сверен на каждом шаге; `native-init.c`'s supervision корректно
перезапустила и `saai-displayd`, и его дочерний `saai-shell` с верными
хэшами. Одноразовый throwaway aarch64-бинарь `registry-probe` (голый
`wayland-client`, собран тем же zig-кросс-тулчейном, что уже использует
`saai-shell`/`saai-demo-surface`, не закоммичен) подключился к настоящему
работающему compositor'у и подтвердил `wl_data_device_manager` среди
объявленных globals -- впервые не в headless/qemu-эмуляции (ADR-021), а
на реальном железе. `ext_session_lock_manager_v1`/`zwlr_layer_shell_v1`
корректно не появились для этого непривилегированного пробника --
существовавшая до этого изменения фильтрация `is_privileged_shell()`,
подтверждено чтением кода, не регрессия. Реальный
`org.saaios.demo-surface` запустился без регрессии под новой сборкой
compositor'а. Устройство возвращено к исходному состоянию.

Change 3: полный ход расследования и семь проверенных вариантов -- в
ADR-022. Host -- `cargo build`/`cargo test -p saai-displayd` зелёные в
обеих конфигурациях, включая новый `text_input_global.rs` (спавнит
настоящий `saai-displayd`, биндит `zwp_text_input_manager_v3`, вызывает
`enable()`+`commit()`, проверяет что roundtrip завершается и процесс жив
-- именно тот сценарий, что упал бы, будь диагноз ADR-022 неверным).
`fmt --check`/`clippy --all-targets -D warnings` чисты в обеих
конфигурациях. `cargo test --workspace` -- 56 test-result блоков, все
зелёные. Кросс-компилирован `saai-displayd` (579 KB).

Физически на устройстве: бинарь развёрнут hot-swap'ом, хэш сверен.
Throwaway aarch64-бинарь `text-input-probe` (plain `wayland-client` +
`wayland-protocols`, тот же zig-тулчейн) на реальном работающем
compositor'е забиндил `zwp_text_input_manager_v3`, вызвал
`enable()`+`commit()`, дождался roundtrip -- `RESULT: PASS`. Отдельно
подтверждено `ps`/`sha256sum /proc/<pid>/exe`, что `saai-displayd` не
перезапустился (тот же pid, тот же хэш) -- то есть не упал. Реальный
`org.saaios.demo-surface` запустился без регрессии. Устройство возвращено
к исходному состоянию.

Change 4: host -- `cargo test -p saai-appd` (34 unit + 2 integration
теста, включая обновлённый `child_receives_only_explicit_runtime_context`
и новый `fontconfig_path_set_only_when_app_bundles_its_own_fonts`)
зелёные; `fmt --check`/`clippy -p saai-appd`/`clippy --workspace
--all-targets -D warnings` чисты; `cargo test --workspace` -- 56
test-result блоков, все зелёные. Кросс-компилирован `saai-appd`.
Перед кодом на R620 через `qemu-aarch64-static` + настоящий Alpine
`gtk4-demo` против актуального `saai-displayd` (уже с data-device-manager
и text-input-v3 из Change 2/3) физически подтверждено: без единой
settings/theme/D-Bus переменной GTK4 всё равно открывает дисплей и
коммитит кадр (тот же sha256 кадра что и с новыми env-переменными
выставленными) -- отсутствие настроек не блокер, только повод для
плагинов рендеринга (EGL/Vulkan) ругаться на отсутствие GPU в qemu,
что ожидаемо и не относится к Change 4.

Физически на устройстве: `saai-appd` развёрнут hot-swap'ом, хэш сверен.
Реальный `org.saaios.demo-surface` (без собственных шрифтов) запущен;
`cat /proc/<pid>/environ` подтвердил `GTK_A11Y=none`, `NO_AT_BRIDGE=1`,
`QT_QPA_PLATFORM=wayland` в окружении настоящего sandboxed-процесса, и
корректное отсутствие `FONTCONFIG_PATH`. Устройство возвращено к
исходному состоянию.

Change 5: без кодовых изменений, кроме уточняющего doc-комментария в
`services/saai-displayd/src/main.rs` -- находка чисто по чтению
исходника smithay, полный разбор в ADR-023. `cargo build -p
saai-displayd`/`fmt --check` подтверждены чистыми после правки
комментария; физическая проверка не требуется, поведение compositor'а
не изменилось.

Change 6: host -- `cargo build/test/fmt --check/clippy -p saai-appd`
(clippy и в `-p saai-appd`, и в `--workspace --all-targets -D warnings`)
зелёные, включая обновлённый тест `non_root_is_fail_closed_unless_test_
override_is_explicit` (новое поле `SandboxPaths::lib_dir`). `cargo test
--workspace` зелёный целиком. Кросс-компилирован `saai-appd`
(`00b1a4b1...bee7b0d90`).

На R620 собран throwaway тестовый пакет `org.saaios.test.gtk4-lib-probe`
(реальный `gtk4-demo` из Alpine sysroot ADR-021 + весь его `lib/`-closure,
753 файла) -- первая попытка (`cp -a`, сохраняет symlinks) отклонена
`store.rs`'s `install()` как небезопасный тип файла (ожидаемое, уже
протестированное поведение `copy_tree`); пересобран `cp -L`
(разыменованные копии), установлен успешно.

Физически на устройстве: диагностическая сборка `saai-appd`
(`Stdio::inherit()` вместо `Stdio::null()` в `supervisor.rs`, временный
патч, backup сохранён, отменён и перепроверен зелёным host-прогоном
после диагностики) развёрнута hot-swap'ом, хэш сверен. Запуск тестового
пакета через `saai-appd`'s реальный IPC (`launch`) дошёл до
`crash_limited` (3 попытки, S05's crash-loop budget) -- `/run/saai-appd.
log` (перенаправленный туда `native-init.c`'s stdout/stderr механизм)
показал, что дочерний процесс реально стартует, линкуется и доходит до
`GLib`/`GTK`-инициализации, а не падает на exec() -- т.е. `/lib`-reveal +
`LD_LIBRARY_PATH` механизм работает. Точная причина падения выделена
прямым воспроизведением вне `saai-appd` (`unshare -m` + `mount --bind
<bundle>/lib /lib`, тот же примитив, что `sandbox.rs`'s
`reveal_directory` использует) с явными env-переменными: без
`/usr/share/X11/xkb` -- `Segmentation fault` сразу после `xkbcommon:
ERROR: failed to add default include path`; после добавления в пакет
скачанного с R620 `/usr/share/X11/xkb` (3.9 MiB из того же Alpine
sysroot) и `XKB_CONFIG_ROOT`, указывающего на него -- эта ошибка и
падение исчезают полностью, процесс доходит до попытки инициализации
`Ngl`/`GL`/`Vulkan`-рендереров, все три отказывают (нет GPU-драйвера в
минимальном пакете), откат на Cairo software-путь падает на `Unable to
create Cairo image surface: invalid value ... for the size of the input`
и `Segmentation fault` там же. Оба пробела (xkb-данные, GPU-рендеринг)
задокументированы как предмет Change 7, не этого Change. Throwaway-пакет
удалён (`remove`), `list` подтвердил возврат к исходному состоянию
(только `org.saaios.demo-surface`), scratch-файлы на устройстве очищены,
диагностический патч `supervisor.rs` отменён и перепроверен (`git diff`
против backup -- пусто), production-сборка (тот же хэш
`00b1a4b1...bee7b0d90`) развёрнута hot-swap'ом и подтверждена запущенной
(`sha256sum /proc/<pid>/exe`).
