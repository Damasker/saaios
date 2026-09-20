# Системный интеллект и самоуправление SaaiOS

## Цель

SaaiOS должна знать, на каком устройстве запущена, какие возможности реально
доступны, что происходит сейчас и какие действия она вправе выполнить. Это
машинно проверяемая модель состояния, а не личность, придуманная языковой
моделью.

## Модель состояния

```text
DeviceIdentity     что это за экземпляр системы
CapabilityCatalog  что умеют установленные адаптеры и приложения
Observation        что измерено, когда и каким источником
HealthState        нормализованное состояние подсистем
Intent             чего хочет пользователь или системное правило
Task / Action       план и атомарные шаги
PolicyDecision     почему действие разрешено, запрещено или ждёт согласия
ActionResult       что вернул исполнитель
Verification       какое новое наблюдение подтверждает эффект
Event              неизменяемая причинная запись
```

Каждое наблюдение содержит источник, время, freshness и quality. Значение без
источника нельзя повышать до системного факта. `Unknown` является нормальным
состоянием, а не поводом для догадки.

## Целевые компоненты

### `saai-deviced`

Целевой native owner World Model (ADR-122). Пока сервиса нет: identity
идёт через `system.identity`, динамика — через `system.metrics` /
TelemetrySampler. WORLD-01 вводит typed Observation без daemon.
WORLD-02 держит ObservationCache внутри runtime, пока нет второго
consumer для `saai-deviced`.

Когда сервис появится (WORLD-03), он:

- собирает identity и состояние hardware adapters;
- поддерживает versioned snapshot;
- публикует изменения;
- не вызывает модель, не исполняет Action, не принимает Policy,
  не открывает уведомления.

Observation всегда имеет subject, source, timestamp и TTL.
**Freshness (Fresh/Stale) отделена от HealthState**
(`Healthy | Degraded | Unhealthy | Unknown`). Stale required evidence
даёт Health `Unknown`, не `Unhealthy` и не last-known `Healthy`.

### Capability registry

- связывает имя действия со schema, risk, исполнителем и способом проверки;
- отличает наблюдение (`battery.read`) от изменения (`display.set_brightness`);
- сообщает UI и planner только фактически зарегистрированные возможности.

### Workflow service (`saai-taskd`)

- превращает `Intent` / `PlanProposal` в устойчивые `Task` и `Action`;
- хранит durable-состояния `pending / waiting_confirmation /
  waiting_clarification / running / verifying / done / failed / cancelled`;
- **не** хранит `ready` как status — Ready Set вычисляется из store;
- валидирует DAG зависимостей до persistence (ADR-121);
- обеспечивает идемпотентность и продолжение после reboot.

### Policy service

- принимает решение до выполнения;
- учитывает пользователя, пространство, приложение, риск и scope разрешения;
- создаёт неподделываемую системную поверхность подтверждения.

UAM (ADR-124) не заменяет этот сервис. Он задаёт общий язык:
Principal, Grant, scope, Confirmation (OneShot по умолчанию).
`PolicyVerdict` остаётся Allow / AskUser / Deny. Attention, World Model
и Planner не выдают grants.

### Planner

- получает минимальную проекцию DeviceContext, task state и semantic actions;
- предлагает `PlanProposal`, но не исполняет его и не объявляет успех;
- не пишет в entity store и не объявляет Verification;
- может быть локальным, удалённым или полностью отключённым.

### Memory (`memory-store`)

- помнит явное знание пользователя, предпочтения и (позже) гипотезы
  с provenance и scope (ADR-125);
- не дублирует SOM, не копирует Observation и не копирует каждый Action;
- compact identity — `(space_id, key)`; одинаковый key в разных Space
  сосуществует;
- значения в model context — data, не `Known facts` и не instructions;
- модель не объявляет ExplicitFact / ExplicitPreference;
- Learning не выдаёт grants, не глушит Critical Attention и не пишет
  SOM / ContextFrame напрямую.

### Verifier

- проверяет postcondition Action по `VerificationContract` (наблюдение,
  не заявление Worker);
- не авторизует, не порождает новые Task, не меняет цель;
- VerificationFailed → Scheduler → bounded `ReplanRequest` → Planner.

### `saai-shell`

- показывает текущее состояние, происходящую работу, запросы согласия и итог;
- формулирует ввод как намерение, а не обязательное сообщение в чат;
- предоставляет ручной путь к каждой критической capability.

## Контракт текущего вертикального среза

До появления `saai-deviced` runtime строит ограниченный snapshot schema 1:

```json
{
  "schema": 1,
  "system": "SaaiOS",
  "deployment": "native_device",
  "device_class": "phone",
  "target": "panther",
  "hardware_model": "observed or null",
  "architecture": "aarch64",
  "kernel_release": "observed or null",
  "boot_slot": "a or null",
  "observed_by": "local_runtime"
}
```

Snapshot доступен через `system.identity`, runtime status и системный контекст
planner. Поля ограничены по длине, управляющие символы удаляются. Полный kernel
cmdline, hostname, serial, MAC/IP, credentials и userdata не включаются.

## Контракт действия над собой

Каждая изменяющая capability обязана определить:

1. входную schema и допустимые границы;
2. risk level и требуемое подтверждение;
3. executor с таймаутом;
4. наблюдение до действия;
5. ожидаемое проверяемое изменение;
6. наблюдение после действия;
7. rollback или честную пометку `irreversible`;
8. audit record без секретов.

Например, `display.set_brightness(40)` считается успешным не после записи в
sysfs, а после повторного чтения яркости с допустимым отклонением.

## UI без чатовой метафоры

Главный экран показывает состояние и следующую полезную работу. Поле ввода
создаёт `Intent`; результат может быть карточкой состояния, изменением
настройки, задачей или вопросом подтверждения. Диалоговый transcript появляется
только когда он действительно полезен, и не является домашним экраном ОС.

Запрещённые формулировки без факта:

- «Я исправила» до Verification;
- «Устройство поддерживает» без CapabilityCatalog;
- «SaaiOS не установлена» при локальном `system=SaaiOS`;
- «У вас всё в порядке» без свежих наблюдений.

## Этапы зрелости

1. Identity: система достоверно знает экземпляр и target.
2. Inventory: знает зарегистрированные capability и их риски.
3. Observation: поддерживает свежий DeviceState.
4. Action: изменяет одну подсистему через policy и verification.
5. Workflow: продолжает многошаговую задачу после reboot.
6. Automation: реагирует на события в рамках budget и capability.
7. Learning: сохраняет только разрешённую память с provenance и scope
   (ADR-125). Гипотеза ≠ факт. Пользователь всегда может исправить или
   стереть запись.
