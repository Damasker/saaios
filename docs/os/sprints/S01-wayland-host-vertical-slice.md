# Sprint 01 — Wayland vertical slice на host

## Паспорт

- Состояние: `Done`.
- Зависит от: S00.
- Архитектурные решения: ADR-002, ADR-004, ADR-005.
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
не заявляется до S06.

## Rollback

Удалить новые workspace members и ADR выбора framework. Существующая сборка
Pixel 7, установленный image и userdata не меняются.

## Evidence

Проверено локально 2026-09-06 на Ubuntu/WSL2, Rust 1.97.1:

- `cargo fmt --all -- --check` — успешно;
- `cargo clippy --workspace --all-targets -- -D warnings` — успешно;
- `cargo test --workspace` — успешно, 62 passed, 0 failed;
- `cargo test -p saai-displayd --all-targets -- --nocapture` — успешно,
  4 unit + 1 multi-process integration tests;
- integration запускает настоящий Wayland socket, три protocol-negative
  клиента, принудительно завершённый клиент и затем исправный demo-client;
- hash XRGB8888 frame:
  `9b05ff34424f63e620a88baecc12950fe13e242ca1645ec9d8d57d939186f99d`;
- framework benchmark и решение записаны в ADR-006: 44 уникальные строки
  dependency tree у выбранного среза против 72 у минимального Smithay
  `wayland_frontend` spike.

Известные ограничения: только host/headless, один XRGB8888 toplevel,
синтетическая клавиатура без keymap, без pointer/touch/output, DRM/KMS,
Pixel 7 и GPU.
