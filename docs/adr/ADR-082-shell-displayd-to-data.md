# ADR-082: `saai-shell`/`saai-displayd` переезжают на `/data`

## Статус

Принято, 2026-09-15.

## Контекст

`saai-appd`, `saai-entityd` и `saaios-runtime` (ADR-074's попутная
находка) уже жили на персистентном `/data`, независимо от
фиксированного 8МБ `init_boot`-образа -- `docs/os/ideas.md` с самого
начала этой серии фиксов называла оставшиеся два (`saai-shell`,
`saai-displayd`) главной причиной, по которой почти каждая правка
сегодняшней сессии (S28-S32, шрифты в песочнице) заканчивалась
полным циклом `fastboot flash init_boot_a` -- единственный канал
доставки для этих двух компонентов был "бери и перешивай".

## Решение

Тот же паттерн, что уже доказан для `APPD_PATH`/`ENTITYD_PATH`/
`RUNTIME_PATH`: новый `DISPLAYD_PATH "/data/saaios/system/saai-
displayd"` в `native-init.c`, `saai-displayd`'s собственный
`SAAI_SHELL_PATH` меняется на `/data/saaios/system/saai-shell`.
`saai-shell` сам по себе не запускается из `native-init.c` вообще --
он дочерний процесс `saai-displayd` (ADR-014), так что менять
пришлось только тот один хардкод в `services/saai-displayd/src/
main.rs`, ничего в C-коде для самого шелла. Две строки `add 0755
saaios/saai-displayd ...`/`add 0755 saaios/saai-shell ...` убраны из
`build-native-c-image.sh`'s cpio-манифеста -- переменные окружения
(`SAAI_DISPLAYD_BIN`/`SAAI_SHELL_BIN`) остаются обязательными
(`:?set ...`), как и у `SAAIOS_RUNTIME_BIN`/`SAAIOS_CONSOLE_BIN`
раньше, только ради provenance-хеша в `INPUTS.SHA256`.

Цепочка отказоустойчивости (ADR-009) не пришлось трогать вообще --
она уже была рассчитана на "бинарь может отсутствовать": если
`/data/saaios/system/saai-shell` когда-нибудь пропадёт (например
чистый `/data` без первоначального бутстрапа), `spawn_shell()`'s
`Command::spawn()` падает чисто, `launch_shell()`'s собственный
`RestartBudget`/`SHELL_RESTART_LIMIT` (3 попытки/60с) отдаёт
управление вверх -- `saai-displayd` завершается с
`SHELL_FAILURE_EXIT_CODE`, что `native-init.c`'s UI-слот уже
интерпретирует как "перезапустить displayd", и после ЕГО собственного
бюджета (`UI_RESTART_BUDGET`) откатывается на `drm-splash`
(остаётся в ramdisk безусловно, как и был). Тот же самый safety net,
что уже страховал падающий `saai-shell` до этого переезда -- ничего
нового не добавлено, просто путь сменился.

## Bootstrap-порядок (важно для безопасности процедуры)

Собранные бинарники физически скопированы на устройство в
`/data/saaios/system/` через `file-recv`/SSH **до** пересборки/
перепрошивки образа -- иначе первая же загрузка с новым `native-init.c`
не нашла бы их вообще нигде (ни в ramdisk, ни на `/data`) и упала бы
сразу в `drm-splash`-fallback. Тот же порядок, которым, судя по всему,
изначально переезжали `saai-appd`/`saai-entityd`/`saaios-runtime`.

## Test

Host: `cargo build`/`cargo test -p saai-displayd` в обеих
конфигурациях (по умолчанию и `--features panther-hardware`) --
2+1+2+1 тестов зелёные, ничего не сломалось (изменение -- один
литерал пути). `cargo build`/`test -p saai-shell` -- не тронут этим
ADR вообще, `saai-shell` не знает собственный путь на диске. Кросс-
компиляция `native-init.c` (`zig cc`) чиста.

Device: бутстрап-копирование на `/data/saaios/system/` подтверждено
(хеши совпадают с только что собранными бинарниками). Полная
пересборка образа: `RAMDISK_SZ` упал с 6697051 до 5593794 байт
(~1.1МБ освобождено) -- подтверждает, что оба бинарника
действительно ушли из cpio. Живая проверка после `fastboot flash
init_boot_a` + перезагрузки -- следующий шаг сразу после этого ADR.

## Последствия

- Единственная оставшаяся причина трогать `fastboot flash` --
  сам `native-init.c`, kernel-модули, `drm-splash` (последний
  fallback, намеренно остаётся в ramdisk безусловно -- ADR-009's
  "always available" инвариант) и сами системные `.ko`/прошивки.
  Все будущие правки `saai-shell`/`saai-displayd` теперь катятся тем
  же `file-recv`/SSH циклом, что уже применялся весь сегодняшний
  день для живой проверки перед пересборкой -- разница в том, что
  теперь это можно оставить КАК ЕСТЬ, без завершающей перепрошивки.
- `docs/os/ideas.md`'s пункт про этот переезд закрыт -- убран из
  списка.

## Ссылки

- `os/targets/panther/src/native-init.c` -- `DISPLAYD_PATH`,
  `start_saai_displayd()`.
- `services/saai-displayd/src/main.rs` -- `SAAI_SHELL_PATH`.
- `os/targets/panther/build-native-c-image.sh` -- убранные cpio-строки.
- ADR-009 -- исходная UI-слот отказоустойчивость, переиспользована
  без изменений.
- ADR-014 -- `saai-displayd` запускает `saai-shell` как дочерний
  процесс, откуда единственный хардкод пути и требовал правки.
- ADR-074 -- прецедент переезда `saaios-runtime` на `/data`, тот же
  паттерн применён здесь к последним двум занимающим ramdisk
  компонентам.
