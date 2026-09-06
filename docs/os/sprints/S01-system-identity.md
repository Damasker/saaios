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

- CI: GitHub Actions run [`34020330689`](https://github.com/Damasker/saaios/actions/runs/34020330689)
  на commit `1b36603` (merge PR #29) — success (format, clippy, workspace
  tests, e2e).
- Device: target `panther`, boot slot `a`, доступ через USB serial console;
  проверено на устройстве, непрерывно работающем без перезагрузки.
- Physical test 1 (`system.identity`): `saaios-console --identity` вернул
  `SAAIOS PHONE / TARGET PANTHER / BOOT SLOT A`. Полный audit trail
  (`/data/saaios/var/runtime/audit.jsonl`) подтверждает: `system.identity`
  вызван ровно один раз на запрос, `policy_decision` — `allow` с причиной
  `read-only / low-risk tool`, результат — `{system: SaaiOS, deployment:
  native_device, device_class: phone, target: panther, boot_slot: a,
  hardware_model: GS201 PANTHER MP based on GS201, schema: 1}`.
- Physical test 2 (свободный вопрос): запрос "what system are you? are you
  installed on this phone?" получил ответ "I am installed on this phone. The
  system is SaaiOS..." — модель не отрицает установку и не утверждает
  неподтверждённых возможностей.
- UI: экран `DEVICE` подтверждён визуально на устройстве — присутствуют
  `SET AN INTENT`, группа `DEVICE ACTIONS` (кнопки `CHECK DEVICE` /
  `CHECK NETWORK` / `CHECK STORAGE`) и `SYSTEM RESULT`; заголовки
  `ASSISTANT`, `ASK A QUESTION`, `AI RESPONSE` больше не используются.
  Нажатие `CHECK DEVICE` на экране показывает тот же результат, что и
  `system.identity`.
- Cold reboot retention: проверено `reboot -f` на устройстве (busybox `reboot`
  без `-f` не работает — наш `/init` не обрабатывает сигнал завершения,
  которого ждёт стандартный path). После холодной перезагрузки
  (`/proc/uptime` сброшен на ~28s) `drm-splash` и `saaios-runtime`
  поднялись автоматически без ручного вмешательства, `--identity` вернул тот
  же результат `SAAIOS PHONE / TARGET PANTHER / BOOT SLOT A`. Секретов не
  обнаружено — схема identity ограничена нечувствительными полями
  (architecture, boot_slot, deployment, device_class, hardware_model,
  kernel_release, schema, system, target), как и требует threat/privacy
  секция.
- SHA-256 установленного образа: build-манифест (`dist/panther/`) не найден
  на машине сборки (артефакт не сохранён после прошивки), поэтому хэш снят
  напрямую с раздела на устройстве:
  `sha256sum /dev/block/sda11` (`init_boot_a`, 16384 x 512-byte секторов,
  8MiB) = `18987941ee7c41a98ea9a9471287933d75a27bf9417050253800d2e937829771`.
  Поведение на устройстве (identity, UI, cold-reboot) соответствует коду на
  HEAD `1b36603`, но без сохранённого build-манифеста нельзя независимо
  доказать, что это именно та сборка, а не более ранний commit между
  `77e78c7` (ground system intelligence) и HEAD. Хэш зафиксирован здесь,
  чтобы его можно было сверить со следующей воспроизводимой сборкой.
