# ADR-079: сторонние приложения могут отправлять уведомления (S30)

## Статус

Принято, 2026-09-15.

## Контекст

ADR-062 (S21) построила обобщённую модель уведомлений
(`entity_type = "saaios.notification"`, свойства `{body, kind,
dismissed}`), но единственным продюсером был сам `saai-shell`
(низкий заряд батареи). Ни один сторонний (sandboxed) процесс не мог
создать уведомление -- `saai-entityd`'s собственный сокет
(`/run/saaios/entityd.sock`) замаскирован песочницей (ADR-020),
недоступен изнутри `saai-appd`'s `sandbox::apply()` в принципе (в
отличие от `wayland_socket`/`portal_socket`, которые точечно
"пробиваются" сквозь маску `/run`).

## Решение

Новый вариант запроса портала (`crates/saai-portal-protocol`) --
`ClientRequest::PostNotification { title, body }`, новая capability
`notifications.post` (ADR-020's словарь, `Capability::NotificationsPost`
в `saai-appd`). `saai-shell`'s `portal_server.rs` -- та же граница
доверия, что уже опосредует `clipboard.read`/`write`/`portal.open_file`
(S07 Change 7) -- теперь дополнительно опосредует создание сущности:
проверяет capability, затем сама вызывает `self.entityd.create_entity(
selected_space_id, "saaios.notification", title, {body, kind})`, тем
же путём, что `check_low_battery()` уже использует для системных
уведомлений.

Намеренно НЕ добавлено: возможность приложению самому задать `kind`.
Сервер сам подставляет `kind = "app:<app_id>"` -- сторонний процесс
не может притвориться системным уведомлением (`low_battery` и т.п.),
подставив чужой `kind` в запросе. Тот же принцип, что уже
использовался для `client_name` в SSH-пейринге (ADR-074) -- то, что
идентифицирует источник, сервер решает сам, а не берёт со слов
клиента.

Первый реальный продюсер -- `org.saaios.mahjong` (ADR-072/075):
`capabilities` в манифесте сменились с `[]` на
`["notifications.post"]` (первый раз в проекте, когда стороннее
приложение реально запрашивает capability, а не остаётся на пустом
списке ради обхода экрана согласия), и игра шлёт "Маджонг: победа!"
ровно один раз -- из того самого тапа, который завершает последнюю
пару (`handle_tap`'s match-arm, не из `redraw()`/`won()`, которые
повторялись бы каждый кадр, пока доска остаётся собранной).

## Test

Host: `cargo test -p saai-shell` -- 41/41 (2 новых: отказ без
capability, отказ при недоступном `saai-entityd` -- оба реальных
fail-closed пути, не просто happy path). `cargo test -p saai-appd`
-- 34/34 (`capability_round_trips_through_as_str` покрывает и новую
`NotificationsPost`). `cargo clippy` чист для всех трёх задетых
крейтов (`saai-portal-protocol`, `saai-appd`, `saai-shell`,
`saai-mahjong`).

Device: подтверждено вживую, без пересборки образа -- бинарники
(`saai-appd`, `saai-shell`, `saai-mahjong`) и манифест подменены на
устройстве через `file-recv`/SSH. Живой прогон целиком: переустановленный
манифест с новой capability -> первый в проекте реальный экран
согласия для стороннего приложения ("Отправка уведомлений") -> тап
"Разрешить" -> игра доиграна до победы -> уведомление "Маджонг:
победа!" реально появилось во "Входящие", подтверждено пользователем
визуально на экране.

## Последствия

- `space.entities.write` (ADR-020's существующий словарь) остаётся
  фактически неиспользуемым для sandboxed-приложений -- у entityd
  нет собственного пути внутрь песочницы вообще, только через
  портал. Не расширено этим ADR -- уведомления были конкретной,
  ограниченной целью S30, не общий "дать приложениям прямой доступ
  к Entity Graph" рефакторинг.
- `title`/`body` ограничены `MAX_NOTIFICATION_TITLE_BYTES`(200)/
  `MAX_NOTIFICATION_BODY_BYTES`(1024) -- та же "не дать недоверенному
  процессу распухать память `saai-shell`" причина, что уже
  ограничивает `MAX_CLIPBOARD_TEXT_BYTES`.
- `PortalServer::poll()`'s сигнатура выросла на два параметра
  (`entityd`, `selected_space_id`) -- тот же способ, каким уже туда
  попадали `apps_by_pid`/`apps_grants`/`clipboard`; не новый паттерн,
  просто ещё двумя внешними состояниями больше.

## Ссылки

- `crates/saai-portal-protocol/src/lib.rs` -- `PostNotification`,
  `ResponseResult::NotificationPosted`, размерные лимиты.
- `services/saai-appd/src/lib.rs` -- `Capability::NotificationsPost`.
- `services/saai-shell/src/portal_server.rs` -- обработка запроса,
  два новых теста.
- `services/saai-mahjong/src/notify.rs` -- первый реальный клиент.
- `apps/mahjong/manifest.toml` -- `capabilities` больше не `[]`.
- ADR-062 -- исходная модель уведомлений, переиспользована как есть.
- ADR-020 section 8 / S07 Change 7 -- портал как единственная точка
  входа для sandboxed-приложений, тот же паттерн применён здесь.
