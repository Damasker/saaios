# ADR-028: Физический touch на Kirigami-приложение подтверждён синтетическим evdev-инжектом

## Статус

Принято, 2026-09-10.

## Контекст

ADR-027 закрыл install→launch→stop→remove для `org.saaios.demo.kirigami`
через настоящий `saai-appd`, но явно оставил touch непроверенным:
`panther-hardware`-сборка `saai-displayd` не имеет синтетического
touch-инжектора (в отличие от headless-сборки's `inject-key`), читает
реальный evdev-девайс `/dev/input/touchscreen`, а serial-консоль не
может физически коснуться экрана. Требовался способ физически
подтвердить touch без человека у устройства.

## Находка

`services/saai-displayd/src/touch.rs` читает `/dev/input/touchscreen`
как сырые `struct input_event` (`EV_ABS`/`ABS_MT_POSITION_X`/`_Y`/
`ABS_MT_TRACKING_ID`, `EV_SYN`/`SYN_REPORT`) без libinput/libudev --
это открывает путь к синтетической инъекции через Linux's `uinput`:
создать виртуальное touch-устройство, чьи события неотличимы от
настоящих для любого userspace-кода, читающего evdev напрямую.

Ключевые физические находки по пути:

- `/dev/uinput` не существовал -- kernel поддержка есть (`223 uinput`
  в `/proc/misc`), но эта минимальная ОС не запускает ни udev, ни mdev
  для hotplug-событий (тот же факт, что `touch.rs`'s собственный
  комментарий уже объяснял про boot-time `/dev`-заполнение) -- создан
  вручную (`mknod /dev/uinput c 10 223`), безопасно и аддитивно,
  соответствует существующему kernel-драйверу.
- Созданное через `uinput` устройство НЕ получает автоматически узел
  `/dev/input/eventN` по той же причине (нет hotplug-демона) -- узел
  нужно создавать вручную, `mknod` с major:minor, прочитанным из
  `/sys/class/input/<sysname>/eventN/dev` (`UI_GET_SYSNAME` возвращает
  `inputN`, не `eventN` -- реальный evdev-узел на уровень ниже в
  sysfs).
- Узел обязательно должен быть создан под `/dev`, не `/tmp` --
  `mount | grep /tmp` показал `nodev` среди опций монтирования; попытка
  открыть device-файл с тем же major:minor, но лежащий на
  `nodev`-примонтированной файловой системе, дала `EACCES` (`Permission
  denied`), не `ENXIO`, что изначально сбило с толку диагностику.
- Дальше -- тот же безопасный `unshare -m`/`mount --bind`-паттерн, что
  уже трижды использован в этом спринте (ADR-025/026/027): виртуальный
  device node bind-mount'ится поверх `/dev/input/touchscreen` только
  внутри изолированного mount namespace одного throwaway-процесса,
  реальный `/dev/input/event2` (настоящий тачскрин) и реальный боевой
  `saai-displayd` (pid, хэш) остаются полностью нетронутыми на всём
  протяжении эксперимента -- подтверждено до и после.
- Throwaway `panther-hardware`-сборка `saai-displayd` (чистый, без
  диагностических патчей коммит, хэш совпадает с боевым) внутри этого
  namespace: `hardware::init()`'s ошибка получения DRM (реальный
  compositor уже держит master) обрабатывается штатно
  (`Err(err) => None`, физически подтверждено логом) -- процесс не
  падает, продолжает поднимать Wayland-сервер и читать touch
  независимо от DRM. Никакого визуального воздействия на реальный
  экран.
- Инъекция двух синтетических тапов (написан throwaway `uinput-touch.c`,
  собран `zig cc` для aarch64-musl) дала полный, самосогласованный
  протокольный след в логе throwaway `saai-displayd`:
  ```
  touch down at (500, 1200), routed to: Some(wl_surface@21[0], 21) (locked=true)
  touch up
  saai-shell: unlocked by touch
  saai-displayd: session unlocked
  touch down at (540, 1200), routed to: Some(wl_surface@15[1], 24) (locked=false)
  touch up
  ```
  Первый тап корректно ушёл на lock-surface (ADR-015's инвариант:
  "пока locked, touch никогда не проваливается до приложения под
  экраном" -- подтверждён, не только прочитан в коде) и физически
  разблокировал реальный `saai-shell`. Второй тап, после разблокировки,
  ушёл именно на `wl_surface@15[1], 24` -- тот же ID, что был
  зарегистрирован как toplevel `org.saaios.demo.kirigami`'s
  `qmlscene-qt5`-процесса чуть раньше в том же логе
  (`focus set to Some(wl_surface@15[1], 24)`). Полное совпадение ID --
  прямое, недвусмысленное доказательство: синтетическое touch-событие
  прошло весь путь от evdev до конкретно Kirigami-приложения.

## Решение

ADR-027's открытый пункт "touch физически не проверен тапом" закрыт.
Метод (uinput + ручной `mknod` под `/dev` + `unshare -m`/`mount --bind`
поверх `/dev/input/touchscreen` + throwaway `panther-hardware`
`saai-displayd`, чей DRM-неудача не мешает touch/Wayland) задокументирован
здесь как воспроизводимый способ физически проверять touch-путь без
присутствия человека у устройства -- полезно для будущих спринтов,
не только S08.

## Последствия

- Change 7's акцептанс-критерий "получают touch" для
  `org.saaios.demo.kirigami` теперь закрыт физическим доказательством,
  не только протокольным наследованием от S02/S03.
- Ни один файл/узел устройства не остался -- `/dev/uinput`,
  `/dev/virtual-touch`, весь throwaway runtime state удалены,
  реальный `saai-displayd` (pid 612, хэш `541a5eeb...`) и `saai-appd`
  (список приложений) подтверждены неизменными до и после.
- Throwaway-инструменты (`uinput-touch.c`, композитный
  `unshare`-скрипт) не закоммичены -- метод описан текстом здесь,
  воспроизводим по шагам, не требует постоянного присутствия в
  репозитории (аналогично throwaway-спайкам ADR-021/025).

## Ссылки

- ADR-015 -- lock-surface touch-инвариант, подтверждённый физически
  здесь впервые (раньше только прочитан в коде).
- ADR-025, ADR-026, ADR-027 -- тот же `unshare -m`/`mount --bind`
  паттерн для безопасного throwaway-воспроизведения без риска для
  боевого стека.
- `services/saai-displayd/src/touch.rs` -- протокол, который сделал
  эту инъекцию возможной без изменения кода проекта.
