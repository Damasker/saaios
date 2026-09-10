# ADR-021: GTK4/Qt-Kirigami совместимость -- источник пакетов и первая находка про clipboard

## Статус

Принято, 2026-09-10.

## Контекст

S08's Definition of Ready (`docs/os/sprints/S08-toolkit-compatibility.md`)
зафиксировала три нерешённых архитектурных вопроса и назначила Change 1
спайком, тестируемым на R620 без телефона, прежде чем Change 2+ можно
начинать. Этот ADR закрывает первый из трёх вопросов -- воспроизводимость
кросс-компиляции -- и попутно превращает второй (clipboard через policy)
из предположения в подтверждённый факт.

ADR-008 уже установил рабочий путь кросс-компиляции C-библиотек под
`aarch64-unknown-linux-musl` -- из исходников (`apt-get source` + патчи
meson/autotools под static-only), но для четырёх небольших библиотек
(`libseat`, `eudev`, `mtdev`, `libinput`). GTK4 (glib, pango, cairo,
gdk-pixbuf, harfbuzz, fontconfig, freetype, graphene, ...) и
Qt5/Qt6+Kirigami (Qt Base, Qt Declarative/QML, KDE Frameworks) -- на
порядки больше зависимостей; тот же подход в том же масштабе -- открытый
по трудозатратам вопрос, который этот спайк должен был закрыть до начала
Change 2.

## Спайк: что сделано

- Установлен `qemu-user-static` + `qemu-user-binfmt` на R620
  (`sudo apt-get install qemu-user-static`) -- aarch64 ELF-бинарники
  теперь исполняются на этом x86_64-хосте прозрачно, через binfmt_misc.
  Это не спайк-специфичный расходный материал, а новая постоянная
  возможность dev-окружения: любой будущий host-only тест aarch64-бинаря
  для S08 может полагаться на это же самое исполнение без телефона.
- Скачан `apk-tools-static` (Alpine's статический `apk`, x86_64,
  `dl-cdn.alpinelinux.org/.../x86_64/apk-tools-static-2.14.4-r1.apk`) --
  умеет ставить пакеты в произвольный `--root` под чужую `--arch` без
  Alpine-хоста и без chroot.
- Через него собран sysroot `aarch64` из официальных Alpine v3.20
  `main`/`community` репозиториев:
  - `gtk4.0-demo` (тянет `gtk4.0` 4.14.4-r0 + полное дерево зависимостей,
    153 пакета, 196 MiB) -- реальный, официально собранный Alpine GTK4
    demo-бинарь.
  - Отдельно, только `gtk4.0` без демки: 152 пакета, **181 MiB** --
    почти тот же объём, что и с демкой, потому что сам `gtk4.0` у
    Alpine уже тянет X11-fallback (`libX11`/`libXcursor`/`libXext`/...),
    `libcups` (печать) и GStreamer -- "полная" сборка, не минимальная.
  - `qt6-qtbase` + `qt6-qtwayland` + `qt6-qtdeclarative` + `kirigami2` --
    **`kirigami2` в этом репозитории тянет Qt5, не Qt6** (KF5-based
    Kirigami, не KF6) -- в индексе v3.20 нет `kf6-kirigami`/аналога под
    Qt6 вообще. Итоговый sysroot с обоими наборами (gtk4-demo closure +
    qt5/qt6/kirigami closure) -- 194 пакета, **387 MiB**.
- `qemu-aarch64-static -L sysroot sysroot/usr/bin/gtk4-demo --version`
  вернул `gtk4-demo 4.14.4`, код возврата 0 -- подтверждает: Alpine's
  aarch64-musl ELF-бинарники запускаются на этой инфраструктуре без
  модификации.
- Собран и запущен текущий закоммиченный `saai-displayd` в headless
  (host, не `--features panther-hardware`) режиме -- слушает
  `WAYLAND_DISPLAY=wayland-1` через `Display::new()`, без DRM/дисплея,
  ровно то, что нужно для host-теста без экрана.
- `gtk4-demo` под `qemu-aarch64-static` подключился к этому реальному
  `saai-displayd` (лог показал `client connected`). `WAYLAND_DEBUG=1`
  подтвердил полный обмен: 8 globals (`wl_compositor`,
  `wl_subcompositor`, `wl_shm`, `xdg_wm_base`, `wl_seat`, `wl_output`,
  `ext_session_lock_manager_v1`, `zwlr_layer_shell_v1`), `wl_shm_pool`
  создан, roundtrip (`wl_callback.done`) получен -- клиент реально
  разговаривает по протоколу с реальным compositor'ом этого проекта, не
  с заглушкой.
- **Находка**: GTK4 всё равно отказался открыть дисплей --
  `Gdk-WARNING: The Wayland compositor does not provide one or more of
  the required interfaces, not using Wayland display`. Проверено по
  апстримному исходнику (`gdk/wayland/gdkdisplay-wayland.c`,
  `_gdk_wayland_display_open()`): GTK4 **безусловно** требует
  `wl_compositor`, `wl_shm` И **`wl_data_device_manager`** одновременно
  (плюс один из `xdg_wm_base`/`zxdg_shell_v6`) -- не мягкая деградация,
  жёсткий отказ открыть дисплей вообще. `saai-displayd` сегодня не
  реализует `wl_data_device_manager` ни в каком виде.
- Qt5/Kirigami2's бинарники (`qmlscene-qt5` + `libqwayland-generic.so`)
  -- валидный aarch64-musl ELF, `readelf -d` показал полностью
  разрешимый граф зависимостей (все `NEEDED` -- стандартные SONAME без
  пропусков). Попытка запустить тестовую Kirigami QML-сцену через
  `qmlscene-qt5` уперлась в нерасследованную до конца проблему
  Qt's `QFactoryLoader`: `faccessat` на каталог плагинов-платформ
  возвращает `ENOENT` изнутри Qt-бинаря, хотя тот же путь под тем же
  `qemu-aarch64-static -L sysroot` корректно листается обычным
  `busybox ls` -- открытый, но узкий (не архитектурный) технический
  вопрос, зафиксирован как задача Change 2, не блокирует это решение.

## Решение

1. **Источник пакетов для GTK4/Qt-Kirigami -- готовые Alpine
   aarch64-musl сборки** (через `apk-tools-static` в project-local
   sysroot), не ADR-008-стиль пересборка из исходников с нуля -- по
   крайней мере для Change 2/3 (реализация недостающих протоколов) и
   демо-приложений. `apt-get source`-путь ADR-008 для GTK4/Qt в этом же
   масштабе -- отдельная, не блокирующая старт задача на будущее, если
   и когда минимальный размер production-бандла станет реальным
   узким местом, а не сейчас.
2. **`wl_data_device_manager` -- обязательная, не факультативная часть
   Change 2.** Это не только про политику доступа к clipboard --
   `saai-displayd` физически не сможет открыть ни один GTK4-клиент без
   этого global'а вообще. Изменяет приоритет внутри Change 2: минимум
   один stub-`wl_data_device_manager` (принимает `bind()`, не обязан
   сразу поддерживать реальный обмен данными) должен появиться раньше
   любого другого протокольного изменения, иначе демо-приложение GTK4
   не пройдёт даже `_gdk_wayland_display_open()`.
3. Сам механизм "clipboard через policy" (как именно `saai-displayd`
   или `saai-shell` сверяет запрос на выдачу/получение selection с
   `saai-appd`'s grants) этим ADR не решается -- отдельная задача
   Change 2/5, теперь основанная на подтверждённом факте, что канал
   обязателен, а не опционален.
4. `qemu-user-static`/binfmt остаётся в dev-окружении R620 как штатная
   host-testing возможность для остатка S08, не разовый спайк-мусор.
5. Text-input/OSK-стратегия (ADR-012's ограничение по `wl_keyboard`)
   этим ADR не решается -- этот спайк её не касался, остаётся открытой
   для отдельного решения до Change 3.
6. Проблема `QFactoryLoader`/Qt5-plugin-loading под qemu -- открытая
   задача Change 2, не переоткрывает вопрос "работает ли Qt/Kirigami
   вообще" (ELF/ABI-уровень уже подтверждён).

## Последствия

Положительные:

- вопрос "воспроизводима ли кросс-компиляция GTK4/Qt под этот проект"
  закрыт быстро (несколько часов, не дни/недели) и с реальным
  физическим подтверждением (настоящий Alpine-бинарь реально
  разговаривает с настоящим `saai-displayd` по протоколу), а не
  предположением;
- `wl_data_device_manager`'s обязательность найдена ДО, а не ПОСЛЕ
  того, как Change 2 начала бы реализовывать протоколы в произвольном
  порядке -- экономит итерацию;
- `qemu-user-static` -- многоразовая инфраструктура для всего
  оставшегося S08, не только этого ADR.

Цена решения:

- Alpine's `gtk4.0` -- "кухонная раковина" сборка (X11-fallback, CUPS,
  GStreamer) -- 181 MiB даже без демо-приложений; финальный
  production-размер per-app bundle для реального demo-app потребует
  либо смириться с этим объёмом на `/data`, либо отдельно инвестировать
  в custom minimal meson-сборку (ADR-008-масштаба, не сделано этим
  ADR);
- `kirigami2` в Alpine v3.20 -- KF5/Qt5, не KF6/Qt6; если продукту
  принципиально важен именно Qt6-Kirigami, потребуется либо более новый
  Alpine release (проверить отдельно), либо смириться с Qt5 для этого
  спринта;
- Qt5's QFactoryLoader-проблема остаётся нерасследованной до конца --
  не доказывает несовместимость, но и не доказывает полную
  работоспособность Qt-стороны так же чисто, как GTK4-сторона уже
  доказана.

## Отклонённые альтернативы

### Собрать GTK4/Qt из исходников сразу, по прецеденту ADR-008

Правильно с точки зрения контроля над итоговым размером и составом
(без X11/CUPS/GStreamer), но объём работы на порядки больше, чем у
четырёх маленьких C-библиотек ADR-008 уже закрыл -- для вопроса "вообще
ли это реалистично" Alpine's готовые пакеты дают тот же ответ на
порядки быстрее. Не отклонено навсегда -- явно вынесено как будущая,
отдельно обосновываемая задача, если минимальный размер станет реальным
блокером.

## Ссылки

- `docs/os/sprints/S08-toolkit-compatibility.md`
- `docs/adr/ADR-008-cross-compiling-drm-backend-c-deps.md`
- `docs/adr/ADR-012-skip-keyboard-on-panther.md`
- `services/saai-displayd/src/main.rs` (headless/host build, протокольный набор)
- Throwaway-спайк не закоммичен: `/tmp/alpine-spike/` на R620
  (`apk-tools-static`, извлечённый sysroot, `kirigami-spike.qml`) --
  воспроизводимо по шагам этого ADR, не входит в репозиторий.
