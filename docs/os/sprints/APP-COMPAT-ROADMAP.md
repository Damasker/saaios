# App Compatibility Roadmap: реальные Linux/Android приложения на SaaiOS

## Статус документа

Черновик roadmap, не сам S-трек (`docs/os/sprints/README.md`) --
отдельный от него, как и `HIA-ROADMAP.md` отдельн от основного трека
(тот же принцип: разный масштаб работы, свой Definition of Ready проход
перед тем, как строка отсюда становится настоящим спринтом в основном
трекере). Источник -- задача пользователя (2026-09-17): запуск готовых
Linux ARM64 и, в перспективе, Android-приложений на Pixel 7 под SaaiOS
без Android как основной ОС.

Это НЕ переформулировка исходного плана пользователя один в один.
Часть пунктов 1-4 исходного плана уже частично или полностью закрыта
предыдущими спринтами (S08 и последующими) -- ниже это явно
разделено на "уже известно" и "требует спайка", и порядок/приоритет
кандидатов скорректирован под реальные, физически проверенные факты
этого проекта, а не под общие рассуждения о том, какой тулкит обычно
проще.

## Конечная цель

Pixel 7 под SaaiOS реально запускает стороннее ПО, которое люди хотят
использовать -- настоящий браузер, файловый менеджер, экранную
клавиатуру для сторонних текстовых полей, и в перспективе настоящие
Android-приложения (включая требующие Google Play Services) -- без
того, чтобы Android когда-либо снова стал основной ОС устройства. Там,
где Android вообще используется, это ограниченный
compatibility-слой поверх `saai-appd`'s единой модели приложений или
явный fallback последней инстанции (VM), никогда не фундамент.

Не обходить Play Integrity/DRM ни для одного приложения -- совпадает с
уже действующей политикой проекта и с собственной формулировкой задачи
пользователя. Приложение, которому нужна полностью сертифицированная
среда, получает честный VM fallback, а не обход проверки.

## Как читать этот документ

Каждая строка ниже -- кандидат в спринт, не сам спринт. Перед тем как
войти в основной S-трек, каждая строка должна пройти Definition of
Ready (`docs/os/DEVELOPMENT_PROCESS.md`): одна цель, исходное
состояние, критерии приёмки с негативным сценарием, способ проверки
(host, где возможно), откат, риски, ADR или явная пометка "нового
архитектурного решения не требуется".

## Что уже физически известно -- не начинать с нуля

Собрано из уже закрытых спринтов и ADR, не предположений:

| Факт | Источник | Значение для этого плана |
|---|---|---|
| GTK4 (Alpine musl 4.14.4) детерминированно крашится на реальном железе при создании `wl_shm`-буфера (`gdk_wayland_display_create_shm_surface`, повреждённый `height`, вероятно из-за отсутствующего `wp-fractional-scale-v1`); причина найдена через `gdb` на устройстве, исправление не сделано | ADR-025 | Любое GTK4-приложение (Epiphany/WebKitGTK, Nautilus, wvkbd, если оно на GTK) физически не запустится, пока это не исправлено или не обойдено. Не начинать с GTK-варианта как с приоритета 1. |
| Qt5/QtQuick и Kirigami2 рендерят и коммитят реальный кадр на этом же железе, тем же software (`wl_shm`) путём, без дефекта GTK4 | ADR-026 | Qt -- доказанно рабочий тулкит здесь уже сегодня, не гипотеза. |
| Реальное Qt/Kirigami-приложение (`org.saaios.demo.kirigami`) прошло install→launch→stop→remove через настоящий `saai-appd` IPC на боевом устройстве | ADR-027 | Путь "стороннее приложение как обычный `saai-appd` пакет" уже физически доказан для Qt, не только теория. |
| Touch физически подтверждён на Qt/Kirigami-поверхность через синтетическую evdev-инъекцию (`uinput`, `unshare -m`), включая корректную маршрутизацию через lock-screen | ADR-028 | Полный цикл "коснулся экрана -- попало в стороннее Qt-приложение" уже проверен на железе. |
| Источник пакетов для сторонних библиотек -- Alpine Linux musl-сборки, не сборка из исходников под glibc | ADR-021 | Раздел 1 исходного плана ("glibc compatibility island") исходит из неверной предпосылки -- уже работающий, проверенный путь этого проекта -- musl, тот же libc, что и весь остальной SaaiOS. Глибц-остров создавал бы второй libc в системе без доказанной необходимости. |
| `wl_keyboard`/`libxkbcommon` физически невозможно поднять на этом железе -- SIGTRAP при компиляции ЛЮБОГО keymap, 7 независимых вариантов, включая полностью self-contained from-string keymap без единой файловой зависимости | ADR-012, ADR-022 | Это не "нет физической клавиатуры" в узком смысле -- это "весь keysym/keymap путь мёртв на этом устройстве", включая для ЛЮБОГО клиента, который получил бы `wl_keyboard` или ввод через `zwp_virtual_keyboard_v1` (получатель всё равно должен интерпретировать keysym через xkbcommon). Раздел 4 исходного плана (wvkbd/Squeekboard через virtual-keyboard-protocol) нужно перепроверить на этом основании, см. ниже. |
| `zwp_text_input_manager_v3` (клиентская часть) реализован и работает без единого обращения к `wl_keyboard`/xkbcommon (проверено чтением исходников smithay: трогает только `TextInputHandle`, не `Seat::get_keyboard()`) | ADR-022 | Путь для СТОРОННИХ (GTK/Qt) текстовых полей существует уже сегодня на клиентской стороне. |
| `zwp_input_method_manager_v2` (серверная часть, нужна ЛЮБОМУ OSK, включая портированный wvkbd/Squeekboard) сознательно НЕ включён -- smithay's собственный `GetInputMethod`-хендлер безусловно вызывает `seat.get_keyboard().unwrap()`, то есть требует рабочий keymap, которого здесь физически нет | ADR-022, комментарий `saai-displayd/src/main.rs` рядом с `_text_input_manager_state` | Это ключевое расхождение с разделом 4 исходного плана -- см. ниже, отдельный абзац. |
| Собственная (bespoke) hit-test клавиатура `saai-shell` (та же `saai-ui-core` `Node`/`layout()`/`hit_test()` система, что уже рисует tab-бар) физически проверена end-to-end через синтетическую evdev-инъекцию -- ввела и собрала строку `"HI!"` без единого обращения к xkbcommon/text-input/input-method | ADR-029 | Для ПЕРВОГО ЛИЦА (saai-shell) вопрос ввода текста уже решён и не требует Wayland OSK-протокола вообще. Для СТОРОННИХ приложений (GTK/Qt) этот трюк не работает напрямую -- они не сами решают, что означает нажатие, им нужен текст через протокол. |
| `saai-displayd` уже реализует (не нужно добавлять с нуля): `wl_compositor`, `wl_shm`, `linux-dmabuf` (ADR-024), `wl_seat` (touch only), `xdg_wm_base`, `ext-session-lock-v1`, `wlr-layer-shell`, `wl_data_device_manager`, `zwp_text_input_manager_v3` | текущий код `saai-displayd/src/main.rs`, `delegate_*!` макросы | Часть протокольной поверхности из раздела 4 исходного плана уже есть; недостающее -- конкретно `zwp_input_method_manager_v2` без keymap-зависимости (см. выше) и `zwp_virtual_keyboard_v1`, если решим всё-таки идти через injected-keysym путь, а не через text-input. |
| `saai-appd`'s manifest схема принимает ровно один тип UI ("wayland") -- любой Wayland-клиент, GTK/Qt/native/будущий Android-мост, ложится в существующую схему без её изменения | S08 Current state | Раздел 9 исходного плана ("единая модель приложений") в значительной части уже реализована как побочный эффект существующей архитектуры -- не отдельный спринт, а естественное следствие того, что ANDROID-02 сделает правильно. |
| ADR-020 sandbox (полный, 18-пунктный физически проверенный `sandbox-probe`) уже применяется к любому стороннему приложению без исключений для тулкита | S07, S08 | GTK/Qt/Android-приложение обязано быть самодостаточным под своим `code_dir` -- нет общесистемного `/usr`, fontconfig/icons/schemas/locale пакуются целиком внутрь пакета приложения. |

## Ключевые расхождения с исходным планом

1. **Раздел 1 (glibc island) -- вероятно не нужен.** ADR-021 уже
   выбрал и проверил Alpine musl как источник пакетов для сторонних
   тулкитов. Прежде чем строить второй libc в системе, нужен спайк,
   который явно называет библиотеку/приложение, которое ДЕЙСТВИТЕЛЬНО
   требует glibc и не собирается под musl (Alpine само собирает
   gtk4/qt6/kirigami2 под musl -- это не абстрактный аргумент).
2. **Разделы 2-3 (браузер, файловый менеджер) -- порядок приоритетов
   инвертирован.** GTK4-варианты (Epiphany/WebKitGTK, Nautilus)
   физически заблокированы известным, не исправленным дефектом
   (ADR-025). Пока GTK4 не починен или не обойдён, приоритет 1 должен
   быть у Qt-эквивалентов (PCManFM-Qt подтверждает это уже сам
   исходный план; для браузера это означает переоценку -- нужен
   отдельный спайк, есть ли жизнеспособный Qt-based браузер под ARM64
   musl, прежде чем ставить GTK-браузер в приоритет 1).
3. **Раздел 4 (экранная клавиатура) -- сложнее, чем "выбрать готовый
   проект".** wvkbd/Squeekboard решают клиентскую часть (рисование
   клавиатуры), но реальная проблема здесь -- серверная: у
   `saai-displayd` физически нет рабочего пути `zwp_input_method_
   manager_v2` без keymap (ADR-022), а smithay's готовая реализация
   этого протокола жёстко требует keymap. Это отдельный, не
   тривиальный спайк на стороне `saai-displayd`
   (патчить/переопределять `InputMethodManagerState` так, чтобы она
   не трогала `Seat::get_keyboard()`, либо писать свой минимальный
   `zwp_input_method_manager_v2` handler с нуля) -- ПЕРЕД тем, как
   вообще имеет смысл портировать любой конкретный OSK-проект.
   Собственный bespoke keyboard (ADR-029) для СТОРОННИХ приложений
   напрямую не переиспользуется -- та техника работает только потому,
   что `saai-shell` сам себе клиент и сам решает семантику нажатий.

## Фаза A -- Linux ARM64 приложения (высокая уверенность, строится на Done S08)

### APP-00: musl vs glibc -- закрыто

- **Статус**: Done, ADR-095, 2026-09-17.
- **Goal**: явное архитектурное решение (ADR), продолжаем ли на musl
  (Alpine-источник, как ADR-021) для всех будущих Linux-приложений,
  или называем конкретную зависимость, которая реально требует glibc.
- **Решение**: musl/Alpine для всех кандидатов дальнейшего плана --
  проверено наличие готовых aarch64-пакетов в Alpine (по определению
  musl) для Qt6, PCManFM-Qt, WebKitGTK (обе ветки, 6.0/GTK4 и
  4.1/GTK3), QtWebEngine, Firefox, Chromium, wvkbd -- ни для одного не
  найдена зависимость, требующая glibc. Отдельный glibc-остров не
  заводим.
- **Побочные находки** (см. ADR-095 целиком):
  - и `wvkbd`, и `squeekboard` напрямую зависят от `libxkbcommon` для
    работы с `zwp_virtual_keyboard_v1`/`zwp_input_method_v2` -- при
    уже доказанном (ADR-012/022) полном отказе xkbcommon на этом
    устройстве это делает оба проекта, вероятнее всего,
    неработоспособными здесь в их нынешнем виде без APP-03. Не выбор
    между ними, а подтверждение, что вопрос в принципе не в выборе
    конкретного OSK-проекта.
  - `webkit2gtk-4.1` (GTK3, не подвержен ADR-025) существует отдельно
    от `webkit2gtk-6.0` (GTK4, подвержен) -- потенциальный путь для
    APP-06, не обязанный ждать APP-02.

### APP-01: Qt hello-world под текущим `saai-appd` -- Done

- **Статус**: Done, 2026-09-17. Переподтверждено на текущем боевом
  образе, после всех изменений сессии (GPU dmabuf/batching/async fence
  pipeline, recompose-оптимизация, "Я"-прокрутка) -- ни одна из них не
  задела путь стороннего приложения.
- **Goal**: то, что просил исходный план как "APP-01", по факту уже
  доказано ADR-026/027/028 для Qt5/QtQuick/Kirigami2. Формальное
  действие здесь -- не новая разработка, а перепроверка/переиздание
  demo-пакета (`org.saaios.demo.kirigami`, ADR-027 указывает, что он
  был удалён после проверки, не оставлен установленным) и явная
  пометка в основном S-треке, что этот пункт закрыт, если задача
  требует именно "видеть работающее приложение", а не только ADR.
- **Проверено**: настоящий `saai-appd` IPC (`unix-json-request` --
  готовый инструмент, уже лежит на устройстве в
  `/data/saaios/system/`, тот же протокол, что `services/saai-appd/
  tests/ipc_lifecycle.rs` использует) -- `install` (пакет перенесён
  как 89 MiB `.tar.gz`, тот же размер, что ADR-027 зафиксировал),
  `launch` (реальный `qmlscene-qt5` процесс, `saai-displayd.log`
  показывает `client connected`, `new xdg_toplevel`, `focus set to
  Some(...)`, и настоящий закоммиченный кадр приложения, прошедший
  через GPU-composite blit), `list` (`state: "running"`, тот же pid
  после паузы), `stop` (процесс завершён, `saai-displayd`/`saai-shell`
  не задеты), `remove` (пакет снова отсутствует в `list`). Physical
  touch не переинъецирован в этот раз (ADR-028 уже закрыл этот вопрос
  отдельно) -- эта проверка нацелена на "не сломался ли рендер/
  compositing путь после переделки GPU pipeline", не на сам touch.
- **Приёмка**: `org.saaios.demo.kirigami` (или эквивалент) снова
  install→launch→touch→remove на текущем боевом образе (после всех
  изменений с S08) -- подтверждено, регрессий не найдено.

### APP-02: GTK4 -- спайк на разблокировку, не сразу приложение

- **Goal**: определить, чинить ли `wp-fractional-scale-v1` (или иначе
  устранить порчу `height`/`scale` в `gdk_wayland_display_create_shm_
  surface`) в `saai-displayd`, патчить GTK4, или сознательно отказаться
  от GTK4 в пользу Qt-only на этой цели.
- **Текущее состояние**: compositor half done on host (ADR-266:
  `wp-fractional-scale-v1` + `wp-viewporter`, `preferred_scale=120`).
  Host GDK-sized shm attach ADR-286. Native clipboard is deny-by-default
  (ADR-294) so x86 keyboard cannot open smithay's ungated path.
  Host GTK4 4.18 glibc cairo commits a hashed shm frame (ADR-305;
  `xdg_toplevel 1280x800`, `frame sha256=`). Panther displayd
  `02c78f9f…` now advertises fractional-scale + viewporter (ADR-309).
  Alpine 4.14.4 musl `gtk4-demo --run=dialog` still SIGSEGV after
  `preferred_scale(120)` (ADR-310): `create_buffer(508, 2337935)`.
  Remaining compositor half: smithay sent `configure_bounds(0,0)`;
  host now sends the window size (ADR-311). No displayd flash this week.
- **Приёмка**: либо GTK4-приложение реально рендерит кадр на железе
  тем же методом верификации, что ADR-026 использовал для Qt, либо ADR
  фиксирует осознанный отказ от GTK4 с обоснованием.
- **Зависит от**: ничего нового, довершение уже начатого в S08.

### APP-03: `zwp_input_method_manager_v2` без keymap -- спайк на стороне `saai-displayd`

- **Goal**: рабочий путь, которым любой OSK (включая будущий
  bespoke) может доставлять ТЕКСТ стороннему приложению через
  text-input-v3/input-method-v2, не касаясь `wl_keyboard`/xkbcommon.
- **Текущее состояние**: compositor half on host (ADR-267:
  `zwp_input_method_manager_v2` + owned text-input-v3, no
  `get_keyboard()`). `commit_string` reaches an enabled field in the
  host test. Panther displayd `02c78f9f…` advertises IME v2 (ADR-309).
  APP-04 is the visible OSK.
- **Приёмка**: тестовое стороннее Wayland-приложение (можно
  переиспользовать существующий demo) включает текстовое поле,
  получает `enter`, и текстовая строка, отправленная НЕ через
  wl_keyboard/xkbcommon, действительно появляется в поле.
- **Зависит от**: ничего из APP-00..02.

### APP-04: экранная клавиатура поверх APP-03

- **Статус**: Host protocol Done, 2026-09-21, ADR-270. Layer geometry
  host ADR-271. Layer blit dest host ADR-272. Shell IME layer host
  ADR-273. Panther displayd `02c78f9f…` advertises IME v2 (ADR-309);
  shell `4dc19018…` binds it. No Qt field was typed this slice.
- **Goal**: тап по текстовому полю стороннего Qt-приложения показывает
  клавиатуру, ввод долетает до приложения -- APP-KEYBOARD-01 из
  исходного плана, но через путь, который реально работает на этом
  железе.
- **Решение**: bespoke `Keyboard` (ADR-029/222), не wvkbd. Третьему
  лицу текст через APP-03 `commit_string` / `delete_surrounding_text`,
  отдельным IME-клиентом. `zwp_virtual_keyboard_v1` запрещён.
- **Приёмка**: host `hi!` from a second client. Visible panel on
  panther is not this slice.
- **Открытый вопрос**: закрыт — не портируем wvkbd.

### APP-05: файловый менеджер -- PCManFM-Qt, не Nautilus, до APP-02

- **Статус**: Done, 2026-09-17, ADR-098.
- **Goal**: APP-FILES-01 из исходного плана.
- **Приоритет**: PCManFM-Qt (не блокирован ADR-025) впереди Nautilus
  (GTK4, блокирован до APP-02).
- **Зависит от**: APP-00 -- закрыт (ADR-095). Уточнение: реальный
  пакет в Alpine v3.20 -- Qt5 (1.4.1-r0), не Qt6, как предполагал
  ADR-095's обзор другой ветки.
- **Проверено физически (ADR-097)**: `org.saaios.demo.pcmanfm` (сборка
  через `build-pcmanfm-qt-package.sh`, тот же метод, что Kirigami)
  реально подключается к боевому `saai-displayd`, создаёт
  `xdg_toplevel`, получает фокус и коммитит настоящие кадры главного
  окна -- подтверждено с обеих сторон (`saai-displayd.log` и Qt's
  собственный `qt.qpa.*` лог). EGL закономерно недоступен (ADR-024),
  Qt тихо откатывается на SHM backing store, как и предсказывал
  ADR-026 для QtQuick -- теперь подтверждено и для чистого QtWidgets.
- **Точный блокер для реального запуска через `saai-appd`**: PCManFM-Qt
  требует рабочий session D-Bus только для проверки "не запущен ли уже
  другой экземпляр" -- без него тихо завершается (`exit(0)`, без единой
  диагностической строки) до создания окна. У SaaiOS сейчас нет ни
  session, ни system D-Bus daemon вообще. Воспроизведено и обойдено
  вручную (временный `dbus-daemon` вне песочницы, не установлен
  постоянно) -- см. ADR-097 для полного протокола.
- **Решено (ADR-098)**: `saai-appd` теперь запускает отдельный
  приватный `dbus-daemon` на каждое приложение (никогда не общий, не
  новая capability -- см. ADR-098) и снимает файл сокета при `stop()`.
  Полный `install` -> `launch` -> `list` -> `stop` -> `remove` цикл
  подтверждён на боевом устройстве через настоящий `saai-appd`, без
  какого-либо ручного обхода песочницы -- `saai-displayd.log` показывает
  реальный `xdg_toplevel`/`focus`/committed-кадр с обеих сторон. Тот же
  механизм автоматически покрывает APP-06 и любой будущий сторонний
  Linux-порт с такой же зависимостью, без доработки для каждого
  отдельно.

### APP-06: браузер -- отдельный спайк на Qt-based кандидата, затем WebKitGTK/Firefox/Chromium

- **Статус**: Spike Done, 2026-09-21, ADR-269. Package tree ADR-281.
  Host qemu shm hello-frame ADR-306 (`libpxbackend`, no wayland-egl).
  Panther `appd` install/launch ADR-312 (chunked PUT, software
  Chromium flags). Hello-frame is hashed shm, not a URL.
- **Goal**: APP-BROWSER-01 из исходного плана.
- **Решение**: первый кандидат — Falkon (`qt6-qtwebengine` на Alpine
  v3.20 aarch64 musl). Angelfish тот же движок плюс Plasma QML.
  Epiphany — GTK4, ждёт APP-02. `webkit2gtk-4.1` есть как движок, без
  браузерного apk в v3.20. `build-falkon-package.sh` packs
  `QtWebEngineProcess` + `.pak`/`v8` snapshot + system ICU +
  `libpxbackend` (ADR-281/306). Panther appd hello-frame ADR-312.

## Фаза B -- Android compatibility island (низкая уверенность, требует отдельных спайков)

Ничего здесь не начинается до Фазы A хотя бы частично (APP-00..04) --
не потому что это архитектурно обязательно, а потому что Фаза A
проверяет и укрепляет ровно ту инфраструктуру (`saai-appd` package
model, sandbox, Wayland surface bridge), от которой Android-мост будет
зависеть напрямую.

### ANDROID-00: разведка существующих проектов -- не изобретать заново

- **Goal**: обзор существующих подходов "APK без полноценного Android
  guest OS" (архитектуры в духе Anbox/Waydroid: контейнер + Binder +
  минимальный набор system services + surface bridge в чужой
  compositor) -- что из их решений реально переносимо на архитектуру
  SaaiOS (ABI firewall bionic-процесс, тот же паттерн, что уже
  физически доказан для закрытого Mali UMD через `saai-gpu-compositor`
  этой же сессией).
- **Host-тестируемо**: да, это чтение чужого кода/документации.
- **Приёмка**: документ, называющий конкретный проект/технику как
  отправную точку, с честной оценкой лицензии и объёма адаптации.

### ANDROID-01: минимальный bionic+ART остров без Binder/system services

- **Goal**: воспроизвести архитектурный паттерн, уже проверенный этой
  сессией для Mali UMD (закрытый/сложный бинарный компонент живёт в
  отдельном bionic-процессе за ABI-границей, musl-процесс остаётся
  владельцем Wayland/DRM) -- но для ART, не для GPU-драйвера. Первая
  цель -- НЕ реальный APK, а "Hello World" managed-код запускается под
  ART на этом устройстве и рисует что-то через surface bridge в
  `saai-displayd`.
- **Зависит от**: ANDROID-00.

### ANDROID-02: запуск простого реального APK

- Соответствует исходному плану. Зависит от ANDROID-01.

## Фаза C -- Google Play / Play Services / VM fallback (самая высокая неопределённость)

Честно: для многих реальных приложений (Google login, Play Integrity,
DRM-зависимый контент) сертифицированная Android-среда может оказаться
практически недостижимой без полноценной, сертифицированной VM --
не как "ещё не сделали", а как вероятный постоянный предел этого
подхода. VM fallback (PLAY-03 в исходном плане) может оказаться
ОСНОВНЫМ путём для этого класса приложений, а не редким запасным
вариантом -- честно закладывать это ожидание заранее, не обещать
обратное.

### PLAY-01/02/03

Соответствуют исходному плану (Android VM/container + Play Store →
package broker → VM fallback для несовместимых приложений). Не
детализируются здесь дальше -- каждый требует собственного спайка
после того, как Фаза B даст первые физически проверенные факты, точно
так же, как GPU-спайк этой сессии определил реальный путь только после
физической проверки на железе, не до неё.

## Итоговый порядок

```text
APP-00  musl vs glibc -- ADR-095, Done
APP-01  Qt hello-world -- Done, переподтверждено 2026-09-17
APP-02  GTK4 -- host 4.18 frame ADR-305; bounds ADR-311; 4.14.4 qemu shm frame ADR-320; panther still 2337935 until next displayd flash (ADR-310)
APP-03  zwp_input_method_manager_v2 -- panther displayd ADR-309
APP-04  экранная клавиатура -- Keyboard→IME (ADR-270 host); GTK 4.18 Entry v3 enable (ADR-322) and OSK types hi! (ADR-325); Qt 5.15 v2 enable (ADR-323); IME forwarded, Filter not focused (ADR-324); host Ctrl+I disables v2 (ADR-326); PathEdit click re-enables v2, still no shm paint (ADR-327); host Qt 5.15 QLineEdit types hi! (ADR-328); packed PCManFM OSK commit+delete after click, still no shm (ADR-329); packed musl QLineEdit types hi! (ADR-330); Filter-band click same protocol-without-paint (ADR-331); host Ctrl+L disables v2 with no re-enable (ADR-332); packed Falkon URL click enables v2 and IME commit_string reaches it (ADR-333; not painted); packed Falkon URL OSK hi! reaches v2 (ADR-334; not painted); packed Falkon URL OSK still no new shm (ADR-335); packed musl Qt 6.6.3 QLineEdit types hi! (ADR-336); Falkon URL v2 disables before update_state settle (ADR-337); Falkon URL second click re-enables v2, still no shm (ADR-338); v2 IME commit deferred until after dispatch (ADR-339); Falkon URL OSK has no discard commit_string, silent focusObject null (ADR-340); packed PCManFM v2 OSK same silent focusObject null (ADR-341); Falkon URL KEY_A after second click still no shm (ADR-342); packed musl GTK 4.14.4 Entry types hi! (ADR-343); gtk4-demo --run=entry binds v3, center click does not enable (ADR-344); packed GTK 4.14 Entry types without grab_focus on keyboard seat (ADR-345); packed GTK 4.14 Entry types without wl_keyboard (ADR-346); packed Qt 6 QLineEdit binds v2 without wl_keyboard and does not enable (ADR-347); packed Qt 5 QLineEdit binds v2 without wl_keyboard and does not enable (ADR-348); host Qt 5 QLineEdit binds v2 without wl_keyboard and does not enable (ADR-349); host GTK 4.18 Entry types without wl_keyboard (ADR-350); host Qt 5 pointer click without wl_keyboard does not enable v2 (ADR-351); xdg Activated without wl_keyboard lets Qt QLineEdit type OSK (ADR-352); packed PCManFM enables v2 without wl_keyboard (ADR-353; not typed PathEdit); packed Falkon without wl_keyboard does not enable v2 (ADR-354); packed Falkon URL click 640 20 enables v2 without wl_keyboard (ADR-355; not typed LocationBar); packed gtk4-demo --run=entry click 640 400 binds v3 without wl_keyboard and does not enable (ADR-356); packed PCManFM v2 stays enabled 2 s without wl_keyboard (ADR-357; not typed PathEdit); packed Falkon URL v2 disables within 2 s after click without wl_keyboard (ADR-358; not typed LocationBar); host Qt5 QLineEdit without setFocus still types OSK after Activated (ADR-359); host Qt5 competing non-IM pane + tap 160 20 types OSK without wl_keyboard (ADR-360); host Qt5 80 ms steal-back after that tap disables v2 and OSK does not type (ADR-361; Falkon URL class); host Qt5 OSK in the enable window types hi! before that steal (ADR-362); packed PCManFM OSK without wl_keyboard hits FolderViewListView not Filter, shm 484823fc (ADR-363); packed PCManFM Filter click 400 760 without wl_keyboard stays FolderView (ADR-364); packed PCManFM PathEdit click 400 40 without wl_keyboard stays FolderView (ADR-365); packed musl Qt 6.6.3 competing pane tap 160 20 types OSK without wl_keyboard (ADR-366; not LocationBar); packed musl Qt 6.6.3 80 ms steal-back after that tap disables v2 and OSK does not type (ADR-367; Falkon URL class); packed musl Qt 6.6.3 OSK in the enable window types before that steal (ADR-368); packed musl Qt 5.15.10 competing pane tap 160 20 types OSK without wl_keyboard (ADR-369; not PathEdit); packed musl Qt 5.15.10 80 ms steal-back after that tap disables v2 and OSK does not type (ADR-370); packed musl Qt 5.15.10 OSK in the enable window types before that steal (ADR-371); packed musl GTK 4.14 competing pane auto-enables v3 and types OSK without a tap (ADR-372; not gtk4-demo); packed gtk4-demo --run=entry is not a --list name (ADR-373; ADR-344/356 clicked the demo browser); packed gtk4-demo --run=search_entry enables v3 without a tap (ADR-374; not typed); packed gtk4-demo --run=search_entry OSK leaves shm 233e0ee2 (ADR-375; protocol without paint); packed gtk4-demo search_entry OSK forwards v3 commit_string (ADR-376; GTK does not paint); search_entry has one v3 object, commit id equals enable (ADR-377); search_entry OSK grows v3 surrounding and still leaves shm 233e0ee2 (ADR-378; GTK applied, demo did not paint); search_entry OSK does not commit shm after OSK (ADR-379); search_entry click 640 40 then OSK surrounding hi! still no toplevel shm (ADR-380); search_entry OSK sends v3 cursor rectangle (ADR-381; mapped IM, still no toplevel commit); packed gtk4-demo --run=password_entry enables v3 without a tap (ADR-382; not typed); packed gtk4-demo password_entry OSK surrounding 9 bytes, shm 1eddcfe1, no toplevel commit (ADR-383); packed Falkon URL click 640 20 then immediate OSK forwards v2, shm 6cd11128 (ADR-384; not typed LocationBar); Falkon URL OSK grows v2 surrounding, still no toplevel shm (ADR-385; Qt applied, LocationBar did not paint); Falkon URL OSK extra shm is cursor not LocationBar (ADR-386); packed PCManFM FolderView OSK surrounding stays 0 (ADR-387; not Falkon URL apply-without-paint); host frame clock lets gtk4-demo search_entry/password_entry OSK commit shm (ADR-388; panther still VBlank-after-present); packed Falkon URL click flashes then disables before OSK (ADR-389; not typed LocationBar); panther idle frame clock when no flip pending (ADR-390; unflashed); packed PCManFM Filter click OSK grows surrounding (ADR-391; not panther); packed PCManFM PathEdit click selects path then disables (ADR-392; not typed PathEdit); packed PCManFM PathEdit disables without OSK (ADR-393; chrome, not OSK-induced); second xdg_toplevel does not steal Activated (ADR-394; QCompleter types; not panther); packed PCManFM PathEdit still disables after no-steal (ADR-395; not typed PathEdit); gtk4-demo entry_completion enables v3 without xdg_popup (ADR-396; not panther); gtk4-demo entry_completion OSK types without xdg_popup (ADR-397; not panther); packed Falkon URL still flash-disables after no-steal (ADR-398; not typed LocationBar); gtk4-demo combobox has no xdg_popup without a click (ADR-399; not panther); gtk4-demo combobox click 200 90 has no xdg_popup (ADR-400; not a Y sweep); packed GTK 4.14 popover maps configured xdg_popup (ADR-401; not panther); packed GTK 4.14 popover Entry types OSK hi! (ADR-402; not panther); host Qt 5.15 QMenu is a second toplevel not xdg_popup (ADR-403; not panther); packed GTK 4.14 ComboBox popup maps xdg_popup (ADR-404; not panther); packed GTK 4.14 ComboBox with_entry types OSK hi! (ADR-405; not panther); packed GTK 4.14 DropDown activate maps xdg_popup (ADR-406; not panther); panther field not typed
APP-05  PCManFM-Qt (файловый менеджер) -- ADR-098, Done
APP-06  браузер -- Falkon host qemu ADR-306; panther paints loopback HTTP hello.html ADR-321 (not NetInternet, not public browse)

ANDROID-00  разведка существующих Anbox/Waydroid-подобных подходов
ANDROID-01  минимальный bionic+ART остров (по образцу gpu-compositor ABI firewall)
ANDROID-02  первый реальный APK

PLAY-01  Android VM/container + Play Store
PLAY-02  package broker: Play Store → SaaiOS runtime
PLAY-03  VM fallback для несовместимых/Integrity-зависимых приложений
         (вероятно основной путь для этого класса, не редкий fallback)
```

Не пытаться пройти всё сразу. Каждый пункт закрывается по критериям
приёмки, физически проверенным на устройстве, не по объёму
написанного кода -- тот же принцип, что весь S00-S32 трек и HIA-
ROADMAP уже используют.
