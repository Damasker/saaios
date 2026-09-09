# Дорожная карта спринтов SaaiOS

Roadmap описывает порядок доказуемых вертикальных результатов. Он не является
обещанием дат: каждый спринт закрывается по критериям, а не по количеству
написанного кода. Процесс и Definition of Done находятся в
[DEVELOPMENT_PROCESS.md](../DEVELOPMENT_PROCESS.md).

## Текущее состояние

| ID | Результат | Состояние | Зависит от |
|---|---|---|---|
| S00 | Архитектура и процесс | Done | рабочий DRM UI |
| S01 | Системная идентичность | Done | S00 |
| S02 | Wayland vertical slice на host | Done | S01 |
| S03 | `saai-displayd` на Pixel 7 | Done | S02 |
| S04 | Отдельный `saai-shell` и lock | Done | S03 |
| S05 | Приложения, manifest и lifecycle | Done | S04 |
| S06 | Настоящие пространства и entity store | Done | S05 |
| S07 | Capability, sandbox и portals | In progress | S05, S06 |
| S08 | GTK и Qt/Kirigami совместимость | Backlog | S07 |
| S09 | Intent → Task → Action workflow | Backlog | S06, S07 |
| S10 | Planner, automation и memory | Backlog | S09 |
| S11 | GPU, power, OTA и release gate | Backlog | S04–S10 |

## S00 — Архитектура и процесс

**Goal:** зафиксировать собственную Wayland-платформу, продуктовые границы,
процесс, качество и последовательность разработки.

**Acceptance:** приняты ADR-005 и продуктовые принципы; описаны компоненты,
инварианты безопасности, Definition of Ready/Done, тестовые gates, roadmap и
rollback policy; документы связаны из основного индекса.

**Rollback:** удалить документационный commit; рабочий phone image не меняется.

**Evidence:** документация и индексы добавлены commit
`407e5feb90979b1a6c6c02c1b0643d8c75950f29`; GitHub Actions run
[`33984921594`](https://github.com/Damasker/saaios/actions/runs/33984921594)
успешно выполнил format, clippy, workspace tests и e2e. Изменений phone image,
разделов и userdata не было.

## S01 — Системная идентичность

Рабочий паспорт и декомпозиция: [S01-system-identity.md](S01-system-identity.md).

**Goal:** система получает удостоверенный локальный DeviceContext и говорит о
Pixel 7 как работающая SaaiOS, а не как внешний чат-помощник.

**Scope:** `system.identity`, контекст planner, runtime status, target от PID 1,
переименование системного экрана и физический тест двух запросов. Полный
device-state service и новые изменяющие actions не входят.

**Acceptance:** runtime и tool согласованно возвращают
`SaaiOS / native_device / phone / panther`; UI использует модель намерения и
системного результата; модель не отрицает установленную SaaiOS; контекст не
содержит идентификаторов пользователя; image проходит rollback gate.

**Rollback:** combined Evidence image source `be9630d`, SHA-256
`c1ebb5afd0ee276ca7c2774b765084a19e001ad1afc4d3385c57dc7b66aedff8`;
Android slot B.

## S02 — Wayland vertical slice на host

Рабочий паспорт и декомпозиция: [S02-wayland-host-vertical-slice.md](S02-wayland-host-vertical-slice.md).

**Goal:** два независимых процесса показывают первую настоящую поверхность
через прототип `saai-displayd` без телефона.

**Scope:** выбор framework на основе измеримого spike; `wl_shm`; один
`xdg_toplevel`; configure/ack/commit; touch-like pointer injection; закрытие
клиента; headless test. DRM, GPU и toolkit-совместимость не входят.

**Acceptance:** тестовый клиент рисует различимый кадр, получает ввод и может
быть закрыт; повреждённый клиент отключается без падения compositor; принято
ADR о framework; CI воспроизводит сценарий.

**Rollback:** workspace остаётся на текущем C DRM UI; новый сервис не входит в
phone image.

## S03 — `saai-displayd` на Pixel 7

Рабочий паспорт и декомпозиция: [S03-panther-displayd.md](S03-panther-displayd.md).

**Goal:** тот же тестовый Wayland-клиент выводится на реальный экран и получает
касания.

**Scope:** DRM/KMS backend, `1080x2400x60`, BGRX output transform, evdev touch,
focus одного fullscreen-клиента, readiness и аппаратный watchdog. Shell ещё
остаётся старым.

**Acceptance:** цветовая таблица совпадает с проверенной; касание доставляется
только активному клиенту; выход compositor автоматически возвращает текущий
DRM fallback и USB-консоль; холодная загрузка воспроизводима.

**Rollback:** физически проверенный образ, предшествующий S02; слот B не
изменяется.

## S04 — Отдельный `saai-shell` и lock screen

Рабочий паспорт и декомпозиция: [S04-saai-shell.md](S04-saai-shell.md).

**Goal:** текущая мобильная оболочка становится независимым системным клиентом.

**Scope:** lock/status/navigation/system overlay, системный слой, список
окон, перезапуск shell, touch wake и idle screen-off. Бизнес-данные остаются
адаптером к текущему состоянию.

**Acceptance:** визуальная и touch-регрессия четырёх разделов пройдена; lock
не пропускает ввод приложению; намеренное падение shell не валит compositor и
приводит к ограниченному восстановлению.

**Rollback:** `drm-splash` включается как boot/recovery UI.

## S05 — Manifest и жизненный цикл приложений

Рабочий паспорт и декомпозиция:
[S05-application-lifecycle.md](S05-application-lifecycle.md).

**Goal:** SaaiOS устанавливает и управляет первым отдельным приложением.

**Scope:** versioned manifest, `app_id`, каталоги `/data/saaios/apps` и
`/data/saaios/var/apps`, launch/stop/switch, single-instance, crash limit,
события состояния. Пока только доверенные приложения.

**Acceptance:** demo-app устанавливается без изменения `init_boot`, запускается
из оболочки, переключается и удаляется без чужих данных; неизвестная схема и
duplicate id отвергаются; crash loop ограничен.

**Rollback:** отключить `saai-appd` и удалить только каталог demo-app, сохранив
shell и recovery.

**Evidence:** host vertical проверил install→launch→stop→remove и сохранность
app data. На Pixel 7 подтверждены установка после boot, fullscreen Wayland,
touch, штатное закрытие, холодный перескан manifest, сохранность данных,
рестарт appd под PID 1 и ограничение третьего app crash за 60 секунд. S05 image
source `6afb663`, SHA-256
`be22d1af98b73c24fa9272f2575ac6bbc4aa858516de2478e18f417da07db215`;
прошит только `init_boot_a`, slot B не менялся.

## S06 — Пространства и entity store

Рабочий паспорт и декомпозиция:
[S06-space-entity-store.md](S06-space-entity-store.md).

**Goal:** `Дом`, `Работа`, `Личное` и `SaaiOS` становятся настоящими областями
данных, а не только сохранённым UI-переключателем.

**Scope:** versioned модели `Space`, `Entity`, `Event`; атомарное локальное
хранилище; проекции для `Сейчас`; миграции; импорт существующего выбранного
контекста.

**Acceptance:** объект одного пространства не появляется в другом без явного
действия; события append-only; незавершённая запись и холодный reboot не
повреждают store; миграция имеет обратимый backup.

**Rollback:** read-only возврат к предыдущей schema и восстановление backup;
никакой молчаливой downgrade-записи.

**Evidence:** host vertical (store/protocol/daemon/shell) прошёл 28+
unit/process tests с all-target clippy на R620. На Pixel 7 физически
подтверждены: `saai-entityd` ARM64 packaging и persistent supervision из
`native-init.c` (`024ccfe`) тем же паттерном, что и `saai-appd`; `kill -9`
работающего `saai-entityd` дал автоматический restart (`/run/boot.log`) и
независимое переподключение `saai-shell`; выбор пространства через
реальный touch-путь (`Пространства` → `Личное`) пережил полный холодный
`fastboot`-цикл (`/proc/uptime` = 42s), подтверждено на трёх уровнях --
файл `selection.json`, прямой запрос к сокету `entityd` в обход shell,
и визуально на экране; изоляция между пространствами подтверждена
`create_entity`/`list_entities` через тот же прямой протокольный запрос.
Полная пересборка `init_boot` для этой версии сделана не в этой сессии
(бинарник уже был на устройстве от параллельной работы) -- проверка шла
против уже прошитого состояния, содержимое бинаря сверено по хешу с
собранным из того же commit.

## S07 — Capability, sandbox и portals

Рабочий паспорт и декомпозиция:
[S07-capability-sandbox.md](S07-capability-sandbox.md).

**Goal:** приложение получает только явно разрешённые действия и данные.

**Scope:** capability vocabulary, effective grants, process isolation,
filesystem/network mediation, системный permission surface, portal для
выбора объекта и clipboard. Threat-model ADR-020 выбрал mount/network/ipc/uts
namespaces + seccomp-bpf после on-device spike, подтвердившего, что
`CLONE_NEWPID`/`CLONE_NEWUSER` физически недоступны на ядре Pixel 7.

**Acceptance:** негативные тесты доказывают запрет чтения другого app/space,
произвольной сети и подделки системного подтверждения; deny является default;
решения аудируются без пользовательского содержимого.

**Rollback:** сторонние приложения отключаются целиком; системные приложения
продолжают работать с минимальным статическим набором capability.

## S08 — GTK и Qt/Kirigami совместимость

**Goal:** по одному настоящему адаптивному приложению обоих toolkit работает
как обычный клиент SaaiOS.

**Scope:** необходимые Wayland-протоколы, fonts/themes/settings portal,
экранная клавиатура, popups, clipboard через policy, упаковка runtime в
`/data`. Полные KDE/GNOME sessions и XWayland не входят.

**Acceptance:** GTK-приложение и Qt/Kirigami-приложение запускаются,
масштабируются, получают touch/text input, переживают switch и закрываются;
системные разрешения нельзя обойти toolkit API; размер и память измерены.

**Rollback:** удалить соответствующий runtime bundle без изменения shell.

## S09 — Intent → Task → Action

**Goal:** строка намерения создаёт наблюдаемый рабочий процесс, а не только
чатовый запрос.

**Scope:** versioned модели `Intent`, `Task`, `Action`, `Result`; состояния и
идемпотентность; подтверждение опасного Action; отображение в `Сейчас` и
`Входящие`.

**Acceptance:** сценарий создаётся, приостанавливается, подтверждается,
возобновляется после reboot и оставляет связный audit trail; повтор события не
повторяет необратимое действие.

**Rollback:** workflow становится read-only; ручные модули продолжают работать.

## S10 — Planner, automation и memory

**Goal:** локальный ИИ предлагает планы и автоматизацию внутри видимых границ.

**Scope:** planner только создаёт предложения; policy исполняет; расписания,
триггеры, лимиты, отмена; memory привязана к пространству и происхождению.

**Acceptance:** модель не может обойти capability; каждый Action объясним и
отменяем где возможно; budget/loop limits проверены; отключение модели не
ломает ручное управление и уже сохранённые задачи.

**Rollback:** остановить planner/automation workers, сохранив объекты и аудит.

## S11 — Производительность, питание, OTA и release gate

**Goal:** новый стек становится стабильным основным UI для ежедневного
использования на тестовом Pixel 7.

**Scope:** измерения latency/memory/idle, GPU только при доказанной пользе,
deep idle, полная регрессия и OTA-контур:

- подписанный versioned manifest и подписанный полный системный artifact;
- возобновляемая staged-загрузка с проверкой размера, SHA-256 и подписи до
  записи разделов;
- проверки питания, свободного места, сети, модели устройства и допустимого
  перехода версии, включая запрет downgrade по умолчанию;
- запись только в неактивный A/B OS slot; активный slot и разделы firmware,
  calibration, identity и hardware metadata обновлятору недоступны;
- ограниченное число boot attempts, health confirmation после запуска,
  автоматический откат на последний healthy slot и внешний recovery drill;
- переход от сохранённого Android в slot B к двум SaaiOS slots разрешается
  только отдельным подтверждённым этапом после готовности внешнего factory
  recovery.

**Acceptance:** согласованные бюджеты производительности выполнены; нет
неограниченных restart loops; штатное, намеренно прерванное, повреждённое и
адресованное не тому устройству обновления проверены. Ни один тест не пишет в
активный slot; неподписанный artifact и запрещённый downgrade отвергаются;
неподтверждённая новая система автоматически откатывается. Пройдены cold-boot,
24-hour soak, power-loss и rollback drills; стабильный архив связан с commit,
manifest, подписью и хешами.

**Rollback:** до подтверждённой OTA-миграции — последний физически принятый
SaaiOS в слоте A и Android в слоте B. После миграции — предыдущий healthy
SaaiOS в неактивном слоте и заранее проверенный внешний factory recovery.

## Управление roadmap

При закрытии спринта его состояние и Evidence обновляются одним commit с
результатом. Следующий спринт получает `Ready` только после Definition of Ready.
Новый крупный запрос сначала помещается в подходящий спринт; если он меняет
архитектурные границы, перед кодом создаётся ADR.

Шаблон новой записи: [SPRINT-TEMPLATE.md](SPRINT-TEMPLATE.md).
