# ADR-007: Минимальный Wayland compositor на `wayland-rs`

## Статус

Принято, 2026-09-06.

## Контекст

ADR-005 требует собственный `saai-displayd`, но не выбирает framework.
Sprint S02 ограничен headless-вертикалью: `wl_shm`, один `xdg_toplevel`,
фокусированный синтетический input, close и изоляция повреждённого клиента.
DRM/KMS, GPU, shell и toolkit compatibility в этот выбор не входят.

На одном чистом WSL2 host выполнены release-spikes из новых Cargo packages:

- `smithay 0.7.0`, `default-features = false`,
  `features = ["wayland_frontend"]`: 84 packages, 92,24 с, peak RSS
  658400 KiB, Cargo.lock 18975 bytes;
- прямые `wayland-server 0.31.14`, `wayland-client 0.31.15` и
  `wayland-protocols 0.32.13`: 27 packages, 57,62 с, peak RSS 399160 KiB,
  Cargo.lock 5702 bytes.

Время включает cold dependency build и используется только как сравнительная
метрика этого host. Более существенный результат: минимальный Smithay frontend
связался с `libxkbcommon` и не смог собрать исполняемый файл на host без
`libxkbcommon-dev`, хотя keyboard отсутствует в scope. Прямая реализация
собрала и выполнила client/server integration без системных Wayland/XKB
библиотек.

## Решение

`saai-displayd` использует сгенерированные server bindings `wayland-rs`
напрямую. Это не собственный wire protocol: framing, object dispatch и
отключение protocol-invalid клиента остаются ответственностью
`wayland-backend`.

В S02 сервер публикует только:

- `wl_compositor` и `wl_surface`;
- `wl_shm` с `ARGB8888`;
- `wl_seat` с pointer;
- стабильный `xdg_wm_base` и один тестовый `xdg_toplevel`.

Состояния configure/ack/commit, границы SHM, focus и close явно проверяются
нашей небольшой state machine. Wayland socket создаётся только в приватном
`XDG_RUNTIME_DIR`; сеть и пользовательские данные не используются.

## Последствия

Положительные:

- headless CI не требует display server и native Wayland/XKB packages;
- dependency и cross-compile surface заметно меньше;
- реализуется только принятый S02 protocol subset;
- malformed client отключается библиотечным backend без падения compositor.

Цена решения:

- SaaiOS отвечает за корректную реализацию каждого добавляемого protocol
  request и его негативные тесты;
- перед расширением до popups, keyboard и production DRM backend в S03
  потребуется повторная оценка объёма;
- переход на Smithay позже потребует отдельного superseding ADR и доказательства
  выигрыша, а не произвольной замены.

## Отклонённые альтернативы

### Smithay только с `wayland_frontend`

Он сокращает protocol boilerplate, но текущий минимальный feature всё равно
приносит существенно больший graph и обязательную native XKB-связь. Для
ограниченного S02 это не окупается.

### Самостоятельный Wayland wire parser

Меньший внешний graph не компенсирует риск ошибок framing, object lifetime,
FD transfer и protocol-error isolation. Используются проверенные bindings
`wayland-rs`.
