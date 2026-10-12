# HDI — лаборатория аппаратного исследования

Статус трека: **HDI-25 в Verify на host. HDI-06 в Verify: снимок MacBookPro11,3, исход B.**
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

Непрочитанный файл не становится `DeviceAbsent`: это
`InsufficientEvidence` со значением `unread`. `DeviceAbsent` остаётся
только когда названный файл уже лежит в пакете. Если каталог фикстуры
меняется после открытия кампании, статус `PackageChanged` и следующие
пробы не исполняются. Отрицательный рецепт `8086:0d26` закрыт только
при исходе A и свидетельствах `pci.txt` и `dmi.txt`. Исход C и
выдуманный `gmux` его не закрывают.

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

- Состояние: `Verify`
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

2026-10-12 снимок MacBookPro11,3 снят и прогнан; числа — в Evidence
ниже. 2026-10-10
по пакету Pixel 7 `C:\Users\Admin\saai-hdi-panther-20261010-163813`
(в git не входит) прогнан скриптовый Hunter после HDI-16: `intent.txt` есть,
5 проходов, 5 Allow, `pass_gain=[5, 9, 9, 14, 20]`,
efficiency 4, accuracy 0, coverage 1, `not_transferred`.
Проход 1 разделяет `drm:card0`, `drm:card0-DSI-1`, `drm:card0-Writeback-1`,
`drm:renderD128` и отсутствие PCI-класса `[0300]`. Проход 2 привязывает
каждый из этих узлов к `exynos_drm`: путь `renderD128` называет
`exynos-drm`, поэтому `mali_kbase` драйвером рендера не записывается.
Проход 3 читает
`pci_drivers`: control получает 9 фактов, включая `s51xx` и `pcieh`,
gain лаборатории остаётся 9. Проход 4 читает отобранный `platform.txt`:
`28000000.mali`, `100b0000.TPU`, `100a0000.ISP`, `19000000.aoc` и пустой
`udc:`. Проход 5 читает ссылки драйверов: `28000000.mali` → `mali`
(модуль `mali_kbase`), `19000000.aoc` → `aoc` (модуль `aoc_core`), у TPU и
ISP ссылки нет. Gain становится 20.
Предложение адаптации:
драйвер `exynos_drm` уже привязан,
модуль не загружается, `driver_gap` не пишется. Ключ Pixel даёт
`PlatformGraphicsMatched`. Ключ MacBookPro11,3 на том же снимке даёт исход C
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

### Evidence

2026-10-12. Машина: Debian `6.12.107+deb13-amd64`, DMI `product_name: MacBookPro11,3`. Снимок снят read-only скриптом этого спринта в `~/saai-hdi-20261012-135101` на машине. Локальная копия для кампании — `C:\Users\Admin\saai-hdi-mbp113-20261012-135101`, в git не входит. `intent.txt` равен `investigate_hardware`. Нормализатор HDI-01 прогнал `raw/` в `redacted/` (13 файлов). В `redacted/` нет `iSerial`, `Iris Pro` и `sealed_key`. Хеш пакета `81d712eeb7608bec7ef7fbf78b320e097d823d641934ab21e2b2d1c7b395ce09`. Ключ стенда лежал рядом со снимком, не внутри `redacted/`.

Исследователь — скриптовый Hunter. Пять вызовов. Исполнено 3 Allow: `drm_class`, `modules`, `pci_drivers`. Один Deny: `platform_nodes`, причина `missing_fixture` (в сборе снимка нет `platform.txt`). Пятый проход повторил уже отклонённую пробу и не исполнялся. `pass_gain=[0, 0, 0, 0, 0]`, efficiency 0, accuracy 0, coverage 0, `false_hypotheses=0`, `not_transferred`. Ложных гипотез нет, в том числе нет утверждения про `gmux`. Control уже держал 50 фактов PCI и привязок драйверов, поэтому повтор `lspci` gain не дал. Строки `ls -l /sys/class/drm` начинаются с режима файла, а Hunter читает имя узла первым токеном, поэтому DRM-утверждений он не записал.

Исход B, `IgpuPresent`. `8086:0d26` есть на `0000:00:02.0`, драйвер в использовании `i915`. Дискретная `10de:0fe9` на `0000:01:00.0`, драйвер `nouveau`. Отрицательный рецепт не закрыт (`negative_recipe_closed=false`): устройство на шине есть, вопрос про прошивку не ставится. Адаптация `Insufficient`, `loaded_module=false`. Графика не объявлена починенной: привязки уже были у запущенного Debian, лаборатория модуль не загружала.

Журнал кампании не содержит `insmod`, `modprobe`, `rmmod` и `.ko`. SHA-256 `/proc/modules` до сбора, после сбора и после прогона один и тот же: `ee4d7628b52c3f1e5da7ba611e4cf1992515bf54da0ead6327f9e5e8747f6133`. Загрузчик и NVRAM не менялись. Телефон и модемный трек не затронуты.

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

## Sprint HDI-13 — ключ стенда Pixel

- Состояние: `Verify`
- Зависит от: HDI-12 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: чужая модель или смесь полей MacBook даёт отказ ключа

### Goal

Ключ Pixel читает модель из device tree и проверяет два узла DRM.
Ключ MacBook на том же снимке остаётся исходом C.

### Current state

Ключ MacBookPro11,3 ищет DMI `product_name` и `8086:0d26`. На Pixel
файла DMI нет, поэтому тот ключ всегда даёт `IdentityInsufficient`.
Модель платы есть в `devicetree-model.txt`: `GS201 PANTHER MP based on GS201`.

### Scope

Входит: второй формат ключа (`expected_model`, `display_node`,
`display_driver`, `render_node`, `render_driver`). Вердикты
`PlatformGraphicsMatched` и `GraphicsMismatch`.

Не входит: замена ключа MacBook, запись ключа внутрь `redacted/`,
привязка Mali к `renderD128`, закрытие HDI-06, загрузка модуля.

### Change

1. Поле `expected_model` выбирает проверку device tree. Поля
   `expected_product` или `igd_id` рядом с ним делают ключ недействительным.
2. Совпадение модели, обоих узлов и отсутствие `[0300]` подтверждает
   две привязки и отсутствие VGA-класса.
3. Другая модель подтверждает только `dt:model` как
   `InsufficientEvidence`. Чужой драйвер на строке узла — `GraphicsMismatch`.

### Test

| Тест | Ожидание |
|---|---|
| `pixel_key_matches_display_and_render` | `PlatformGraphicsMatched`; подтверждены `card0-DSI-1` и `renderD128` на `exynos_drm`, плюс отсутствие `0300` |
| `wrong_model_stays_insufficient` | `IdentityInsufficient`, субъект `dt:model`, привязок нет |
| `mali_render_driver_mismatches` | `GraphicsMismatch`; заявка `mali_kbase` на `renderD128` отклонена |

### Acceptance criteria

Ключ `pixel-key.txt` на снимке Pixel даёт `PlatformGraphicsMatched`.
Ключ `verifier-key.txt` на том же снимке даёт `IdentityInsufficient`.
Образ телефона не меняется.

### Threat / privacy impact

Ключ хранит публичные имена узлов и модулей. Он не лежит в пакете
исследователя и не содержит серийных идентификаторов.

### Rollback

Оставить только ключ PCI/DMI и исходы A/B/C.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --all-targets` — 46 passed
(HDI-13: 3). `cargo clippy -p hdi-contract -p hdi-exec -p hdi-campaign --all-targets -- -D warnings` — чисто.
Ключ Pixel: `PlatformGraphicsMatched`. Ключ MacBook на том же пакете:
`IdentityInsufficient`. Образ panther не собирался.

## Sprint HDI-14 — платформенные узлы

- Состояние: `Verify`
- Зависит от: HDI-13 в Verify
- Архитектурные решения: ADR-426. Проба `platform_nodes` в белом списке v1.
- Рабочий fallback: файл без точных имён не добавляет утверждений

### Goal

Четвёртый проход читает короткий список платформенных имён. Mali, TPU,
ISP и AoC — отдельные устройства. Пустой класс USB-gadget записывается
как отсутствие. Загруженный модуль сам по себе имя не создаёт.

### Current state

Проходы 4–5 повторяли `pci_id`. Консоль показала `28000000.mali`,
`100b0000.TPU`, `100a0000.ISP`, `19000000.aoc`. `/sys/class/udc` пуст.
Сырой дамп в пакет исследователя не входит.

### Scope

Входит: проба `platform_nodes`, файл `platform.txt`, утверждения
`platform:mali`, `platform:tpu`, `platform:isp`, `platform:aoc` и
`usb:udc` = `none`.

Не входит: сырой список `/proc/device-tree`, привязка Mali к
`renderD128`, загрузка модуля, закрытие HDI-06.

### Change

1. Hunter просит `platform_nodes` один раз после `pci_drivers`.
2. Берутся точные `mali` / суффикс `.mali`, `.TPU`, `.ISP`, `aoc` / `.aoc`.
   Из двух имён Mali остаётся более длинное.
3. Строка `udc:` — `device_absent`. Соседние `dbgdev-pd-tpu` и похожие
   имена не становятся устройствами.

### Test

| Тест | Ожидание |
|---|---|
| `platform_nodes_are_named_once` | Проход 4 — `platform_nodes`; gain растёт на 5; значения `28000000.mali`, `100b0000.TPU`, `100a0000.ISP`, `19000000.aoc`; `usb:udc` отсутствует; `dbgdev-pd-tpu` и короткое `mali` не записаны |

### Acceptance criteria

На снимке Pixel `pass_gain=[5, 9, 9, 14, 14]`, efficiency 2.8,
`control_facts=9`. Пять новых утверждений не входят в control.
Адаптация карты остаётся `already_bound`. Образ телефона не меняется.

### Threat / privacy impact

В пакет попадает отобранный список из пяти строк, не дерево устройств
целиком. Серийные номера в этот файл не пишутся.

### Rollback

Убрать пробу `platform_nodes`. Проходы 4–5 снова повторяют `pci_id`.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --all-targets` — 46 passed
(HDI-14: 1). Whitelist HDI-02: 13 имён.
Повторный прогон пакета Pixel: `pass_gain=[5, 9, 9, 14, 14]`,
efficiency 2.8, `control_facts=9`, адаптация `already_bound`
`exynos_drm`, `loaded_module=false`. Образ panther не собирался.

## Sprint HDI-15 — драйвер платформенного узла

- Состояние: `Verify`
- Зависит от: HDI-14 в Verify
- Архитектурные решения: ADR-426. Проба `platform_drivers` в белом списке v1.
- Рабочий fallback: строка без принятого имени узла не даёт утверждения

### Goal

Пятый проход читает имя драйвера из ссылки sysfs. Имя берётся как есть.
Загруженный модуль с похожим именем это имя не заменяет. Узел без ссылки
остаётся непривязанным.

### Current state

Проход 5 повторял `pci_id`. На консоли ссылка `28000000.mali/driver`
называет `mali`, `19000000.aoc/driver` называет `aoc`. У
`100b0000.TPU` и `100a0000.ISP` ссылки драйвера нет.

### Scope

Входит: проба `platform_drivers`, файл `platform_drivers.txt`,
`driver_bound` и `driver_unbound` для уже принятых платформенных узлов.

Не входит: подстановка `mali_kbase` вместо `mali`, загрузка модуля,
сырой вывод `ls`, закрытие HDI-06.

### Change

1. Hunter просит `platform_drivers` один раз после `platform_nodes`.
2. Строка `имя драйвер` принимается, только если имя уже есть в
   `platform.txt`.
3. Слово `unbound` — `driver_unbound`. Иначе значение — второе слово
   строки. Лишние слова и чужое имя узла отбрасываются.

### Test

| Тест | Ожидание |
|---|---|
| `platform_driver_comes_from_the_driver_file` | Проход 5 — `platform_drivers`; gain растёт на 4; `platform:mali` привязан к `mali`, `platform:aoc` к `aoc`; TPU и ISP непривязаны; `mali_kbase` не записан |

### Acceptance criteria

На снимке Pixel `pass_gain=[5, 9, 9, 14, 18]`, efficiency 3.6,
`control_facts=9`. Четыре утверждения прохода 5 не входят в control.
Адаптация карты остаётся `already_bound`. Образ телефона не меняется.

### Threat / privacy impact

В пакет попадают четыре строки вида «узел и имя драйвера». Сырой
вывод консоли и серийные номера туда не копируются.

### Rollback

Убрать пробу `platform_drivers`. Проход 5 снова повторяет `pci_id`.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-contract -p hdi-exec -p hdi-campaign --all-targets` зелёный
(HDI-15: 1, campaign 47). `cargo clippy` по этим трём крейтам с
`-D warnings` — чисто. Whitelist HDI-02: 14 имён.
Повторный прогон пакета Pixel: `pass_gain=[5, 9, 9, 14, 18]`,
efficiency 3.6, `control_facts=9`, адаптация `already_bound`
`exynos_drm`, вердикт `PlatformGraphicsMatched`. Образ panther не собирался.

## Sprint HDI-16 — модуль за именем драйвера

- Состояние: `Verify`
- Зависит от: HDI-15 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: третье слово, которого нет в `modules.txt`, не записывается

### Goal

Имя драйвера и модуль за ним — два утверждения. Модуль берётся из ссылки
`module` у драйвера и только если он есть в списке загруженных. Имя
драйвера при этом не меняется. Шестая проба не добавляется.

### Current state

Проход 5 записывал `platform:mali` = `mali` и `platform:aoc` = `aoc`.
Ссылка `drivers/mali/module` называет `mali_kbase`, `drivers/aoc/module`
называет `aoc_core`. Оба модуля загружены.

### Scope

Входит: необязательное третье поле строки `platform_drivers.txt` и
утверждение `driver:<имя>` = модуль.

Не входит: замена значения `platform:mali` на `mali_kbase`, новая проба,
шестой проход, загрузка модуля, закрытие HDI-06.

### Change

1. Два поля строки остаются привязкой драйвера, как в HDI-15.
2. Третье поле — модуль. Оно пишется, если имя есть в `modules.txt`.
3. Четвёртое поле и третье поле у `unbound` отбрасывают строку.

### Test

| Тест | Ожидание |
|---|---|
| `driver_module_is_a_separate_fact` | `platform:mali` остаётся `mali`; `driver:mali` = `mali_kbase`; `driver:aoc` = `aoc_core`; незагруженный `mali_pixel` не записан |

### Acceptance criteria

На снимке Pixel `pass_gain=[5, 9, 9, 14, 20]`, efficiency 4,
`control_facts=9`. Два модульных утверждения не входят в control.
Адаптация карты остаётся `already_bound`. Образ телефона не меняется.

### Threat / privacy impact

В пакет добавляются два публичных имени модуля. Сырой вывод `ls` не копируется.

### Rollback

Игнорировать третье поле. Проход 5 снова даёт gain 18.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --test hdi15 --test hdi16` — оба зелёные.
`cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
Повторный прогон пакета Pixel: `pass_gain=[5, 9, 9, 14, 20]`,
efficiency 4, `control_facts=9`, адаптация `already_bound`
`exynos_drm`, вердикт `PlatformGraphicsMatched`. Образ panther не собирался.

## Внедрение в ОС

Очередь после HDI-16. Потолок v1 не меняется: 5 проходов, 5 проб,
`read_only`. Кампанию по-прежнему создаёт только явный
`investigate_hardware`. `native-init` лабораторию не запускает.
Предусловие `graphics_class_incomplete` остаётся выключенным.
Нового демона нет: один Task выполняет уже запущенный `saai-taskd`.

HDI-06 в эту очередь не входит. Снимок MacBook его не заменяет, и
внедрение на Pixel его не закрывает. Модемный трек эта очередь не
сдвигает. Большая модель по-прежнему требует отдельной правки этого
файла.

Образ panther меняет только HDI-21, и только userspace-бинарник.
До этого спринта телефон не прошивается.

## Sprint HDI-17 — accuracy считает ключ стенда

- Состояние: `Verify`
- Зависит от: HDI-13 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: пустой ключ по-прежнему даёт accuracy 0

### Goal

Метрика кампании получает подтверждённые утверждения вердикта.
Ключ остаётся снаружи пакета исследователя. На снимке Pixel accuracy
больше не равна нулю из-за пустого аргумента.

### Current state

`snapshot_run` печатает вердикт `PlatformGraphicsMatched` и отдельно
accuracy 0. `Metrics::from_run` считает accuracy по списку ключа, а
пример передаёт пустой список.

### Scope

Входит: передача подтверждённых утверждений вердикта в метрику.
Повторный прогон того же снимка Pixel.

Не входит: правка `saai-taskd`, прошивка, показ результата в оболочке,
включение предусловия, закрытие HDI-06.

### Change

1. Пример и библиотечный отчёт считают accuracy от утверждений, которые
   вердикт подтвердил.
2. JSON исследователя ключ не содержит.
3. Ключ MacBook на снимке Pixel по-прежнему даёт `IdentityInsufficient`,
   и accuracy считается только по подтверждённым утверждениям этого исхода.

### Test

| Тест | Ожидание |
|---|---|
| новый тест HDI-17 | Совпавший ключ Pixel даёт accuracy 1 на фикстуре `pixel-graphics`; пустой ключ даёт 0; пакет исследователя не содержит `expected_model` |

### Acceptance criteria

Повторный прогон пакета Pixel печатает accuracy, равную доли
подтверждённых утверждений ключа. Coverage остаётся 1. Образ телефона
не меняется.

### Threat / privacy impact

Ключ по-прежнему не лежит в `redacted/` и не попадает в запрос
исследователя.

### Rollback

Снова передавать в метрику пустой список.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --test hdi17` — 1 passed.
`cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
Пакет Pixel с ключом стенда: accuracy 1, вердикт `PlatformGraphicsMatched`.
Ключ MacBook на том же пакете: accuracy 0, вердикт `IdentityInsufficient`.
Образ panther не собирался.

## Sprint HDI-18 — один вызов кампании из действия

- Состояние: `Verify`
- Зависит от: HDI-05 и HDI-17 в Verify
- Архитектурные решения: ADR-426, ADR-120
- Рабочий fallback: неизвестное действие кампанию не создаёт

### Goal

Библиотека принимает идентификатор действия и каталог `redacted/`.
Только `investigate_hardware` запускает одну кампанию и возвращает отчёт.
Другой идентификатор возвращает отказ без журнала проб.

### Current state

`IntentGate` в `hdi-campaign` открывает кампанию на явный id, но
наружу нет функции, которую мог бы вызвать `saai-taskd`. Сам `saai-taskd`
о лаборатории не знает.

### Scope

Входит: функция в `hdi-campaign` и host-тест на фикстуре.

Не входит: правка `saai-taskd`, `intent-resolution`, `native-init`,
прошивка, чтение живого `/sys` хоста Windows.

### Change

1. Функция читает намерение, гоняет Hunter в потолке v1 и возвращает
   метрики, вердикт и `loaded_module=false`.
2. Свободный текст и событие загрузки в эту функцию не входят: у них
   нет explicit id.
3. `adapt_hardware` из этой функции не исполняется.

### Test

| Тест | Ожидание |
|---|---|
| новый тест HDI-18 | Явный id на фикстуре даёт один отчёт с пятью проходами; другой id не создаёт журнал; `loaded_module` ложен |

### Acceptance criteria

`cargo test -p hdi-campaign` зелёный. `native-init.c` не содержит
`investigate_hardware`. Образ телефона не меняется.

### Threat / privacy impact

Вызов не расширяет белый список проб и не получает право загрузки модуля.

### Rollback

Удалить функцию. `IntentGate` HDI-05 остаётся.

### Evidence

2026-10-10, этот Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-campaign --test hdi18` — 2 passed.
`cargo clippy -p hdi-campaign --all-targets -- -D warnings` — чисто.
`native-init.c` не содержит `investigate_hardware`. `saai-taskd` не менялся.
Образ panther не собирался.

## Sprint HDI-19 — saai-taskd выполняет это действие

- Состояние: `Verify`
- Зависит от: HDI-18 в Verify
- Архитектурные решения: ADR-426, ADR-030, ADR-120
- Рабочий fallback: прежний путь diagnose для текста без explicit id

### Goal

Уже запущенный `saai-taskd` на Intent с `semantic_action_id =
investigate_hardware` вызывает функцию HDI-18 и пишет отчёт в Result
этого Task. Новый процесс не создаётся.

### Current state

Уже запущенный `saai-taskd` на Intent с `semantic_action_id =
investigate_hardware` зовёт `start_hardware_campaign`. Каталог берётся
из свойства `redacted_dir`, ключ — из `verifier_key` и только если он
вне этого каталога. Result получает gain, efficiency, accuracy,
coverage и вердикт. Без каталога журнал проб не создаётся. Свободный
текст в эту ветку не входит. Сборка и тесты прошли на R620. Образ
panther не прошит.

### Scope

Входит: одна ветка в `saai-taskd` и тест там, где taskd собирается.

Не входит: пункт `native-init`, автозапуск, udev, включение
предусловия, прошивка panther, большая модель.

### Change

1. Совпавший explicit id запускает одну кампанию по переданному каталогу
   фикстуры.
2. Текст без explicit id идёт прежним путём.
3. Result хранит метрики и вердикт. Серийные идентификаторы в Result
   не пишутся.

### Test

| Тест | Ожидание |
|---|---|
| тест taskd на unix-хосте | Один Intent — один Result с конечными метриками; свободный текст лабораторию не открывает |

### Acceptance criteria

Ветка taskd собрана там, где есть unix-сокеты. `native-init.c` не
содержит `investigate_hardware`. Образ panther не прошит. Если сборка
возможна только не на Windows, это записано в Evidence, как в HDI-05.

### Threat / privacy impact

Task не получает права сверх белого списка. Чужой Intent список не
расширяет.

### Rollback

Убрать ветку. Функция HDI-18 остаётся в библиотеке.

### Evidence

2026-10-10, R620 `192.168.168.110`, toolchain
`1.97.1-x86_64-unknown-linux-gnu`, дерево `/home/mike/worktrees/hdi-lab`
(коммит HDI-17 `7385d2f` и незакоммиченная ветка HDI-18/19):
`cargo test -p saai-taskd --lib` — 108 passed, включая
`explicit_fixture_finishes_one_campaign`,
`free_text_does_not_open_a_journal`,
`missing_directory_does_not_open_a_journal`.
`cargo test -p hdi-campaign --test hdi18` — 2 passed.
`cargo clippy -p hdi-campaign --all-targets --no-deps -- -D warnings` — чисто.
`cargo clippy -p saai-taskd --all-targets --no-deps -- -D warnings` останавливается
на прежних замечаниях `too_many_arguments` в `runtime_bridge.rs` и
`cloned_ref_to_slice_refs` в `replan.rs` / `scheduler.rs`. С теми же
двумя allow новая ветка собирается чисто. `native-init.c` не менялся.
Образ panther не собирался.

## Sprint HDI-20 — белый список читает корень sysfs

- Состояние: `Verify`
- Зависит от: HDI-02 и HDI-18 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: корень фикстуры, как сейчас

### Goal

Тот же исполнитель читает заранее названные файлы из корня, который
на устройстве будет `/`. На host корень — временный каталог, не живой
`/sys` Windows.

### Current state

`sysfs_read` читает `{sys_root}/sys/...`. Без отдельного корня
`sys_root` совпадает с каталогом фикстуры. Имена проб те же, белый
список по-прежнему из 14 имён. Живой `/sys` host-тест не открывает.

### Scope

Входит: режим корня и host-тест, что путь вне белого списка получает Deny.

Не входит: чтение живого телефона, `modprobe`, запись в sysfs, прошивка.

### Change

1. Имена проб те же. Меняется только корень.
2. `sysfs_read` остаётся внутри прежних префиксов.
3. Символы оболочки и выход из корня — Deny и строка журнала.

### Test

| Тест | Ожидание |
|---|---|
| новый тест HDI-20 | Файл внутри временного корня читается; путь с `..` или вне префикса — Deny; журнал содержит оба вердикта |

### Acceptance criteria

Белый список v1 не длиннее, чем после HDI-15. Образ телефона не меняется.

### Threat / privacy impact

Живой корень на host-тесте не открывается. Редактор по-прежнему
выбрасывает строки с идентификаторами до пакета исследователя.

### Rollback

Оставить только корень фикстуры.

### Evidence

2026-10-10, Windows-хост, toolchain `1.97.1-x86_64-pc-windows-gnu`:
`cargo test -p hdi-exec --tests` — hdi02 8 passed, hdi20 1 passed.
`cargo clippy -p hdi-exec --all-targets -- -D warnings` — чисто.
2026-10-10, R620 `192.168.168.110`, toolchain
`1.97.1-x86_64-unknown-linux-gnu`: те же команды, hdi20 2 passed,
включая отказ по символической ссылке за пределы корня.
Белый список — 14 имён. Образ panther не собирался.

## Sprint HDI-21 — одна команда на panther

- Состояние: `Verify`
- Зависит от: HDI-19 и HDI-20 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: taskd без ветки HDI, телефон как до прошивки

### Goal

После загрузки человек один раз отправляет `investigate_hardware`.
Кампания читает userspace-файлы телефона, пишет журнал и Result.
Загрузка сама кампанию не начинает.

### Current state

На `/data/saaios/system/saai-taskd` стоит бинарник с веткой HDI.
`native-init.c` не менялся и лабораторию не вызывает. Один явный Intent
уже дал Result. Повторная загрузка второй кампании не создала.

### Scope

Входит: прошивка userspace-бинарника `saai-taskd` с веткой HDI-19 и
один сознательный запуск. Журнал на устройстве.

Не входит: пункт init, udev, загрузка модуля, модем, EFS, NV, SIM,
закрытие HDI-06, включение предусловия.

### Change

1. Прошить только этот userspace-бинарник.
2. После обычной загрузки отправить один explicit Intent.
3. Сверить Result с уже известными фактами снимка: привязка
   `exynos_drm`, драйвер `mali` / модуль `mali_kbase`, драйвер `aoc` /
   модуль `aoc_core`. Расхождение записать как факт, не замалчивать.

### Test

| Тест | Ожидание |
|---|---|
| физический запуск | Один Intent, пять Allow или явный Deny в журнале, метрики конечны, `loaded_module=false`, повторная загрузка без нового Intent кампанию не создаёт |

### Acceptance criteria

В Evidence есть идентификатор прошитого бинарника, текст Result и
подтверждение, что init-лог не содержит `investigate_hardware`.
Модемный трек не отмечен как продвинутый. HDI-06 остаётся в Backlog.

### Threat / privacy impact

Журнал на устройстве проходит тот же редактор. PIN, IMSI, ICCID и IMEI
в Result не пишутся. Кампания не пишет в sysfs.

### Rollback

Вернуть предыдущий `saai-taskd`. Каталог кампании на data удалить.

### Evidence

2026-10-10, panther, userspace-бинарник
`/data/saaios/system/saai-taskd`, sha256
`2535b4578a8e996dc20bbeb13a70ea9ac7e70ef2ded95a78ba9ab22e71505e44`.
Образ `init_boot` не прошивался. `native-init.c` не менялся.
`dmesg` не содержит `investigate_hardware`.

Один Intent `3d4fd277-751d-430a-9d3f-1e47a64d35d2`. Result:
`HDI gain=19 efficiency=3.8 accuracy=1 coverage=1 verdict=PlatformGraphicsMatched loaded_module=false`.
Журнал: пять Allow — `drm_class`, `modules`, `pci_drivers`,
`platform_nodes`, `platform_drivers`. Привязка `exynos_drm`, драйвер
`mali` / модуль `mali_kbase`, драйвер `aoc` / модуль `aoc_core`
совпали со снимком. Класс USB на этом запуске не пуст
(`11210000.dwc3`), поэтому утверждения `usb:udc absent` нет: gain 19,
а не 20. Модуль не загружался.

После `reboot` pid taskd новый, тот же sha256, в `/tmp` нового журнала
нет, Result тот же один, лог taskd не содержит повторной кампании.
HDI-06 не закрыт. Модемный трек не сдвинут.

## Sprint HDI-22 — Result на существующем экране

- Состояние: `Verify`
- Зависит от: HDI-21 в Verify
- Архитектурные решения: ADR-426, ADR-301
- Рабочий fallback: Result остаётся в журнале taskd, экран не меняется

### Goal

Уже существующий экран Result показывает вердикт и пять чисел метрики
этого Task. Нового экрана и нового визуального трека нет.

### Current state

Экран Result этого Task берёт вердикт и записанные числа метрики.
Result без этих чисел остаётся на прежнем `summary`. Холодная загрузка
без нового Intent лабораторию не открывает.

### Scope

Входит: чтение уже записанного Result.

Не входит: новая страница «Лаборатория», иконка, жест, автопоказ после
загрузки.

### Change

1. Экран берёт вердикт и метрики из Result HDI-21.
2. Пустой Result показывает прежнее пустое состояние экрана.
3. Строки с идентификаторами на экран не выводятся.

### Test

| Тест | Ожидание |
|---|---|
| host-тест оболочки и один просмотр на panther | Совпавший Result виден; Task без HDI рисуется как раньше |

### Acceptance criteria

На телефоне после HDI-21 человек видит вердикт без новой прошивки
init. Повторный холодный старт без Intent экран лаборатории не открывает.

### Threat / privacy impact

Экран показывает только уже отредактированный Result.

### Rollback

Убрать подстановку. Экран Result остальных Task остаётся.

### Evidence

Host, R620, `cargo test -p saai-shell --bin saai-shell object_view_`: 34 passed, включая `object_view_shows_hdi_verdict_and_metrics_without_identifiers` и `object_view_empty_result_keeps_the_previous_empty_observation`. Совпавший Result даёт строку `Вердикт: PlatformGraphicsMatched · точность 1 · покрытие 1 · прирост 19 · эффективность 3.8 · ложные 0`. Токен `IMEI` и 14-значный номер на экран не попадают. Result без чисел метрики оставляет прежнее пустое наблюдение. Task без HDI по-прежнему показывает свой `summary`.

На panther заменён только `/data/saaios/system/saai-shell`, sha256 `e3aaf0fcd365240ba225d0cca22e6658c93b2d2338bf35e6b61bc0315426dc40` (1531592 байта). Предыдущий бинарник сохранён как `saai-shell.hdi22-prev`. `init_boot` и `native-init` не менялись. После `reboot -f` процесс `saai-shell` pid 397, `/proc/397/exe` с тем же хешем. В `dmesg` нет `investigate_hardware`, каталога `/tmp/hdi-*` нет. Result один: `36875cf5-c782-4497-b90d-13bc36518441`, `verdict=PlatformGraphicsMatched`, `discovery_gain=19`, `identification_accuracy=1.0`, `evidence_coverage=1.0`, `probe_efficiency=3.8`. Поля `false_hypotheses` в этом Result нет, поэтому на телефоне четыре записанных числа и вердикт; пятое число экран показывает, когда оно записано. Кадр экрана с хоста не снимался.

## Sprint HDI-23 — адаптация на устройстве останавливается

- Состояние: `Verify`
- Зависит от: HDI-21 в Verify
- Архитектурные решения: ADR-426, ADR-260
- Рабочий fallback: AskUser без подтверждения не загружает модуль

### Goal

Если кампания предлагает пробел драйвера, Task переходит в
`waiting_confirmation`. Подтверждение в приёмочном прогоне не
отправляется. Модуль не загружается. Уже привязанный драйвер вопрос
не задаёт.

### Current state

На panther `card0` привязан к `exynos_drm`. Кампания завершила Task
без вопроса, `loaded_module=false`. Пробел драйвера на host-фикстуре
останавливается в `waiting_confirmation` и модуль не грузит.
Предусловие `graphics_class_incomplete` выключено.

### Scope

Входит: состояние Task и проверка, что модуль не появился в списке
загруженных из-за лаборатории.

Не входит: загрузка предложенного модуля, включение предусловия,
исполнитель `adapt_hardware`.

### Change

1. `DriverGap` пишет AskUser и `waiting_confirmation`.
2. Приёмочный прогон подтверждение не шлёт.
3. `AlreadyBound` оставляет Task завершённым без вопроса.

### Test

| Тест | Ожидание |
|---|---|
| host-фикстура пробела и прогон Pixel | Пробел ждёт человека, `loaded_module=false`; снимок Pixel вопроса не задаёт |

### Acceptance criteria

После прогона список модулей не получил новый модуль от лаборатории.
Предусловие в поставляемых данных остаётся `enabled=false`.

### Threat / privacy impact

Ожидание подтверждения не даёт taskd права `insmod`. Перезагрузка не
превращает ожидание в загрузку.

### Rollback

Не писать AskUser из кампании. Отчёт HDI-21 остаётся.

### Evidence

Host, Windows и R620, `cargo test -p hdi-campaign --test hdi23`: 3 passed. Пробел `exec-root` — `DriverGap`, `confirmation_required`, `loaded_module=false`, в тексте нет `insmod`/`modprobe`/`.ko`. Снимок Pixel с строкой `card0 -> …/exynos-drm/…` — `AlreadyBound { exynos_drm }`, вопроса нет. `SHIPPED_PRECONDITIONS` для `graphics_class_incomplete` остаётся `enabled=false`. R620, `cargo test -p saai-taskd --lib finishes`: пробел ставит `asks_user`, привязанный драйвер — нет. Журнал пробела не содержит `insmod`.

На panther заменён только `/data/saaios/system/saai-taskd`, sha256 `3837e7e4d8af15d582a1e65ba1829084a746769bb58981d1dac60f414536d403`. Предыдущий бинарник — `saai-taskd.hdi23-prev`. Init не менялся. Один Intent `c0b9e44a-4bfe-405a-9b44-9d28135b8979` дал Task `f64b4c6d-6d47-4229-be8f-e383634e35b3` со статусом `done`. Result `33f9d1de-8ad8-429b-959d-ee15f182690c`: `verdict=PlatformGraphicsMatched`, `loaded_module=false`. Action — `investigate_hardware`, не `adapt_hardware`. Сущностей `waiting_confirmation` нет, подтверждение не отправлялось. До и после кампании `/proc/modules` sha256 `6d862c26e9ab48455290f0e9a16b656ad42b828ca7ace4478014548432f8975d`. После `reboot -f` taskd pid 548 с тем же хешем, Task остался `done`, нового `/tmp/hdi-*` нет, в `dmesg` нет `investigate_hardware`. Хеш `/proc/modules` после новой загрузки другой (`096f232eb54d98733eaf32f9c6b890a87d6060d3dddfcbc613a2652577a86b4f`): это набор модулей свежей загрузки, не запись лаборатории между двумя хешами одного сеанса.

## Sprint HDI-25 — имя узла в длинном списке DRM

- Состояние: `Verify`
- Зависит от: HDI-06 в Verify, HDI-08 в Verify
- Архитектурные решения: ADR-426
- Рабочий fallback: строка без имени узла не добавляет утверждение

### Goal

Скриптовый Hunter читает узел из `ls -l /sys/class/drm` и связывает его с драйвером, который уже назван в `pci_drivers.txt` и загружен. Каталог `drm` на этой строке драйвером устройства не считается.

### Current state

Прогон HDI-06 записал gain 0: первая колонка `ls -l` — режим файла, а Hunter брал первый токен. Control уже держал PCI и `Kernel driver in use`.

### Scope

Входит: имя узла перед `->`; карты `cardN` и их разъёмы; последний PCI-слот строки, не мост перед ним.

Не входит: загрузка модуля, `apple_gmux` как причина, правка снимка, телефон, большая модель.

### Change

1. Узел — токен сразу перед `->`. Строка `card0 -> …` по-прежнему начинается с узла.
2. Токен `drm` — каталог класса, не драйвер.
3. Если на строке нет имени загруженного модуля, драйвер берётся из последнего слота этой строки по `pci_drivers.txt`, и только если модуль есть в `modules.txt`.

### Test

| Тест | Ожидание |
|---|---|
| `ls_long_names_the_node_before_the_arrow` | `card1` и `renderD129` есть, `version` нет |
| `class_directory_drm_is_not_the_device_driver` | Один `device_present`, без привязки `drm` |
| `last_pci_slot_binds_the_loaded_driver` | `card1` → `i915`, `card0` → `nouveau`, не `pcieport` и не `apple_gmux` |

### Acceptance criteria

На том же пакете MacBookPro11,3 проход 1 и проход 5 различаются gain. Исход остаётся B. Модуль не загружается. `card0` не объявляется пробелом.

### Threat / privacy impact

В утверждение попадают имя узла и имя уже загруженного драйвера. Ключ стенда и серийные номера не копируются.

### Rollback

Снова брать первый токен строки. Привязка по слоту исчезает вместе с этим разбором.

### Evidence

Host, Windows, toolchain `1.97.1-x86_64-pc-windows-gnu`: `cargo test -p hdi-campaign --test hdi25 --test hdi08 --test hdi12 --test hdi23` — 13 passed. Повтор того же пакета `81d712eeb7608bec7ef7fbf78b320e097d823d641934ab21e2b2d1c7b395ce09`, ключ снаружи `redacted/`: `pass_gain=[9, 9, 18, 18, 18]`, efficiency 6, accuracy 0, coverage 1, `false_hypotheses=0`, `not_transferred`. Проход 1 записывает девять узлов, включая `drm:card1` и `drm:renderD129`. Проход 3, после `pci_drivers`, привязывает `card0` и `renderD128` к `nouveau`, `card1` и `renderD129` к `i915`. Исход B, `negative_recipe_closed=false`. Адаптация `AlreadyBound { nouveau }`, `loaded_module=false`, вопроса нет. Журнал без `insmod` / `modprobe` / `rmmod` / `.ko`. Утверждения `gmux` нет.

## Каталог фикстур

Синтетика, безопасная для git:

| Имя | Зачем |
|---|---|
| `nvidia_only` | VGA NVIDIA есть, `00:02.0` нет, DMI `MacBookPro11,3` |
| `intel_present` | `8086:0d26` есть |
| `secrets` | Фальшивые MAC, `iSerial`, uuid |
| `unknown_product` | DMI другого имени |
| `machine-b` | Второе шасси для отказа переноса |
| `pixel-graphics` | Модель GS201, два узла `exynos-drm`, без `[0300]` |
| `pixel-key.txt` | Ключ стенда Pixel, снаружи каталога `redacted` |

Живой снимок MacBook в фикстуры не копируется.

## Порядок доказательств

1. HDI-00 собирается и тестируется на host до любых остальных
   крейтов.
2. HDI-01…04 остаются host-only. Образ panther для них не собирается.
3. HDI-05 не включается в init и не прошивается.
4. HDI-06 измерен на снимке MacBookPro11,3, исход B. Большая модель подключается
   только отдельной правкой этого файла, после скриптового прогона,
   тем же контрактом пробы.
5. HDI-17…20 остаются host-only. Образ panther для них не собирается.
6. HDI-21 — первая прошивка этого трека, и только userspace-бинарник
   `saai-taskd`. Init, udev и предусловие не включаются.
7. HDI-22 и HDI-23 читают Result уже прошитого Task. Они не открывают
   новый экран и не загружают модуль.
8. HDI-25 остаётся host-only: тот же снимок MacBook, без новой
   записи на машину и без прошивки panther.

Состояние `Done` у любого спринта требует записи Evidence: команда
тестов, результат, известный предел. Зелёная сборка телефона сюда
не подставляется вместо этих доказательств.
