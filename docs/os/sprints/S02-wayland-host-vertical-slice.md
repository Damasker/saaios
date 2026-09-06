# Sprint 02 — Wayland vertical slice на host

## Паспорт

- Состояние: `Done`.
- Зависит от: S01.
- Архитектурные решения: ADR-002, ADR-004, ADR-005, ADR-007.
- Рабочий fallback: существующий `drm-splash`; phone image не меняется.

## Goal

Доказать стандартную Wayland-границу между первым прототипом `saai-displayd`
и отдельным тестовым приложением, не затрагивая телефон.

## Current state

Pixel 7 использует один статический C-процесс для DRM UI и прямого чтения
touch. Независимого display server, surface protocol и app lifecycle пока нет.
Текущий UI физически проверен и остаётся стабильным fallback.

## Scope

Входит:

- короткий измеримый spike Smithay и минимальной реализации на доступном host;
- ADR выбора framework с размером dependency graph, временем сборки и
  сложностью cross-compile;
- headless compositor с `wl_shm` и одним `xdg_toplevel`;
- отдельный demo-client, рисующий однозначную тестовую поверхность;
- configure/ack/commit, focus, синтетическое событие ввода и close;
- protocol-negative и crash-recovery тесты;
- CI-команда для полного сценария.

Не входит:

- DRM/KMS, evdev или flash Pixel 7;
- системная оболочка и перенос текущего UI;
- GPU, `linux-dmabuf`, GTK, Qt, XWayland;
- sandbox, app store и постоянный app manifest.

## Change

1. Зафиксировать benchmark-критерии framework и выполнить оба spikes.
2. Принять ADR без скрытого обязательства использовать чужую оболочку.
3. Создать `saai-displayd` и минимальный headless backend.
4. Создать независимый `saai-demo-surface`.
5. Реализовать жизненный цикл первой поверхности и focus.
6. Добавить негативные protocol fixtures и принудительное завершение клиента.
7. Подключить сценарий к CI и записать ограничения следующего спринта.

## Test

- unit: состояния поверхности и focus;
- protocol: configure до buffer attach, ack serial, invalid buffer/role;
- integration: compositor и client являются разными процессами;
- fault injection: client exit и malformed request не завершают compositor;
- determinism: итоговый тестовый frame имеет стабильный hash;
- CI: чистое окружение повторяет весь сценарий без дисплея.

## Acceptance criteria

- demo-client подключается через настоящий Wayland socket;
- после configure/ack он представляет `wl_shm` buffer;
- compositor получает и проверяет ожидаемый frame hash;
- синтетический input доставляется только focused client;
- close корректно завершает demo-client;
- malformed client отключается, а следующий исправный client запускается;
- нет прямой зависимости от DRM и кода `panther`;
- выбранный framework и причины выбора записаны отдельным ADR;
- `cargo fmt`, `clippy`, workspace tests и headless integration зелёные.

## Threat / privacy impact

Спринт не читает пользовательские данные и не открывает сеть. Wayland socket
создаётся в приватном временном runtime-каталоге. Проверяется, что второй
процесс не может переиспользовать чужую роль/serial. Полная изоляция приложений
не заявляется до S07.

## Rollback

Удалить новые workspace members и ADR выбора framework. Существующая сборка
Pixel 7, установленный image и userdata не меняются.

## Evidence

- Framework spike записан в
  [ADR-007](../../adr/ADR-007-minimal-wayland-rs-compositor.md): Smithay
  `wayland_frontend` — 84 packages, 92,24 с и обязательный
  `libxkbcommon`; прямой `wayland-rs` — 27 packages, 57,62 с и без native
  Wayland/XKB dependency.
- Workspace содержит два исполняемых процесса: `saai-displayd` и
  `saai-demo-surface`. Они общаются только через настоящий Wayland Unix socket
  в приватном `XDG_RUNTIME_DIR`.
- `cargo test -p saai-displayd --all-targets -- --nocapture` проверяет:
  configure/ack/commit, точный `wl_shm` frame hash
  `b71d8fd372b5947dd4bb9d23dde37f777ccc57547fa0fcd59a7e924ef69efc61`,
  pointer только focused client, close, ранний client exit, повреждённый wire
  header, buffer до configure ack и повторную xdg role. После каждого
  ошибочного клиента исправный client завершает сценарий, уничтожает первый
  toplevel и успешно проходит новый configure/ack с повторно созданной ролью.
- Локально собраны static stripped AArch64 musl artifacts:
  `saai-displayd` 1235072 bytes,
  SHA-256 `63144a94825a850ff12bd71d2985c0c49d0d808b6a5a4f3146ff3a1ec19ba3ae`;
  `saai-demo-surface` 1112440 bytes,
  SHA-256 `93135bcdee42381cf893a11a2490adbcc2e31fdddb5c79b2403340e2d7fde292`.
- Реализация зафиксирована commit
  `65b274007ce777572ba0e5b8074d3c2a1e5bef42`; GitHub Actions run
  [`34035418665`](https://github.com/Damasker/saaios/actions/runs/34035418665)
  успешно выполнил format, clippy, отдельный headless integration, workspace
  tests, e2e и cross-build.
- Pixel 7 image, `drm-splash`, userdata и slots не изменялись. Ограничения:
  это host proof без output backend; keyboard, touch protocol, popups,
  shell, DRM/KMS и GPU остаются последующим scope.
