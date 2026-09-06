# S01 — Pixel 7 system identity UX

Статус: ready for implementation  
Устройство: Pixel 7, 1080×2400, touch-first

## Scope

Телефон воспринимается как SaAIOS на Pixel 7, а не как экран чат-ассистента.
Модель остаётся одной из системных возможностей и не подменяет системные
факты, действия или состояние устройства.

UX-gate S01 состоит из четырёх постоянных блоков:

1. `DEVICE`
2. `SET AN INTENT`
3. `DEVICE ACTIONS`
4. `SYSTEM RESULT`

Сохраняются корневые разделы `Сейчас`, `Входящие`, `Пространства`, `Я`.
Вне scope: новые приложения, Scheduler, многоагентность и полный редизайн
оболочки.

## DEVICE

Штатное состояние:

```text
DEVICE
SaAIOS READY
Pixel 7 · LOCAL SYSTEM

[CHECK DEVICE]  [DEVICE DETAILS]
```

Запуск:

```text
DEVICE
SaAIOS STARTING
4 OF 6 SYSTEM SERVICES READY

[VIEW STARTUP]
```

Название модели не показывается в системной шапке. Статус устройства всегда
основан на данных системных служб.

## SET AN INTENT

Строка присутствует во всех четырёх корневых разделах и не выглядит как чат.

Точные строки:

- метка: `SET AN INTENT`
- placeholder: `What should SaAIOS do?`
- модель недоступна: `Find a setting or device action`
- подсказка клавиатуры: `Describe the result you want`
- кнопки: `CANCEL`, `CONTINUE`
- submit: `ACCEPTING INTENT…`
- ошибка: `INTENT NOT ACCEPTED`
- retry: `TRY AGAIN`

Поведение:

- Tap открывает клавиатуру с сохранением текущего Space.
- Submit создаёт намерение, но не заявляет о выполненном действии.
- Пустой submit ничего не делает.
- Повторный tap во время submit не создаёт дубль.
- Back сохраняет введённый текст в текущем сеансе.
- При недоступной модели остаются доступны ручная навигация и поиск модулей.

## DEVICE ACTIONS

Минимальный набор S01:

```text
DEVICE ACTIONS
DISPLAY  ·  AUDIO
NETWORK  ·  BLUETOOTH
```

Tap открывает системную панель выбранного модуля. Изменяющее состояние
действие сначала показывается как предложение:

```text
ACTION PROPOSED
SET BRIGHTNESS TO 35%
CONFIRM TO CONTINUE

[CONFIRM]  [CANCEL]
```

Только после подтверждения системной службой:

```text
ACTION COMPLETED
BRIGHTNESS SET TO 35%
SYSTEM SERVICE · NOW
```

Timeout, отказ или отмена не маркируются как выполненное действие. Повторное
касание не создаёт вторую операцию с тем же `action_id`.

## CHECK DEVICE

В коде и telemetry действие называется `CHECK_DEVICE`. Видимая строка —
`CHECK DEVICE`. Это системная проверка без обращения к модели и без изменения
настроек.

Минимальные наблюдения: display, touch, storage, battery, Wi‑Fi, Bluetooth,
audio и runtime.

### Waiting

```text
CHECK DEVICE
WAITING FOR SYSTEM SERVICES
CHECK WILL START AUTOMATICALLY

[CANCEL]
```

Через 2 секунды строка уточняется: `WAITING FOR: STORAGE`. Бесконечный spinner
без текстового статуса запрещён.

### Running

```text
CHECKING DEVICE
DISPLAY AND INPUT
STEP 2 OF 6

[STOP]
```

Допустимые названия шагов:

- `DISPLAY AND INPUT`
- `POWER`
- `STORAGE`
- `NETWORK`
- `AUDIO AND BLUETOOTH`
- `LOCAL SERVICES`

Статусы шагов: `CHECKED`, `CHECKING`, `WAITING`. Искусственный процент
запрещён.

### Result

Успех:

```text
SYSTEM RESULT
DEVICE READY
8 OBSERVATIONS · NO ACTIONS

[VIEW DETAILS]  [DONE]
```

Частичный результат:

```text
SYSTEM RESULT
NETWORK NEEDS ATTENTION
7 OF 8 OBSERVATIONS RECEIVED

[VIEW DETAILS]  [RETRY]
```

Копия появляется во `Входящих` как `DEVICE CHECK RESULT`. Главная карточка не
превращается в историю чата.

### Error

Известный timeout:

```text
SYSTEM RESULT
CHECK NOT COMPLETED
NO RESPONSE FROM BLUETOOTH

[RETRY BLUETOOTH]  [VIEW DETAILS]
```

Неизвестная ошибка:

```text
SYSTEM RESULT
CHECK NOT COMPLETED
SYSTEM SERVICE STOPPED

[RETRY]  [CLOSE]
```

Безопасный код `DEVICE_CHECK_FAILED` доступен в деталях. Нельзя показывать
`DEVICE BROKEN`, когда известен только timeout probe.

### Retry и остановка

- Retry запускает только неуспешный probe, сохраняя валидные наблюдения.
- Полный retry получает новый `check_id`; один tap создаёт один запуск.
- Во время retry кнопка блокируется до принятия команды.
- `STOP` завершает активный probe и сохраняет частичные наблюдения.
- Результат остановки: `CHECK STOPPED`; выполненных действий нет.
- После reboot: `CHECK INTERRUPTED BY RESTART`; автоматического retry нет.

## SYSTEM RESULT

Тип каждой записи обозначается словом, а не только цветом.

Наблюдаемый факт:

```text
OBSERVED
WI-FI: NOT CONNECTED
SOURCE: SYSTEM · NOW
```

Предположение модели:

```text
MODEL ASSUMPTION
NETWORK MAY BE OUT OF RANGE
NOT VERIFIED BY SYSTEM
```

Предложение:

```text
ACTION PROPOSED
CONNECT TO HOME WI-FI
CONFIRMATION REQUIRED
```

Подтверждённое действие:

```text
ACTION COMPLETED
BRIGHTNESS SET TO 35%
SYSTEM SERVICE · 04:14
```

Без системного evidence запрещены формулировки `CAUSE FOUND`, `FIXED` и
`DEVICE HEALTHY`. При конфликте системный факт показывается первым, а
предположение получает подпись `CONFLICTS WITH CURRENT OBSERVATION`.

## Переходы

```text
Сейчас
  ├─ DEVICE / CHECK DEVICE → WAITING → RUNNING
  │                                      ├─ SYSTEM RESULT
  │                                      └─ ERROR RESULT
  ├─ DEVICE / DEVICE DETAILS → Обзор устройства
  ├─ DEVICE ACTIONS → PROPOSED → COMPLETED | ERROR
  └─ SET AN INTENT → Клавиатура → ACCEPTED | ERROR

Входящие
  └─ SYSTEM RESULT → Сведения → Назад во Входящие
```

Lock не отменяет проверку. После unlock текущий статус остаётся видимым.

## Edge cases

- Модель недоступна: `CHECK DEVICE` и ручная навигация работают.
- Нет сети: `WI-FI: NOT CONNECTED` является наблюдением, а не общей ошибкой.
- Probe timeout: показывается имя службы, частичные факты сохраняются.
- Двойное касание: один `check_id`, один запуск.
- Экран погас: проверка продолжается без принудительного wake.
- Устаревший факт: `UPDATED 1 MIN AGO` и действие `REFRESH`.
- Нет места для результата: `RESULT NOT SAVED`; UI не заявляет о записи.
- Поворот или смена DRM mode: layout остаётся в design space 1080×2400,
  hit-regions используют тот же transform.

## Text и touch constraints

- Заголовок: до 24 знаков, одна строка.
- Основной вывод: до 32 знаков в строке, максимум две строки.
- Одна карточка содержит одну главную мысль.
- Не более двух действий в одной строке; primary слева.
- Минимальная touch-цель: 120×120 design px.
- Расстояние между соседними целями: минимум 24 design px.
- Состояние не полагается только на цвет, анимацию или spinner.
- IMEI, адреса устройств, ключи и prompt пользователя не выводятся в карточках
  и журнале.

## Visual acceptance

- За 5 секунд пользователь называет продукт `SaAIOS` и устройство `Pixel 7`,
  а не «чат с ИИ».
- На `Сейчас` последовательно читаются `DEVICE`, `SET AN INTENT`,
  `DEVICE ACTIONS`, затем последний `SYSTEM RESULT`.
- `OBSERVED`, `MODEL ASSUMPTION`, `ACTION PROPOSED` и `ACTION COMPLETED`
  различимы в grayscale и без анимации.
- Все строки помещаются на 1080×2400 без clipping.
- Waiting, running, result и error сохраняют геометрию Back и навигации.

## Physical acceptance на Pixel 7

1. Cold boot без модели оставляет доступными разделы и `CHECK DEVICE`.
2. На устройстве воспроизводятся waiting, running, result и error; error
   вызывается контролируемым timeout test probe.
3. Каждое `OBSERVED` сверяется с соответствующим sysfs или service source.
4. Lock/wake во время проверки сохраняет состояние; input не утекает.
5. Двойной tap по primary CTA создаёт один запуск.
6. `STOP` не выполняет системных действий и сохраняет частичные наблюдения.
7. `SYSTEM RESULT` появляется во `Входящих`; Details и Back работают.
8. Проверяются крайние touch-цели, BGRX-цвета, кириллица и отсутствие tearing
   на 1080×2400×60.
9. После остановки shell доступны recovery UI и USB-консоль.
10. Журнал содержит только `check_id`, переход состояния, source и безопасный
    error code; prompt, секретов и Bluetooth identifiers в нём нет.

## Definition of Done

- UI-строки хранятся в одном наборе констант.
- `CHECK_DEVICE` не вызывает модель и не изменяет настройки.
- State machine: `idle → waiting → running → result | error | cancelled`.
- Fact, assumption, proposal и action различаются на уровне данных.
- Unit-тесты покрывают переходы, double submit, retry и stale result.
- Deterministic previews содержат все четыре состояния `CHECK DEVICE`.
- Все пункты physical acceptance записаны как evidence с физического Pixel 7.
