# Sprint 01 — Wayland vertical slice на host

## Паспорт

- Состояние: `Backlog`, до закрытия S00.
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

Заполняется при переходе в `Done`: commit, CI run, команды тестирования,
benchmark framework, frame hash и известные ограничения.
