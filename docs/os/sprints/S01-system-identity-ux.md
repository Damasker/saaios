# S01 — Pixel 7 system identity UX

Статус: implemented in `drm-splash` (Issue #31); physical Evidence via #28
Экран: native `drm-splash`, Pixel 7, 1080×2400, touch-first
Команда: один read-only вызов `system.identity`

## Scope

`CHECK DEVICE` в S01 читает только системную идентичность. Один принятый tap
создаёт ровно одну попытку `system.identity`. Проверка не вызывает модель, не
проверяет здоровье компонентов, не меняет настройки и ничего не устанавливает.

Существующий экран `DEVICE` сохраняет блоки `SET AN INTENT`,
`DEVICE ACTIONS` и `SYSTEM RESULT`. Этот контракт меняет только строку
`CHECK DEVICE` и содержимое `SYSTEM RESULT`; новая навигация и новые экраны не
нужны.

Вне scope: display/touch/battery/network/audio probes, пошаговый progress,
частичные результаты, Inbox/persistence, device settings, action workflow,
telemetry contract и reboot-resume.

## Иерархия существующего экрана

Порядок сверху вниз не меняется:

1. заголовок `DEVICE`;
2. индикатор runtime: `SAAIOS CORE ACTIVE` или `SYSTEM CORE OFFLINE`;
3. `SET AN INTENT`;
4. `DEVICE ACTIONS`, где первая строка — `CHECK DEVICE`;
5. `SYSTEM RESULT`;
6. существующая нижняя навигация.

Состояние проверки определяется только последней попыткой в текущем процессе.
Отдельный экран результата и сохранение истории не вводятся.

## State contract

### 1. Idle

Используется до первого запуска и после входа на экран без результата.

```text
DEVICE ACTIONS
CHECK DEVICE

SYSTEM RESULT
READY TO CHECK IDENTITY
```

- `CHECK DEVICE` доступна для tap.
- `SYSTEM RESULT` и строка состояния используют neutral/muted tokens:
  `#7E96B2` для label и `#8CA9C8` для текста.
- Tap принимается только при отсутствии in-flight попытки.
- Принятый tap даёт один haptic acknowledgement, блокирует строку действия и
  переводит UI в `running`.

### 2. Running

Показывается, пока единственный вызов `system.identity` ожидает ответ.

```text
DEVICE ACTIONS
CHECKING…

SYSTEM RESULT
CHECKING DEVICE
READING SYSTEM IDENTITY
```

- Строка действия disabled; повторные tap, touch-up и key repeat игнорируются.
- Новый процесс, запрос или fallback tool не запускается.
- `CHECKING…` и `CHECKING DEVICE` явно передают состояние без зависимости от
  animation.
- Status token — active violet `#8C86FF`; основной текст — `#F5F8FC`.
- Spinner допустим только как дополнительный признак.
- Timeout: 10 секунд от принятого tap по monotonic clock.
- Валидный ответ переводит в `success`. Timeout, недоступный runtime/tool,
  non-zero exit или невалидный ответ переводят в `error`.

### 3. Success

Показывает только нормализованные поля из успешного ответа `system.identity`.

Полный пример:

```text
SYSTEM RESULT
IDENTITY OBSERVED
SAAIOS
PIXEL 7 / PANTHER
SLOT A
```

Пример с неизвестными полями:

```text
SYSTEM RESULT
IDENTITY OBSERVED
SAAIOS
DEVICE / TARGET UNKNOWN
SLOT UNKNOWN
```

- `IDENTITY OBSERVED` — semantic success label; token `#00CFA0`.
- Факты используют primary text `#F5F8FC`; неизвестные значения — muted
  `#8CA9C8`.
- После завершения строка снова называется `CHECK DEVICE` и доступна для новой
  независимой попытки.
- Success означает только успешное чтение ответа. Он не утверждает исправность
  устройства, доступность hardware или успешную установку SaAIOS.
- Запрещены строки `DEVICE HEALTHY`, `ALL SYSTEMS READY`, `INSTALLED` и
  `FIXED`, если таких фактов нет в ответе.

### 4. Error / retry

Runtime offline:

```text
SYSTEM RESULT
ERROR
SYSTEM CORE OFFLINE
IDENTITY NOT READ

DEVICE ACTIONS
RETRY
```

Tool unavailable:

```text
SYSTEM RESULT
ERROR
IDENTITY TOOL MISSING
IDENTITY NOT READ

DEVICE ACTIONS
RETRY
```

Timeout:

```text
SYSTEM RESULT
ERROR
IDENTITY CHECK TIMED OUT
IDENTITY NOT READ

DEVICE ACTIONS
RETRY
```

- `ERROR` и status marker используют error token `#D56D6D`; основной error
  text — `#FFD0D0`.
- Ошибка всегда обозначена словом и не выглядит как нормальный result.
- Ошибка не сохраняет и не показывает данные от предыдущей попытки.
- `RETRY` — единственное действие ошибки. Оно появляется только после
  завершения или отмены предыдущей попытки.
- Первый принятый tap по `RETRY` запускает одну новую попытку и немедленно
  возвращает `running`; последующие tap игнорируются до terminal state.
- При offline runtime dispatch завершается в `error` без model fallback и без
  ложного success.

## Переходы и guards

```text
enter DEVICE ───────────────→ idle
idle ── accepted tap ───────→ running
running ── valid response ──→ success
running ── timeout/failure ─→ error
success ── accepted tap ────→ running
error ── accepted RETRY ────→ running
```

Инварианты реализации:

1. Одновременно существует максимум одна identity attempt.
2. Guard проверяется до spawn/dispatch, а disabled state устанавливается в том
   же event turn.
3. Attempt получает внутренний monotonic generation number. Ответ от старой
   generation после timeout игнорируется.
4. Terminal transition выполняется один раз; exit и timeout одной attempt не
   могут создать два результата.
5. `system.identity` — единственный допустимый tool; model request и fallback
   на другой tool запрещены.

## Нормализация identity

Рендерер не показывает произвольные сырые строки:

- product: фиксированное `SAAIOS`, только если подтверждено ответом;
- device: canonical `PIXEL 7` либо `DEVICE UNKNOWN`;
- target: `[A-Z0-9_-]`, максимум 12 символов, иначе `TARGET UNKNOWN`;
- slot: только `A` или `B`, иначе `SLOT UNKNOWN`.

Canonical Pixel 7 target отображается как `PIXEL 7 / PANTHER`. Длинные,
отсутствующие, malformed или не-ASCII значения заменяются на соответствующий
`UNKNOWN`, а не обрезаются в потенциально ложный факт. Сырые identity fields,
серийные номера и hardware identifiers не попадают в UI или журнал.

## Text, touch и accessibility

- Все видимые runtime-строки выше являются точными и uppercase ASCII.
- Максимум 24 символа в одной строке результата.
- `SYSTEM RESULT` содержит не более четырёх строк под label.
- Текст не уменьшается ниже текущего native body scale ради длинного значения.
- Touch target `CHECK DEVICE`/`RETRY` сохраняет текущую геометрию
  972×150 design px; visual bounds и hit bounds совпадают.
- Контраст: минимум 4.5:1 для текста и 3:1 для крупного status marker.
- Idle, running, success и error различимы в grayscale и без animation.
- Focus/selected state не кодирует success или error.
- Нижняя навигация остаётся доступной; возврат на `DEVICE` показывает текущее
  состояние in-flight попытки.

## Error/retry rationale

`SYSTEM CORE OFFLINE` находится внутри error state, потому что отсутствие
runtime не является результатом identity. Старые данные скрываются, чтобы
пользователь не принял их за ответ новой попытки. Retry разрешён только после
terminal transition: это исключает duplicate spawn и сохраняет правило «один
принятый tap — одна attempt».

## Physical verification на Pixel 7

1. Открыть `DEVICE`: видны `CHECK DEVICE` и idle copy
   `READY TO CHECK IDENTITY`.
2. Нажать один раз: сразу видны disabled `CHECKING…`, `CHECKING DEVICE` и
   `READING SYSTEM IDENTITY`.
3. Дважды быстро нажать `CHECK DEVICE`: в runtime наблюдается ровно один
   `system.identity`.
4. На success сверить `SAAIOS`, `PIXEL 7`, `PANTHER` и slot с фактическим
   ответом; неподтверждённое поле отображается как `UNKNOWN`.
5. Остановить runtime и запустить проверку: `ERROR` и
   `SYSTEM CORE OFFLINE` визуально красные и текстово отличимы от success.
6. На error быстро дважды нажать `RETRY`: запускается одна attempt, UI
   возвращается в running.
7. Симулировать ответ дольше 10 секунд: появляется
   `IDENTITY CHECK TIMED OUT`; поздний ответ не меняет error.
8. Проверить четыре состояния без animation и в grayscale; тексты не
   обрезаются, touch target срабатывает у всех четырёх краёв.

Выполнено 2026-09-09 на combined Evidence image source `be9630d`, SHA-256
`c1ebb5afd0ee276ca7c2774b765084a19e001ad1afc4d3385c57dc7b66aedff8`.
Пользователь подтвердил idle/running/success, offline/error/retry и timeout;
audit подтвердил один `system.identity` на принятую попытку и подавление
быстрого двойного tap. Поздний ответ после timeout не изменил terminal error.

## Implementation-facing decisions

- Один request: `system.identity`; multi-probe orchestration отсутствует.
- Четыре состояния: `idle`, `running`, `success`, `error`.
- Timeout фиксирован на 10 секунд по monotonic clock.
- Running не имеет cancel/stop; единственная recovery action — terminal
  `RETRY`.
- Success содержит только bounded normalized identity facts.
- Offline, missing tool, malformed, non-zero exit и timeout являются error.
- Existing `DEVICE`, `SET AN INTENT`, `DEVICE ACTIONS`, `SYSTEM RESULT` и
  navigation сохраняются без нового service или persistence contract.
