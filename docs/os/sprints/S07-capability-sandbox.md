# Sprint 07 — Capability, sandbox и portals

## Паспорт

- Состояние: `Ready`.
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
2. Capability vocabulary как typed enum/schema, negative tests (неизвестное
   имя, дубликат, capability вне vocabulary) -- расширение существующей
   manifest-валидации в `saai-appd`.
3. Хранилище effective grants: `{app_id, granted, manifest_capabilities_hash}`
   отдельно от manifest; re-consent при изменении запрошенного набора.
4. `saai-shell`: экран согласия при установке/первом запуске, использует
   существующий SUI/action путь.
5. `saai-appd`'s `spawn()`: `pre_exec()` с `unshare(NEWNS|NEWNET|NEWIPC|NEWUTS)`
   (кроме `NEWNET` при `net.internet`), private/rslave root, tmpfs-маскирование
   `/data/saaios/{apps,packages}`, `/data/saaios/var/apps`,
   `/data/saaios/var/entities` с explicit bind-mount назад собственных путей
   приложения.
6. seccomp-фильтр (`seccompiler`) с deny-list из ADR-020, применяется тем же
   `pre_exec()` после `PR_SET_NO_NEW_PRIVS`.
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
