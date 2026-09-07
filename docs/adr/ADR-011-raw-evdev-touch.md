# ADR-011: Raw evdev вместо libinput для touch

## Статус

Принято, 2026-09-08.

## Контекст

S03 Scope (Change шаг 5) изначально называл `backend_libinput` явно —
естественный первый выбор, тот же путь, что и `backend_drm` до
[ADR-010](../adr/ADR-010-drop-libseat.md): взять готовый Smithay backend,
открыть устройство напрямую по пути, без сессии.

Физический тест на Pixel 7 показал: `Libinput::new_from_path()` +
`path_add_device("/dev/input/touchscreen")` падает с

```
libinput error: libinput bug: udev device never initialized (/dev/input/touchscreen)
libinput error: client bug: Invalid path /dev/input/touchscreen
```

`libinput_path_add_device()` внутри оборачивает путь в `udev_device` и
проверяет `udev_device_get_is_initialized()` — это становится `true`
только после того, как **работающий `udevd`** обработает uevent устройства
и запишет его свойства в `/run/udev/data/`. На этой системе `udevd` не
запускается вообще: `os/targets/panther/build-cross-sysroot.sh` (ADR-008)
собирает только `libudev` (через eudev) как библиотеку, самого демона нет
ни в sysroot'е, ни в `native-init.c`. Даже "path mode" у libinput — не
настоящий обход udev, только обход *seat-арбитража*; сама библиотека
udev ему всё равно нужна для метаданных устройства.

Добавить `udevd` как временный/постоянный процесс означало бы новую
инфраструктуру демона — прямо противоречит уже принятому дизайну: ADR-009
(один процесс владеет UI-слотом, никакого арбитра) и ADR-010 (libseat
убран по той же причине — эта система always-root, single-owner, без
session/seat-менеджеров в принципе).

`drm-splash.c` уже читает тот же `/dev/input/touchscreen` напрямую, сырыми
`struct input_event`, без libudev/libinput вообще — и это работает в
продакшене прямо сейчас. Более того, его собственная "калибровка"
(`design_x = x * 1080 / width` с `width` уже равным 1080) оказалась
тождественным преобразованием: этот тачскрин уже репортит координаты в
нативном пространстве панели (1080×2400), масштабировать нечего.

## Решение

`saai-displayd` читает `/dev/input/touchscreen` как сырой evdev-поток
(`services/saai-displayd/src/touch.rs`): `File::open` напрямую (root,
без прав/сессии — тот же принцип, что и ADR-010 для DRM), парсит
`struct input_event` (24 байта на 64-бит цели) вручную, отслеживает
`ABS_MT_TRACKING_ID`/`ABS_MT_POSITION_X`/`ABS_MT_POSITION_Y` по кадрам
`SYN_REPORT`, без масштабирования координат (те же значения, что и
`drm-splash.c` передаёт напрямую). Синтезирует down/motion/up в
Smithay `TouchHandle` API — протокольная часть (`wl_touch` и т.д.)
не меняется, меняется только источник событий.

`smithay/backend_libinput` и `smithay/backend_udev` убраны из
cargo-фичи `panther-hardware` — они больше не нужны никаким кодом в этом
крейте (DRM тоже открывается напрямую с ADR-010, `backend_udev` был
нужен только для `UdevBackend`, которым мы не пользуемся). Один
побочный эффект: библиотека `libxkbcommon` линкуется через голый
`#[link(name = "xkbcommon")]` без своего build.rs (`xkbcommon` крейт
версии 0.8.0 не имеет `build.rs` вовсе) — раньше путь до неё в
sysroot'е случайно находился благодаря `-L`, который добавлял pkg-config
при пробе `libinput` в build.rs, написанном для этой (впоследствии
отброшенной) libinput-попытки. Теперь этот build.rs оставлен, но
упрощён: он просто добавляет `$PKG_CONFIG_SYSROOT_DIR/usr/local/lib` как
`rustc-link-search`, не завязываясь на пробу конкретной библиотеки —
покрывает `xkbcommon` и любую будущую голую `#[link(...)]`-зависимость
из того же sysroot'а.

## Последствия

Положительные:

- одним классом зависимостей меньше в рантайме (`libinput`, `libudev`,
  `mtdev`, `libevdev` больше не линкуются в `saai-displayd` вообще — было
  вытянуто в Change 5's первой попытке, теперь снова не нужно);
- логика идентична уже проверенной в продакшене (`drm-splash.c`) — не
  новый, непроверенный путь чтения тачскрина, а тот же самый, просто на
  Rust;
- никакой daemon-инфраструктуры (`udevd`) не потребовалось — согласуется
  с ADR-009/ADR-010.

Цена решения:

- если когда-нибудь понадобится реальный multi-device input arbitration
  (несколько тачскринов/мышей одновременно, hotplug через udev) — этот
  путь придётся заменить на настоящий libinput+udevd; ADR-008's
  cross-compile инфраструктура для `libinput`/`libudev`/`mtdev`/`libevdev`
  остаётся в sysroot'е неиспользуемой, как и `libseat` после ADR-010 —
  задокументированный, воспроизводимый, но сейчас мёртвый код;
- ручной парсинг `struct input_event` не проверяет протокол так строго,
  как libinput (нет quirks-базы, нет фильтрации дребезга) — не проблема
  для одного, уже известного устройства этого конкретного телефона.

## Ссылки

- `docs/adr/ADR-008-cross-compiling-drm-backend-c-deps.md`
- `docs/adr/ADR-009-ui-supervision-and-drm-handoff.md`
- `docs/adr/ADR-010-drop-libseat.md`
- `services/saai-displayd/src/touch.rs`
- `os/targets/panther/src/drm-splash.c` (тот же путь чтения touchscreen)
