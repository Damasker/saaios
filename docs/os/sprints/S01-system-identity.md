# Sprint 01 — Системная идентичность

## Паспорт

- Состояние: `In progress` (#26 merged; UX #31 implemented pending merge; Evidence #28).
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

### Repo-side gate (landed via #26 on shared HEAD)

- Один `DeviceContext` создаётся в `saaios-runtime` при старте и клонируется в
  `IdentityTool`, runtime status и planner context. Unit/e2e проверяют object
  equality.
- Slot и hardware читаются из `/proc/bootconfig` first; `/proc/cmdline` —
  fallback. Тесты покрывают приоритет, fallback и отсутствие значения.
- PID 1 выставляет `native_device / phone / panther` до storage setup.
  `SAAIOS_DATA` очищен в раннем fallback и появляется только после успешного
  mount userdata. Host C test исполняет сценарий без userdata.
- `CHECK DEVICE` остаётся прямым read-only вызовом `system.identity`; e2e
  подтверждает один policy/tool/audit cycle без обращения к модели.
- Accepted UX contract: `docs/os/sprints/S01-system-identity-ux.md` (PR #29).
  Four UI states are implemented in `drm-splash` (`identity-ux` + DEVICE
  CHECK/RETRY rendering). Final physical close still requires Issue #28.
- CI must pass on the merge commit: format, clippy, workspace tests, PID 1
  identity fallback host test, e2e, and Panther cross-build including
  `saaios-runtime` / `console-tui`.

### Prior physical baseline (divergent image; not final Evidence)

The following device run was recorded against a build path that was **not** on
shared HEAD when S02+ continued. Treat as historical baseline only. Final
acceptance requires a fresh image from the merged #26 commit plus UX state
verification (Issue #28).

- Identity JSON observed: `SaaiOS / native_device / phone / panther / boot_slot=a`.
- One `CHECK DEVICE` / identity request produced one tool call + allow policy.
- Free-form answer grounded in device_context without denying installation.
- Cold reboot retained identity; no secrets in identity schema.
- Recorded partition hash (historical):
  `sha256sum /dev/block/sda11` =
  `18987941ee7c41a98ea9a9471287933d75a27bf9417050253800d2e937829771`.

### Physical verification checklist (required to close S01)

1. Build image from shared HEAD that includes merged #26 **and** #31
   (PR #33); record filename, size, SHA-256, source commit.
2. Flash only slot A after explicit operator approval; keep Android slot B.
3. Confirm status/tool/prompt share one DeviceContext object.
4. Exercise idle → running → success and error/retry per UX contract; prove
   exactly one `system.identity` per accepted tap.
5. Free-form system question; cold reboot; redact secrets before posting.
6. Update this Evidence section and close Issue #28.
