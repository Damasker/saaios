# Sprint 07 — Capability, sandbox и portals

## Паспорт

- Состояние: `In progress`.
- Зависит от: S05 (`Done`), S06 (`Done`).
- Архитектурные решения: ADR-005, ADR-018, ADR-020.
- Рабочий fallback: S06 image, приложения запускаются без sandbox (текущее
  поведение) до появления `saai-appd`'s spawn с namespaces/seccomp.

## Goal

Приложение получает доступ только к тем действиям и данным, на которые явно
согласился пользователь -- не потому что оно "доверенное", а потому что
mount/network namespace и seccomp-фильтр физически не дают ничего другого.

## Current state

- manifest (S05, ADR-018) уже объявляет `capabilities`, но `saai-appd`
  никогда не превращает их в разрешения -- doc-comment прямо это
  фиксирует;
- `saai-appd`'s `spawn()` -- голый `Command::new(...).env_clear()`, без
  mount/network/syscall изоляции;
- вся система работает под единственным UID (root), без
  `/etc/passwd`/multi-user модели;
- on-device spike (ADR-020) подтвердил: `CLONE_NEWNS`/`NEWNET`/`NEWIPC`/
  `NEWUTS` доступны на этом ядре (`6.1.157-android14`), `CLONE_NEWPID`/
  `CLONE_NEWUSER` -- физически недоступны (`CONFIG_PID_NS`/
  `CONFIG_USER_NS` не собраны в ядро);
- `saai-shell` уже показывает permission-подобные экраны через SUI/action
  (root.sui, ADR-017) -- инфраструктура для экрана согласия уже есть.

## Scope

Входит: фиксированный capability vocabulary v1; install-time
accept/decline-all экран согласия с хранением, привязанным к hash
запрошенного набора; process isolation через `unshare()` (mount/net/ipc/
uts) в `saai-appd`'s `pre_exec()`; filesystem mediation через
tmpfs-маскирование общих каталогов приложений и entity store с explicit
bind-mount назад собственных путей; network mediation как бинарный
`net.internet` переключатель через `CLONE_NEWNET`; syscall mediation через
seccomp-bpf (`seccompiler`) с deny-list, компенсирующим отсутствие PID/
user namespace; system permission surface в `saai-shell`; точка входа
portal-протокола для clipboard и file picker (без детального wire-формата).

Не входит: per-capability runtime-prompt (только install-time, целиком);
per-host network allowlist (только вкл/выкл); полноценный `pivot_root` в
собранный с нуля rootfs; cgroups/resource limits; недоверенные сторонние
приложения за пределами уже существующего demo-app; полный wire-протокол
portal (только точка входа "кто через кого").

## Change

1. ADR-020 (capability vocabulary, effective grants, sandbox mechanism) --
   принят, включая on-device namespace spike.
2. **Готово (2026-09-10, `45f3c26`).** Capability vocabulary как typed
   enum/schema (`Capability`, 6 записей из ADR-020), negative tests
   (неизвестное имя, дубликат, capability вне vocabulary) -- расширение
   существующей manifest-валидации в `saai-appd`. `AppManifest.capabilities`
   сменил тип `Vec<String>` -> `Vec<Capability>` (ни один другой файл ещё
   не читал это поле, проверено перед сменой типа).
3. **Готово (2026-09-10, `2269973`).** Хранилище effective grants
   (`GrantStore`, `services/saai-appd/src/grants.rs`):
   `{app_id, granted, requested_hash}` отдельно от manifest, с re-consent
   при изменении запрошенного набора capability. Тот же fsync-temp ->
   rename -> fsync-directory паттерн, что уже использует
   `saai-entity-store` (S06). Путь -- `var/appd/grants/`, вне любого
   каталога, который `saai-appd` когда-либо смонтирует внутрь sandbox
   приложения (иначе приложение могло бы выдать себе разрешения само).
4. **Готово (2026-09-10, `ef33ff3`, `24acbb1`).** `saai-shell`: экран
   согласия при launch, использует существующий SUI/action путь.
   `saai-appd`'s Launch теперь проверяет `GrantStore::covers()` и
   отвечает `ConsentRequired{app_id, requested}` вместо запуска, если
   решения ещё нет; новый `DecideConsent{app_id, accept}` записывает
   ответ через `GrantStore::record_decision` и отвечает
   `ConsentDecided{app_id, granted}` -- демон сам знает запрошенный
   набор из manifest, клиент только говорит да/нет. `AppSummary`
   получил `requested_capabilities`/`consent_needed` для проактивного
   показа. `saai-shell` рисует второй полноэкранный `ui_tree`-layout
   (`consent_view()`, рядом с `root_view()`) с заголовком, списком
   разрешений (человекочитаемые русские подписи) и кнопками
   "Разрешить"/"Отклонить"; hit-test читает то же дерево, что рисует
   `draw()` -- то же правило ADR-017, что и у панели вкладок.

   По пути найден и исправлен баг в уже закоммиченном `GrantStore`:
   пустой список capability считался непокрытым (нет записи -> нет
   согласия), из-за чего `effective_capabilities()` падал на `.expect()`
   -- пустой запрос теперь тривиально покрыт без обращения к диску.
5. **Готово (2026-09-10, `0807124`, fix `9af6a9e`).** `saai-appd`'s
   `spawn()`: новый модуль `sandbox.rs`, вызывается из
   `Command::pre_exec()` (после `fork()`, до `execve()`, единственный
   поток дочернего процесса) -- `unshare(NEWNS|NEWIPC|NEWUTS)` (плюс
   `NEWNET`, кроме случая `net.internet` в выданных grants), затем
   `mount("/", MS_REC|MS_PRIVATE)`, затем tmpfs-маскирование
   `apps_dir`/`apps_data_dir` с explicit bind-mount назад собственных
   code/data путей приложения (code -- read-only, data -- read-write),
   затем tmpfs-маскирование `var/entities` без reveal вообще (S06's
   `saai-entityd` -- единственная санкционированная дверь). Деградирует
   в отсутствие изоляции без ошибки, если процесс не root (`saai-appd`'s
   собственный test suite на R620 работает под непривилегированным
   пользователем и не может создавать namespaces/mount) -- реальное
   устройство (`native-init.c`) всегда root, так что production launch
   этим fallback'ом не затронут.

   На первой же реальной попытке запуска (`org.saaios.demo-surface`)
   найден и физически продиагностирован (throwaway C-спайк
   `mount-mask-spike.c`) баг: маскирование родителя *после*
   bind-mount'а каталога на самого себя не сохраняет каталог
   достижимым по тому же пути -- маскирование родителя пересобирает
   путь заново через новый (пустой) tmpfs. Исправлено (коммит
   `9af6a9e`, второй спайк `mount-mask-spike2.c` подтвердил каждую
   проверку) через промежуточный scratch-путь вне маскируемого дерева:
   bind-mount оригинала в `/run/saaios/.sandbox-reveal/<pid>/{code,data}`
   *до* маскирования, затем `MS_MOVE` scratch-mount на место после
   маскирования и создания свежего каталога внутри нового tmpfs. Тем же
   коммитом устранена побочная утечка: пустой per-pid scratch-каталог
   больше не остаётся в `/run/saaios/.sandbox-reveal/` после запуска.

   Отдельно расследовано и закрыто как false alarm: реальный
   `saai-demo-surface` завершается почти сразу после запуска
   (`state: stopped`, не `crash_limited`) -- временная инструментация
   (`Stdio::inherit()` вместо `Stdio::null()`, откачена после проверки)
   показала в `/run/saai-appd.log`, что это штатное поведение
   тестового клиента (подключился, отрисовал один кадр, вышел), не
   баг sandbox.
6. **Готово (2026-09-10, `0807124`).** seccomp-фильтр (`seccompiler`) с
   deny-list из ADR-020 (`ptrace`/`kill`/`tkill`/`tgkill`, `mount`/
   `umount2`/`pivot_root`/`chroot`, `unshare`/`setns`, `reboot`/
   `init_module`/`delete_module`/`kexec_load`/`personality`), применяется
   тем же `pre_exec()` сразу после mount-масок. Известный, задокументированный
   в коде пробел: `clone` сам по себе не запрещён (иначе сломалось бы
   обычное создание потоков любым рантаймом на платформе, включая Rust's
   std) -- аргумент-специфичная фильтрация только `CLONE_NEW*`-флагов вне
   рамок этой версии.
7. Portal точка входа: `saai-shell` посредник для `clipboard.read/write` и
   `portal.open_file`, без реализации UI выбора файла целиком (достаточно
   протокольной точки входа и одного сквозного сценария).
8. Физическая приёмка на устройстве: fault injection, cold reboot, negative
   tests (попытка выйти за границы sandbox из демо-приложения).

## Test

- unit/schema: capability vне vocabulary, дубликат, невалидное имя;
- unit: grants store -- новый hash требует re-consent, старый переиспользуется;
- host integration: demo-app запущен с sandbox, подтверждена mount-маска
  (не видит чужой app/data каталог, не видит raw entity store файлы);
- negative/fault injection: попытка `ptrace`/`kill` чужого PID из sandboxed
  процесса блокируется seccomp (EPERM/SIGSYS, не успех); попытка выйти в
  сеть без `net.internet` не проходит (нет маршрута, не просто медленно);
- device: полный install→consent→launch цикл на Pixel 7; cold reboot не
  сбрасывает и не обходит выданные grants.

## Acceptance criteria

- приложение без `net.internet` физически не может открыть сетевое
  соединение (не firewall-предупреждение, а отсутствие маршрута);
- приложение не видит на диске каталоги других приложений и сырые файлы
  entity store, только собственные code/data пути и разрешённые сокеты;
- `ptrace`/`kill`/`tgkill`/`mount`/`reboot`/`unshare` из sandboxed процесса
  завершаются ошибкой, не успехом;
- обновление manifest, расширяющее capabilities, требует нового согласия
  прежде следующего запуска -- старое согласие не покрывает новый набор;
- отказ от согласия на установке не ломает установку остальных приложений
  и не оставляет процесс в частично разрешённом состоянии.

## Threat / privacy impact

Впервые вводится реальная граница между "приложение попросило" и
"приложение может" -- до этого ADR это было чисто декларативным полем.
Explicit deny-list (не allow-list) для seccomp означает, что новый опасный
syscall, не попавший в список, остаётся разрешён по умолчанию -- список
пересматривается отдельными ADR-правками по мере находок, не считается
закрытым перечнем на старте. Отсутствие PID/user namespace -- постоянное,
не временное ограничение этого конкретного ядра; оба класса угроз, которые
эти неймспейсы обычно закрывают, явно перечислены в Известных ограничениях
ADR-020, не молчаливо приняты.

## Rollback

Откатить `saai-appd`'s `spawn()` к текущему (без namespaces/seccomp) --
приложения продолжают работать, просто без изоляции, тем же путём, что и
до этого спринта. Grants store игнорируется, если откат происходит --
не требует миграции назад.

## Evidence

Заполняется по каждому Change только после зелёных host/device проверок.

Change 2: `Capability` enum задаёт ровно 6 записей ADR-020's vocabulary.
`AppManifest::parse_toml` теперь возвращает `ManifestError::
UnknownCapability` для синтаксически корректного, но не входящего в
vocabulary имени (`net.bluetooth` -- негативный тест) -- отдельно от уже
существующих `InvalidCapability` (формат) и `DuplicateCapability`.
Round-trip тест подтверждает `Capability::parse(cap.as_str()) == Some(cap)`
для всех 6 вариантов. 25 unit + 1 integration теста `saai-appd` зелёные,
fmt/clippy `-D warnings` чисты по всему workspace на R620.

Change 3: `GrantStore` подтверждён 6 unit-тестами: без записи -- нет
покрытия и нет разрешений; принятое решение выдаёт ровно запрошенный
набор; отклонённое решение хранится как настоящий пустой grant (не
"неизвестно") и не требует повторного вопроса при том же запросе;
расширение запрошенного набора делает старое согласие непокрывающим
именно для нового (более широкого) набора, но не трогает уже принятое
согласие на прежний, более узкий набор; изменение порядка capability в
запросе не меняет hash и не требует повторного согласия; запись с
несовпадающей схемой отклоняется. 31 unit + 1 integration тест
`saai-appd` зелёные, fmt/clippy `-D warnings` чисты по всему workspace,
кросс-компиляция под `aarch64-unknown-linux-musl`
(`build-saai-appd.sh`) подтверждена (`ARM aarch64`, `statically
linked`) на R620.

Change 4: host-level -- новый integration тест
(`daemon_gates_launch_on_consent_and_records_decision`) прогоняет
install -> `ConsentRequired` -> `decide_consent` (decline) ->
`Launched` -> `list` против настоящего запущенного демона; отдельно
перезапущен уже существующий host-скрипт
`tests/application-wayland-host.sh` (S05's acceptance evidence) после
этого изменения -- PASS без изменений, подтверждая, что launch с
пустым capability-списком (реальный demo-app) по-прежнему проходит
напрямую, без consent-гейта. 32 unit + 2 integration теста `saai-appd`,
12 тестов `saai-shell` (4 новых -- обе половины кнопочного ряда экрана
согласия, мёртвая зона заголовка, fallback подписи) зелёные;
fmt/clippy `-D warnings` чисты по всему workspace; кросс-компиляция
`saai-appd` и `saai-shell` под `aarch64-unknown-linux-musl` подтверждена
на R620.

Физически на устройстве (Pixel 7, panther), через hot-swap уже
запущенных `saai-appd`/`saai-shell` (без пересборки `init_boot` --
`saai-shell` живёт в нём, но подмена того же паттерна, что уже
использовался для `saai-displayd`/`saai-entityd` в предыдущих раундах,
сработала одинаково для обоих): т.к. интерфейс `saai-shell` привязан
к единственному реальному `org.saaios.demo-surface` (карточка
"Saai Demo"), а его манифест намеренно не меняется (см. Change 4's
appd-коммит), для визуальной проверки временно (1) удалён текущий
`org.saaios.demo-surface` (только код, `var/apps` данные сохраняются
API'ем store), (2) переустановлен под тем же id с manifest, запрошившим
`net.internet`/`clipboard.read`/`space.entities.read`, (3) после
проверки удалён и переустановлен обратно оригинальный manifest
(`capabilities = []`) -- подтверждено идентичное `consent_needed:
false`, `requested_capabilities: []` до и после. Прямые запросы к
`appd.sock` через одноразовый zig-cc пробник (`sockprobe`, не
закоммичен) подтвердили server-side: `install` вернул
`consent_needed: true` с точным списком; `launch` вернул
`consent_required` с тем же списком, не `launched`. Через реальный
touch-путь `saai-shell` (тап по карточке "Saai Demo") пользователь
физически увидел экран согласия с заголовком "Saai Demo запрашивает
доступ", списком разрешений и кнопками "Разрешить"/"Отклонить" --
подтверждено визуально дважды (сначала decline, затем на новом наборе
capability -- accept), лог показал `consent declined for
org.saaios.demo-surface` и позже `consent accepted for
org.saaios.demo-surface`; после accept demo-app физически запустился и
получил фокус (собственная тестовая поверхность). Устройство
возвращено к исходному состоянию (проверено `list()`).

Change 5/6: реальный namespace/seccomp-спайк (тот же, что лёг в основу
ADR-020) подтвердил на этом ядре доступность `CLONE_NEWNS`/`NEWNET`/
`NEWIPC`/`NEWUTS` и присутствие `CONFIG_SECCOMP`/`CONFIG_SECCOMP_FILTER`
до начала этой работы. Wiring в `spawn()` прошёл через один реальный
баг (см. Change 5 выше) и один false alarm (демо-приложение штатно
завершается после одного кадра), оба закрыты физической диагностикой
на устройстве, а не предположением.

Финальная (после коммита `9af6a9e`) headless-проверка на R620: `cargo
build`/`cargo test -p saai-appd` (33 unit + 2 integration теста,
включая `sandbox::tests::
non_root_degrades_to_no_isolation_instead_of_failing_the_launch`),
`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
-- -D warnings`, `cargo test --workspace` (все крейты зелёные) и
`tests/application-wayland-host.sh` (S05's acceptance-скрипт) -- все
чисто. Кросс-компиляция `build-saai-appd.sh` под
`aarch64-unknown-linux-musl` подтверждена (`file`: `ELF 64-bit LSB
executable, ARM aarch64, ..., statically linked, stripped`; sha256
`fc331e31f1f9c57ed40c2ebe521bf9e3f8d17f5c1b306778b93ca1bd24224526`).

Физически на устройстве: бинарь развёрнут тем же hot-swap-паттерном
(scp -> HTTP -> `wget` на телефоне -> атомарный `mv` поверх
`/data/saaios/system/saai-appd` -> `kill` старого pid -> проверка
sha256 нового pid'а через `/proc/<pid>/exe`), хэш подтверждён
идентичным на обоих концах на каждом шаге. Два последовательных
запуска реального `org.saaios.demo-surface` через прямой запрос к
`appd.sock` (`sockprobe`) оба вернули `launched` -> чистый `stopped`;
`/run/saaios/.sandbox-reveal/` не получил ни одного нового
scratch-каталога ни после первого, ни после второго запуска (три
каталога-свидетеля старого бага, `583`/`601`/`616`, оставлены как есть
-- `/run` это tmpfs, переживёт только до перезагрузки). Финальный
`list()` подтвердил, что состояние реального приложения не тронуто:
`requested_capabilities: []`, `consent_needed: false`, `state:
"stopped"`.

Не проверено отдельным устройство-специфичным тестом в этом раунде
(оставлено как открытый пункт): явное подтверждение, что `CLONE_NEWNET`
физически блокирует сетевое соединение изнутри sandboxed процесса, и
что seccomp-фильтр реально возвращает `EPERM` на запрещённый syscall
(например `ptrace`) из живого процесса, а не только успешно
устанавливается. Текущее свидетельство для обоих механизмов --
косвенное (спайк ADR-020 для namespace-семантики; отсутствие ошибки
при `install_seccomp_filter()` во время успешного запуска для BPF).
Это отдельный пункт fault-injection'а в Change 8, не закрывается этим
изменением.
