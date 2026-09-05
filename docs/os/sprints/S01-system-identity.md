# Sprint 01 — Системная идентичность

## Паспорт

- Состояние: `In progress`.
- Зависит от: S00.
- Архитектурные решения: ADR-002, ADR-004, ADR-006.
- Рабочий fallback: установленный образ commit `7b29c62`, Android slot B.

## Goal

SaaiOS достоверно отвечает как работающая ОС текущего Pixel 7, показывает
устройство и доступные действия вместо отдельного чат-помощника.

## Current state

Runtime имеет реальные Linux tools, но системный prompt не содержит локальную
идентичность. Экран называется `ASSISTANT`, а свободный запрос физически вернул
ложное утверждение `SAAIOS SYSTEM IS NOT INSTALLED IN YOUR PHONE`.

## Scope

Входит:

- локальный DeviceContext schema 1 без идентификаторов пользователя;
- `system.identity` как read-only tool;
- контекст в каждом запросе planner;
- тот же snapshot в runtime status;
- явные target-поля от native PID 1 Pixel 7;
- UI `DEVICE / SET AN INTENT / DEVICE ACTIONS / SYSTEM RESULT`;
- unit, prompt capture, CI, cross-build и физический тест ответа.

Не входит:

- полная автономность и отдельный `saai-deviced`;
- новые изменяющие hardware tools;
- постоянный DeviceState, workflow и автоматизация;
- Wayland или изменение графической архитектуры.

## Change

1. Зафиксировать ADR-006 и модель самоуправления.
2. Создать bounded identity snapshot из локального окружения и kernel facts.
3. Зарегистрировать `system.identity` и вывести snapshot в runtime status.
4. Добавить snapshot в системный prompt с защитой от подмены данными.
5. Переформулировать экран с чата на устройство и намерения.
6. Собрать новый runtime/UI, проверить image и установить только в slot A.
7. Физически вызвать `CHECK DEVICE` и свободный вопрос о системе.

## Test

- unit: mock identity и очистка локальных строк;
- prompt capture: `native_device / phone / panther` есть до user message;
- policy: `system.identity` read-only и не требует подтверждения;
- status: snapshot совпадает с tool result;
- CI: format, clippy, workspace tests и e2e;
- image: SHA-256, target и отсутствие private state;
- device: target `panther`, slot A, загрузка, USB runtime, новый UI;
- physical: система называет себя SaaiOS на Pixel 7 и не утверждает
  неподтверждённые возможности;
- cold reboot: identity и UI сохраняются без записи секретов.

## Acceptance criteria

- `system.identity` возвращает `SaaiOS / native_device / phone / panther`;
- planner всегда получает этот локальный snapshot;
- runtime status показывает тот же target и boot slot;
- UI больше не использует заголовки `ASSISTANT`, `ASK A QUESTION` и
  `AI RESPONSE`;
- `CHECK DEVICE` вызывает `system.identity` ровно один раз;
- свободный вопрос «что ты за система?» получает правдивый ответ;
- изменение не открывает новый сетевой endpoint и не расширяет полномочия;
- установленный binary/image связан с commit и SHA-256;
- физический rollback остаётся доступен.

## Threat / privacy impact

Контекст может передаваться настроенному model provider, поэтому запрещены
hostname, serial, MAC/IP, полный cmdline, credentials и пользовательские данные.
Строки ограничены по длине и не считаются инструкциями. Identity не выдаёт
новых прав.

## Rollback

Вернуть физически проверенный init_boot image commit `7b29c62` в slot A или
переключиться на сохранённый Android slot B. Userdata не мигрируется.

## Evidence

Заполняется после CI, установки и двух физических ответов.
