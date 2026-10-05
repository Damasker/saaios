# MODEM-10 — реальный camp и bearer через валидный dual-handle NV

## Паспорт

- Состояние: `Ready` (host-часть реализована, не закоммичена; ждёт
  проверки на SaaiOS в слоте A). Разворот относительно всего MODEM-06:
  провал регистрации — это **артефакт пустого `NV_NORM`**, а не «PIN
  soft-lock / Present==2 / отсутствие CDMA RatMap». Подтверждено
  стоковым контролем (см. Current state).
- Зависит от: MODEM-02 (CP ONLINE), MODEM-03 (runtime SIT),
  MODEM-07 (карантинная RFS-политика). Не зависит от MODEM-06's
  CDMA/Present гипотез — они сняты как ошибочные.
- Архитектурные решения: обслуживать **верифицированные копии обоих**
  NV-handle (1 = `nv_normal`, 3 = `nv_protected`), источник — реальный
  EFS, открытый **read-only**; записи handle-1 от CP уходят в
  записываемую карантинную копию, не в реальный EFS. Vendor
  `cbd`/`rfsd`/`rild` как сервисы **не** запускаются (сохраняем запрет
  MODEM-ROADMAP § Non-goals и AGENTS.md).
- Рабочий fallback: сегодняшний passive owner (handle-3-only) и
  известный-хороший сток в слоте B остаются путём возврата без
  изменений.

## Goal

На SaaiOS (слот A) модем проходит тот же путь, что физически
подтверждён на стоке: SIM READY → CS+PS регистрация на домашней сети →
`SETUP_DATA_CALL` → поднятый `rmnet` с IPv4. Один наблюдаемый
результат: **живой bearer** (rx/tx ненулевые и/или IPv4 на `rmnet`).

## Current state

Проверенные факты до изменения.

**Стоковый контроль (2026-10-05, тот же телефон, та же SIM).** После
прошивки стока + root, слот `_b`: CS/PS `IN_SERVICE` / `REG_HOME`, RAT
**LTE**, PLMN **25503** (Kyivstar), `SETUP_DATA_CALL cause=NONE cid=2
ifname=rmnet1 UP`, время до camp ~1 с. Железо, RF-cal, прошивка модема
и SIM исправны. Полный рецепт:
[modem-stock-reproduction.md](../targets/panther/modem-stock-reproduction.md),
смежное железо: [hardware-risks.md](../targets/panther/hardware-risks.md).

**Корневая причина пробела.** Сток: `cbd` UDL-грузит `NV_NORM` +
`NV_PROT` из реальных EFS-файлов (`/mnt/vendor/efs/nv_normal.bin`,
magic `ERIG`), а `rfsd` обслуживает **оба** handle (1 normal, 3
protected) на `umts_rfs0`. SaaiOS ранее: пустой UDL `NV_NORM` (crc 0),
RFS-карантин **только на handle 3**, записи handle-1 оставались без
ответа. Без валидного `nv_normal` mutable RAT/band/SIM-NV модема не
инициализируется → он деградирует до WCDMA и даёт `REG_DENIED`
(reject 0). `WCDMA REG_DENIED` — это симптом неудачи NV/FLASH, **не**
проблема APN или SIT-опкода.

**MODEM-06 разворот.** Вся серия MODEM-06 (PIN soft-lock,
Present==2-писатель только через CDMA MEAS/LATCH, «EU No-CDMA не может
READY», `0x2f50` OEM-IPC охота, `0x074f`/`0x0705`/`0x0706`/`0x0734`
отказы с GENERIC_FAILURE) объясняется одним фактом: CP всё это время
работал на пустом `nv_normal`. Операторские SET отклонялись не из-за
host-кодирования (VERDICT 22 доказал байт-идентичность фреймов, VERDICT
26 показал, что `0x0734` вообще был **принят** error 0) — а потому что
модем был в деградированном NV-состоянии. Это снимает вывод «CP-internal
RF-cal стена» (VERDICT 24-26): сток на том же железе встаёт в LTE за
секунду.

**Host-реализация (готова, не закоммичена).** `diagnostics/
modem-rfs-full-quarantine-owner.c` получил `seed_normal_candidate()`:
копирует полный 524288 B baseline `nv_normal.bin` (read-only источник)
в карантинную `normal-candidate.bin`, затем handle-1 запись CP
накладывается поверх начала; handle-1 обслуживается тем же recovered
grant-layout, что handle-3 (выведен из-под `#ifdef
SAAIOS_RFS_NORMAL_CAPTURE`). Реальный EFS и baseline никогда не
пишутся, payload-байты не логируются.

**Что ещё не проверено.** Достаточно ли runtime-обслуживания handle-1
(RFS-чтения при инициализации CP), или cp-boot UDL NV-стадия тоже
обязана подавать **реальный** `nv_normal`/`nv_protected`, а не пустой.
Это решается первым же device-прогоном (Change 1 → Change 2).

## Scope

- входит:
  - подача валидного содержимого `nv_normal` и `nv_protected` модему
    через уже проверенный механизм (UDL NV-стадия и/или RFS-чтения
    handle-1/3), источник — реальный EFS `sda5`, открытый read-only,
    копия верифицируется по factory-контрольной сумме;
  - обслуживание owner'ом **обоих** handle из верифицированных копий;
    записи handle-1 — только в записываемую карантинную копию;
  - после устойчивого READY + camp — именованная RIL-последовательность
    стока: `SET_INITIAL_ATTACH_APN` (APN из конфигурации SIM/carrier,
    не захардкоженный 25503) → `SETUP_DATA_CALL` `accessNetworkType=
    EUTRAN` → подъём `rmnet` + host IPv4/маршрут/DNS;
  - честная фиксация: достаточно ли runtime-serve, или нужна правка
    UDL NV-стадии.
- не входит:
  - запуск vendor `cbd`/`rfsd`/`rild` как сервисов (остаётся запретом);
  - любая запись в реальный EFS / `nv_protected` / `nv_normal` /
    `sda5` / SIM EF;
  - cross-slot A/B, слот B (сток) не трогаем — это контроль/возврат;
  - voice / SMS / IMS / VoNR (отдельные поздние вехи);
  - ручной выбор оператора и редактор APN.

## Change

Малые вертикальные задачи в порядке выполнения.

1. **Device-прогон runtime-serve (слот A).** Загрузить SaaiOS, запустить
   owner с dual-handle serve (уже в рабочем дереве). Наблюдать: приходят
   ли handle-1 RFS-запросы при инициализации, доходит ли SIM до READY,
   меняется ли camp с WCDMA/`REG_DENIED` на домашнюю сеть/LTE. Без SET,
   бюджет наблюдения ~60-120 с.
2. **Если camp не наступил — UDL NV-стадия.** Проверить содержимое
   `NV_NORM`/`NV_PROT`, подаваемого cp-boot UDL. Если пустое/нулевое —
   подать верифицированную копию реального `nv_normal.bin`/
   `nv_protected.bin` (read-only источник, factory-checksum), без записи
   в EFS. Повторить прогон.
3. **Регистрация → bearer.** После READY + `REG_HOME` (или домашний RAT
   SIM) выполнить `SET_INITIAL_ATTACH_APN` + `SETUP_DATA_CALL EUTRAN`
   по именованной RIL-последовательности стока, поднять `rmnet` и host
   IP/route/DNS. Проверить реальный трафик.
4. **Фиксация и чистка.** Записать точную последовательность и
   доказательства bearer; восстановить passive owner; убедиться, что
   реальный EFS прошёл read-only сверку до/после.

## Test

- unit/protocol: host-фикстуры `seed_normal_candidate` (полный baseline,
  overlay начала, хвост = baseline, отказ при неверном размере/правах),
  handle-1 grant-layout == handle-3, отказ промоушена кандидата;
- host integration: синтетический RFS-клиент, выдающий handle-1
  grant-request, проверка что serve идёт из копии и записи уходят в
  карантин;
- image inspection: содержимое UDL NV-стадии (пустое vs реальное);
- fault injection: прерванная handle-1 передача не портит baseline;
  карантинный кандидат не промоутится в boot-копию;
- device/physical/cold reboot: прогон на реальном Pixel 7 в слоте A;
  реальный EFS сверяется read-only до и после; холодная перезагрузка
  между прогонами, не runtime-подмена.

## Acceptance criteria

- модем доходит до SIM READY без ручного PIN (карта PIN-disabled) и до
  **домашней/роуминговой регистрации** (`REG_HOME` или домашний RAT),
  а не WCDMA `REG_DENIED` — подтверждено живым чтением registration;
- `SETUP_DATA_CALL` возвращает `cause=NONE` и **`rmnet*` UP с IPv4**
  (и/или IPv6), rx/tx ненулевые под реальным трафиком — это единственный
  критерий «связь есть» (AGENTS.md bearer gate), не «ACK на пробе»;
- обе NV обслуживаются из верифицированных копий; реальный EFS /
  `nv_protected` / `nv_normal` / SIM EF **ни разу не записаны**
  (read-only сверка до/после идентична);
- vendor `cbd`/`rfsd`/`rild` как сервисы не запускались; секреты
  (PIN/IMSI/ICCID/AID/NV/RF-cal/IMEI) не логировались и не коммитились;
- честно зафиксировано: достаточно ли был runtime-serve или
  потребовалась правка UDL NV-стадии; что именно подаётся модему.

## Threat / privacy impact

- новые данные: валидное содержимое `nv_normal`/`nv_protected` в
  карантинных копиях на устройстве — не коммитится в git, не логируется,
  живёт только на устройстве (тот же принцип, что identity в protected
  NV на стоке);
- канал: уже существующий `umts_rfs0`/owner, новой сетевой поверхности
  нет; bearer поднимает host-маршрут на `rmnet` только после реальной
  регистрации;
- негативные тесты: прерванная/битая handle-1 передача не доходит до
  промоушена; реальный EFS никогда не writable для owner'а;
- запрет: ни одного invented NV-байта, ни одной записи реального EFS,
  ни одного vendor-демона как сервиса.

## Rollback

- passive owner (handle-3-only) и его проверенный бинарь —
  восстановление по умолчанию; реальный EFS не менялся, возврат не
  требуется;
- слот B (сток) остаётся физически подтверждённым рабочим контролем и
  аварийным возвратом; ручной `fastboot` с хоста — независимый fallback.

## Evidence

Заполняется при закрытии: commit, host test log, точная
RFS/UDL-последовательность, доказательство bearer (имя `rmnet`,
IPv4, rx/tx), read-only EFS сверка до/после, и честные ограничения.
Контроль для сравнения —
[modem-stock-reproduction.md](../targets/panther/modem-stock-reproduction.md)
(стоковый LTE HOME + rmnet1).
