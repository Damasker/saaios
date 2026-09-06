# ADR-007: Smithay как основа `saai-displayd`

## Статус

Принято, 2026-09-07.

## Контекст

S02 требует короткий измеримый spike для выбора framework перед тем, как
писать `saai-displayd` и `saai-demo-surface` (ADR-005 зафиксировал, что
компоненты Wayland-платформы собственные, но не определил, на чём именно
реализован compositor). Критерии сравнения из паспорта S02: размер
dependency graph, время сборки и сложность cross-compile.

Сравнивались два пути:

- [Smithay](https://smithay.github.io/) 0.7.0 — библиотека для написания
  Wayland-компоситоров поверх `wayland-server`/`wayland-backend`, с
  опциональными backend-модулями (DRM, libinput, udev, X11, winit, Vulkan) и
  готовыми абстракциями `desktop` (surface/window lifecycle, xdg-shell).
- Прямое использование `wayland-server` 0.31 — низкоуровневый крейт
  wayland-rs, без abstractions поверх протокола: xdg-shell, конфигурацию
  toplevel и focus пришлось бы реализовывать вручную.

## Spike

Оба варианта собраны как одноразовые крейты в `spikes/` (вне основного
workspace, `[workspace]` в собственном `Cargo.toml`, не участвуют в CI),
создающие `wayland_server::Display` — минимальная проверка, что crate
собирается и линкуется на host (x86_64-unknown-linux-gnu).

Smithay по умолчанию включает `backend_drm`, `backend_gbm`,
`backend_libinput`, `backend_udev`, `backend_session_libseat`, `backend_x11`,
`backend_winit`, `backend_vulkan`, `xwayland` — весь этот набор требует
системных библиотек (udev, libseat, X11, GBM) и pkg-config, что заметно
больше, чем нужно для headless host-цели S02 (`DRM/KMS, evdev... не входят`).
С `default-features = false, features = ["wayland_frontend", "desktop"]`
единственная системная зависимость — `libxkbcommon` (нужна и в минимальном
варианте на wayland-server для будущей клавиатуры/раскладок, S02 wl_seat).

## Результаты

| Метрика | Smithay (headless features) | Прямой wayland-server |
|---|---|---|
| Зависимостей в Cargo.lock | 82 | 23 |
| Чистая сборка (release) | ~29s | ~10.5s |
| Размер `target/` | 284MB | 60MB |
| Системные библиотеки при headless-конфигурации | `libxkbcommon` | `libxkbcommon` |

При отключенных default-features системные зависимости у обоих вариантов
совпадают — разница целиком в чистом Rust-коде: `desktop`-abstractions
(surface/window lifecycle, xdg-shell handling), `calloop` event loop,
геометрия (`cgmath`), протокольные extensions (`wayland-protocols-wlr`,
`wayland-protocols-misc`).

Cross-compile для S03 (aarch64-unknown-linux-musl, тот же toolchain, что уже
использует `os/targets/panther` через zig) не проверялся отдельным spike,
так как S02 сознательно host-only. Но обе библиотеки используют одни и те же
базовые крейты (`wayland-server`, `wayland-backend`, `wayland-scanner`) без
собственного C-кода помимо системного `libxkbcommon` — сложность
cross-compile для этой части не должна отличаться. Backend-модули Smithay
(`backend_drm`, `backend_libinput`, `backend_udev`, `backend_session_libseat`),
нужные для S03, потребуют системных библиотек на самой сборке под aarch64,
но это тот же класс работы, что и ручная реализация DRM/evdev с нуля при
минимальном варианте — только уже написанная и протестированная в Smithay.

## Решение

`saai-displayd` строится на Smithay 0.7.0 с `default-features = false` и
explicit feature list, начиная с `["wayland_frontend", "desktop"]` для S02.
Backend-фичи (`backend_drm`, `backend_libinput`, `backend_udev`,
`backend_session_libseat`) включаются только в S03, когда потребуется
реальный DRM/KMS и evdev вместо headless-теста.

Причина: за умеренную цену (3.5x больше Rust-зависимостей, +18s чистой
сборки — незначительно в абсолютных числах) Smithay даёт готовый и
протестированный протокольный слой (surface/window lifecycle, xdg-shell
configure/ack/commit, seat/focus routing) и прямой путь к DRM/libinput
backend'ам S03 на той же кодовой базе. Реализация этого вручную поверх
`wayland-server` означала бы переписывать существенную часть Smithay же —
без выигрыша в контроле над продуктовой моделью, которую ADR-005 и так
защищает на уровне `saai-shell`/`saai-appd`, а не на уровне протокольного
цикла xdg-shell.

## Последствия

Положительные:

- меньше протокольного кода писать и тестировать вручную для S02
  (configure/ack/commit, invalid buffer/role, focus routing уже покрыты
  Smithay);
- S03 (DRM/KMS на Pixel 7) переиспользует тот же compositor core, включая
  недостающие backend-фичи, а не создаёт вторую кодовую базу;
- headless-конфигурация (`default-features = false`) не тянет системные
  зависимости сверх уже необходимого `libxkbcommon`.

Цена решения:

- дополнительный Rust dependency graph (82 против 23 крейтов) и связанное
  время компиляции CI;
- команда обязана понимать API Smithay (state/handler traits), а не только
  голый протокол wayland-rs;
- backend-фичи для S03 добавят системные зависимости (udev, libseat) на
  этапе кросс-сборки под aarch64-unknown-linux-musl — не проверено в этом
  ADR, тестируется отдельно в S03.

## Отклонённые альтернативы

### Прямая реализация на `wayland-server`

Меньший dependency graph и более быстрая сборка, но каждый элемент
протокольного цикла (xdg-shell state machine, focus, будущий DRM/evdev
backend) пришлось бы писать и тестировать с нуля — по объёму это
приближается к переписыванию значимой части самого Smithay.

### Другой существующий Rust-compositor toolkit (например, обёртки для wlroots)

Не рассматривался отдельным spike: экосистема Rust-compositor framework'ов
вне wayland-rs/Smithay существенно менее зрелая на момент решения, а
wlroots-based пути означали бы C-зависимость, чего ADR-005 сознательно
избегает для собственного стека.

## Ссылки

- [Smithay](https://smithay.github.io/)
- [wayland-rs (wayland-server)](https://github.com/Smithay/wayland-rs)
- `docs/adr/ADR-005-owned-wayland-application-platform.md`
- `docs/os/sprints/S02-wayland-host-vertical-slice.md`
