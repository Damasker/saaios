# Дорожная карта спринтов SaaiOS

Roadmap описывает порядок доказуемых вертикальных результатов. Он не является
обещанием дат: каждый спринт закрывается по критериям, а не по количеству
написанного кода. Процесс и Definition of Done находятся в
[DEVELOPMENT_PROCESS.md](../DEVELOPMENT_PROCESS.md).

## Текущее состояние

| ID | Результат | Состояние | Зависит от |
|---|---|---|---|
| S00 | Архитектура и процесс | Done | рабочий DRM UI |
| S01 | Wayland vertical slice на host | Ready | S00 |
| S02 | `saai-displayd` на Pixel 7 | Backlog | S01 |
| S03 | Отдельный `saai-shell` и lock | Backlog | S02 |
| S04 | Приложения, manifest и lifecycle | Backlog | S03 |
| S05 | Настоящие пространства и entity store | Backlog | S04 |
| S06 | Capability, sandbox и portals | Backlog | S04, S05 |
| S07 | GTK и Qt/Kirigami совместимость | Backlog | S06 |
| S08 | Intent → Task → Action workflow | Backlog | S05, S06 |
| S09 | Planner, automation и memory | Backlog | S08 |
| S10 | GPU, power, update и release gate | Backlog | S03–S09 |

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

## S01 — Wayland vertical slice на host

Рабочий паспорт и декомпозиция: [S01-wayland-host-vertical-slice.md](S01-wayland-host-vertical-slice.md).

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

## S02 — `saai-displayd` на Pixel 7

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

## S03 — Отдельный `saai-shell` и lock screen

**Goal:** текущая мобильная оболочка становится независимым системным клиентом.

**Scope:** lock/status/navigation/system overlay, системный слой, список
окон, перезапуск shell, touch wake и idle screen-off. Бизнес-данные остаются
адаптером к текущему состоянию.

**Acceptance:** визуальная и touch-регрессия четырёх разделов пройдена; lock
не пропускает ввод приложению; намеренное падение shell не валит compositor и
приводит к ограниченному восстановлению.

**Rollback:** `drm-splash` включается как boot/recovery UI.

## S04 — Manifest и жизненный цикл приложений

**Goal:** SaaiOS устанавливает и управляет первым отдельным приложением.

**Scope:** versioned manifest, `app_id`, каталоги `/data/saaios/apps` и
`/data/saaios/var/apps`, launch/stop/switch, single-instance, crash limit,
события состояния. Пока только доверенные приложения.

**Acceptance:** demo-app устанавливается без изменения `init_boot`, запускается
из оболочки, переключается и удаляется без чужих данных; неизвестная схема и
duplicate id отвергаются; crash loop ограничен.

**Rollback:** отключить `saai-appd` и удалить только каталог demo-app, сохранив
shell и recovery.

## S05 — Пространства и entity store

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

## S06 — Capability, sandbox и portals

**Goal:** приложение получает только явно разрешённые действия и данные.

**Scope:** capability vocabulary, effective grants, process isolation,
filesystem/network mediation, системный permission surface, portal для
выбора объекта и clipboard. Реализация выбирается отдельным threat-model ADR.

**Acceptance:** негативные тесты доказывают запрет чтения другого app/space,
произвольной сети и подделки системного подтверждения; deny является default;
решения аудируются без пользовательского содержимого.

**Rollback:** сторонние приложения отключаются целиком; системные приложения
продолжают работать с минимальным статическим набором capability.

## S07 — GTK и Qt/Kirigami совместимость

**Goal:** по одному настоящему адаптивному приложению обоих toolkit работает
как обычный клиент SaaiOS.

**Scope:** необходимые Wayland-протоколы, fonts/themes/settings portal,
экранная клавиатура, popups, clipboard через policy, упаковка runtime в
`/data`. Полные KDE/GNOME sessions и XWayland не входят.

**Acceptance:** GTK-приложение и Qt/Kirigami-приложение запускаются,
масштабируются, получают touch/text input, переживают switch и закрываются;
системные разрешения нельзя обойти toolkit API; размер и память измерены.

**Rollback:** удалить соответствующий runtime bundle без изменения shell.

## S08 — Intent → Task → Action

**Goal:** строка намерения создаёт наблюдаемый рабочий процесс, а не только
чатовый запрос.

**Scope:** versioned модели `Intent`, `Task`, `Action`, `Result`; состояния и
идемпотентность; подтверждение опасного Action; отображение в `Сейчас` и
`Входящие`.

**Acceptance:** сценарий создаётся, приостанавливается, подтверждается,
возобновляется после reboot и оставляет связный audit trail; повтор события не
повторяет необратимое действие.

**Rollback:** workflow становится read-only; ручные модули продолжают работать.

## S09 — Planner, automation и memory

**Goal:** локальный ИИ предлагает планы и автоматизацию внутри видимых границ.

**Scope:** planner только создаёт предложения; policy исполняет; расписания,
триггеры, лимиты, отмена; memory привязана к пространству и происхождению.

**Acceptance:** модель не может обойти capability; каждый Action объясним и
отменяем где возможно; budget/loop limits проверены; отключение модели не
ломает ручное управление и уже сохранённые задачи.

**Rollback:** остановить planner/automation workers, сохранив объекты и аудит.

## S10 — Производительность, питание и release gate

**Goal:** новый стек становится стабильным основным UI для ежедневного
использования на тестовом Pixel 7.

**Scope:** измерения latency/memory/idle, GPU только при доказанной пользе,
deep idle, signed app/update metadata, A/B userspace update, recovery drill и
полная регрессия.

**Acceptance:** согласованные бюджеты производительности выполнены; нет
неограниченных restart loops; обновление и намеренно прерванное обновление
восстанавливаются; пройдены cold-boot, 24-hour soak и rollback drill;
стабильный архив связан с commit и хешами.

**Rollback:** последний физически принятый образ SaaiOS в слоте A и сохранённый
Android в слоте B.

## Управление roadmap

При закрытии спринта его состояние и Evidence обновляются одним commit с
результатом. Следующий спринт получает `Ready` только после Definition of Ready.
Новый крупный запрос сначала помещается в подходящий спринт; если он меняет
архитектурные границы, перед кодом создаётся ADR.

Шаблон новой записи: [SPRINT-TEMPLATE.md](SPRINT-TEMPLATE.md).
