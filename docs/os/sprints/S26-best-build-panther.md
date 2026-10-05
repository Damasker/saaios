# Sprint 26 — лучшая сборка panther (модем + GPU + образ)

## Паспорт

- Состояние: `In progress` (план зафиксирован; исполнение начато с
  host-частей, device-проверки ждут возврата телефона на SaaiOS слот A).
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

**Модем.** Разворот VERDICT 27: провал регистрации — артефакт пустого
`NV_NORM` + обслуживания только handle-3, а не CP-internal RF-стена.
Сток на том же железе встаёт в LTE HOME (25503) + rmnet1 за ~1 с.
Host-фикс (обслуживание обоих handle из верифицированных копий)
реализован, ждёт device-проверки. Детали:
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
(S12 `Done`), `build-gpu-vendor-boot.sh`, `deploy/package.sh`. Хост —
Windows; сборочные скрипты `.sh` вероятно требуют Linux/WSL-тулчейна
(подтвердить до сборки).

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

## Evidence

Заполняется при закрытии каждого воркстрима: commit, test log, image
SHA-256, bearer-доказательство (`rmnet`/IPv4/rx-tx), render-node
подтверждение, read-only EFS сверка, и известные ограничения. Контроли:
[modem-stock-reproduction.md](../targets/panther/modem-stock-reproduction.md)
и [gpu.md](../targets/panther/gpu.md) § Live stock.
