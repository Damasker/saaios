# ADR-026: Qt5/QtQuick рендерит корректно на реальном железе там, где GTK4 падает -- Qt становится основным toolkit'ом Change 7

## Статус

Принято, 2026-09-10.

## Контекст

ADR-025 детально расследовал и локализовал в апстриме (не в коде
проекта) детерминированный крэш GTK4/GDK при создании wl_shm-буфера на
реальном Tensor G2 -- воспроизводится на любом compositor'е, любом
содержимом окна, не лечится `PIXMAN_DISABLE`, не про memfd/ядро.
Дальнейшая локализация требует `gdb`/`rr` на устройстве или бисекции
версий GTK4/Cairo -- вне объёма расследования. Открытым остался вопрос:
падает ли так же Qt/Kirigami (другой rendering-стек, не Cairo), или
GTK4-специфичный дефект.

## Находка

Тем же методом, что ADR-025 (throwaway headless-без-клавиатуры
`saai-displayd` + `unshare -m`/`mount --bind` воспроизведение
`sandbox.rs`'s reveal-механизма, без риска для боевого стека),
физически на реальном Pixel 7 проверен `qmlscene-qt5` (Alpine's Qt5
5.15.10, `QtQuick` минимальный QML-файл -- один `Rectangle`, без
Kirigami, чтобы исключить ADR-021's отдельную, узкую находку про
`QFactoryLoader`):

- Первая попытка не хватало `QT_PLUGIN_PATH` -- "Could not find the Qt
  platform plugin wayland". Добавлен `usr/lib/qt5/plugins/platforms/`
  (1.7 MiB) -- дошли до "No shell integration named xdg-shell found":
  недостающий `usr/lib/qt5/plugins/wayland-shell-integration/
  libxdg-shell.so`. Добавлен весь `plugins/`-каталог (6.9 MiB суммарно)
  -- оба пробела устранены, оба узко про упаковку Qt-приложения
  (аналог `usr/share/X11/xkb`-находки Change 6), не архитектурные.
- С полным набором плагинов и `XKB_CONFIG_ROOT`: EGL закономерно
  отказывает (`Failed to initialize EGL display` -- нет GPU-драйвера,
  ADR-024), но Qt **не падает** -- тихо откатывается на свой
  собственный software/shm rendering-путь (не через Cairo, у Qt
  отдельная от GTK's GDK реализация Wayland-backend'а) и **успешно
  коммитит реальный кадр** (`saai-displayd`'s лог: `new xdg_toplevel`,
  `commit ... frame sha256=761a9e67...`). Воспроизведено дважды подряд
  -- идентичный хэш кадра оба раза, не случайность.
- Повторено против РЕАЛЬНОГО боевого `saai-displayd` (pid 612,
  `panther-hardware`, тот самый процесс, против которого крашился GTK4
  в Change 6) -- тот же результат: рендерится, коммитит кадр, никакого
  крэша. `saai-displayd` пережил тест без единого перезапуска (тот же
  pid, тот же хэш `/proc/612/exe` до и после).

## Решение

Change 7 переключает основной toolkit-таргет с GTK4 на Qt/Kirigami --
GTK4 остаётся физически заблокированным (ADR-025) и не является целью
для физической приёмки этого спринта, пока кто-то не разберётся с его
дефектом отдельно (не блокирует Change 7). Основная демо-приложение
Change 7 собирается на Qt5/QtQuick (Kirigami2 -- поверх того же QtQuick
рендеринг-пути, значит ожидаемо не унаследует этот конкретный крэш, но
это не проверено этим ADR -- Kirigami2 сам по себе не тестировался
здесь, только голый QtQuick через `qmlscene`; проверка Kirigami2
конкретно -- часть основной работы Change 7).

Упаковочная конвенция для Change 7's Qt-приложений должна включать,
сверх `bin/`+`lib/`+`share/X11/xkb` (уже установлено Change 6):
`usr/lib/qt5/plugins/` целиком (платформенный плагин, shell-интеграция,
generic-плагины) и `usr/lib/qt5/qml/<используемые модули>` (для
QtQuick/Kirigami-приложений, работающих через QML, не для чистых
Qt Widgets).

## Последствия

- Change 7's acceptance criteria (install→launch→touch→switch→remove)
  нацелены на Qt-based демо, не GTK4-based -- нужно обновить исходный
  план Change 7 (изначально предполагал "один GTK-демо и один Qt/
  Kirigami-демо" -- GTK-часть теперь явно blocked, не отменена).
- `mesa-dri-gallium`/llvmpipe (ADR-024, уже решено не паковать) тем
  более не нужен для Qt -- Qt's собственный software-путь работает без
  него.
- Throwaway-диагностическая правка `saai-displayd/src/main.rs`
  (headless-без-клавиатуры, повторно использована из ADR-025's backup)
  снова НЕ закоммичена -- подтверждено чистым `git status` после теста.

## Ссылки

- ADR-025 -- GTK4's крэш, метод расследования, откуда взят
  headless-no-keyboard `saai-displayd` для этого теста.
- ADR-021 -- источник Alpine-пакетов, включая Qt5.
- ADR-024 -- GPU-driver spike, почему EGL закономерно недоступен.
