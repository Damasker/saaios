# HDI — лаборатория аппаратного исследования

Статус трека: **HDI-12 в Verify на host. HDI-06 не начат: нет снимка MacBook.**
Активный аппаратный эксперимент на Pixel этот трек не открывает и
модемный трек не сдвигает. HDI-00…05 — host. HDI-06 — одна
read-only команда на MacBookPro11,3, когда снимок уже собран и
предыдущие спринты в Verify или Done.

Порядок можно менять только записью в этом файле. Спринт длиннее
одного вертикального результата делится до кода, а не во время него.

## Общий контракт

Вход: явный Intent `investigate_hardware` в `IntentGate` либо файл
намерения с теми же полями. Выход кампании: журнал вердиктов, утверждения,
метрики против control, записи знания. Исполнитель принимает только
`ProbeRequest` из ADR-426. Потолок v1: 5 проходов, 5 исполненных
проб, `read_only`.

Откат всего трека до появления кода: удалить ADR-426, этот файл,
запечатанный ключ и абзац в индексе спринтов. После появления кода
откат каждого спринта записан в его паспорте и не включает образ
телефона.

## Sprint HDI-00 — контракт кампании

- Состояние: `Verify`
- Зависит от: ADR-426
- Архитектурные решения: ADR-426. Нового решения в коде не требуется.
- Рабочий fallback: крейт не подключён к init, taskd и модемному
  сервису. Их поведение остаётся прежним.

### Goal

Невалидный запрос пробы и кампания без намерения отвергаются
типизированным контрактом на host, без устройства и без модели.

### Current state

Есть только ADR и этот план. Крейта `hdi-contract` нет. Снимка
MacBook нет. `investigate_hardware` в резолвере не зарегистрировано.

### Scope

Входит: типы кампании, намерения, пробы, утверждения и лимитов;
разбор JSON; отказ от лишней пробы, чужого риска и неизвестного
имени.

Не входит: чтение живого `/sys`, процесс-исполнитель, вызов модели,
запись Intent в entity store, UI.

### Change

1. Крейт `crates/hdi-contract` без зависимости от `saai-modemd`,
   `saai-taskd` и `native-init`.
2. Конструктор кампании требует документ намерения. Пустой ввод и
   отсутствие `semantic_action_id` дают ошибку, не кампанию по
   умолчанию.
3. Парсер `ProbeRequest` принимает пример из ADR-426 и отвергает
   набор негативных фикстур ниже.
4. Лимит останавливает кампанию на шестом проходе и на шестой
   исполненной пробе до вызова исполнителя.

### Test

Host, `cargo test -p hdi-contract`. Телефон не нужен.

| Тест | Ожидание |
|---|---|
| `campaign_requires_intent` | Нет документа — нет кампании |
| `campaign_rejects_other_action` | `delete_entity` не открывает HDI |
| `probe_request_accepts_adr_example` | Пример ADR разбирается, риск `read_only` |
| `probe_unknown_name_denied` | Имя вне белого списка — ошибка контракта |
| `probe_shell_metacharacters_denied` | `;`, `&&`, перевод строки, кавычки в аргументе — отказ |
| `probe_risk_write_denied` | Любой риск кроме `read_only` — отказ |
| `probe_path_escape_denied` | `..`, абсолютный путь вне префикса — отказ |
| `sixth_pass_not_executable` | После пяти проходов исполнитель не запрашивается |
| `claim_without_evidence_not_knowledge` | Пустой `evidence_ids` не становится записью знания |
| `claim_equality_is_kind_subject_value` | Тот же смысл другим текстом гипотезы не считается новым видом |

### Acceptance criteria

Все тесты таблицы зелёные. `cargo clippy -p hdi-contract --all-targets -- -D warnings` чистый для этого крейта. В графе зависимостей крейта нет modem, taskd, shell, displayd. Образ panther не собирается заново.

### Threat / privacy impact

Контракт ещё не читает железо. Негативные тесты фиксируют границу,
через которую позже не пройдут запись в sysfs и строка оболочки.
Секретов в фикстурах нет.

### Rollback

Удалить член workspace `hdi-contract`, строку `workspace.dependencies` и каталог крейта. Другие пакеты не меняются.

### Evidence

2026-10-10, этот Windows-хост, toolchain репозитория `1.97.1`:
`cargo test -p hdi-contract --all-targets` — 12 passed.
`cargo clippy -p hdi-contract --all-targets -- -D warnings` — чисто.
`cargo fmt -p hdi-contract -- --check` — чисто.
R620 в этой сессии недоступен (SSH: Permission denied). Образ panther не собирался.
До `Done` не хватает записи в общий журнал после commit.

## Sprint HDI-01 — нормализатор и control

- Состояние: `Verify`
- Зависит от: HDI-00 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: сырой каталог снимка остаётся на диске оператора и в git не входит

### Goal

Редактированный пакет и детерминированный список устройств строятся
из фикстуры снимка. Секреты не доходят до входа модели. Control
отличает присутствующий VGA от отсутствующего `8086:0d26`.

### Current state

Контракт умеет отвергать запросы. Живого разбора `lspci` нет.

### Scope

Входит: разбор уже собранных текстовых файлов фикстуры; редактирование;
control-утверждения видов `device_present`, `device_absent`,
`driver_bound`.

Не входит: запуск `lspci` на машине разработчика как обязательный
тест, вызов модели, физический MacBook.

### Change

1. Фикстуры в `crates/hdi-contract/tests/fixtures/`:
   `nvidia_only`, `intel_present`, `secrets`, `unknown_product`.
   Это синтетика по форме реальных команд, не дамп ноутбука.
2. Normalizer копирует в `redacted/` только белый набор файлов и
   вычищает MAC, строки `iSerial`, поля serial/uuid.
3. Control по `pci.txt` выпускает утверждения со ссылкой на
   `evidence_id` файла.

### Test

| Тест | Ожидание |
|---|---|
| `redaction_strips_mac_and_iserial` | В `redacted/` нет MAC и `iSerial` из `secrets` |
| `redaction_omits_usb_verbose_and_full_dmesg` | Этих файлов нет в пакете модели |
| `control_sees_nvidia_and_absent_igd` | `nvidia_only`: discrete present, `8086:0d26` absent |
| `control_sees_igd_when_enumerated` | `intel_present`: `device_present` для `8086:0d26` |
| `control_does_not_emit_causal` | Вида `causal` у control нет |
| `unknown_product_has_no_mbp_claim` | Чужой DMI не получает утверждение про 11,3 |

### Acceptance criteria

Пакет `redacted/` на фикстуре `secrets` проходит поиск запрещённых
шаблонов с нулём совпадений. Control на двух графических фикстурах
даёт взаимоисключающие утверждения про `8086:0d26`. Discovery gain
относительно самого control равен нулю.

### Threat / privacy impact

Фикстура `secrets` содержит заведомо фальшивые MAC и серийники.
Реальные идентификаторы ноутбука в репозиторий не кладутся. Тест
падает, если редактор их пропускает.

### Rollback

Удалить normalizer, control и каталог `tests/fixtures`. Контракт HDI-00 остаётся.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1`:
`cargo test -p hdi-contract --all-targets` — 18 passed (HDI-00: 12, HDI-01: 6).
`cargo clippy -p hdi-contract --all-targets -- -D warnings` — чисто.
Фикстуры синтетические. Сырой снимок MacBook в git не входил.
R620 по-прежнему недоступен по SSH. Образ panther не собирался.

## Sprint HDI-02 — исполнитель read-only

- Состояние: `Verify`
- Зависит от: HDI-01 в Verify
- Архитектурные решения: ADR-426, вердикты ADR-260
- Рабочий fallback: исполнитель читает корень фикстуры, не системный `/sys` разработчика

### Goal

Разрешённая проба возвращает свидетельство из корня фикстуры.
Запрещённая проба получает Deny, пишется в журнал и не оставляет
нового файла свидетельства.

### Current state

Контракт отвергает плохой JSON. Процесса, который ходит в дерево
файлов по имени пробы, нет.

### Scope

Входит: `crates/hdi-exec`, журнал, префиксы `sysfs_read`, падение
исполнителя посередине пробы.

Не входит: PolicyEngine внутри модема, живой ноутбук, повышение
привилегий, запись куда-либо кроме каталога кампании.

### Change

1. Исполнитель получает корень фикстуры и каталог кампании.
2. Allow только после успешного разбора контракта и совпадения
   аргументов со схемой пробы.
3. Журнал: `pass`, имя пробы, вердикт, хеш результата или причина
   отказа. Тело отказа не содержит аргумент целиком, если он похож
   на путь за пределами корня.

### Test

| Тест | Ожидание |
|---|---|
| `allowlisted_pci_slot_reads_fixture` | Есть свидетельство и Allow |
| `sysfs_read_prefix_only` | Путь под разрешённым префиксом читается |
| `sysfs_read_parent_denied` | `..` — Deny, файл свидетельства не создан |
| `denied_probe_is_journaled` | В журнале ровно одна строка Deny |
| `executor_crash_does_not_widen_next` | Обрыв после Allow не превращает следующую запрещённую пробу в Allow |
| `no_write_syscall_templates` | В таблице проб нет argv с перенаправлением в sysfs или `modprobe` |

Последний тест читает таблицу шаблонов, а не устраивает запись на
машине разработчика.

### Acceptance criteria

На фикстуре исполняются все пробы белого списка v1 и ни одной вне
его. Журнал полон для Allow и Deny. Повторный запуск по тому же
каталогу не удваивает уже записанное свидетельство с тем же хешем.

### Threat / privacy impact

Исполнитель v1 не получает канал записи в ядро. Негативный тест
на `..` закрывает чтение файлов вне корня фикстуры. На хосте
разработчика системный `/sys` целью тестов не является.

### Rollback

Удалить член workspace `hdi-exec` и каталог крейта. `hdi-contract` остаётся.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1`:
`cargo test -p hdi-contract -p hdi-exec --all-targets` — contract 18 passed, `hdi-exec` 8 passed.
`cargo clippy -p hdi-contract -p hdi-exec --all-targets -- -D warnings` — чисто.
Исполнитель читает фикстуру, не системный `/sys`. Образ panther не собирался.
R620 по-прежнему недоступен по SSH.

## Sprint HDI-03 — пять проходов и метрики

- Состояние: `Verify`
- Зависит от: HDI-02 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: исследователь в тестах скриптовый; сетевого вызова нет

### Goal

Скриптовый исследователь за не более чем пять проходов меняет или
не меняет счёт Discovery gain, и метрики это отличают. Запечатанный
ключ в его вход не сериализуется.

### Current state

Одна проба исполняется. Цикла кампании и формул метрик нет.

### Scope

Входит: `crates/hdi-campaign`, интерфейс исследователя, два
скрипта (`repeater`, `hunter`), расчёт шести метрик на фикстуре.
Поле transferability на одной машине равно «не переносилось».

Не входит: облачная модель, локальная обученная модель, MacBook.

### Change

1. Цикл: пакет → исследователь → контракт → исполнитель →
   обновление пакета. Ошибка контракта завершает проход без
   исполнения и без повтора той же просьбы.
2. `repeater` каждый раз просит `pci_id`, который уже есть.
3. `hunter` один раз просит ещё не снятое разрешённое свидетельство,
   из которого верификатор HDI-04 сможет подтвердить факт. До HDI-04
   тест проверяет только то, что проба новая и лимит удержан.
4. Вход исследователя сериализуется в JSON. В нём нет
   `sealed_key` и нет текста файла запечатанного ключа.

### Test

| Тест | Ожидание |
|---|---|
| `repeater_gain_is_zero` | Пять одинаковых проб, gain 0, efficiency 0 |
| `hunter_requests_unseen_probe` | Первая просьба отсутствует в начальном пакете |
| `pass_cap_five` | Шестой вызов исследователя не происходит |
| `sealed_key_is_absent_from_investigator_input` | Сериализация не содержит ключ |
| `protocol_fault_stops_pass` | Ответ не-JSON не исполняется |
| `metrics_cover_definitions` | Поля таблицы ADR-426 присутствуют и конечны |

### Acceptance criteria

Оба скрипта заканчиваются со статусом кампании, а не паникой.
`repeater` не увеличивает gain. Лимит пяти проходов соблюдён на
исследователе, который просит бесконечно. Вход модели в тесте
собирается без сети.

### Threat / privacy impact

Скрипты не содержат секретов и не открывают сокет. Подмена
исследователя на процесс с сетью этим спринтом не включается.

### Rollback

Удалить член workspace `hdi-campaign` и каталог крейта. `hdi-exec` остаётся.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1`:
`cargo test -p hdi-campaign --all-targets` — 6 passed.
`cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
Исследователь скриптовый, сокета нет. Образ panther не собирался.

## Sprint HDI-04 — верификатор и знание

- Состояние: `Verify`
- Зависит от: HDI-03 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: опровергнутое утверждение остаётся в журнале гипотез и не попадает в подтверждённое знание

### Goal

Вердикт верификатора на фикстурах совпадает с запечатанными исходами
A/B/C. Знание машины A не становится фактом машины B. Записи
`driver_gap` не содержат команды загрузки модуля.

### Current state

Метрики считаются. Подтверждение по ключу и переносимость не
записаны как знание.

### Scope

Входит: сравнение с ключом фикстуры, виды `negative_recipe`,
`driver_gap`, `insufficient_evidence`, пометка машины на записи.

Не входит: применение рецепта, генерация исходников драйвера,
второй физический компьютер.

### Change

1. Ключ фикстуры — отдельный файл рядом со снимком, не внутри
   `redacted/`.
2. Исход A на `nvidia_only`: подтверждённый `negative_recipe`,
   `driver_gap` для отсутствующего `8086:0d26` опровергнут.
3. Исход B на `intel_present`: `negative_recipe` про скрытую
   прошивкой iGPU опровергнут.
4. Исход C на `unknown_product`: идентичность —
   `insufficient_evidence`, исход A не копируется.
5. Запись знания несёт `machine_id`. Чтение знания для другого
   `machine_id` не добавляет утверждений в control этой машины.

### Test

| Тест | Ожидание |
|---|---|
| `outcome_a_negative_recipe` | Подтверждён отрицательный рецепт, gain больше нуля только если control этот вид не выпускал |
| `outcome_a_rejects_driver_gap` | `driver_gap` при отсутствующем устройстве опровергнут |
| `outcome_b_refutes_hidden_gpu` | Присутствующий `8086:0d26` опровергает исход A |
| `outcome_c_insufficient_identity` | Чужой продукт не получает рецепт 11,3 |
| `causal_without_evidence_not_confirmed` | Голый `causal` не входит в знание |
| `knowledge_does_not_transfer` | Факт `machine-a` отсутствует в утверждении control `machine-b` |
| `driver_gap_has_no_insmod` | В записи нет `insmod`, `modprobe`, пути `.ko` |

### Acceptance criteria

Три фикстуры дают три разных вердикта идентичности или графики.
Перенос между двумя синтетическими машинами пуст. Ни одна запись
знания не является исполняемой строкой.

### Threat / privacy impact

Подтверждённое знание — структурированные вид, субъект и значение.
Сырой `dmesg` в запись знания не копируется. Опровергнутая гипотеза
остаётся аудитом, но не инструкцией к действию.

### Rollback

Удалить `verify` из `hdi-campaign` и файлы `key.txt` у фикстур.
Цикл HDI-03 остаётся.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1`:
`cargo test -p hdi-campaign --all-targets` — 13 passed (HDI-03: 6, HDI-04: 7).
`cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
Ключ фикстуры не попадает в `redacted/`. Образ panther не собирался.

## Sprint HDI-05 — намерение SaaiOS

- Состояние: `Verify`
- Зависит от: HDI-04 в Verify
- Архитектурные решения: ADR-426, ADR-120, ADR-260, ADR-276
- Рабочий fallback: незарегистрированное действие остаётся Unsupported, как до этого спринта

### Goal

Кампания на host стартует из Intent человека или из выключенного по
умолчанию предусловия работника. Загрузка, udev и чужое действие
кампанию не создают. `adapt_hardware` остаётся AskUser и в v1 не
исполняется.

### Current state

Лаборатория запускается файлом намерения в тестах. `saai-taskd` о
ней не знает. Предусловий ОС нет.

### Scope

Входит: разбор явного `semantic_action_id`; одно предусловие
`graphics_class_incomplete` со значением `enabled=false`; журнал
созданного Intent; отказ исполнителя адаптации.

Не входит: пункт меню, рисунок нового экрана, автозапуск taskd,
`native-init`, включение предусловия на panther, физическое
подтверждение на телефоне.

### Change

1. Явный идентификатор `investigate_hardware` резолвится в действие
   без вызова модели (правило ADR-120 для explicit id).
2. Текст без явного id на этом спринте лабораторию не угадывает и
   уходит прежним путём diagnose либо Unsupported, как решит уже
   существующий резолвер для незнакомой фразы. Новый угадыватель
   не добавляется.
3. Worker создаёт Intent только если предусловие включено и
   наблюдение совпало. При `enabled=false` наблюдение «графики
   нет» Intent не пишет.
4. Действие `adapt_hardware` получает AskUser. Тест не шлёт
   подтверждение, и исполнитель модулей не вызывается.

### Test

| Тест | Ожидание |
|---|---|
| `explicit_intent_starts_one_campaign` | Один Intent — одна кампания |
| `free_text_does_not_start_hdi` | Фраза без explicit id не создаёт кампанию HDI |
| `boot_fixture_creates_no_campaign` | Событие загрузки само по себе пусто |
| `disabled_precondition_creates_no_intent` | `enabled=false` — ноль Intent |
| `enabled_precondition_matches_once` | Включённое в тесте правило пишет один Intent и не пишет второй на то же наблюдение |
| `worker_intent_names_precondition` | У worker-Intent заполнен `precondition_id` |
| `adapt_hardware_asks_user` | Вердикт AskUser, модуль не загружен |
| `unknown_action_still_unsupported` | Старое неизвестное действие не стало HDI |

### Acceptance criteria

Существующие тесты `intent-resolution` и `saai-taskd` остаются
зелёными. Новый путь не добавлен в `native-init`. Предусловие в
поставляемых данных выключено. Образ телефона не меняется.

### Threat / privacy impact

Включённое предусловие — единственный способ ОС начать кампанию, и
по умолчанию оно выключено. Worker не получает право писать в
систему: адаптация спрашивает человека и в этом спринте не
продолжается. Чужой Intent не расширяет белый список проб.

### Rollback

Убрать регистрацию двух действий и предусловие. Крейты HDI-00…04
остаются вызываемыми из своих тестов и из файла намерения.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --all-targets` — 22 passed
(HDI-03: 6, HDI-04: 7, HDI-05: 9).
`cargo test -p intent-resolution --all-targets` — 16 passed,
исходники резолвера не менялись.
`cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
`SHIPPED_PRECONDITIONS[0].enabled` остаётся `false`.
`native-init.c` не содержит `investigate_hardware`.
`saai-taskd` этим спринтом не менялся. На этом Windows-хосте
его тесты не пересобираются: клиент использует `tokio::net::unix`.
Образ panther не собирался.

## Sprint HDI-06 — один прогон MacBookPro11,3

- Состояние: `Backlog`
- Зависит от: HDI-04 в Verify; HDI-05 желателен, но команда с файлом
  намерения достаточна. Снимок `redacted/` с машины. Аппаратно
  изменяющий эксперимент на Pixel в этот момент не требуется
  останавливать: прогон ничего не прошивает и системные файлы
  ноутбука не меняет.
- Архитектурные решения: ADR-426. Ключ:
  [HDI-SEALED-KEY-mbp113.md](HDI-SEALED-KEY-mbp113.md)
- Рабочий fallback: Debian остаётся как была. Нет юнита, нет
  модуля, нет правки загрузчика.

### Goal

Одна сознательная команда сравнивает control, проход 1 и проход 5
на реальном редактированном снимке. В журнал записаны метрики и
исход A, B или C. Отрицательный рецепт принимается как успех
измерения.

### Current state

Снимка MacBookPro11,3 нет, спринт остаётся в Backlog. 2026-10-10
по пакету Pixel 7 `C:\Users\Admin\saai-hdi-panther-20261010-163813`
(в git не входит) прогнан скриптовый Hunter после HDI-12: `intent.txt` есть,
5 проходов, 5 Allow, `pass_gain=[5, 9, 9, 9, 9]`,
efficiency 1.8, accuracy 0, coverage 1, `not_transferred`.
Проход 1 разделяет `drm:card0`, `drm:card0-DSI-1`, `drm:card0-Writeback-1`,
`drm:renderD128` и отсутствие PCI-класса `[0300]`. Проход 2 привязывает
каждый из этих узлов к `exynos_drm`: путь `renderD128` называет
`exynos-drm`, поэтому `mali_kbase` драйвером рендера не записывается.
Проход 3 читает
`pci_drivers`: control получает 9 фактов, включая `s51xx` и `pcieh`,
gain лаборатории остаётся 9. Проходы 4–5 повторяют `pci_id`. Предложение адаптации:
драйвер `exynos_drm` уже привязан,
модуль не загружается, `driver_gap` не пишется. Ключ MacBookPro11,3 даёт исход C
(`IdentityInsufficient`): отрицательный рецепт Iris Pro на этот
снимок не переносится. Исторические сведения о ноутбуке в пакет
исследователя не отправлялись.

### Scope

Входит: сбор снимка командой из раздела ниже; один запуск кампании;
отчёт метрик в журнале спринта.

Не входит: второй ноутбук, включение iGPU, установка драйвера,
облачная модель, если локальный скриптовый прогон ещё не отделён
от ключа. Подключение большой модели — отдельная запись в этом
файле после того, как скриптовый прогон HDI-03 уже измерен. Иначе
нельзя отличить новизну модели от новизны цикла.

### Сбор снимка

Команда на Debian только читает. Каталог `raw/` остаётся у оператора.
В кампанию уходит `redacted/`. Архив в git не кладётся.

```bash
#!/usr/bin/env bash
set -euo pipefail

OUT="${1:-$HOME/saai-hdi-$(date +%Y%m%d-%H%M%S)}"
mkdir -p "$OUT/raw" "$OUT/redacted"

capture() {
  local name="$1"
  shift
  if command -v "$1" >/dev/null 2>&1; then
    "$@" > "$OUT/raw/$name.txt" 2>&1 || true
  fi
}

capture uname uname -a
capture cpu lscpu
capture pci lspci -nn
capture pci_drivers lspci -nnk
capture pci_tree lspci -tv
capture usb lsusb
capture usb_tree lsusb -t
capture block lsblk -o NAME,SIZE,TYPE,MODEL,TRAN
capture modules lsmod
capture hardware lshw -short
capture acpi_tables ls /sys/firmware/acpi/tables
capture drm ls -l /sys/class/drm
capture pci_sysfs ls /sys/bus/pci/devices
capture net ls /sys/class/net

journalctl -k -b --no-pager -p warning > "$OUT/raw/kernel_warnings.txt" 2>&1 || true
dmesg > "$OUT/raw/dmesg.txt" 2>&1 || true

{
  for f in product_name product_version board_name bios_version bios_date sys_vendor; do
    p="/sys/class/dmi/id/$f"
    if [[ -r "$p" ]]; then
      printf '%s: %s\n' "$f" "$(cat "$p")"
    fi
  done
} > "$OUT/raw/dmi.txt"

cp "$OUT/raw/uname.txt" "$OUT/raw/cpu.txt" "$OUT/raw/pci.txt" \
   "$OUT/raw/pci_drivers.txt" "$OUT/raw/pci_tree.txt" \
   "$OUT/raw/usb.txt" "$OUT/raw/usb_tree.txt" \
   "$OUT/raw/block.txt" "$OUT/raw/modules.txt" \
   "$OUT/raw/drm.txt" "$OUT/raw/pci_sysfs.txt" \
   "$OUT/raw/dmi.txt" "$OUT/redacted/" 2>/dev/null || true

sed -E 's/([0-9a-fA-F]{2}:){5}[0-9a-fA-F]{2}/[mac]/g' \
  "$OUT/raw/kernel_warnings.txt" > "$OUT/redacted/kernel_warnings.txt" || true

printf '%s\n' 'investigate_hardware' > "$OUT/intent.txt"
tar -czf "$OUT.tar.gz" -C "$(dirname "$OUT")" "$(basename "$OUT")"
echo "Saved: $OUT.tar.gz"
```

Файл `intent.txt` — сознательный запуск. Без него HDI-06 кампанию не
начинает. Нормализатор HDI-01 всё равно прогоняет `redacted/` своей
редакцией; `sed` в скрипте только уменьшает сырой журнал заранее.

### Change

1. Оператор на Debian выполняет сбор выше. Архив не коммитится.
2. На host кампания идёт по `redacted/` с исследователем, который
   к этому моменту разрешён отдельной записью (скрипт или модель).
3. Верификатор применяет запечатанный ключ только после пятого
   прохода.
4. В Evidence этого спринта попадают метрики, хеш редактированного
   пакета, имя исследователя и исход A/B/C. Сырой снимок и
   серийные номера не попадают.

### Test

До ноутбука обязательны зелёные host-тесты HDI-00…04. На самом
прогоне дополнительно:

| Проверка | Ожидание |
|---|---|
| Команда без файла намерения | Кампания не начинается, система не изменена |
| Пакет модели | Нет MAC, серийников и текста запечатанного ключа |
| Control против прохода 1 | Проход 1 не получает gain за повторение `lspci` |
| Проход 5 | Не больше пяти исполненных проб |
| Итог | Один из исходов A, B, C; при отсутствии свидетельства причины — `insufficient_evidence`, а не выдуманный gmux |
| Машина после прогона | Тот же набор модулей, загрузчик не менялся |

Последняя строка — сравнение `lsmod` и отсутствие команд записи в
журнале кампании, а не ощущение, что «ничего не трогали».

### Acceptance criteria

Таблица метрик заполнена числами. Ложные гипотезы перечислены.
Полезность дополнительных проб видна из разницы gain прохода 1 и
прохода 5. Прогон не объявляет графику «починенной». Прогон не
объявляет модем и телефон затронутыми.

### Threat / privacy impact

Архив может содержать идентификаторы в `raw/`. Коммитится только
отчёт без них. Модель, если её подключат отдельным решением, видит
`redacted/`. Команда не выполняется от загрузки.

### Rollback

Прекратить команду. Каталог кампании в домашнем каталоге оператора
удаляется. Пакеты ОС и загрузчик не откатываются, потому что прогон
их не ставит.

## Sprint HDI-07 — утверждение из свидетельства

- Состояние: `Verify`
- Зависит от: HDI-03 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: исследователь без `observe` записывает ноль утверждений, gain остаётся 0

### Goal

Скриптовый исследователь записывает один факт, которого нет у control,
только после того, как файл свидетельства уже лежит в пакете.
Повтор уже известной пробы gain не увеличивает.

### Current state

До этого спринта цикл вызывал `propose` и держал список утверждений
пустым. На снимке Pixel gain был 0 при пяти Allow.

### Scope

Входит: метод `observe` у скриптового Hunter; отказ хранить
утверждение без evidence и утверждение с `insmod` / `modprobe` / `.ko`;
ряд gain по проходам.

Не входит: облачная или локальная модель, ключ MacBook в пакете
исследователя, загрузка модуля, правка `native-init`, закрытие HDI-06.

### Change

1. После каждой пробы цикл спрашивает `observe` по уже собранным файлам.
2. Hunter, увидев в `drm.txt` имя `card0`, пишет одно
   `device_present` / `drm:card0` / `card0` со свидетельством `drm.txt`.
3. Повтор того же факта не добавляет вторую запись.
4. Repeater по-прежнему ничего не пишет.

### Test

| Тест | Ожидание |
|---|---|
| `repeater_still_has_zero_gain_each_pass` | Пять нулей |
| `hunter_drm_claim_gains_once` | Gain 1 с первого прохода и дальше не растёт |
| `executable_claim_is_not_stored` | Строка загрузки модуля не попадает в запись |
| `claim_without_evidence_is_not_stored` | Пустой `evidence_ids` отбрасывается |

### Acceptance criteria

`cargo test -p hdi-campaign --all-targets` зелёный. На пакете Pixel
проходы 1 и 5 различаются только тем, что новый факт появился на
проходе 1. Исход ключа MacBook остаётся C. Образ телефона не меняется.

### Threat / privacy impact

В утверждение попадает имя `card0`, не содержимое файла целиком.
Ключ стенда в JSON исследователя не копируется.

### Rollback

Удалить `observe` и снова передавать в метрики пустой список.
Пробы HDI-03 остаются.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --all-targets` — 26 passed
(HDI-03: 6, HDI-04: 7, HDI-05: 9, HDI-07: 4).
`cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
Повторный прогон пакета Pixel: `pass_gain=[1, 1, 1, 1, 1]`,
efficiency 0.2, coverage 1, вердикт `IdentityInsufficient`.
Образ panther не собирался.

## Sprint HDI-08 — привязка DRM по модулю

- Состояние: `Verify`
- Зависит от: HDI-07 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: если `modules.txt` нет или имя не совпало, остаётся только `drm:card0`

### Goal

Второй проход читает список модулей. `driver_bound` для `drm:card0`
появляется только когда токен пути в `drm.txt` совпадает с именем
загруженного модуля. Иначе привязка не записывается.

### Current state

После HDI-07 проходы 2–5 повторяли `pci_id`. Факт карты был, факта
драйвера не было.

### Scope

Входит: порядок проб `drm_class`, затем `modules`, затем повтор
`pci_id`; нормализация `-` в `_`; выбор самого длинного совпавшего
имени; два свидетельства у `driver_bound`.

Не входит: `driver_gap`, предложение `insmod`, облачная модель,
ключ MacBook, закрытие HDI-06, правка ядра и `native-init`.

### Change

1. Hunter не просит файл, который уже есть в пакете.
2. Совпадение `vendor-drm` в пути и `vendor_drm` в модулях даёт
   `driver_bound` со значением `vendor_drm`.
3. Путь без такого модуля оставляет только `device_present`.
4. Имя с точкой, включая `.ko`, не сравнивается и не записывается.

### Test

| Тест | Ожидание |
|---|---|
| `hunter_reads_modules_before_repeating_pci` | Проход 2 — `modules`, на фикстуре без совпадения gain остаётся 1 |
| `hunter_binds_drm_driver_from_module` | Gain 1, затем 2; значение `vendor_drm`; свидетельства `drm.txt` и `modules.txt` |
| `path_without_loaded_module_does_not_bind` | Нет `driver_bound` |
| `longer_module_name_wins` | Из `drm` и `vendor_drm` выбирается длинное имя |

### Acceptance criteria

На снимке Pixel проход 2 поднимает gain до 2 и дальше не растит.
Значение привязки — `exynos_drm`. Исход ключа MacBook остаётся C.
Образ телефона не меняется.

### Threat / privacy impact

В утверждение попадает имя модуля, не список модулей целиком и не
путь sysfs. Загрузка модуля не предлагается.

### Rollback

Вернуть Hunter к одной пробе `drm_class` и одному утверждению
`drm:card0`.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --all-targets` — 30 passed
(HDI-03: 6, HDI-04: 7, HDI-05: 9, HDI-07: 4, HDI-08: 4).
`cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
Повторный прогон пакета Pixel: `pass_gain=[1, 2, 2, 2, 2]`,
efficiency 0.4, coverage 1, привязка `exynos_drm`,
вердикт `IdentityInsufficient`. Образ panther не собирался.

## Sprint HDI-09 — нет PCI VGA

- Состояние: `Verify`
- Зависит от: HDI-08 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: строка `[0300]` в `pci.txt` не добавляет утверждение

### Goal

Если в снимке PCI нет класса VGA `[0300]`, Hunter записывает одно
`device_absent`. Если класс есть, control уже описал устройство, и
новое утверждение не пишется.

### Current state

На Pixel графика видна как `drm:card0` / `exynos_drm`. В PCI четыре
функции, ни одна не `[0300]`. Этот факт control не формулирует:
control отмечает только конкретный DID `8086:0d26`.

### Scope

Входит: одно отсутствие `pci:class:0300` со свидетельством `pci.txt`.

Не входит: имя GPU, отрицательный рецепт Iris Pro, `driver_gap`,
загрузка модуля, модель, закрытие HDI-06.

### Change

1. При наличии `pci.txt` без подстроки `[0300]` пишется
   `device_absent` / `pci:class:0300` / `0300`.
2. Подстрока `[0300]` гасит это утверждение.
3. Повтор прохода не добавляет вторую копию.

### Test

| Тест | Ожидание |
|---|---|
| `missing_vga_class_is_one_absence` | Одно отсутствие, свидетельство `pci.txt` |
| `present_vga_class_adds_no_claim` | Пустой список |
| `absence_does_not_name_a_gpu` | В утверждении нет `8086`, `Iris` и `.ko` |

### Acceptance criteria

На снимке Pixel gain становится 2 уже на проходе 1 и 3 на проходе 2.
Исход ключа MacBook остаётся C. Образ телефона не меняется.

### Threat / privacy impact

В утверждение попадает номер класса PCI, не список устройств и не
идентификатор из запечатанного ключа.

### Rollback

Убрать `pci_vga_absent_claim`. Утверждения HDI-07 и HDI-08 остаются.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --all-targets` — 33 passed
(HDI-03: 6, HDI-04: 7, HDI-05: 9, HDI-07: 4, HDI-08: 4, HDI-09: 3).
`cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
Повторный прогон пакета Pixel: `pass_gain=[2, 3, 3, 3, 3]`,
efficiency 0.6, coverage 1, вердикт `IdentityInsufficient`.
Образ panther не собирался.

## Sprint HDI-10 — адаптация без загрузки

- Состояние: `Verify`
- Зависит от: HDI-08 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: `adapt_hardware` по-прежнему AskUser и модуль не грузит

### Goal

По уже записанным утверждениям лаборатория либо называет привязанный
драйвер, либо пишет `driver_gap` со значением `unbound`. Ни один исход
не загружает модуль и не содержит команду загрузки.

### Current state

`request_adaptation` всегда отвечает AskUser и `loaded_module=false`,
не глядя на утверждения кампании. На Pixel драйвер `exynos_drm` уже
записан как `driver_bound`.

### Scope

Входит: функция предложения по списку утверждений; три исхода
`already_bound`, `driver_gap`, `insufficient`.

Не входит: подтверждение пользователя, `insmod`, правка gain,
включение предусловия, закрытие HDI-06, модель.

### Change

1. `driver_bound` для `drm:card0` с обычным именем модуля даёт
   `already_bound` и пустой gap.
2. Есть только `device_present` карты — пишется `driver_gap` /
   `unbound` со свидетельством карты.
3. Имя с `.ko`, `insmod` или `modprobe` не считается привязкой.
4. Нет карты — `insufficient`.

### Test

| Тест | Ожидание |
|---|---|
| `bound_driver_does_not_propose_a_gap` | `vendor_drm`, gap пуст, gain кампании не меняется |
| `unbound_card_is_a_gap_without_a_load` | `driver_gap` / `unbound`, свидетельство `drm.txt` |
| `no_card_is_insufficient` | Нет gap |
| `executable_driver_name_is_not_bound` | Имя `exynos.ko` не становится привязкой |

### Acceptance criteria

На снимке Pixel исход `already_bound` / `exynos_drm`,
`loaded_module=false`. Gain остаётся `[2, 3, 3, 3, 3]`.
Ключ MacBook остаётся C. Образ телефона не меняется.

### Threat / privacy impact

В предложение попадает имя уже записанного модуля. Команда загрузки
не строится и не исполняется.

### Rollback

Удалить `adapt`. `request_adaptation` HDI-05 остаётся.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --all-targets` — 37 passed
(HDI-03: 6, HDI-04: 7, HDI-05: 9, HDI-07: 4, HDI-08: 4, HDI-09: 3, HDI-10: 4).
`cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
Повторный прогон пакета Pixel: gain 3, адаптация `already_bound`
`exynos_drm`, `loaded_module=false`, gap нет, вердикт
`IdentityInsufficient`. Образ panther не собирался.

## Sprint HDI-11 — проба pci_drivers

- Состояние: `Verify`
- Зависит от: HDI-08 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: без файла `pci_drivers.txt` проба получает Deny, gain не меняется

### Goal

Третий проход читает уже снятый `pci_drivers.txt`. Привязки PCI
становятся фактами control. Hunter их не копирует, поэтому discovery
gain не растёт.

### Current state

Control умел читать `pci_drivers.txt`, а белый список пробы с таким
именем не содержал. Кампания этот файл не запрашивала.

### Scope

Входит: имя пробы `pci_drivers`, чтение одного файла фикстуры,
порядок Hunter после `modules`. Снятие UTF-8 BOM, чтобы первая
строка PCI не пропадала.

Не входит: новая команда shell, `modprobe`, рост gain за повторение
control, закрытие HDI-06, модель.

### Change

1. Белый список принимает `pci_drivers` и читает `pci_drivers.txt`.
2. Hunter просит этот файл один раз, затем снова `pci_id`.
3. Факты `driver_bound` из этого файла остаются в control.
4. Ведущий BOM не прячет первое устройство.

### Test

| Тест | Ожидание |
|---|---|
| `third_pass_reads_pci_drivers_without_raising_gain` | Проход 3 — `pci_drivers`, gain остаётся 1, в пакете есть `nouveau` |
| `pci_driver_fact_stays_in_control` | `pcieport` не входит в утверждения Hunter, gain 3 |
| `bom_does_not_hide_the_first_pci_device` | Первая строка с BOM даёт устройство и драйвер |

### Acceptance criteria

На снимке Pixel control насчитывает 9 фактов, gain остаётся
`[2, 3, 3, 3, 3]`. Среди фактов control есть `s51xx` и `pcieh`.
Адаптация экрана остаётся `already_bound`. Ключ MacBook остаётся C.
Образ телефона не меняется.

### Threat / privacy impact

В control попадают имена драйверов PCI из уже редактированного
файла. Новых идентификаторов проба не читает.

### Rollback

Удалить `PciDrivers` из белого списка и запрос Hunter. Control
по-прежнему понимает файл, если он лежит в пакете заранее.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-contract -p hdi-exec -p hdi-campaign --all-targets`
зелёный (в том числе HDI-11: 2, BOM-тест в HDI-01, whitelist HDI-02: 12 имён).
`cargo clippy` по этим трём крейтам с `-D warnings` — чисто.
Повторный прогон пакета Pixel: gain 3, `control_facts=9`,
адаптация `already_bound` `exynos_drm`, вердикт `IdentityInsufficient`.
Образ panther не собирался.

## Sprint HDI-12 — дисплей и рендер отдельно

- Состояние: `Verify`
- Зависит от: HDI-08 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: строка без имени узла не даёт утверждения

### Goal

Каждая строка `drm.txt` с именем `card0`, `card0-…` или `renderD…`
становится своим устройством. Драйвер берётся только из токенов этой
строки. Загруженный модуль, которого на строке нет, не привязывается.

### Current state

Hunter писал одну карту `drm:card0` и один `exynos_drm`. На Pixel в
том же файле есть `card0-DSI-1`, `card0-Writeback-1` и `renderD128`.
Путь рендера называет `exynos-drm`. Модуль `mali_kbase` загружен, но
в пути не встречается.

### Scope

Входит: отдельные `device_present` и `driver_bound` по строкам DRM.
Снятие BOM в начале `drm.txt`.

Не входит: ключ стенда Pixel, узлы TPU/ISP/AoC, утверждение «Mali
владеет renderD128» без пути, закрытие HDI-06, загрузка модуля.

### Change

1. Имя слева от `->` — субъект `drm:<имя>`.
2. Самый длинный токен строки, совпавший с модулем, — `driver_bound`
   этого узла.
3. `mali_kbase` на строке `vendor-drm` не побеждает `vendor_drm`.

### Test

| Тест | Ожидание |
|---|---|
| `display_and_render_are_separate_nodes` | `card0-DSI-1` и `renderD128` присутствуют и привязаны к `vendor_drm`; gain 4, затем 7 |
| `loaded_mali_is_not_the_render_driver` | Значение привязки `vendor_drm`, не `mali_kbase` |
| `bom_does_not_hide_card0` | Первая строка с BOM всё ещё `drm:card0` |

### Acceptance criteria

На снимке Pixel gain `[5, 9, 9, 9, 9]`. Четыре узла привязаны к
`exynos_drm`. `mali_kbase` драйвером рендера не записан. Адаптация
карты остаётся `already_bound`. Ключ MacBook остаётся C.
Образ телефона не меняется.

### Threat / privacy impact

В утверждения попадают имена узлов DRM и имя модуля из пути.
Список модулей целиком не копируется.

### Rollback

Вернуть одно утверждение `drm:card0` и одну привязку на весь файл.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --all-targets` — 42 passed
(HDI-12: 3). `cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
Повторный прогон пакета Pixel: `pass_gain=[5, 9, 9, 9, 9]`,
efficiency 1.8, `control_facts=9`, адаптация `already_bound`
`exynos_drm`, вердикт `IdentityInsufficient`. Образ panther не собирался.

## Каталог фикстур

Синтетика, безопасная для git:

| Имя | Зачем |
|---|---|
| `nvidia_only` | VGA NVIDIA есть, `00:02.0` нет, DMI `MacBookPro11,3` |
| `intel_present` | `8086:0d26` есть |
| `secrets` | Фальшивые MAC, `iSerial`, uuid |
| `unknown_product` | DMI другого имени |
| `machine-b` | Второе шасси для отказа переноса |

Живой снимок MacBook в фикстуры не копируется.

## Порядок доказательств

1. HDI-00 собирается и тестируется на host до любых остальных
   крейтов.
2. HDI-01…04 остаются host-only. Образ panther для них не собирается.
3. HDI-05 не включается в init и не прошивается.
4. HDI-06 ждёт снимок и зелёных тестов. Большая модель подключается
   только отдельной правкой этого файла, после скриптового прогона,
   тем же контрактом пробы.

Состояние `Done` у любого спринта требует записи Evidence: команда
тестов, результат, известный предел. Зелёная сборка телефона сюда
не подставляется вместо этих доказательств.
