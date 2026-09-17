# ADR-097: PCManFM-Qt рендерит и получает фокус на боевом compositor'е -- единственный настоящий блокер -- отсутствие session D-Bus

## Статус

Принято, 2026-09-17. APP-COMPAT-ROADMAP.md, спринт-кандидат APP-05.

## Контекст

APP-05 просил проверить PCManFM-Qt (Qt6 по исходной формулировке
ADR-095 -- на самом деле Qt5, см. ниже) как кандидата в файловый
менеджер. Собран через тот же рецепт, что build-kirigami-demo-
package.sh (Alpine aarch64, ADR-021/095) -- os/targets/panther/
build-pcmanfm-qt-package.sh, пакет org.saaios.demo.pcmanfm.

Первое расхождение с ADR-095: пакет pcmanfm-qt в Alpine v3.20 (та же
ветка, что уже использует рабочий Kirigami-пакет) -- это 1.4.1-r0,
собранный на Qt5 (libQt5Core/Gui/Widgets/X11Extras/DBus), не Qt6.
ADR-095, "2.4.0-r0" был про другую ветку Alpine, не проверялся живьём.
Это новая для проекта форма тулкита -- чистый QtWidgets (не
QtQuick/declarative, как весь Kirigami-путь), с реальной зависимостью
от libQt5DBus.so.5 и libgio-2.0/libglib-2.0/libgobject-2.0.

qt5-qtbase-x11, собственный platformthemes/libqgtk3.so тянет gtk+3.0
как жёсткую apk-зависимость, хотя ни pcmanfm-qt, ни libfm-qt не
используют GTK/cairo/pango напрямую (apk info -R подтверждает: только
Qt5/X11Extras/DBus/libc/glib/xcb/libexif/libmenu-cache). Файл
plugins/platformthemes/libqgtk3.so удалён из пакета -- он загружается
только если QT_QPA_PLATFORMTHEME называет gtk3, а saai-appd, spawn()
этого не делает.

## Проверка

Через saai-appd, реальный install/launch (unix-json-request, тот же
протокол, что APP-01) -- install прошёл, launch вернул pid, но
saai-displayd.log показал client connected pid=1557 и НИЧЕГО дальше
(ни new xdg_toplevel, ни commit) -- list через секунды показал state:
"stopped". Процесс исчезает почти мгновенно, без единой диагностической
строки (даже с QT_LOGGING_RULES=*.debug=true через ручной перезапуск) --
ключевой симптом, разобранный ниже.

Ручной перезапуск вне saai-appd, песочницы (тот же метод, что
ADR-025/026 -- unshare -m / mount --bind <package>/lib /lib,
воспроизводя sandbox.rs, reveal-механизм без риска для боевого стека)
изолировал причину пошагово:

1. --version/--help работают, платформенный Wayland-плагин грузится и
   подключается к реальному saai-displayd (qt.qpa.wayland: using input
   method: QComposeInputContext).
2. Обычный запуск (./bin/pcmanfm-qt -n, платформа minimal ИЛИ wayland)
   выходит с кодом 0 почти мгновенно, без единой строки в stderr, даже
   под полным *.debug=true -- никакого файла не создаётся в $HOME
   (пустая директория до и после). Не крэш (SIGSEGV/SIGABRT дали бы код
   139/134, не 0).
3. Тот же запуск с временно поднятым dbus-daemon --session (Alpine,
   dbus пакет, тот же apk-источник, только для этого теста -- не
   установлен постоянно нигде) и DBUS_SESSION_BUS_ADDRESS указывающим
   на него: приложение стартует полностью -- isPrimaryInstance, создаёт
   все виджеты главного окна (Go Up, Reload, Go, actionNewWin и т.п.,
   подтверждено логом accessibility/shortcut регистрации).
4. Вывод: без рабочего session D-Bus PCManFM-Qt интерпретирует
   отсутствие шины как "другой экземпляр уже запущен" (типичный паттерн
   D-Bus-based single-instance проверки) и тихо завершается через
   exit(0) до создания какого-либо окна -- не крэш, архитектурный гэп:
   на SaaiOS нет ни одного D-Bus daemon, ни session, ни system.

С поднятым dbus-daemon и QT_QPA_PLATFORM=wayland против РЕАЛЬНОГО
боевого saai-displayd (не тестового инстанса) -- полный успех,
подтверждённый с обеих сторон:
- Со стороны Qt (qt.qpa.*=true): Using the xdg-shell shell integration,
  Received xdg_toplevel.configure with QSize(1080, 2400) (реальный
  размер экрана Pixel 7), qt.qpa.wayland.backingstore: handleUpdate
  (SHM backing store, не EGL -- см. ниже).
- Со стороны saai-displayd.log: client connected pid=2872, new
  xdg_toplevel, focus set to Some(ObjectId(wl_surface@15[1], 25)), как
  минимум два разных реальных commit'а с уникальным frame
  sha256=77537a7885c38e5e318fd55b68503aeaa5383cc3a2cf8211e34644d4636e7ba2
  -- не пустой кадр, содержимое главного окна PCManFM-Qt реально прошло
  GPU-composite blit.

Побочно подтверждено (не новая находка, но первое прямое
переподтверждение для QtWidgets, а не только QtQuick): EGL
детерминированно отказывает (Failed to initialize EGL display 3001 --
swrast_dri.so/zink_dri.so физически отсутствуют, ADR-024) -- ADR-026
уже задокументировал это для QtQuick/Kirigami, здесь то же самое верно
и для чистого QtWidgets-приложения: Qt тихо продолжает на собственном
SHM backing store, никакого крэша. Опробована установка
mesa-dri-gallium (даёт swrast_dri.so через LLVMpipe) как способ закрыть
EGL полностью -- физически работает, но стоит дополнительно 332 MiB (в
основном libLLVM-17.so, 136 MiB) ради возможности, которая приложению и
не требовалась ни разу за всё тестирование -- не включено в финальный
пакет, чтобы не тащить недоказанно нужную зависимость (тот же принцип,
что вывод ADR-095: не заводить зависимость без названного случая
необходимости).

## Решение

1. org.saaios.demo.pcmanfm физически подтверждён как рендерящий,
   получающий фокус, коммитящий реальные кадры на боевом
   saai-displayd -- НО только при наличии working session D-Bus,
   которого saai-appd, sandbox.rs сейчас не предоставляет ни в каком
   виде (ни socket, ни env var, ни сам daemon как системный сервис).
   Пакет не оставлен установленным (тот же принцип, что ADR-027) --
   remove выполнен, тестовый dbus-daemon и все временные файлы на
   устройстве удалены.
2. Это НЕ считается закрытием APP-05 -- открыт новый, точно
   локализованный вопрос: нужен ли SaaiOS собственный минимальный
   session D-Bus daemon как системный сервис (аналог saai-displayd/
   saai-appd по масштабу решения, но с другими последствиями для
   модели безопасности -- общая шина потенциально открывает
   inter-app-канал связи, не покрытый существующей моделью
   capability, ей из services/saai-appd/src/lib.rs). Это отдельное
   архитектурное решение, требующее собственного обсуждения, не
   принимается неявно этим ADR.
3. webkit2gtk-4.1/qt6-qtwebengine (APP-06) и, вероятно, любой более
   сложный сторонний Linux-дистрибутив-пакет (файловые менеджеры,
   браузеры, LXQt/GNOME-экосистема) будут иметь ту же самую
   зависимость от D-Bus для похожих причин (single-instance,
   notifications, GVolumeMonitor/udisks2 и т.п.) -- APP-05, находка
   вероятно блокирует не только файловый менеджер, стоит решить один
   раз на уровне платформы, а не переоткрывать при каждом APP-0x.

## Последствия

- APP-05 остаётся Backlog, но с точным, физически подтверждённым
  блокером вместо гипотезы -- см. обновление
  docs/os/sprints/APP-COMPAT-ROADMAP.md.
- Новый явный вопрос для будущего Definition-of-Ready прохода: "session
  D-Bus как системный сервис SaaiOS" -- затрагивает
  services/saai-appd/src/sandbox.rs (новый bind-mount/socket в
  reveal-механизме) и модель capability, ей, не только упаковку
  приложения.
- os/targets/panther/build-pcmanfm-qt-package.sh и apps/pcmanfm-demo/
  остаются в дереве как рабочий, физически проверенный рецепт --
  переиспользуются без изменений, когда/если session D-Bus появится.

## Ссылки

- docs/os/sprints/APP-COMPAT-ROADMAP.md -- APP-05.
- ADR-095 -- musl/Alpine источник пакетов, тот же путь здесь.
- ADR-021, ADR-026, ADR-027 -- предыдущие Qt-физические проверки.
- ADR-024 -- почему EGL закономерно недоступен на этом железе.
- services/saai-appd/src/sandbox.rs -- текущая (неполная для этого
  случая) модель раскрытия файловой системы/сокетов приложению.
