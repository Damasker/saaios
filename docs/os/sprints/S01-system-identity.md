# Sprint 01 — Системная идентичность

## Паспорт

- Состояние: `Repo gate complete; physical acceptance pending`.
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

### Repo-side gate, 2026-09-06

- Один `DeviceContext` создаётся в `saaios-runtime` при старте и клонируется в
  `IdentityTool`, runtime status и planner context. Unit/e2e проверяют object
  equality.
- Slot читается из `/proc/bootconfig`; `/proc/cmdline` используется только как
  fallback. Тест покрывает приоритет, fallback и отсутствие значения.
- PID 1 выставляет `native_device / phone / panther` до storage setup.
  `SAAIOS_DATA` очищен в раннем fallback и появляется только после успешного
  mount userdata. Host C test исполняет сценарий без userdata.
- `CHECK DEVICE` остаётся прямым read-only вызовом `system.identity`; e2e
  подтверждает один policy/tool/audit cycle без обращения к модели.
- Выполнены:
  - `cargo fmt --all -- --check`;
  - `cargo clippy --workspace --all-targets -- -D warnings`;
  - `cargo test --workspace`;
  - `cargo test -p diagnose-slow-system -- --nocapture` — 12 passed;
  - host C fallback test;
  - Pixel 7 cross-build для `aarch64-unknown-linux-musl`.
- Cross-built ELF:
  - `saaios-runtime`, 3,256,312 bytes,
    SHA-256 `104dd4d61618f280166b2c5e4f2dd5b8678527fd0569bc90dda5a8b6c2524319`;
  - `saaios-console`, 991,040 bytes,
    SHA-256 `6cb1845260ff70453e954bbfd6719b1f40bb517a51686fb8eff0f36101b47484`.
  Оба — stripped static AArch64 ELF.
- Factory archive
  `panther-cp2a.260705.006-factory-ed94a24e.zip`, 3,890,506,966 bytes,
  SHA-256 `ed94a24e693a28f236e87c9e03436871c2dd6b03b4b56c5839598115d0372b0b`.
- Из него извлечён stock `init_boot.img`, 8,388,608 bytes,
  SHA-256 `eb43f67c545f647b0aec709df5ddbe0f594d32ecaffe5d26c18ee82b22c5764d`.
- Собран `dist/panther/saaios-panther-s01-init_boot.img`, 8,388,608 bytes,
  SHA-256 `18987941ee7c41a98ea9a9471287933d75a27bf9417050253800d2e937829771`.
  Повторный unpack подтвердил header v4, Android 17.0.0 / patch 2026-07,
  static AArch64 `init`, runtime, console и DRM shell. Хэши embedded runtime и
  console совпали с cross-built artifacts.
- После отдельного явного разрешения image установлен только в `init_boot_a`.
  `fastboot` подтвердил `product=panther`, `unlocked=yes`, `current-slot=a`;
  slot B, userdata и active-slot не изменялись.

### Live preflight, 2026-09-06

- Pixel 7 отвечает на `172.31.7.1`, runtime TCP `38127` доступен.
- Установленный runtime имеет 10 tools без `system.identity`; status не
  содержит `device`, запрос `system_identity` не поддерживается.
- Это подтверждает старый baseline до S01, а не physical acceptance нового
  image. Запись разделов и смена slot не выполнялись.

### Physical run after S01 image

- `fastboot flash init_boot_a` и `fastboot reboot` завершились `OKAY`; SaaiOS
  вернул USB `172.31.7.1` и runtime TCP `38127`.
- Runtime status и `system.identity` вернули одинаковый JSON object:
  `SaaiOS / native_device / phone / panther / boot_slot=a`, architecture
  `aarch64`, `observed_by=local_runtime`.
- Один запрос identity создал ровно один `tool_call`, один allow
  `policy_decision` и один успешный `tool_result` с общим `call_id`. Записи
  проверены read-only непосредственно в persistent JSONL.
- `audit_tail` не читает весь старый журнал: до установки S01 в строке 1342
  уже находился блок NUL. Файл не изменялся и не очищался.
- Свободный вопрос о системе завершился timeout configured local model
  provider через 60 секунд. Правдивый model answer физически не подтверждён.
- До полного physical acceptance остаются: визуально проверить CTA/UI,
  повторить cold reboot и получить правдивый свободный ответ при доступном
  provider.

### Physical verification checklist

После получения factory archive:

1. Проверить SHA-256 архива, извлечь matching stock `init_boot` и подтвердить
   `panther / CP2A.260705.006`.
2. На зафиксированном commit повторить CI-команды и cross-build; собрать
   `saaios-panther-init_boot.img`, записать его размер и SHA-256.
3. До записи выполнить `fastboot getvar product`, `fastboot getvar
   current-slot` и сохранить рабочий fallback image. Остановиться, если product
   не `panther`.
4. Только после явного разрешения пользователя записать согласованный image в
   slot A. Не трогать slot B.
5. После cold boot проверить `/proc/bootconfig`, `/proc/cmdline` и runtime
   status: `SaaiOS / native_device / phone / panther / boot_slot=a`.
6. Нажать `CHECK DEVICE` один раз. Сверить один вызов `system.identity`, один
   policy verdict и одну audit-запись с тем же JSON object.
7. Задать «что ты за система?» и убедиться, что ответ называет SaaiOS на Pixel
   7, не заявляет Android и неподтверждённые возможности.
8. Повторить cold reboot и проверки identity/UI; убедиться, что audit и prompt
   не содержат hostname, serial, MAC/IP, полного cmdline или credentials.
9. Проверить fallback без userdata на отдельном test boot: platform identity
   остаётся `native_device / phone / panther`, а `SAAIOS_DATA` отсутствует.

### Physical rollback

При неуспешной проверке прекратить тест. После отдельного явного разрешения
либо вернуть сохранённый init_boot commit `7b29c62` в slot A, либо сделать
активным нетронутый Android slot B. Userdata не форматировать и не мигрировать.
