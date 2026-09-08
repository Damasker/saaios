# ADR-018: Manifest, установка и жизненный цикл приложений

## Статус

Принято, 2026-09-08.

## Контекст

S04 отделил системную оболочку от компоновщика. Следующий слой должен
устанавливать и запускать обычные Wayland-приложения без перепаковки
`init_boot`, не превращая SaaiOS в копию Android package manager или Linux
desktop environment. До S07 приложения считаются только доверенными: наличие
manifest ещё не создаёт sandbox и не выдаёт capability.

## Решение

`saai-appd` становится единственным владельцем установки и процессов
приложений. `saai-shell` обращается к нему через локальный Unix-сокет
`/run/saaios/appd.sock`; shell не запускает app-процессы напрямую.

### Manifest v1

Корень пакета содержит UTF-8 TOML-файл `manifest.toml`:

```toml
schema = 1
id = "org.saaios.example.notes"
name = "Заметки"
exec = "bin/notes"
version = "0.1.0"
ui = "wayland"
single_instance = true
capabilities = []
```

- неизвестная schema и неизвестные поля отвергаются;
- `id` состоит из lowercase DNS-подобных сегментов и никогда не используется
  до валидации как часть пути;
- `exec` — относительный нормализованный путь внутри `bin/`, без `..`, root и
  platform-prefix компонентов;
- capability имеют lowercase dotted-имена, дубликаты отвергаются;
- manifest объявляет запросы, но effective grants до S07 всегда пусты.

### Размещение и атомарность

- код: `/data/saaios/apps/<app_id>`;
- изменяемые данные: `/data/saaios/var/apps/<app_id>`;
- установка сначала копирует пакет в sibling staging-каталог, повторно
  проверяет manifest и executable, затем делает atomic rename;
- существующий `app_id` считается duplicate и не перезаписывается;
- обычное удаление останавливает процесс и удаляет только код этого app;
  собственные данные сохраняются, `purge` удаляет их отдельной явной командой.

Каталоги установленных приложений являются источником истины. Отдельная
изменяемая база package registry в S05 не вводится: на старте `saai-appd`
повторно сканирует и валидирует manifests.

### Процессы

Для app задаются `SAAIOS_APP_ID`, `SAAIOS_DATA_DIR`, `XDG_RUNTIME_DIR` и
`WAYLAND_DISPLAY`; рабочий каталог — корень установленного app. Для
`single_instance=true` повторный launch возвращает уже существующий instance.
Каждый выход учитывается в окне 60 секунд; после трёх неуспешных выходов app
остаётся stopped до явного launch. Падение app не завершает `saai-appd`,
`saai-shell` или `saai-displayd`.

IPC S05 — versioned JSON-lines request/response с ограничением размера одного
сообщения. Он локальный и не является публичным сетевым API. Команды:
`list`, `install`, `launch`, `stop`, `remove`; события состояния:
`installed`, `running`, `stopped`, `crash_limited`, `removed`.

## Последствия

Положительные:

- приложение появляется и удаляется без изменения boot-образа;
- код, собственные данные и жизненный цикл имеют разные явные границы;
- путь к будущему sandbox не требует менять manifest или API оболочки;
- повреждённый/чужой manifest отвергается до любых операций с путями.

Ограничения S05:

- package signing, sandbox, uid/cgroup isolation и реальные grants входят в
  S07, поэтому S05 допускает только доверенные локальные пакеты;
- update/rollback нескольких версий не входит в S05;
- JSON-lines выбран для наблюдаемого bring-up, а не как вечный wire format.

## Отклонённые альтернативы

### Запускать приложения прямо из `saai-shell`

Связывает UI с процессами, crash policy и файловой системой. Падение/перезапуск
shell потерял бы владение app-процессами.

### Использовать desktop `.desktop` как manifest

Формат описывает запуск и интеграцию desktop, но не versioned schema,
single-instance, capability requests и каталоги данных SaaiOS.

### Встроить пакеты в `init_boot`

Нарушает цель устанавливаемых приложений и снова связывает app release с
прошивкой системного раздела фиксированного размера.

## Ссылки

- `docs/adr/ADR-005-owned-wayland-application-platform.md`
- `docs/os/architecture/application-platform.md`
- `docs/os/sprints/S05-application-lifecycle.md`
