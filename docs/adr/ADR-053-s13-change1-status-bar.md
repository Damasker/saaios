# ADR-053: S13 Change 1 -- реальный статус-бар (часы, Wi-Fi, батарея)

## Статус

Принято, 2026-09-13.

## Контекст

Статус-бар (`wlr-layer-shell`-поверхность, ADR-015) с самого своего
появления закрашивал себя одним сплошным цветом (`render::BACKGROUND`)
и не рисовал ничего сверх этого -- его внутренний namespace
(`"saai-shell-statusbar-test"`) честно признавался в этом. S13's
Definition of Ready назвал это первым, наименее рискованным Change:
чисто аддитивно, ни один существующий экран не трогается.

## Решение

- `wifi_is_up()` читает `/sys/class/net/wlan0/operstate`, `true` только
  при значении `up` -- физически подтверждено, что честно возвращает
  `down`, когда `wpa_supplicant` не ассоциирован ни с одной сетью, а не
  предполагает подключение по факту существования интерфейса.
- `read_battery()` читает `/sys/class/power_supply/maxfg/{capacity,
  status}` -- узел назван `maxfg` (fuel gauge), не `battery`, что
  подтверждено листингом `/sys/class/power_supply/` на устройстве.
- `current_time_string()` шеллится в `busybox date +%H:%M` вместо
  добавления `chrono` в реальные зависимости (`chrono` в этом крейте
  сейчас только `dev-dependencies`, для тестов) -- одна короткая
  строка раз в секунду не оправдывает новый рантайм-зависимость на
  size-оптимизированном `pixel7` профиле.
- `render::draw_status_bar()` -- время слева, Wi-Fi и батарея
  справа (batteries flush с правым краем, Wi-Fi слева от неё), тем же
  измерь-потом-раздели подходом, что `draw_text_centered` уже
  использует для центрирования.
- `present_status_bar()`/`refresh_statusbar_if_due()` копируют
  структуру уже существующих `present_lock_surface`/`check_deep_idle`
  и `refresh_apps_if_due` соответственно -- обновление раз в секунду
  (`STATUSBAR_REFRESH_INTERVAL`), без push-механизма (ни один из трёх
  источников его не даёт дёшево).
- Namespace layer-поверхности переименован из
  `"saai-shell-statusbar-test"` в `"saai-shell-statusbar"` -- содержимое
  теперь настоящее, суффикс `-test` больше не соответствует
  действительности.

## Test

Host: `cargo test -p saai-shell` -- 22/22 зелёные, ни один существующий
тест не тронут (не тестировался напрямую -- чтение `/sys` и `date`
не мокается ради этого небольшого Change, соответствует уже принятому
уровню покрытия остальных `Command`-based мест в этом файле). `cargo
clippy -p saai-shell` чист. Кросс-компиляция чиста, hash
`747865ee07d33a1a97c13bb0806e20057823822f980579a45918793d191b0278`.

Device (`cp && mv` + `chmod`, ADR-042): физически подтверждено --
пользователь увидел время, индикатор Wi-Fi и процент батареи в
статус-баре сразу после разворачивания.

## Последствия

- Namespace-переименование -- чисто косметическое, не влияет ни на
  один протокольный контракт (namespace layer-surface не проверяется
  нигде в тестах или в `saai-displayd`).
- Три новых свободных функции (`wifi_is_up`, `read_battery`,
  `current_time_string`) читают только уже существующие, доступные
  только для чтения источники -- новых полномочий или каналов не
  вводится (см. S13 DoR, "Threat / privacy impact").

## Ссылки

- `docs/os/sprints/S13-user-interface-completion.md` -- Definition of
  Ready, Change 1 из 5.
- ADR-015 -- исходное появление layer-поверхности как протокольного
  теста без реального содержимого.
- `services/saai-shell/src/main.rs` -- `present_status_bar`,
  `refresh_statusbar_if_due`, `wifi_is_up`, `read_battery`,
  `current_time_string`.
- `services/saai-shell/src/render.rs` -- `draw_status_bar`.
