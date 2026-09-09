# Sprint 03 — `saai-displayd` на Pixel 7

## Паспорт

- Состояние: `In progress` (repo-side candidate готов; physical pending).
- Зависит от: S02.
- Архитектурные решения: ADR-004, ADR-005, ADR-007.
- Рабочий fallback: физически принятый S01 image и существующий `drm-splash`;
  Android остаётся в slot B.

## Goal

Тот же отдельный Wayland client из S02 выводит тестовую поверхность через
`saai-displayd` на DSI-панель Pixel 7 и получает реальное касание, а падение
нового стека автоматически возвращает проверенный DRM UI и не затрагивает USB.

## Current state

Repo-side S03 candidate собран: Panther DRM/evdev backend, C supervisor,
PID 1 enable/fallback contract, host/watchdog gates и AArch64 image inspection
зелёные. Образ `saaios-panther-s03-init_boot.img` готов, но **не прошит**.
Physical acceptance ждёт явного разрешения на flash только `init_boot_a`.

## Scope

Входит:

- legacy DRM/KMS dumb-buffer backend только для подключённого DSI
  `1080x2400x60`;
- проверенный Pixel 7 BGRX output transform;
- `/dev/input/touchscreen`, нормализация координат и pointer только активному
  fullscreen client;
- readiness и heartbeat нового compositor;
- ограниченный supervisor: старт, timeout, crash/hang и возврат к
  `drm-splash`;
- host tests, AArch64 static cross-build, image inspection и физическая
  приёмка после отдельного разрешения на flash.

Не входит:

- `saai-shell`, перенос текущего пользовательского UI или lock screen;
- GPU/EGL/Vulkan, `linux-dmabuf`, page-flip optimization;
- keyboard, popup, toolkit compatibility и app lifecycle;
- Scheduler, automation, sandbox или последующие спринты.

## Change

1. Отделить output transform и touch state от аппаратного backend.
2. Добавить узкий DRM/KMS backend в существующий `saai-displayd`.
3. Перевести S02 demo-client в интерактивный тестовый режим.
4. Добавить supervisor и PID 1 restart contract с `drm-splash` как fallback.
5. Включить static binaries в `init_boot` без изменения vendor boot/userdata.
6. Пройти host gates, cross-build, review и image inspection.
7. После явного разрешения проверить экран, touch, forced crash и cold boot.

## Test

- unit: BGRX transform, nearest-neighbour scale, touch bounds/state;
- protocol: S02 configure/ack/role/malformed regressions остаются зелёными;
- supervisor: no-ready, early exit, stale heartbeat и clean exit возвращают
  fallback без бесконечного restart loop;
- host integration: mock output/input доказывают focus и второй frame после
  touch;
- cross-build: static stripped AArch64 musl `saai-displayd` и demo-client;
- image inspection: оба binary, supervisor, fallback и PID 1 имеют ожидаемые
  хеши.

## Acceptance criteria

- DSI работает строго в `1080x2400x60`, source frame проходит BGRX transform;
- S02 color table различима и совпадает с уже принятой физической таблицей;
- реальное касание получает только активный Wayland client и изменяет frame;
- no-ready, crash или stale heartbeat возвращают `drm-splash`;
- USB console/network доступны во время fallback;
- cold boot воспроизводит takeover без ручного восстановления;
- slot B, userdata и vendor boot не изменены;
- format, clippy, workspace tests, supervisor tests, CI и cross-build зелёные.

## Threat / privacy impact

Compositor читает только DRM, evdev и bounded protocol metadata. Координаты и
содержимое `wl_shm` не пишутся в persistent logs. Socket и readiness находятся
в `/run`; image не содержит userdata, credentials или device calibration.

## Rollback

До flash сохранить установленный S01 `init_boot_a` и его SHA-256. При
неуспешном новом backend supervisor запускает встроенный `drm-splash`. Из
bootloader вернуть сохранённый `init_boot_a`; slot B не изменять.

## Evidence

### Repo-side candidate, 2026-09-06

- `saai-displayd --backend panther` требует подключённый DSI
  `1080x2400x60`, создаёт `XRGB8888` dumb buffer через DRM `ADDFB2` и
  масштабирует `64x48 ARGB8888` nearest-neighbour с уже принятой перестановкой
  байтов Pixel 7 BGRX.
- FocalTech bounds читаются из evdev. `ABS_MT_POSITION_X/Y` нормализуются в
  координаты fullscreen surface; enter/motion/button получает только pointer
  того же client, которому принадлежит активный surface.
- Интерактивный demo-client после press перерисовывает заметный белый marker в
  своём `wl_shm` и повторно commit-ит buffer. Headless S02 lifecycle и
  malformed-client isolation сохранены.
- PID 1 запускает C supervisor, который держит `drm-splash` до готовности USB,
  затем требует readiness compositor и следит за heartbeat раз в 250 мс.
  No-ready, exit compositor/client или heartbeat старше 2 с уничтожают trial и
  возвращают `drm-splash`; PID 1 отдельно восстанавливает fallback при падении
  самого supervisor.
- Unit/integration gates проверяют frame hash, BGRX scale, touch bounds,
  focused routing, повторный frame, C watchdog state, no-userdata PID 1 и все
  S02 protocol-negative сценарии.
- Static stripped AArch64 musl artifacts:
  - `saai-displayd`: 559528 bytes,
    SHA-256 `9af2694aab75daac8754e5bcd8708ac3ae36f1ab088af5f2145ce55da6f07ca5`;
  - `saai-demo-surface`: 486328 bytes,
    SHA-256 `39b6d22865a34760bf5dd7e2f33d885431a54ab76502ce47fa141af91aec4cfc`;
  - C `saai-display-supervisor`: 20400 bytes,
    SHA-256 `4ebb8c59d494155e4555507913cc06c28344763496fb41c28360e141a20f170c`.
- Собран, но не установлен
  `dist/panther/saaios-panther-s03-init_boot.img`: 8388608 bytes,
  SHA-256 `9cb39c849f36ef2c317ac55a2595dd5ba7142fc6610ba0112bf4ed60d1d0c880`.
  Повторный unpack подтвердил header v4, Android 17.0.0 / patch 2026-07,
  LZ4 ramdisk 8303399 bytes и совпадение хешей embedded PID 1, fallback,
  compositor, client и supervisor. Image ровно помещается в штатный 8 MiB
  `init_boot`; vendor boot, userdata и slot B не изменялись.
- Host re-verify 2026-09-09: `cargo fmt`/`clippy -D warnings`/`test -p saai-displayd`,
  C identity + watchdog + supervisor compile, pixel7 AArch64 musl rebuild
  (`DISPLAYD_MATCH`/`CLIENT_MATCH`) и magiskboot unpack хешей embedded binaries.
- Commit `410c7f9`; GitHub Actions
  [`34319708084`](https://github.com/Damasker/saaios/actions/runs/34319708084)
  зелёный (fmt, clippy, Wayland slice, workspace tests, watchdog, e2e, cross-build).
  Docs follow-up `dabe21d` /
  [`34320077611`](https://github.com/Damasker/saaios/actions/runs/34320077611) тоже зелёный.
- Local S01 rollback image hash совпадает с passport
  (`18987941…29771`); S03 image hash совпадает (`9cb39c84…c880`).
- Gated physical helper: `os/targets/panther/tools/s03-physical-verify.ps1`
  (`-EnterBootloader` / `-AllowFlash` / `-RollbackToS01` только при
  `SAAIOS_ALLOW_FLASH=yes`; `-VerifyLive` проверяет S03 markers по COM13).
  На текущем live image `-VerifyLive` корректно падает: нет supervisor/demo.
- Live device baseline 2026-09-09 (до S03 flash): USB NCM `172.31.7.1`
  отвечает, runtime TCP `38127` открыт, COM13 shell доступен. На slot A сейчас
  **не** S03 candidate, а более ранний experimental image:
  `/saaios/saai-displayd` + `/saaios/saai-shell` (lock/spaces), без
  `saai-display-supervisor` / `saai-demo-surface`. Log показывает blit
  `1080x2400` и touch routing. Flash S03 заменит этот UI на demo-surface +
  supervisor/`drm-splash` fallback — только после явного разрешения.
- Pull request: https://github.com/Damasker/saaios/pull/38
  (base `feat/s02-wayland-host`; physical flash still pending explicit permission).
- Physical run S03 candidate добавляется после отдельного явного разрешения
  на flash.

### Physical verification checklist

1. До записи подтвердить `product=panther`, `unlocked=yes`,
   `current-slot=a`; сохранить S01 image
   `18987941ee7c41a98ea9a9471287933d75a27bf9417050253800d2e937829771`.
2. После отдельного явного разрешения записать только S03 image в
   `init_boot_a`; не выполнять `--set-active`, не писать `vendor_boot`,
   userdata или slot B.
3. Проверить последовательность: текущий `drm-splash`, доступность USB, затем
   fullscreen четырёхцветный Wayland frame.
4. Сверить red/green/blue/yellow с физически принятой BGRX таблицей и режим
   `1080x2400x60` в supervisor/compositor log.
5. Коснуться четырёх областей. На экране должен появляться белый marker в месте
   касания, а log должен содержать один `CLIENT_TOUCH` на press.
6. Во время Wayland frame проверить ping `172.31.7.1`, runtime `38127` и вход в
   USB console.
7. Принудительно завершить только `saai-displayd`: не позднее watchdog timeout
   должен вернуться `drm-splash`, reason — `CompositorExited`, USB остаётся
   доступным.
8. После cold reboot повторить takeover, одно касание и USB check без ручной
   настройки.

### Physical rollback

- Основной: из bootloader записать сохранённый S01 image только в
  `init_boot_a`, перезагрузить slot A и проверить старый `drm-splash`/USB.
- Аварийный: загрузить сохранённый Android slot B, не изменяя его разделы.
