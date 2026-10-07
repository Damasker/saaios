# Sprint 26 — лучшая сборка panther (модем + GPU + образ)

## Паспорт

- Состояние: `In progress`. Слот A — SaaiOS и сейчас ONLINE.
  Bearer подтверждён на одноразовом diagnostic owner (см. Evidence);
  owner не вшит в `native-init`. Образ S27 старше модемных коммитов
  после `0d8c228`. Пока этот owner держит модем: не перезагружать
  Pixel, не прошивать, не трогать COM13 и текущий процесс owner.
  Прошивка нового образа — отдельный подтверждённый шаг.
- Зависит от: S01-S12 (`Done`), MODEM-ROADMAP (MODEM-02/03/07),
  MODEM-10 (новый, центровой), gpu.md acceptance-последовательность.
- Архитектурные решения: собрать единый образ, сводящий три воркстрима —
  (1) реальный модемный camp+bearer через валидный dual-handle NV
  (MODEM-10), (2) интеграцию Mali-кита в vendor_boot без автозагрузки +
  трек Mesa Panthor (gpu.md), (3) корректный порядок загрузки CPIF/PCIe/
  PMIC/GSA модулей и инвентарь железа. Проприетарные блобы и NV — только
  на устройстве, в git идут хеши/доки.
- Рабочий fallback: сток в слоте B (физически подтверждён: LTE HOME +
  rmnet1, Mali r54p3) и текущий SaaiOS слот A — путь возврата без
  изменений; ручной `fastboot` с хоста остаётся независимым.

## Goal

Один образ SaaiOS для panther, который мы можем собрать и проверяемо
прошить, объединяющий максимум подтверждённых возможностей: реальная
сотовая связь (bearer на `rmnet`), задел под GPU-ускорение (render node
на подтверждённом драйвере) и корректная инициализация radio-adjacent
железа — каждое с измеримым приёмочным гейтом, а не «вроде работает».

## Current state

Проверенные факты до изменения.

**Модем.** Слот A (SaaiOS) ONLINE, 2026-10-06. На том же diagnostic
owner, без новой загрузки: LTE HOME, `rmnet1` IPv4 `/32` и IPv6 `/64`,
IPv4 DNS, ICMP и TCP (`wget http://example.com/` код 0, тело 577 Б,
счётчики `rmnet1` сдвинулись). Это одноразовый diagnostic, не сервис
в `native-init`. После data call стоковая цепочка отправлена по одному
разу до `0x070c` GetCellInfoList (ответ 2228 Б, `error 0`) и `0x0108`
SMSC (ответ 25 Б, `error 0`); тела этих ответов в лог не пишутся.
`ims` и `sos` дают `error_raw=2`, как в стоке. На кадрах LTE HOME
`lac=0` и `cid=0`. IPv6 DNS не установлен. Намеренно не отправлены:
`0x0100` (271 Б, строки из захвата), `0x075c` (в TD1A `libsitril` без
имени, в стоке `err 6`), `0x0208` (длинный SIM status), `0x0c14`
(опкода нет в `libsitril` и `libril_sitril`). Следующий именованный
безопасный GET — `0x0953` GetVonrCapa; в этой записи он не
реализуется. Разворот VERDICT 27 сохраняется: прежний провал
регистрации был артефактом пустого `NV_NORM` и обслуживания только
handle-3. Детали прогонов:
[MODEM-10](MODEM-10-REAL-NV-DUAL-HANDLE.md),
[modem-stock-reproduction.md](../targets/panther/modem-stock-reproduction.md).

**GPU.** Подтверждённый путь SaaiOS — Exynos DRM/KMS scanout (BGRX,
`drm-splash`), GPU-ускорение **не** заявлено. Сток гоняет Mali-G710 MP7
на kbase r54p3 + CSF `mali_csffw-r54p3.bin`. Кит снят и проверен по
SHA-256 (`os/targets/panther/build-gpu-vendor-boot.sh` пакует модули+CSF
в vendor_boot без автозагрузки; userspace-блобы Bionic на musl не
заведутся → открытый путь Mesa Panthor). Детали:
[gpu.md](../targets/panther/gpu.md).

**Radio-adjacent железо.** Порядок модулей `shm_ipc → cpif_page → cpif →
cp_thermal_zone` **после** `pcie-exynos-gs` + `google_modemctl`; PCIe RC
`11920000` (Shannon) ≠ `14520000` (BCM4389 Wi-Fi); PMIC `s2mpg12`;
зарезервированные `cp_shmem`/`cp_rmem*`; GSA/Trusty для protected-NV.
Инвентарь на `origin/wip/local-pixel7-orphan` (камеры/аудио/Qi-PD/
термалка/Mali). Детали:
[hardware-risks.md](../targets/panther/hardware-risks.md).

**Сборка.** Entry-points: `os/targets/panther/build-native-c-image.sh`,
OTA-цепочка `build-saai-ota-stage.sh`/`-write.sh`/`-health.sh`
(S12 `Done`), `build-gpu-vendor-boot.sh`, `deploy/package.sh`. Образ
S27 собран на сервере с `0d8c228` и не содержит модемные коммиты после
него, включая `7084de0` (cell-info и SMSC). `saai-gpu-compositor` в
бандле нет: NDK для него нет. Повторная сборка образа идёт отдельно и
в этот шаг не входит. Пока слот A ONLINE, собранный образ не
прошивается.

## Scope

- входит:
  - **Воркстрим M (модем):** MODEM-10 целиком — валидный dual-handle NV →
    camp → `SETUP_DATA_CALL EUTRAN` → bearer;
  - **Воркстрим G (GPU):** упаковка Mali-кита (модули + `mali_csffw-r54p3`)
    в vendor_boot без PID-1-автозагрузки через `build-gpu-vendor-boot.sh`;
    параллельный трек Mesa Panthor + GBM/EGL как SaaiOS-native userspace;
    scanout остаётся Exynos DRM `card0` (BGRX);
  - **Воркстрим H (железо/образ):** корректный порядок загрузки CPIF/
    PCIe/PMIC/GSA модулей в образе; вкатывание инвентаря из
    `wip/local-pixel7-orphan`;
  - **Воркстрим B (сборка):** определить и зафиксировать тулчейн/
    окружение, собрать единый образ из последних изменений, записать
    артефакты+хеши; прошивка — отдельным подтверждённым шагом, не в этом
    DoR автоматически.
- не входит:
  - запуск vendor `cbd`/`rfsd`/`rild`/RIL как сервисов;
  - запись реального EFS/NV/RF-cal/прошивки модема; cross-slot A/B
    failover; использование слота B под SaaiOS;
  - voice/SMS/IMS; GPU в PID 1 (только задел render node вне PID 1);
  - коммит проприетарных блобов (Mali, NV) в git.

## Change

Малые вертикальные задачи в порядке выполнения.

1. **B1 — окружение сборки.** Проверить наличие Linux/WSL + cross-тулчейна
   (`aarch64-...-musl`, zig cc), прочитать build-скрипты и S12 OTA-гейт,
   зафиксировать точную последовательность сборки. Если окружения нет —
   честно сообщить, что именно отсутствует.
2. **M1 — device-проверка модема (слот A).** Прогнать dual-handle owner
   (MODEM-10 Change 1-3) до camp + bearer. Центровой результат спринта.
   На diagnostic owner выполнен (Evidence). В `native-init` owner не
   вшит; повторный прогон и прошивка сейчас запрещены.
3. **G1 — vendor_boot с Mali.** `build-gpu-vendor-boot.sh` против кита
   (`C:\Users\Admin\Desktop\saaios-panther-gpu-kit`, SHA сверен), модули
   + CSF в ramdisk без автозагрузки; затем bounded ручная загрузка
   `gpu_cooling → mali_pixel → mali_kbase` + проверка `/dev/mali0` и
   `renderD128`.
4. **H1 — порядок модулей в образе.** Зафиксировать и проверить
   CPIF/PCIe/PMIC/GSA load-order в образе; вкатать инвентарь железа.
5. **B2 — единый образ.** Собрать образ из последних изменений (M/G/H),
   записать артефакты, размеры, версии, хеши. Прошивка — отдельно, с
   подтверждением.

## Test

- unit/protocol: host-фикстуры MODEM-10 (serve/seed), `build-gpu-vendor-
  boot.sh` dry-run, манифест/хеши образа;
- host integration: сборка workspace чисто (`cargo build`/clippy/fmt где
  применимо), сверка SHA-256 всех блобов по `gpu-kit.sha256`;
- image inspection: `inspect-image` на собранном образе; порядок модулей;
  наличие CSF на firmware search path; отсутствие автозагрузки Mali в
  PID 1;
- fault injection: прерванная сборка/битый блоб отвергается по хешу;
- device/physical/cold reboot: M1 bearer-прогон и G1 render-node — на
  реальном Pixel 7 слот A, с холодной перезагрузкой; реальный EFS
  read-only сверка до/после.

## Acceptance criteria

- **Модем (гейт связи):** `rmnet*` UP с IPv4, rx/tx ненулевые под
  реальным трафиком, после `SETUP_DATA_CALL cause=NONE` и домашней/
  роуминговой регистрации — не «ACK на пробе» (MODEM-10);
- **GPU:** `/dev/mali0` + `/dev/dri/renderD128` присутствуют после
  bounded ручной загрузки подтверждённых модулей + CSF (трек stock-
  драйвера); либо зафиксированная веха Panthor render — без добавления
  GPU в PID 1;
- **Железо/образ:** порядок загрузки CPIF/PCIe/PMIC/GSA в образе
  соответствует подтверждённому стоковому; инвентарь вкатан;
- **Сборка:** единый образ собран из последних изменений, артефакты и
  SHA-256 записаны; проприетарные блобы и NV в git **не** попали;
- честно зафиксированы ограничения: что проверено на устройстве, что
  осталось заделом (Panthor userspace, GPU в PID 1, cross-slot).

## Threat / privacy impact

- максимальный blast radius — прошивка образа (S12-паттерн: явное
  согласие пользователя перед первой реальной записью/прошивкой);
- проприетарные блобы (Mali) и NV — только на устройстве, в git хеши;
  приватных ключей/идентификаторов в образе и логах нет;
- модемный bearer открывает host-маршрут только после реальной
  регистрации; новой сетевой поверхности сверх уже проверенной нет;
- `google_bcl.ko` не грузить отдельно «ради GPU» (тянет charger/MFD) —
  только в составе подтверждённого стокового дерева питания.

## Rollback

- каждый воркстрим ревертится независимо: модем → passive owner; GPU →
  нет vendor_boot-пака / нет ручной загрузки; образ → предыдущий
  подтверждённый артефакт слота A;
- слот B (сток) и ручной `fastboot` с хоста — независимый возврат при
  любом состоянии нового образа.

## Консолидация веток (2026-10-06)

Единая линия Pixel 7 — `pixel-7-main`. `feat/som-v1` целиком в ней.

- Влито: `feat/lockscreen-policy-widgets` (ADR-424 revoke confirm +
  protect current key, ADR-425 lock widgets).
- Уже есть в `pixel-7-main` своими решениями: `feat/pixel7-native-saaios`
  ADR-148 (PIN-точки) → ADR-149 lock-pin-field; ADR-152 (lock attention)
  → ADR-148 lock-attention; `feat/vui-04-navigation` ADR-117
  (`SystemStatus`, `pressed_tab`, `reduced_motion`); серверная
  `fix/wayland-drm-pipeline` → `flip_pending` gating (ADR-390).
- Не перенесено: `Frame::Root` removal (native ADR-149) — чистка
  мёртвого кода в разошедшемся `saai-shell`; старые `saai-displayd`
  (S01–S03, `display-supervisor.c`) — заменены текущим displayd.
- Архив веток: `archive/feat/som-v1`, `archive/feat/pixel7-active`,
  `archive/wip/local-pixel7-orphan`, `archive/feat/s03-pixel7-displayd`,
  `archive/research/panther-modem-20260924`.
- Незакоммиченное из worktree (только исходники и доки, без блобов и
  логов): `archive/rescue/pc-*-2026-10-06`,
  `archive/rescue/server-*-2026-10-06`, серверные stash —
  `archive/rescue/server-stash-*`.
- Из rescue в `pixel-7-main` взяты инструменты: `sit-hold-channels.c`,
  `super-ro-map.c`, `rfs-peek.c`, `saaios-verify-nv-copies.sh`,
  `rfs-nv-store.{c,h}` (MODEM-07B, не подключён к owner),
  `tools/com13-*.ps1`, `tools/file-recv-{put,get}.ps1`,
  `scripts/collect-panther-artifacts.sh`.

## Host checklist (2026-10-06)

Host-шаг без устройства. Телефон не перезагружался, COM13 и текущий
owner не открывались, образ не собирался и не прошивался.

Уже в дереве, на устройстве проверено тем же diagnostic owner
(коммиты после `0d8c228`, вершина `7084de0`):

- LTE HOME, `rmnet1` IPv4 `/32` и IPv6 `/64`, маршрут по умолчанию,
  IPv4 DNS в `/run/resolv.conf`, ICMP и TCP (`example.com`, код 0,
  тело 577 Б, счётчики `rmnet` сдвинулись).
- После data call по одному разу: fast dormancy, профили `ims`/`sos`
  (`error_raw=2`, как в стоке), modem activity, ENDC, SetVonrCapa,
  RC network type, data throttling, unsolicited filter, screen state,
  `0x070c` (ответ 2228 Б, `error 0`), `0x0108` (ответ 25 Б, `error 0`).
  Тела `0x070c` и `0x0108` не логируются.

Ещё не сделано на host и не отправляется, пока owner живой:

- IPv6 DNS: `camp_write_dns` пишет только IPv4 `nameserver`.
- Логи скаляров `lac`/`cid` на LTE HOME дают 0; тела cell-info и SMSC
  по-прежнему не разбираются и не пишутся (SMSC в лог не попадает).
- Не отправлять: `0x0100`, `0x075c`, `0x0208`, `0x0c14` — причины в
  Current state. `0x0953` GetVonrCapa не реализуется в этом шаге.
- Owner не запускается из `native-init`. Дерево `97afedf`
  (модемные коммиты по `7084de0` включительно) собрано как S28 и не
  прошито: слот A остаётся на S27.

## Evidence

**Bearer (2026-10-06, слот A, diagnostic owner, без новой загрузки).**
LTE HOME. `rmnet1`: IPv4 `/32`, IPv6 `/64`, IPv4 DNS. ICMP и TCP:
`wget http://example.com/` код 0, тело 577 Б, счётчики `rmnet1`
сдвинулись. Последние отправленные кадры этой серии — `0x070c`
(ответ 2228 Б, `error 0`) и `0x0108` (ответ 25 Б, `error 0`); тела не
логируются. `ims`/`sos`: `error_raw=2`. На кадрах LTE HOME `lac=0`,
`cid=0`. IPv6 DNS нет. Owner — одноразовый diagnostic, не
`native-init`. Пока он держит модем, устройство не перезагружать и
не прошивать. Прогоны:
[MODEM-10](MODEM-10-REAL-NV-DUAL-HANDLE.md) от `0x4600` и
carrierconfig до cell-info и SMSC.

**Сборка S27 (2026-10-06, `0d8c228`, сервер, `dist/panther/s27`).**
Host-тесты `saai-ui-core` + `saai-shell`: 319 + 82 passed.
`init_boot` `c3210f9d…` (по INPUTS изменился только `saai-shell`
`99f39b55…`; остальные бинарники бит-в-бит как S26). `vendor_boot`
`6e7be35d…` и `vendor_boot-wifi-gpu` `73da3b9a…` — идентичны S26.
`s27-data.tgz` `1e41e0b8…` теперь содержит и `saai-shell`.
Прошит `init_boot_a`, `/data/saaios/system` обновлён; загрузка: displayd,
shell (`99f39b55…`), appd, entityd, taskd, runtime, file-recv запущены.
`saai-gpu-compositor` (NDK) в бандле по-прежнему нет.
Этот образ старше модемных коммитов `da25c4e`…`7084de0`: bearer и
кадры после data call в S27 не входят. Пока слот A ONLINE, S27
повторно не прошивается.

**Сборка S28 (2026-10-06, `97afedf`, сервер, `dist/panther/s28`).**
Тот же host-путь, что у S27 (`~/worktrees/som-best-build` на
home-server), дерево `origin/pixel-7-main` на момент сборки.
`97afedf` включает модемные коммиты по `7084de0`. Телефон не
перезагружался, `fastboot` не использовался, COM13 не открывался,
текущий modem owner не заменялся. Образ не прошит.
Host-тесты `saai-ui-core` + `saai-shell`: 319 + 82 passed.
`init_boot` `c3210f9d85379ac2bef72e5636c2d397582083c70c4a611ae1dda67129c661f6`
байт-в-байт как S27. `vendor_boot`
`6e7be35de39fca996a9836f1dcd23265a30b40c710010208355d1a070184378c`
и `vendor_boot-wifi-gpu`
`73da3b9a019157923f9fffd1f98a438871146ce43bd38a6bb7f27909526ba932`
тоже как S27. `s28-data.tgz`
`b309a3567875171dafb7b7020097059acf168e9155c2e0cd2b32326f64be44df`:
по манифесту и INPUTS изменился только `saai-shell`
`5778944abc07105f8b093986d2714051f8fafeedfc4580a2f138e0afc16274e3`
(в S27 был `99f39b55…`). В shell зашит короткий id `97afedf9b1f6`.
`saaios-runtime` и остальные входы `init_boot` бит-в-бит как S27.
Модемный owner (`da25c4e`…`97afedf`) в `native-init` не входит и из
образа не стартует. `saai-gpu-compositor` (NDK) в бандле нет.

**Прошивка autostart (2026-10-07, `init_boot` `3888d4f`).**
`init_boot_a`
`934bd2633948566c25db91114a2465b99948da3adaff3913379cced0a4fc9710`.
`vendor_boot` не пересобирался и не прошивался. Слот B не трогался.
PID 1 один раз запускает `/data/saaios/bin/modem-boot.sh`.
Скрипт на `/data` — `905947d`: ждёт `pcie_exynos_gs` и
`google_modemctl`, затем выставляет `PATH=/saaios:/bin:/usr/bin`.
Загрузка с этим скриптом: CP ONLINE, `camp_setup` rmnet1
`up=1 add=1 route=1` для IPv4 и IPv6. wget `example.com` код 0,
тело 577 Б, rmnet1 rx `0→1445`, tx `432→1000`.

Заполняется при закрытии каждого воркстрима: commit, test log, image
SHA-256, bearer-доказательство (`rmnet`/IPv4/rx-tx), render-node
подтверждение, read-only EFS сверка, и известные ограничения. Контроли:
[modem-stock-reproduction.md](../targets/panther/modem-stock-reproduction.md)
и [gpu.md](../targets/panther/gpu.md) § Live stock.
