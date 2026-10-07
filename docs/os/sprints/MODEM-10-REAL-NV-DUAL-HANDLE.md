# MODEM-10 — реальный camp и bearer через валидный dual-handle NV

## Паспорт

- Состояние: `In progress` — Change 1 и Change 2 выполнены на
  устройстве (2026-10-05): ни runtime-serve handle-1, ни стоковая
  стадия REPLAY не дали camp; CRC NV-стадий у стока нет. UDL теперь
  совпадает со стоком по стадиям; отличие ищется после UDL. Разворот относительно всего MODEM-06:
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

### Change 1 — device run (2026-10-05, слот A, сборка S26 `2d0a9d8`)

- Host: `RFS_HOST_TEST self-test` PASS для обоих вариантов (обычный и
  `-DSAAIOS_RFS_CAMP`) на рабочем дереве с `seed_normal_candidate`.
  Owner `e19ef917…`, probe `e32538e8…`, прошивка — стоковый
  `449eeab3…` (`g5300q-260317-260505-B-15346003`), не `PATCHED-ready`.
- NV-копии: свежо сняты из `sda5` read-only
  (`ro,norecovery,nodiscard`), factory checksum
  (`MD5(file||"Samsung_SIT_RIL")`) PASS для обоих;
  `verify-original-efs-readonly.sh verify-read-only` PASS до и после.
- Попытка 1 недействительна: owner вышел на предусловии (нет
  `/data/saaios/var/rfs-quarantine` после форматирования userdata),
  CP остался `BOOTING`. Каталог `0700` создан, AP перезагружен.
- Попытка 2: probe `ONLINE`, owner жив. Handle 3: полная карантинная
  передача (95 grant'ов, 189446 B, final ACK). Handle 1: **только
  запись от CP** (`cmd=0x20006`, ~476776 B, повторяется), `NORMAL_SEED
  baseline=524288` — карантин, `NO_PROMOTION`. Запросов чтения handle-1
  при инициализации нет.
- Результат без SET (~240 с): SIM READY (`app_state_raw=5`), voice
  `registration_raw=3` (DENIED) `reject_raw=0` на UMTS (`tech_raw=3`),
  PLMN `25501` (не домашняя 25503), data NOT_REG. Т.е. **runtime-serve
  handle-1 недостаточен**.
- Поправка к Current state: UDL-стадия `NV_NORM`/`NV_PROT` в
  `cp-boot-probe.c` уже отправляет **реальные** проверенные копии из
  `/data/saaios/var/efs-copy` (`read_file(NV_NORM_PATH)`), а `crc=0x0`
  в логе — это CRC-аргумент `sit_send_stage(..., 0)`, а не пустой NV.
  Остающееся отличие от стока для Change 2 — какой CRC/поле стадии
  стоковый `cbd` шлёт для NV-стадий; восстановить его из стокового
  `cbd` read-only, не подбирать.
- Owner после SIGTERM в терминальном состоянии продолжает держать
  ipc/rfs (по дизайну), CP `ONLINE`; не убит, `IOCTL_POWER_OFF` не
  выполнялся.

### Change 2 — read-only разбор стокового `cbd` (CP2A)

`/vendor/bin/cbd` из `CP2A.260705.006` `vendor.img` (debugfs dump,
только чтение), SHA-256 `9b2fc0a9…`, 157744 B, objdump.

- Таблица стадий: массив по 44 B от `ctx+0x214`; поля `+0` idx,
  `+4` START, `+8` есть данные, `+12` **CRC-флаг**, `+16` DONE,
  `+20` fd, `+24` смещение в файле, `+32` размер, `+36` crc (из TOC).
- Разбор TOC `1a0e8–1a2b8`: `+12 = (strcmp(name, "MAIN") == 0)`
  (`1a14c–1a160`, строка `0x48db` = `MAIN`). Для `NV_NORM`/`NV_PROT`
  `+20` — fd открытого EFS-файла, `+24/+32` — `b_off`/`size` из TOC.
- Цикл стадий `f380–f8d0`: CRC-пакет `{0xA301|bits, crc}` и ожидание
  `0xC300|bits` отправляются только при `+8 && +12` (`f820–f8c4`).
- Вывод: **сток не шлёт CRC для NV-стадий** — START → BIN → DONE, как
  `cp-boot-probe.c`. CRC-гипотеза снята; UDL NV-стадия probe
  структурно совпадает со стоком, payload — проверенная копия.
- Найдено одно реальное отличие UDL: **стадия REPLAY**. Сток
  (`1fd70–20338`) делает `chdir /mnt/vendor/modem_userdata`,
  `timeout 10 tar -cvf replay_region.bin replay/*`, дополняет нулями до
  512 KiB (при ошибке или >512 KiB — пустой нулевой файл), открывает его
  как fd REPLAY. В TOC `REPLAY` имеет `idx=6` (как `NV_PROT`); `172a0`
  увеличивает число стадий, а номер стадии — позиция в TOC, т.е. **7**
  (`START 0xA170`). Probe эту стадию не отправлял.
- На устройстве (`sda7`, ro): `replay/dds.bin` 64 KiB и
  `replay_region.bin` 512 KiB (ustar, единственный член `replay/dds.bin`),
  созданные стоковой загрузкой слота B.

### Change 2 — device run (2026-10-05, слот A)

- `cp-boot-probe.c` `PROBE_REPLAY` (guarded): копия
  `efs-copy/replay_region.bin` (ro из `sda7`, 0:0600, sha `f5aca90d…`),
  проверка TOC (позиция 7, idx 6, b_off 0, size 0x80000), отправка
  стадии 7 после `NV_PROT` без CRC. Без `PROBE_REPLAY` сборка
  байт-идентична `e32538e8…`; с ним `cdbaa170…`.
- Один прогон после sysrq-перезагрузки: `UDL REPLAY idx=7 start=0xa170
  … complete`, FIN, CP `ONLINE`, `probe_rc=0`.
- Через ~265 s без изменений: SIM READY (`app_state_raw=5`), voice
  `registration_raw=3` DENIED `reject_raw=0` `tech_raw=3` (UMTS),
  PLMN 25501, data `registration_raw=0`, `mask_low7=2`, rmnet rx/tx 0,
  IPv4 нет. **REPLAY — не причина.**

### Почему только UMTS — read-only сверка boot-цепочки (2026-10-05)

- NV-набор закрыт: конфиги s5100 в `cbd` (`0x25058–0x25138`) — ровно
  `nv_normal`, `nv_protected`, `replay_region` (0x80000); `nv_data.bin` /
  `nv_5g_data.bin` — других типов модема. Все три теперь шлются.
- Handover builder `1b630–1bfc0` (тот же `cbd`, что прошлый разбор):
  `cpinfo2` = `modem_flag` из `fscanf` (файла нет → 0); `cpinfo1` только
  при `persist.vendor.modem.efs.clear.action` ≠ `none` (default `none`);
  `reserved[3]` только для `ro.vendor.build.type=userdebug`; `cpinfo0`
  только для `ro.bootmode` factory/fbm. Для normal/user все 0 — как у
  нас. dmesg: `sku: 1, rev: 0` в обоих.
- Цепочка ioctl/dmesg совпадает (`POWER_ON` → `START` normal → magic
  `0xBDBD` → BOOT `0x16800` → `0x3FFF` → `COMPLETE` → `INIT_START` →
  `PHONE_START` → ONLINE), **кроме порядка handover**: сток —
  `HANDOVER_BLOCK_INFO` **до** `POWER_ON`; у нас — после `START`, когда
  BOOT уже на `0x3FFF` (+0.5 s). Ядро лишь копирует 161 B в cmsg
  `ap2cp_handover_block_info` и не очищает его при power-on; если BOOT
  читает блок при старте, он видит нули.
- Прогон (2026-10-05, слот A): `PROBE_HANDOVER_EARLY` + `PROBE_REPLAY`
  (`0d390920…`; без флага — прежние `e32538e8…`/`cdbaa170…`). dmesg:
  `HANDOVER_BLOCK_INFO` → `POWER_ON` → `START` → `COMPLETE`, CP `ONLINE`.
  Через ~245 s 27/27 опросов: voice DENIED UMTS `reject_raw=0`, PLMN
  25501, data 0, `mask_low7=2`, rmnet 0, IPv4 нет. **Порядок handover —
  не причина.** Boot-цепочка теперь совпадает со стоком по стадиям,
  handover и порядку ioctl.

### Прочие стоковые демоны у модемных узлов (CP2A vendor.img, ro)

| Демон | Узлы | Роль |
|---|---|---|
| `shared_modem_platform -s` (class core) | `oem_ipc1`, `oem_ipc3`, `oem_test`, `umts_boot0` | Google ModemService, gipc/pw_rpc: `property`, `remote_file_system`, `uecap_file`/`CarrierCapa` (`/vendor/firmware/uecapconfig/`), `modem_stats`, `gtemperature`, `extended_log`, `at_command` |
| `dmd` | `umts_dm0`, `umts_boot0` | DM-логирование |
| `wfc-pkt-router` | `umts_wfc1`, `umts_boot0` | Wi-Fi calling |

- Пассивный захват (2026-10-05): читатели `oem_ipc1`/`oem_ipc3`/`oem_test`
  открыты до `POWER_ON`, окно 300 s через загрузку и camp — **0 байт** на
  всех трёх. CP не шлёт туда запросов сам; SMP — инициатор со стороны
  AP. Что он шлёт при старте, не разобрано (3.8 MB Rust/C++), байты не
  восстановлены — ничего не отправлялось.
- Оставшиеся отличия от стока лежат после UDL: полная init-последовательность
  `rild_exynos` до/вокруг RADIO_POWER (стоковый radio-лог захвачен уже
  после RADIO_POWER) и непрерывное обслуживание `rfsd`. Повторный
  стоковый захват требует загрузки слота B, которая сейчас
  переформатирует общий `userdata` SaaiOS.

### Стоковый захват с загрузки (2026-10-05, слот B, CP2A + KernelSU LKM)

Слот B: `init_boot_b` из factory-zip (KernelSU LKM). Скрипт в
`post-fs-data.d` цепляет strace (`-s 16`/`-s 8`, только заголовки) к
`shared_modem_platform` (3.68 s) и `rild_exynos` (4.04 s). logd увеличен до 16M.
Сырые данные лежат в `artifacts/private-stock-2026-10-05/boot-capture/` (gitignored).
Сеть: LTE.

У SMP до attach уже открыты `umts_boot0` (fd 7), `oem_ipc3` (fd 8) и
`oem_ipc1` (fd 9). Дальше AP сам начинает обмен:

- `umts_boot0`: 18 ioctl с интервалом около 100 ms (1.0–3.7 s после attach).
  Это опрос состояния CP до ONLINE.
- `oem_ipc3`, сразу после ONLINE: короткие protobuf-сообщения
  (`2a 12 0a 0e "uim_…"`, `"Prot…"`, `"rf_s…"`) и ответы CP.
  Похоже на сервис `property`.
- `oem_ipc1`: сначала два коротких запроса (18 и 25 Б, канал 7). Потом
  SMP читает `/vendor/firmware/uecapconfig/WILDCARD.binarypb` (496 290 Б;
  `/data/vendor/radio/ota_uecap/WILDCARD.binarypb` отсутствует) и за
  0.2 s пишет 119 кусков по 4031 Б и хвост 321 Б, канал 0x0c, поле 3
  растёт 0x0e…0x85. По объёму это примерно весь файл. Это
  `uecap_file`/`CarrierCapa`: конфиг UE capability (какие RAT и полосы
  разрешено объявлять сети), который отдаётся CP при каждой загрузке.
- Ещё SMP читает `/data/vendor/radio/shared_modem_platform_config` (3 Б)
  и `/vendor/etc/modem_stat.conf`.

SaaiOS не шлёт ничего из этого, и пассивные читатели видели 0 байт.
Отсутствие uecap-конфига — первый кандидат на «только UMTS, нет LTE».

#### Структура кадров (второй захват, полные байты только SMP fd 8/9)

Захват: `--trace-fds`, файл 0600, в консоль выведена только структура
(строки-идентификаторы и малые числа, остальное — длина и sha). Во время
загрузки SMP один раз перезапустился (pid 1044, затем 1071), снят второй
экземпляр. Оба канала — protobuf без внешней рамки, один `write` = один кадр.

`oem_ipc3` — pw_rpc, канал 132 (поле 1 type, 3/4 — fixed32 хэши
service/method, 5 payload, 7 call_id):

1. AP→CP REQUEST: 17 Б, без payload, `call_id=1`. Это первый кадр всей
   сессии. CP отвечает (`5:{1:1}`) примерно через 1 s, и только после этого CP
   начинает запросы на `oem_ipc1`.
2. AP→CP: три запроса с именами `uim_statistics`, `Protocol_stats`,
   `rf_stats` (пустые ответы CP).
3. CP→AP каждые 10 s: 6 температурных пар; AP подтверждает.

`oem_ipc1` — gipc, здесь клиент CP (поле 1: 1 — запрос CP, 2 — ответ AP,
3 — поток CP; 2 — fixed32 канал; 3 — fixed32 seq, общий на сессию):

| Канал | Направление | Смысл |
|---|---|---|
| 7 | CP→AP запрос / AP→CP ответ | чтение свойств: `persist.vendor.modem.experiment.volte_mif_off` (пусто), `persist.vendor.verbose_logging_enabled` → `"false"` |
| 8 | CP→AP поток | логи CP |
| 12 | CP→AP `{1:500000, 2:4000, 3:3, 4:"WILDCARD.binarypb"}` | запрос uecap-файла (лимит, размер чанка) |
| 12 | AP→CP × 125 | `12:{2:{1:offset, 2:bytes[≤4000], 3:0, 4:0}}`, ответ на тот же seq |

Склейка 125 чанков по offset даёт 496 290 Б с sha `7e556ae2…`. Это
байт-в-байт `/vendor/firmware/uecapconfig/WILDCARD.binarypb`. Значит, AP
отдаёт CP неизменённый публичный vendor-файл по запросу CP. Для SaaiOS
нужен host-side ответчик: (а) стартовый pw_rpc-запрос на `oem_ipc3`,
байты которого берутся из стокового захвата; (б) ответы на запросы свойств
на канале 7; (в) отдача файла на канале 12.

### `oem-ipc-responder` — device run (2026-10-05, слот A)

- `diagnostics/oem-ipc-responder.c`. Режим `test` проигрывает
  захваченные кадры CP→AP. На стоковом захвате ответчик выдал **все 133
  кадра AP→CP байт-в-байт** (127 на `oem_ipc1`, 6 на `oem_ipc3`). Других
  байтов нет: известные свойства — из захвата, неизвестные get и
  неизвестные каналы остаются без ответа, в лог пишутся только имена и
  форма кадра.
- Обвязка: вариант `owner-handoff` с REPLAY+EARLY probe (`0d390920…`).
  Ответчик стартует до `POWER_ON` и шлёт hello по `modem_state=ONLINE`.
  uecap — копия стокового vendor-файла (sha `7e556ae2…` проверяется).
- CP ответил, как в стоке: hello-ответ, stats, property set
  (`volte_mif_off`, а ещё `vendor.modem.cat_profile_ongoing_0/1` ×2 — в
  стоке их не было), запрос `WILDCARD.binarypb` → отдано 125 чанков.
- Новое: CP каждые 5 s повторяет запрос на gipc-канале **9** (16427 Б).
  Форма: `{1:'' 2:65536 3:0 4:bytes[16384] 5:2 6:'dds.bin'}`, то есть
  CP хочет записать `dds.bin` (64 KiB; в стоке это
  `modem_userdata/replay/dds.bin`, уходит в CP стадией REPLAY). В
  стоковом захвате записи не было. Формат ответа неизвестен, поэтому без
  ответа.
- Результат через ~250 s: **без изменений** — voice DENIED UMTS
  `reject_raw=0`, PLMN 25501, data 0, `mask_low7=2`, rmnet 0. Отдачи
  uecap-конфига самой по себе недостаточно.

### Стоковый `rild_exynos` против нашего owner (read-only, тот же захват)

`rild.st` (strace `-s 8`, только SIT-заголовок: тип, id, длина) и
logcat той же загрузки. Имена — из `sit-stream.so`
(`tmp-sit-id-names.py`, по первому MOVZ id в builder'е; эвристика,
`0x0800` = RADIO_POWER).

- Сток шлёт около 100 разных SIT-запросов на `umts_ipc0`/`umts_ipc1`.
  Наш owner шлёт 9: `0x093f`, `0x0404`, `0x0800` и GET'ы `0x0200`, `0x0801`,
  `0x0900`, `0x0700`, `0x0701`, `0x0702`.
- До RADIO_POWER у стока дополнительно: `0x0949` SetApSystemTime,
  `0x0922` SendDeviceInfo, `0x4605` SvNumber, `0x090b` SetDebugTrace,
  `0x0c33` ×4 (имя не найдено).
- Ключевое — тайминг по logcat. До ~11.1 s uptime RILJ получает
  `NOT_REG_MT_NOT_SEARCHING_OP`, то есть CP **вообще не ищет сеть**.
  Затем: `< SET_INITIAL_ATTACH_APN` (11.049), `< ENABLE_VONR` (11.070),
  `< GET_RADIO_CAPABILITY` RAF 906119 (11.078),
  `> SET_ALLOWED_NETWORK_TYPES_BITMAP` (11.096) → `<` успех (11.151),
  и в 11.63 SST уже IN_SERVICE на LTE (EARFCN 1500). В strace это
  `0x0603` ×2 (02.86), `0x0954` (04.76), `0x074f` ×2 (04.84). Сдвиг
  wallclock−uptime = 16:25:53.775 по attach rild (4.04 s).
- Значит, стоковый CP начинает поиск только после allowed-network
  bitmap. У нас CP ищет сам (без `0x074f`) и садится на UMTS 25501
  DENIED. Раньше `0x074f` в SaaiOS всегда получал `error_raw=2`, но во
  всех тех прогонах не было обмена SMP на `oem_ipc` (uecap, свойства).
  Повтор `0x074f` (стоковые байты уже восстановлены, VERDICT 22) после
  обмена ответчика ещё не проверялся.

### `0x074f` после обмена ответчика — device run (2026-10-05, слот A)

- sysrq-перезагрузка, затем тот же `owner-handoff-rfs-oemipc.sh` с
  `/data/saaios/etc/ratbm`. Ответчик: hello, stats, свойства, uecap 125
  чанков (как в прошлом прогоне). Owner: stage-1, preferred LTE_WCDMA
  (ok), allow-data (ok).
- `0x0750` до SET: `wire=0x403fe` — **LTE, WCDMA, GSM и NR уже
  разрешены**. `0x0709` band mode `01 00…`. SET `0x074f` (`0x3fe`) →
  **`error_raw=2`**, как во всех прежних прогонах. `0x0750` после:
  `0x3fe`, то есть в RAM применилось, сохранить не удалось (VERDICT 23).
- Через ~4 мин без изменений: voice DENIED UMTS 25501 `reject_raw=0`,
  data 0, `mask_low7=2`, rmnet0 rx/tx 0, IPv4 нет.
- Вывод: RAT-гейт в CP открыт и без нашего SET. Отказ сохранения
  `0x074f` не зависит от обмена на `oem_ipc`. Остающийся видимый сбой
  хранения на стороне CP — неотвеченная запись `dds.bin` на gipc-канале
  9; в стоке её нет.

### Ответ на запись `dds.bin` (gipc ch 9) — device run (2026-10-05, слот A)

- Схема восстановлена из встроенных дескрипторов стоковой
  `libmodem_svc_proto_legacy_soong.so` (CP2A vendor.img, ro, sha
  `13a10cdd…`): `ModemSvcMessage.file_message = 9` →
  `FileMessage{1 write_request, 2 write_response, …}`,
  `FileWriteRequest{1 path, 2 size, 3 offset, 4 data, 5 file_dir,
  6 file_name}`, `FileWriteResponse{1 result}`, `FM_SUCCESS=0`,
  `REPLAY_PATH=2` (SMP: `/mnt/vendor/modem_userdata/replay/`).
- Ответчик принимает только `REPLAY_PATH` + `dds.bin`, размер ≤ 1 MiB,
  пишет в `/data/saaios/var/rfs-quarantine/replay/dds.bin` (0600,
  `O_NOFOLLOW`, `pwrite`+`fsync`) и отвечает
  `08 02 15 09000000 1d <seq> 20 00 4a 04 12 02 08 00`. `modem_userdata`
  (sda7) не открывается. Офлайн: стоковый обмен по-прежнему IDENTICAL
  (127 + 6 кадров); синтетические 4×16 KiB собираются точно, ответ
  разбирается стоковой схемой, запись в `MDS_LOG_PATH`/чужое имя
  отклоняется.
- Прогон: CP принял ответ — вместо повтора чанка 0 каждые 5 с он
  отправил все 4 чанка (seq 11–14, 62.8 с) и дальше периодически
  пересохраняет файл (28 чанк-записей за ~1 мин). Файл 65 536 Б в
  карантине. Необработанных кадров на `oem_ipc` нет.
- `0x074f`: `0x0750` до `0x403fe`, SET (через ~5 с после записи
  `dds.bin`) → **`error_raw=2`**, после `0x3fe`. Voice DENIED UMTS 25501
  `reject_raw=0`, data 0, `mask_low7=2`, rmnet rx 0.
- Вывод: гипотеза «хранение CP заблокировано на `dds.bin`» опровергнута.
  Все запросы CP по `oem_ipc` теперь отвечены, а отказ сохранения
  `0x074f` и UMTS-DENIED остаются. Остающаяся разница со стоком — набор
  SIT-команд rild (сток ~100 id, owner 9; первыми в стоке до успешного
  `0x074f` идут `SET_INITIAL_ATTACH_APN`, `ENABLE_VONR`,
  `GET_RADIO_CAPABILITY`).

### Полный захват стокового `rild` (2026-10-05, слот B, `boot-capture-2`)

strace `-s 65536 -P /dev/umts_ipc0 -P /dev/umts_ipc1`, полные тела
кадров; файл и logcat — в приватных артефактах (gitignored).

- `0x074f` в стоке: тело `fe 03 04 00` = `0x403fe` (RAF LTE|WCDMA|GSM
  + NR, RAF-бит 20 → wire-бит 18), `error 0`. Наш `0x3fe` давал
  `error_raw=2`.
- `0x0603` (CP2A): длина 251, ненулевые байты: `[12]=1`, `[13]=0x0e`,
  APN `internet` с 16, `[218]=3` (IPV4V6), `[248]=3`.
- `0x070a` / `0x0710` сток не шлёт. Вместо них: `0x0740 [00]`,
  `0x0625 [01 00]`, `0x0600` SetupDataCall (983 Б), `0x0613` профили
  данных (246 Б; ims/sos → `error 2`).
- До RADIO_POWER, кроме уже известных `0x0949`, `0x0922`, `0x4605`,
  `0x090b`, `0x093f`, `0x0404`, `0x0800 [02 00 00 00 00 00]`, сток шлёт
  `0x0c20 [01]`, **`0x4600`** (532 Б) и `0x0c33 [00]`.
- `0x4600` = `ProtocolMiscBuilder::BuildSetCpCarrierConfig(const char*,
  char*, int)`: `[12]=0x1e`, путь `/vendor/firmware/carrierconfig` с 16,
  `[272]=0x14` (длина 20), с 276 — 20 байт имени файла
  `manifests/<sha256(manifest)[:20]>`. Манифест выбирает
  `CarrierConfigManager` rild из `cfg.db` по SIM. Связанные индикации:
  `ProtocolMiscCarrierConfigSimInfoIndAdapter`,
  `ProtocolMiscCarrierConfigStatusIndAdapter`.
- Тайминг стока: reg=0 без оператора до ~2.2 с после `0x074f`, затем
  data/voice reg=1, tech 14 (LTE), PLMN 25503.
- `rfsd` CP2A содержит строку `carrierconfig` (старый — нет) и
  `RfsService::File`-сообщения (`Incorrect path`, `Wrong file ID`,
  `RFS_CLOSE`, `Too many opened files`). Гипотеза: CP читает файлы
  carrierconfig только на чтение через RFS File service.
- Каталог `/vendor/firmware/carrierconfig` (CP2A vendor.img,
  `debugfs -c`, ro): `build.info`, `cfg.db`, `cfg.sha2`, `confseqs/`,
  `manifests/`, `release-label`, symlink-mapping'и; 1301 файл, 7.2 MB.

### `0x074f` = `0x403fe` и `0x0603` в форме CP2A — device run (2026-10-05, слот A)

- Owner: SET `0x074f` теперь шлёт `raf_to_sit_ratbm(RATBM_RAF_STOCK)` =
  `fe 03 04 00`; `0x0603` — 251 Б с `[218]=3`, `[248]=3`. Self-test
  сверяет точные байты.
- Результат: SET `0x074f` → **`error 0`**, `0x0603` → `error 0`. Но
  voice `registration_raw=3` DENIED, tech UMTS, PLMN 25501, data 0,
  rmnet rx/tx 0, IPv4 нет.
- Вывод: `error_raw=2` у `0x074f` был из-за неполного bitmap (без NR),
  а не из-за хранения CP; к camp это не относилось.
- Ведущая гипотеза: SaaiOS не шлёт `0x4600` и не отдаёт CP файлы
  carrierconfig, поэтому CP работает с дефолтной конфигурацией
  (автономный поиск, чужой UMTS, без LTE). Следующий шаг:
  восстановить из `rfsd` CP2A, как он отдаёт пути carrierconfig, отдавать
  копию `/vendor/firmware/carrierconfig` только на чтение и повторить
  `0x4600` стоковыми байтами (плюс, возможно, `0x0922`/`0x0c20`/`0x0c33`).

Контроль для сравнения —
[modem-stock-reproduction.md](../targets/panther/modem-stock-reproduction.md)
(стоковый LTE HOME + rmnet1).

### `0x4600` и read-only carrierconfig — device run (2026-10-06, слот A)

- На телефон положена копия дерева в `/data/saaios/var/carrierconfig`.
  Owner шлёт оба стоковых кадра `0x4600` (532 Б, token перезаписывается)
  перед `0x0800` и отвечает на RFS File service в живом цикле, пока идёт
  NV: OPEN cmd 4, cmd 3 без ответа, READ cmd 6 кусками по 2012, cmd 1
  дочитывает остаток и затем статус 0, CLOSE cmd 5, завершающий cmd 3.
  Запись в эти файлы отклоняется.
- CP прочитал оба манифеста и набор `confseqs/` целиком, включая файлы
  больше одного куска (до 49508 Б).
- Счётчик последовательности RFS общий с File service, поэтому NV cmd 7
  приходит с ненулевым seq. Ответ и grant повторяют seq кадра CP.
  Карантин завершился: 95 grant, 189446 байт, `final_ack_sent=1`.
  Исходный EFS не монтировался на запись.
- После этого: SIM ready, voice и data `registration_raw=1`,
  `reject_raw=0`, `tech_raw=14` (`rat_mapped=14`, LTE),
  `plmn_numeric=25503#`. `0x093f` / `0x0404` / `0x0800` с `error_raw=0`.
- rmnet0..3 rx и tx остаются 0, IPv4 на rmnet нет. Канал данных ещё
  не подтверждён.

### `0x0600` SetupDataCall — device run (2026-10-06, слот A)

- Из захвата CP2A извлечён один кадр `0x0600` длиной 983 Б. В теле два
  слота APN с именем `internet`, поля учётных данных пустые. Owner
  подставляет только token в байты 6..9 и шлёт кадр один раз после
  data `registration_raw=1`.
- На устройстве: `camp_setup=sent len=983`, ответ `error_raw=0`.
  Voice и data остаются HOME, LTE, `tech_raw=14`, `reject_raw=0`.
  NV-карантин снова `final_ack_sent=1`, 95 grant, 189446 байт.
- rmnet0..3 rx и tx остаются 0. IPv4 есть только на lo и usb0.
  Канал данных не подтверждён. Ответ на `0x0600` в том прогоне в лог
  не попал по длине.

### Профиль `internet` перед `0x0600` — device run (2026-10-06, слот A)

- В стоковом захвате до первого `0x0600` уходят `0x0625` (14 Б, байт 12
  равен 1, ответ `error 0`) и `0x0613` с APN `internet` (246 Б, ответ
  `error 0`). Owner шлёт их по одному разу после data HOME, затем тот
  же `0x0600`.
- На устройстве: `camp_vonr` `error_raw=0` len 12, `camp_profile`
  `error_raw=0` len 12, `camp_setup` `error_raw=0` **len 304**.
  304 Б — длина стокового успешного ответа SetupDataCall. Voice и data
  остаются HOME, LTE, `tech_raw=14`.
- rmnet0..3 по-прежнему `down`, rx и tx 0. Ядро само интерфейс не
  поднимает. Канал данных не подтверждён. В ответе 304 Б адрес есть,
  но owner его не читает и на `rmnet` не назначает.

### Адрес из ответа `0x0600` на `rmnet1` — device run (2026-10-06, слот A)

- Разбор `ProtocolPsSetupDataCallAdapter::Init` в `libsitril`: в ответе
  cid на байте 14, тип PDP на 16 (1 или 3 — есть IPv4), четыре байта
  адреса на 17. Сток при cid 2 поднимает `rmnet1`. Owner назначает
  этот адрес как `/32` и делает `ip link set up`. Сам адрес в лог не
  пишется. Маршрут по умолчанию не трогается.
- На устройстве: ответ снова `error_raw=0` len 304,
  `camp_setup if=rmnet1 ipv4=yes prefix=32 up=1 add=1`.
  `rmnet1` operstate `unknown`, затем один ICMP на 8.8.8.8 через
  `rmnet1` завершился с кодом 0. Счётчики `rmnet1`: rx 84 / 1 пакет,
  tx 324 / 6 пакетов. Канал данных подтверждён.

### Маршрут и DNS — device run (2026-10-06, слот A)

- Тот же разбор `libsitril`: семья DNS на байте 37, IPv4-серверы на
  38 и 58, если семья 1 или 3. Owner пишет их в `/run/resolv.conf` и
  ставит `default dev rmnetN`. Связанный маршрут usb0 не заменяется.
  Адреса в лог не пишутся.
- На устройстве: `route=1`, `dns=yes count=2`, в таблице
  `default dev rmnet1` и `usb0` /24. ICMP на 8.8.8.8 без привязки к
  интерфейсу — код 0. ICMP на имя `one.one.one.one` — код 0.
  `rmnet1` rx 378, tx 482.

### IPv6 из того же ответа `0x0600` — device run (2026-10-06, слот A)

- `SetIfAddrIpv6` в `libsitril` кладёт префикс 64 (`mov w8, #0x40`).
  16 байт адреса лежат на смещении 21, если тип PDP 2 или 3. Owner
  назначает их как `/64` на тот же `rmnet(cid-1)` и ставит
  `ip -6 route replace default dev rmnetN`. Адрес в лог не пишется.
- На устройстве: `camp_setup if=rmnet1 ipv6=yes prefix=64 up=1 add=1
  route=1`. На `rmnet1` два `inet6` (link-local и назначенный).
  `ip -6 route get` для 2001:4860:4860::8888 выбирает `dev rmnet1`.
  Один ICMPv6 туда завершился с кодом 0: rx 104 → 208, tx 344 → 448.

### TCP по тому же bearer — device run (2026-10-06, слот A)

- Тот же owner, без новой загрузки. `busybox wget` на `http://example.com/`
  завершился с кодом 0, тело 577 Б. Счётчики `rmnet1`: rx 208 → 1654,
  tx 640 → 1208. Тело ответа не читалось.

### `0x0605` SetFastDormancy — device run (2026-10-06, слот A)

- Сразу после успешного `0x0600` сток шлёт `BuildSetFastDormancyInfo`,
  16 Б, четыре байта из захвата. Owner повторяет этот кадр один раз
  после назначения адресов. Тело в лог не пишется.
- На устройстве: `camp_fastdorm=sent len=16`, ответ `error_raw=0`
  len 12. IPv4/IPv6 на `rmnet1` остаются. `wget` на `example.com`
  снова код 0, тело 577 Б, rx 0 → 1446, tx 192 → 760.

### Профили `ims` и `sos` — device run (2026-10-06, слот A)

- После `0x0605` сток шлёт ещё два `0x0613`, APN-слоты `ims` и `sos`.
  Отличие от кадра `internet` — только этот слот и два числовых поля
  в конце. Owner шлёт каждый кадр один раз.
- На устройстве оба ответа `error_raw=2`, len 12. Канал данных не
  сломался: `wget` на `example.com` код 0, тело 577 Б, rx 0 → 1444,
  tx 144 → 712. В стоковом захвате эти же кадры `ims` и `sos` сразу
  после data call тоже получают `err=2`; отказ совпадает со стоком.

### `0x090c` GetModemActivityInfo — device run (2026-10-06, слот A)

- `ProtocolMiscBuilder::GetModemActivityInfo` — пустой GET на 12 Б.
  Сток шлёт его после SetupDataCall и до fast dormancy. Ответ 140 Б
  в лог не пишется.
- На устройстве: `camp_activity=sent len=12`, ответ `error_raw=0`
  len 140. Дальше fast dormancy снова `error_raw=0`. `wget` на
  `example.com` код 0, тело 577 Б, rx 0 → 1445, tx 192 → 760.

### ENDC, VoNR capability и RC network type — device run (2026-10-06, слот A)

- После профилей сток читает `0x073e` GetEndcMode (пустой GET, ответ
  14 Б), ставит `0x0954` SetVonrCapa со словом 0 (кадр 16 Б) и читает
  `0x0718` GetRCNetworkType (пустой GET, ответ 16 Б). Все три в стоке
  с `err=0`. Owner шлёт каждый кадр один раз; тела ответов в лог не
  пишутся.
- На устройстве: `camp_endc` `error_raw=0` len 14, `camp_vonrcapa`
  `error_raw=0` len 12, `camp_rcnet` `error_raw=0` len 16. `ims` и
  `sos` снова `error_raw=2`. `wget` на `example.com` код 0, тело
  577 Б, rx 0 → 1445, tx 240 → 808.

### Data throttling, unsolicited filter и screen state — device run (2026-10-06, слот A)

- После `0x0718` сток ставит `0x094d` SetDataThrottling (21 Б: байт 0
  и длительность 30000), `0x0928` SetUnsolicitedResponseFilter (слово
  `0x7f`) и `0x0902` SetScreenState (слово 1). Все три в стоке с
  `err=0`, ответ 12 Б. Кадр `0x0208` между ними не шлётся: это длинный
  SIM status. Повтор `0x074f` тоже не шлётся — bitmap уже ушёл при
  регистрации.
- На устройстве все три ответа `error_raw=0` len 12. `wget` на
  `example.com` код 0, тело 577 Б, rx 0 → 1445, tx 192 → 760.

### Cell info и SMSC — device run (2026-10-06, слот A)

- После screen state сток читает `0x070c` GetCellInfoList (пустой GET,
  ответ 2228 Б) и `0x0108` BuildSmscAddress (пустой GET, ответ 25 Б).
  Оба в стоке с `err=0`. Тела ответов в лог не пишутся. `0x075c` в
  TD1A `libsitril` не назван и в стоке получает `err=6`, поэтому не
  шлётся. `0x0100` длиной 271 Б содержит строки из захвата и тоже не
  шлётся.
- На устройстве: `camp_cellinfo` `error_raw=0` len 2228, `camp_smsc`
  `error_raw=0` len 25. `wget` на `example.com` код 0, тело 577 Б,
  rx 0 → 1446, tx 192 → 760.

### `0x0953` GetVonrCapa — device run (2026-10-06, слот A)

- Последний именованный кадр стока после SMSC: пустой GET
  `BuildGetVonrCapa`, в захвате `err=0`, ответ 16 Б. Owner шлёт его
  один раз. Тело ответа в лог не пишется.
- На устройстве: `camp_vonrget` `error_raw=0` len 16. `wget` на
  `example.com` код 0, тело 577 Б, rx 0 → 1446, tx 240 → 808.
- Именованный хвост после data call на этом закрыт. Дальше сток только
  повторяет регистрацию, activity и cell info. Не отправляются:
  `0x0100` (271 Б, в захвате есть строки), `0x075c` (в TD1A
  `libsitril` не назван, в стоке `err=6`), `0x0208` (длинный SIM
  status), `0x0c14` (opcode в `libsitril` и `libril_sitril` не
  находится).

### IPv6 DNS из ответа `0x0600` — device run (2026-10-06, слот A)

- `ProtocolPsSetupDataCallAdapter::Init` при семье DNS 2 или 3 копирует
  16 байт с item+30 и item+50. Owner дописывает их в `/run/resolv.conf`
  после IPv4. Адреса в лог не пишутся.
- На устройстве: `dns=yes count=2`, `dns6=yes count=2`, в
  `resolv.conf` четыре `nameserver`, две строки с двоеточием.
  `wget` на `example.com` код 0, тело 577 Б, rx 0 → 1446, tx 192 → 760.

### Ранние именованные кадры после VoNR GET

- В захвате до data call сток ещё шлёт кадры, которых не было в хвосте.
  Из `libsitril` названы и не содержат идентификаторов:
  `0x0949` `BuildSetApSystemTime` (18 Б, шесть полей `localtime`:
  год-1900, месяц, день, час, минута, секунда — на устройстве текущие,
  не снимок из захвата), `0x090b` `SetDebugTrace` (байт 0),
  `0x0903` `SetTtyMode` (слово 0), `0x0711` `BuildGetPsService`
  (пустой GET), `0x0740` `BuildSetPreferredDataModem` (байт 0).
  Все в стоке с `err=0`. Owner шлёт каждый один раз после `0x0953`.
  Тела ответов в лог не пишутся.
- На устройстве: `camp_aptime` `error_raw=0` len 13, `camp_dbgtrace`
  `error_raw=0` len 12, `camp_tty` `error_raw=0` len 12, `camp_pssvc`
  `error_raw=0` len 13, `camp_prefmodem` `error_raw=0` len 12.
  `wget` на `example.com` код 0, тело 577 Б, rx 0 → 1457, tx 240 → 808.
  В `resolv.conf` по-прежнему четыре `nameserver`, две строки с
  двоеточием.
- По-прежнему не шлются: `0x0100`, `0x075c`, `0x0208`, `0x0c14`,
  `0x0403` GetDevID, `0x0922` SendDeviceInfo, `0x4605` SvNumber.

### Слот SIM, пороги сигнала и SMS broadcast

- После preferred-data modem сток ещё читает `0x024d`
  `BuildSimGetSlotStatus` (пустой GET, ответ 433 Б) и ставит `0x0943`
  `BuildSetSignalReportCriteria` (67 Б: гистерезис 3000, четыре порога
  −109/−103/−97/−89, слово доступа 1, два флага 1) и `0x0107`
  `BuildSmsBroadcastActivation` (слово 0). Все в стоке с `err=0`.
  Owner шлёт каждый один раз. Тело слота в лог не пишется.
- На устройстве: `camp_slot` `error_raw=0` len 433, `camp_sigcrit`
  `error_raw=0` len 12, `camp_smsact` `error_raw=0` len 12.
  `wget` на `example.com` код 0, тело 577 Б, rx 0 → 1443, tx 240 → 868.
  В `resolv.conf` четыре `nameserver`, две строки с двоеточием.

### Пороги ёмкости канала `0x0944`

- `BuildSetLinkCapaReportCriteria` в захвате десять раз с одним телом:
  гистерезис 3000, два слова 50, списки скоростей 14 и 11 значений.
  Слово доступа в конце — ненулевые элементы таблицы `libsitril`
  `0xd8914`, по порядку 1, 2, 3, 5, 4. Owner шлёт эти пять кадров
  один раз. Тело в лог не пишется, в строке отправки только индекс
  доступа. На устройстве все пять ответов `error_raw=0` len 12,
  доступ 1, 2, 3, 5, 4. `wget` на `example.com` код 0, тело 577 Б,
  rx 0 → 1444, tx 240 → 808.

### Конфиг широковещательных SMS `0x0106`

- `BuildSetBroadcastSmsConfig` кладёт счётчик в байт +12 и дальше
  записи по 7 байт: границы service id и три хвостовых байта из
  захвата. Сток шлёт два кадра, оба с `err=0`: 7 записей (62 Б) и
  9 записей (76 Б). Диапазоны — публичные ETWS/CMAS. Owner шлёт
  оба один раз. Список записей в лог не пишется. На устройстве оба
  ответа `error_raw=0` len 12, длины 62 и 76, счётчики 7 и 9.
  `wget` на `example.com` код 0, тело 577 Б, rx 0 → 1445, tx 240 → 868.

### Список вызовов `0x0000`

- Сток один раз читает `BuildGetCallList`, пустой GET, в захвате
  `err=0`, ответ 16 Б. Owner раньше слал его только после исходящего
  звонка. Теперь ещё один раз после SMS-конфига. В лог попадают
  код ошибки, длина и число записей.
- Устройство: `elapsed_ms=81138`, `error_raw=0`, `len=16`, `count=0`.
  wget `example.com` exit 0, тело 577 Б, rmnet1 rx `0→1446`, tx `192→760`.

### Голос `0x091a` и версия baseband `0x0901`

- Это последние два кадра стока, у которых есть имя в `libsitril` и
  восстановленное тело без идентификаторов. `BuildSetVoiceOperation`
  кладёт int32 `3` в кадр 16 Б. `GetBaseBandVersion` — байт-селектор
  `0xFF`, длина 13. В захвате оба с `err=0`. Owner уже умел оба, но
  голос уходил только при файле `opx-step`, а версия — только при
  `bbver`. На обычной загрузке (`opx_step=0`, `bbver=off`) теперь
  оба уходят один раз после списка вызовов. В лог версии попадает
  только строка SW; HW и дата RF-cal не печатаются.
- Устройство: `camp_voiceop` mode 3 на `elapsed_ms=81988`, ответ
  `error_raw=0`. `camp_bbver` на `elapsed_ms=83621`,
  `sw_version=g5300q-260317-260505-B-15346003`, `error_raw=0`.
  wget `example.com` exit 0, тело 577 Б, rmnet1 rx `0→1446`, tx `192→760`.
### GPS lock, GPS NFW и SA mode

- `rcmMsgToString` в `sit-base.so` называет три кадра, которые сток
  шлёт с коротким телом: `0x0c20` `SIT_SET_GPS_LOCK_MODE` (байт 1,
  13 Б), `0x0c33` `SIT_SET_GPS_NFW_STATUS` (байт 0, 13 Б) и `0x0755`
  `SIT_GET_SA_MODE` (пустой GET). В захвате все три с `err=0`.
  Owner шлёт каждый один раз после baseband. Тело ответа SA в лог
  не пишется.
- Устройство: `camp_gpslock` `elapsed_ms=84709` `error_raw=0` len 12,
  `camp_gpsnfw` `elapsed_ms=86239` `error_raw=0` len 12,
  `camp_samode` `elapsed_ms=87765` `error_raw=0` len 25.
  wget `example.com` exit 0, тело 577 Б, rmnet1 rx `0→1445`, tx `240→808`.

### AIMS: остановка XCAP, остановка стека, старт XCAP

- `rcmMsgToString` называет `0x0d3c` `SIT_AIMS_XCAPM_STOP_REQ`,
  `0x0d3a` `SIT_AIMS_STACK_STOP_REQ` и `0x0d3b`
  `SIT_AIMS_XCAPM_START_REQ`. В захвате первый кадр каждого — 14 Б,
  тело `01 01`, `err=0`. Owner шлёт эти три один раз после SA mode.
- Устройство: `camp_xcapstop` `elapsed_ms=89520` `error_raw=0` len 12,
  `camp_aimstop` `elapsed_ms=91051` `error_raw=0` len 12,
  `camp_xcapstart` `elapsed_ms=92582` `error_raw=0` len 12.
  wget `example.com` exit 0, тело 577 Б, rmnet1 rx `0→1445`, tx `240→808`.
- Сток потом шлёт те же stop-коды ещё раз с телом `00 01`, оба
  с `err=0`: сначала `0x0d3a`, затем `0x0d3c`. Owner шлёт и их.
- Устройство: `camp_aimstop0` `elapsed_ms=93566` `error_raw=0` len 12,
  `camp_xcapstop0` `elapsed_ms=95088` `error_raw=0` len 12.
  wget `example.com` exit 0, тело 577 Б, rmnet1 rx `0→1445`, tx `240→808`.

### Битовая карта сетей `0x074f`

- Сток шлёт `SetAllowedNetworkTypeBitmap`, 16 Б, слово `0x403fe`
  (`fe 03 04 00`), `err=0`. Owner собирал это слово и слал его сразу
  после регистрации, только если есть файл `/data/saaios/etc/ratbm`.
  С файлом на устройстве ранний SET ответил `error_raw=2` при
  `elapsed_ms=17945`; GET до и после показал уже `wire=0x403fe`
  (LTE, WCDMA, GSM, NR), `error_raw=0`.
- Файл убран в `archive-cc30/armed-run`, и тот же кадр уходит один
  раз после второго AIMS stop. Если файл вернуть, длинная цепочка
  GET/SET снова занимает этот opcode и хвост его не повторяет.
- Устройство без файла: `camp_ratbm=off`, `camp_xcapstop0`
  `elapsed_ms=89716` `error_raw=0` len 12,
  `camp_ratbm=sent step=set_allowed_bitmap_stock wire=0x403fe
  elapsed_ms=91217`, ответ `error_raw=2`. Повтор не шлётся.
  wget `example.com` exit 0, тело 577 Б, rmnet1 rx `0→1446`,
  tx `240→808`.
- `0x0d39` `SIT_AIMS_STACK_START_REQ` в захвате без ответа, поэтому
  не шлётся. `0x0d0b` `SIT_AIMS_SET_FRAME_TIME` несёт меняющуюся
  метку времени, её не повторяем.
- По-прежнему не шлются: `0x0413` `SIT_GET_IMEI_MAPPING_INFO`,
  `0x0c14` `SIT_SET_GPS_SUPL_NI_READY` (два байта приходят из
  буфера вызывающего, не константа), `0x020e` `SIT_CLOSE_SIM_CHANNEL`
  (в сыром захвате тела `u32` 1 и 2, кадр закрывает SIM-канал),
  плюс кадры с секретами `0x0400`, `0x0403`, `0x0100`, `0x0208`,
  `0x0209`, `0x020f`, `0x0247`, `0x0922`, `0x090d`,
  `0x096d` null-cipher, `0x097b` identifier disclosure,
  `0x097e` security algorithms, и `0x075c` (строка в запросе,
  в стоке `err=6`).

### Остальные пороги сигнала `0x0943`

- В первую секунду захвата сток шлёт десять тел
  `SetSignalReportCriteria`. Первое (доступ 1, пороги
  −109/−103/−97/−89) owner уже слал. Ещё девять — из той же
  секунды, с `err=0`: доступы 2, 3, 5, затем выключенный 3,
  доступ 3 с порогами −3/1/5/13, доступ 4, и три выключенных
  (доступы 4, 4, 2). Пороги, которые сток поправляет секундой
  позже, не повторяем.
- Устройство, все девять `error_raw=0` len 12:
  доступ 2 `elapsed_ms=92724`, 3 `94281`, 5 `95835`,
  3/флаг 4 `97536`, 3/флаг 5 `99259`, 4 `100965`,
  4/флаг 7 `102658`, 4/флаг 8 `104407`, 2/флаг 9 `106157`.
  wget `example.com` exit 0, тело 577 Б, rmnet1 rx `0→1446`,
  tx `240→808`.

### Фильтр индикаций `0x0928`

- Сток сначала шлёт `SIT_SET_IND_CMD_FILTER` со словом `0xff`
  (`ff 00 00 00`, два раза, `err=0`), затем слово `0x7f`. Owner
  раньше слал только `0x7f`. Теперь перед ним один раз уходит
  `0xff`, и последним остаётся `0x7f`.
- Устройство: `camp_unsolff` `elapsed_ms=42270` `error_raw=0` len 12,
  `camp_unsol` `elapsed_ms=43855` `error_raw=0` len 12.
  wget `example.com` exit 0, тело 577 Б, rmnet1 rx `0→1446`,
  tx `192→760`.

### Возможности телефона `0x0615`

- Сток дважды читает `SIT_GET_PHONE_CAPABILITY`. Запрос — 12 байт,
  поле длины в заголовке остаётся 0 (у остальных пустых GET там 12).
  Ответ 16 байт, `err=0`, четыре счётчика `2,1,1,2`. Owner шлёт
  этот запрос один раз после остальных порогов сигнала.
- Устройство: `camp_phonecap` `elapsed_ms=108343` `error_raw=0`
  len 16, `pay=2,1,1,2`. wget `example.com` exit 0, тело 577 Б,
  rmnet1 rx `0→1445`, tx `240→808`.

### Третий AIMS stop, ATR и SVN

- Сток шлёт `0x0d3c` ещё раз с телом `00 02`, ответ `err=0` len 12.
  Парная запись `0x0d3a` с тем же телом в захвате без ответа,
  поэтому не шлётся. Дальше пустой `SIT_GET_ATR` `0x0212`
  (12 байт, поле длины 12, в стоке `err=0`, ответ 47 байт) и
  `SendSvnInfo` `0x4605` (14 байт, две цифры из захвата,
  `err=0`, ответ 12 байт). Тело ATR и цифры SVN в лог не пишутся.
- Устройство: `camp_xcapstop2` `elapsed_ms=110035` `error_raw=0`
  len 12, `camp_atr` `elapsed_ms=111574` `error_raw=0` len 47,
  `camp_svn` `elapsed_ms=113084` `error_raw=0` len 12.
  wget `example.com` exit 0, тело 577 Б, rmnet1 rx `936→2381`,
  tx `720→1288`.

### Индикации, которые сток не запрашивает

- Именованные запросы из сырого захвата, у которых `err=0` и тело
  не секрет, уже уходят по одному разу. Три тела `0x0943` в
  `21:00:02` — живые правки уже отправленных порогов, их не повторяем.
  `0x0d39` и `0x0d3a` с телом `00 02` в захвате без ответа.
- Индикация `0x070e` `SIT_IND_NTW_STATE_CHANGED` (290 Б) уже пишет
  трассер `sit_event`. Тело не логируется.
- `0x0742` `SIT_IND_PHYSICAL_CHANNEL_CONFIG` при длине 1004 байт
  идёт в `FillPhysicalChannelConfigV1_6`. Счётчик — знаковое слово
  по смещению 8, первая запись с +12, шаг 62. В лог один раз:
  status, RAT через таблицу `0xd8afc`, номера каналов, ширина
  downlink/uplink, PCI и band. Список context id не пишется.
- Устройство: `camp_ind phy id=0x0742 len=1004 count=1 status=1
  rat=14 dl_ch=1500 ul_ch=19500 dl_bw=20000 ul_bw=20000 pci=114
  band=3`. wget `example.com` exit 0, тело 577 Б, rmnet1 rx
  `0→1445`, tx `240→808`.
- `0x0720` `SIT_IND_AC_BARRING_INFO`, кадр 41 байт.
  `OnAcBarringInfo` отдаёт пять байт: `forEmc` по смещению 8,
  `forMoSig` по 9, `forMoData` по 17, `forMmtelVoice` по 25,
  `forMmtelVideo` по 33. Слова factor/timer из той же записи
  в лог не пишутся.
- Устройство: `camp_ind acbar id=0x0720 len=41 emc=0 mo_sig=100
  mo_data=0 voice=255 video=255`. wget `example.com` exit 0,
  тело 577 Б, rmnet1 rx `2551→3996`, tx `1048→1676`.
- `0x074b` `SIT_IND_BARRING_INFO_CHANGED`. Байт типа соты по
  смещению 8 выбирает длину префикса через таблицу `0xd8958`
  (версии 3 и 4 дают одну длину: LTE и NR — 282). Дальше слово
  count и записи по 17 байт: service, barring type, factor,
  time, isBarred. В лог идёт первая запись. Тело cell identity
  не пишется.
- Устройство: `camp_ind barring id=0x074b len=413 cell=2 count=7
  svc=8 kind=0 factor=0 time=0 barred=0`. wget `example.com`
  exit 0, тело 577 Б, rmnet1 rx `720→2165`, tx `960→1528`.
- Вне этой маски сток ещё шлёт `0x0906` `SIT_IND_SIGNAL_STRENGTH`,
  `0x0604` `SIT_IND_DATA_CALL_LIST_CHANGED` и
  `0x0945` `SIT_IND_CURRENT_LINK_CAPACITY_ESTIMATE`. Первая запись
  увидела сигнал (206 Б) и список звонков данных (301 Б). Оценка
  канала пришла позже тем же кадром, 24 Б.
- Адаптер стока читает у `0x0906` знаковое полуслово по смещению 8
  и берёт младшие семь бит присутствия технологий. У `0x0945` четыре
  слова кбит/с: downlink и uplink по смещениям 8 и 12, вторичные
  по 16 и 20. Отрицательное слово сток считает отсутствующим.
  Тело `0x0604` — счётчик и тот же элемент на 292 байта, что в ответе
  `0x0600`. В лог идут count, cid, active и тип PDP. Адрес не пишется.
- Устройство: `camp_ind signal id=0x0906 len=206 mask_low7=4`,
  `camp_ind datacall id=0x0604 len=301 count=1 cid=2 active=1 pdp=3`,
  `camp_ind linkcap id=0x0945 len=24 dl=59454 ul=4800 dl2=0 ul2=0`.
  wget `example.com` exit 0, тело 577 Б, rmnet1 rx `480→1926`,
  tx `720→1288`.
