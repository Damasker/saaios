# Sprint 01 — Системная идентичность

## Паспорт

- Состояние: `Done` (#26/#31 implementation + physical Evidence #28).
- Зависит от: S00.
- Архитектурные решения: ADR-002, ADR-004, ADR-006.
- Рабочий Evidence/rollback image:
  `dist/panther/saaios-panther-combined-evidence-init_boot.img`, SHA-256
  `c1ebb5afd0ee276ca7c2774b765084a19e001ad1afc4d3385c57dc7b66aedff8`,
  source `be9630de7a8fa45d17ea512eb852822c9527f599`, 8388608 bytes.
  Android slot B сохранён.

## Goal

SaaiOS достоверно отвечает как работающая ОС текущего Pixel 7, показывает
устройство и доступные действия вместо отдельного чат-помощника.

## Current state

Repo-side and physical gates are closed. Single `DeviceContext`,
bootconfig-first identity, early PID1 target and four-state `CHECK DEVICE` UX
are on shared HEAD. The combined image was built from explicit same-checkout
inputs, flashed to Pixel 7 slot A and accepted through success, offline/retry,
timeout/late-result and cold-reboot scenarios on 2026-09-09.

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

Вернуть физически проверенный combined Evidence image source `be9630d` в
slot A или переключиться на сохранённый Android slot B. Userdata не
мигрируется.

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

### Physical verification checklist (completed)

1. Build image from shared HEAD that includes merged #26 **and** #31
   (PR #33); record filename, size, SHA-256, source commit.
2. Flash only slot A after explicit operator approval; keep Android slot B.
3. Confirm status/tool/prompt share one DeviceContext object.
4. Exercise idle → running → success and error/retry per UX contract; prove
   exactly one `system.identity` per accepted tap.
5. Free-form system question; cold reboot; redact secrets before posting.
6. Update this Evidence section and close Issue #28.

### Final physical Evidence (2026-09-09)

- Первая собранная для #28 версия
  `saaios-panther-init_boot.img` (SHA-256
  `463c9de929625019c69a219667fbe7167b7f6613ce416284197064ca9f5f6b7e`,
  declared source `da4005e`) была **отклонена**: после прошивки её
  встроенный `saai-displayd` имел старый SHA-256 и не запускал
  `saai-shell`. Это выявило, что source commit образа не удостоверял отдельно
  поданные Rust binaries.
- Build path исправлен commit `be9630d`: runtime, console, displayd и shell
  теперь передаются явными путями, лимит 8 MiB проверяется, рядом с образом
  записываются `.SOURCE_COMMIT`, `.SHA256` и `.INPUTS.SHA256`.
- Принятый образ:
  `saaios-panther-combined-evidence-init_boot.img`, ровно 8388608 bytes,
  SHA-256
  `c1ebb5afd0ee276ca7c2774b765084a19e001ad1afc4d3385c57dc7b66aedff8`,
  source `be9630de7a8fa45d17ea512eb852822c9527f599`.
- Хэши входов, повторно сверенные на работающем телефоне:
  `saaios-runtime` =
  `41df5f524f3566e0fcb7cbbb4acad6d65968f2ec7abe4157bd80d67e7e3a8d38`,
  `saaios-console` =
  `6cb1845260ff70453e954bbfd6719b1f40bb517a51686fb8eff0f36101b47484`,
  `saai-displayd` =
  `3a4472c46571de225dac8446825c0c4fc3dbbf736a9ab87d26802672e40582ea`,
  `saai-shell` =
  `cbf6cac9374cd37707b3a6597c9f3aede3c11dfe7cee81c64afb9c56b2246cb6`,
  `native-init` =
  `212e8e6fd395ea3a11af0cf53057c5668c6b0bed608b90ae9c39812dc55be5f8`,
  `drm-splash` =
  `d085c48f6332a27eeaf01a76cf25143e3f8d116cdd390830200c5c034520842b`.
- Flash guard подтвердил `panther`, unlocked bootloader и активный slot A;
  записан только `init_boot_a`, slot B не менялся. При первом boot автоматически
  запустились `saai-displayd`, его supervised `saai-shell` и runtime.
- Прямой `--identity` вернул `SAAIOS PHONE / TARGET PANTHER / BOOT SLOT A`.
  Audit каждой принятой UI-попытки содержит ровно один согласованный цикл
  `tool_call(system.identity) -> policy allow -> tool_result`; быстрый двойной
  tap не создал второй цикл.
- Пользователь физически подтвердил idle, running и success (`SAAIOS`,
  `PIXEL 7 / PANTHER`, `SLOT A`), затем offline error + `RETRY`. После
  восстановления runtime один retry вернул success.
- Timeout проверен `SIGSTOP` runtime без изменения файлов: через 10 секунд UI
  показал `IDENTITY CHECK TIMED OUT`; после `SIGCONT` поздний ответ не изменил
  terminal error, а новый `RETRY` успешно завершился.
- Свободный ASCII-вопрос `what system are you?` получил правдивый ответ:
  `I am SaaiOS, the operating system running on this device.` Русский текст
  через serial не использован как Evidence из-за потери UTF-8 в терминале.
- После холодной перезагрузки снова автоматически поднялась основная Wayland-
  связка `saai-displayd -> saai-shell`; `drm-splash` отсутствовал, все четыре
  хэша входов совпали, slot остался A, повторный identity дал тот же результат.

S01 physical gate закрыт; зависимые S02-S04 становятся `Done`, S05 разморожен.
