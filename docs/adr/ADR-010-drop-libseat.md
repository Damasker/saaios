# ADR-010: Прямое открытие DRM без libseat

## Статус

Принято, 2026-09-07.

## Контекст

ADR-007 включил `backend_session_libseat` в `saai-displayd` по умолчанию
(стандартная пара для Smithay-компоситора с `backend_drm`), ADR-008
кросс-компилировал `libseat` для этого. Физический тест на Pixel 7
(S03 Change 4) показал: `LibSeatSession::open("/dev/dri/card0")` падает с
`EAGAIN` при любом собранном backend'е (`seatd` — нет демона; `builtin` —
скорее всего не может взять VT из-за `console=ttynull` в kernel cmdline,
подтверждённого в S03's Current state). Прямой
`std::fs::OpenOptions::new().read(true).write(true).open(...)` от root
сработал немедленно и довёл до следующей стадии (enumeration/mode/dumb
buffer allocation) без единой ошибки.

`libseat` решает три задачи: (1) делегирование прав непривилегированным
процессам, (2) арбитраж между несколькими сессиями/сиденьями, (3)
VT-switching для сосуществования нескольких графических сессий. Ни одна
не применима здесь: `native-init.c`/`saai-displayd` всегда root (PID 1
и его дети), ADR-009 уже зафиксировал строгий инвариант "один UI-процесс
за раз" на уровне supervision, VT-переключение никогда не понадобится
(нет getty/login, нет параллельных X11-подобных сессий).

## Решение

`saai-displayd` открывает `/dev/dri/card0` напрямую (`OpenOptions`), без
`LibSeatSession`. Убрана фича `smithay/backend_session_libseat` из
`panther-hardware`. `hardware::init()` возвращает только
`(HardwareOutput, DrmDeviceNotifier)` — без session notifier.

Инфраструктура кросс-компиляции `libseat` из ADR-008
(`build-cross-sysroot.sh`, `patches/seatd-static-library.patch`)
не удаляется — она остаётся частью sysroot'а как задокументированный,
воспроизводимый паттерн для будущих C-зависимостей, даже если сама
`libseat` сейчас не используется. Пересобирать sysroot без неё —
отдельная, не срочная уборка.

## Последствия

Положительные:

- одним C-зависимостью меньше в рантайме (`libseat` больше не линкуется в
  `saai-displayd`), меньше поверхность для рантайм-сюрпризов вроде этой;
- код проще: нет второго notifier'а для регистрации в event loop, нет
  session pause/resume событий, которые в любом случае не имели бы
  смысла без VT-переключения.

Цена решения:

- если когда-нибудь понадобится настоящий multi-session/VT сценарий
  (маловероятно для этого продукта, но не невозможно), `libseat`
  придётся возвращать — код и cross-build инфраструктура для этого уже
  существуют и проверены, возврат не с нуля;
- прямой `open()` не проверяет права доступа так, как это делал бы
  session manager — не проблема, поскольку весь UI-слот и так работает
  исключительно от root по дизайну этой ОС.

## Ссылки

- `docs/adr/ADR-007-smithay-compositor-framework.md`
- `docs/adr/ADR-008-cross-compiling-drm-backend-c-deps.md`
- `docs/adr/ADR-009-ui-supervision-and-drm-handoff.md`
- `services/saai-displayd/src/hardware.rs`
