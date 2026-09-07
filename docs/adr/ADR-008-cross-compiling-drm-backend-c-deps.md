# ADR-008: Кросс-компиляция C-зависимостей DRM-бэкенда для aarch64-musl

## Статус

Принято, 2026-09-07.

## Контекст

S03 включает в `saai-displayd` реальный DRM/KMS-бэкенд Smithay
(`backend_drm`, `backend_session_libseat`, `backend_udev`,
`backend_libinput` — фичи, которые ADR-007 сознательно отложил на этот
момент). Эти Smithay-фичи тянут Rust-обёртки (`drm`, `libseat`, `udev`,
`input`), три из которых (`libseat-sys`, `libudev-sys`, `input-sys`)
линкуются против настоящих системных C-библиотек через pkg-config:
`libseat`, `libudev`, `libinput` (которая сама зависит от `mtdev` и
`libevdev`). `drm`/`drm-ffi` — исключение: чистый Rust поверх
сгенерированных из заголовков ядра ioctl, без `libdrm.so` вообще.

На хосте (x86_64, glibc) все три библиотеки ставятся через apt.
Панther-таргет — `aarch64-unknown-linux-musl`, и pkg-config отказывается
кросс-компилировать без sysroot с `.pc`-файлами, заголовками и
библиотеками именно для этой цели (`pkg-config has not been configured
to support cross-compilation`) — такого sysroot никогда не существовало,
т.к. до сих пор кросс-компилировались только чистые Rust-крейты
(`saaios-runtime`, `saai-console`) и `tinyalsa` (уже собираемая из
исходников по прецеденту `patches/tinyalsa-period-write.patch`).

## Решение

Каждая из четырёх недостающих C-библиотек (`libseat`, `libudev`, `mtdev`,
`libevdev`) и их потребитель (`libinput`) собираются из исходников тем же
zig-кросс-тулчейном, что уже используется для Rust, в локальный sysroot
(`os/targets/panther/artifacts/toolchain/sysroot-aarch64-musl/`,
gitignored как и весь `artifacts/`). Источники — не upstream git (часть
недоступна анонимно, см. ниже), а **исходные пакеты Debian** (`apt-get
source`) там, где они существуют — тот же принцип версионирования и
патчей, что Debian уже применяет к нашей нативной x86_64-сборке.
Исключение — `libudev`: используется **eudev**
(github.com/eudev-project/eudev), systemd-независимый форк, а не
Debian-пакет `libudev1` (часть systemd целиком — несоразмерно тяжело
кросс-компилировать ради маленькой API-поверхности, которую реально
использует `backend_udev`).

Всё собирается **статически** (`.a`, не `.so`): цель `aarch64-unknown-
linux-musl` по умолчанию линкуется полностью статически, отдельные `.so`
не нужны. Апстримные `meson.build` `seatd` и `libinput` жёстко вызывают
`shared_library()` независимо от `--default-library` — оба патчатся на
`static_library()` (`patches/seatd-static-library.patch`,
`patches/libinput-static-library.patch`), иначе сборка `.so` падает на
несовместимой TLS-модели у объектов `libudev.a`
(`R_AARCH64_TLSLE_ADD_TPREL_HI12 ... cannot be used with -shared`) — эти
объекты в принципе не предназначались для попадания в динамическую
библиотеку.

Отдельная тонкость тулчейна: существующий `tools/zig-aarch64-musl.sh`
(обёртка для rustc) жёстко передаёт `-nostdlib`, что нужно только
потому, что rustc сам приносит свой musl-сисрут и линкует его напрямую —
но тот же флаг ломает поиск обычных заголовков (`<arpa/inet.h>` не
находился) для произвольной C-библиотеки через autotools/meson. Добавлен
отдельный `tools/zig-aarch64-musl-cc.sh` — тот же `zig cc -target
aarch64-linux-musl`, но без `-nostdlib`, специально для C-сборок.

Весь процесс воспроизводится одним скриптом,
`os/targets/panther/build-cross-sysroot.sh`, по аналогии с
`build-native-c-image.sh`/`build-wifi-vendor-boot.sh`. Он идемпотентен
(каждая библиотека пересобирается с нуля при повторном запуске) и
физически проверен дважды подряд с нуля на R620.

`saai-displayd`'s `Cargo.toml` получил cargo-фичу `panther-hardware`
(выключена по умолчанию), включающую все четыре DRM-related Smithay
backend-фичи разом — headless-тесты S02 продолжают собираться на обычном
CI-раннере без этих системных библиотек; кросс-сборка под панель требует
`--features panther-hardware` явно.

## Последствия

Положительные:

- воспроизводимый, задокументированный путь кросс-компиляции для любых
  будущих C-зависимостей, не только этих четырёх;
- S02's headless CI остаётся лёгким и hardware-независимым — новая
  инфраструктура не заражает его системными зависимостями;
- патчи маленькие (меняют ровно вызов `library()`), легко проверяются в
  ревью, не форкают апстрим целиком.

Цена решения:

- ещё один слой воспроизводимости для поддержки (Debian source packages
  могут менять версию/патчи при апдейте `apt`; `build-cross-sysroot.sh`
  их не пинит — если апстрим изменится существенно, патчи могут
  перестать применяться; фиксируется по мере необходимости, не заранее);
- eudev — не тот же код, что реальный systemd-udev на устройстве
  (устройство и не использует udevd вообще, per `native-init.c`'s
  `create_input_node()`, так что это не расхождение поведения, только
  выбор источника библиотеки);
- сборка sysroot добавляет несколько минут к CI при подключении (см.
  открытый вопрос ниже).

## Отклонённые альтернативы

### Дождаться готового aarch64-musl sysroot (например, из Alpine)

Быстрее в теории, но версии/ABI Alpine-пакетов не обязаны совпадать с
тем, что реально нужно этому Smithay/wayland-rs стеку; отладка
несовместимостей вслепую менее предсказуема, чем сборка из тех же
источников, что уже используются нативно.

### Отключить эти Smithay-фичи и написать DRM/evdev с нуля самим

В точности то, от чего ADR-007 уже отказался — переписывание
протестированной и поддерживаемой части Smithay без выигрыша в контроле
над продуктовой моделью.

## Открытый вопрос

CI-джоб, реально прогоняющий `build-cross-sysroot.sh` +
`--features panther-hardware` кросс-сборку на чистом раннере (S03's own
"host: кросс-компиляция ... зелёная в CI") — не подключён в этом ADR;
локально проверено дважды с нуля на R620 (Debian trixie), но раннер
`ubuntu-latest` в GitHub Actions — Ubuntu, не Debian, и `apt-get source`
там может тянуть другие версии/патчи тех же пакетов. Следующий шаг —
либо `container: debian:trixie` для этого job, либо явная проверка
Ubuntu-совместимости.

## Ссылки

- `os/targets/panther/build-cross-sysroot.sh`
- `os/targets/panther/patches/seatd-static-library.patch`
- `os/targets/panther/patches/libinput-static-library.patch`
- `os/targets/panther/tools/zig-aarch64-musl-cc.sh`
- `docs/adr/ADR-007-smithay-compositor-framework.md`
- `docs/os/sprints/S03-panther-displayd.md`
