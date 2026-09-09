# Sprint 02 — Wayland vertical slice на host

## Паспорт

- Состояние: `Done` (implementation Evidence accepted; S01 physical gate
  closed 2026-09-09).
- Зависит от: S01.
- Архитектурные решения: ADR-002, ADR-004, ADR-005.
- Рабочий fallback: существующий `drm-splash`; phone image не меняется.

## Goal

Доказать стандартную Wayland-границу между первым прототипом `saai-displayd`
и отдельным тестовым приложением, не затрагивая телефон.

## Current state

S02 завершён: независимые `saai-displayd` и `saai-demo-surface` проходят
headless Wayland vertical slice, protocol-negative и crash-recovery сценарии
в CI. Последующие S03/S04 перенесли тот же compositor boundary на Pixel 7 и
отдельный системный shell; статический C UI сохранён только как проверенный
fallback.

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

- Framework spike + ADR: commit `c0c7092` — Smithay 0.7.0 (headless
  features) против сырого `wayland-server`: 82 против 23 зависимостей,
  ~29s против ~10.5s чистой сборки; см.
  [ADR-007](../../adr/ADR-007-smithay-compositor-framework.md).
- Реализация: commits `99a1981`, `495184a`, `46e3dcc` —
  `saai-displayd` (headless compositor: `wl_compositor`, `wl_shm`,
  `xdg_wm_base`, `wl_seat`) и `saai-demo-surface` (независимый клиент);
  жизненный цикл surface (commit → configure → ack → attach → commit),
  keyboard focus (первый закоммитивший surface), sha256 хеш кадра,
  `ensure_configured()`-проверка на protocol-negative commit, устойчивость
  compositor к отключению некорректного клиента.
- CI: commits `6cadda9` (`libxkbcommon-dev`), `c351c9e`/`656a2e9`
  (headless-сценарий как реальный `cargo test`, плюс явный `cargo build
  --workspace` перед `test` — `cargo test --workspace` сам по себе не
  гарантирует собранный бинарник для соседнего пакета), `bb11554`
  (отдельный тест на focused-only input delivery). Финальный зелёный прогон:
  [`34099976911`](https://github.com/Damasker/saaios/actions/runs/34099976911).
- Тестовые команды: `cargo test -p saai-displayd --test
  headless_vertical_slice` — два теста, `headless_vertical_slice` (обычный
  клиент, protocol-negative, fault-injection resilience, frame hash
  determinism) и `focused_input_delivery` (synthetic input доставлен
  только сфокусированному клиенту); оба стабильно зелёные (3 подряд локальных
  прогона, ~21-30s).
- Известные ограничения: cross-compile под `aarch64-unknown-linux-musl` не
  проверялся (S02 сознательно host-only); backend-фичи Smithay для
  DRM/libinput/udev не включены — предмет S03; frame hash сравнивается
  только между двумя клиентами одного прогона, не с заранее записанным
  эталонным значением.
