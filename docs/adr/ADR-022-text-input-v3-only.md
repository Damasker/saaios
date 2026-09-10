# ADR-022: text-input-v3 без input-method-v2 -- xkbcommon не работает на этом ядре ни в каком виде

## Статус

Принято, 2026-09-10.

## Контекст

S08's DoR/ADR-021 оставили "text-input/OSK-стратегия" как отдельный,
нерешённый вопрос перед Change 3, конкретно из-за ADR-012's известного
ограничения: `seat.add_keyboard(XkbConfig::default(), ...)` падает с
`SIGTRAP` на реальном Pixel 7 (`HandleAliasDef`, `keycodes.c:406`),
поэтому `panther-hardware`-сборка `saai-displayd` вообще не подключает
`wl_keyboard`.

Смысл Change 3 -- дать сторонним GTK4/Qt-приложениям способ получать
текстовый ввод. Стандартный Wayland-механизм для этого без физической
клавиатуры -- `zwp_text_input_v3` (клиентская сторона: приложение
включает/выключает текстовое поле, получает `commit_string`) в паре с
`zwp_input_method_v2` (сторона IME/экранной клавиатуры -- в нашем случае
`saai-shell`, единственный клиент, которому имеет смысл быть системной
клавиатурой).

## Находка

Чтение исходника smithay 0.7.0 (`wayland/input_method/mod.rs:217`)
показало, что обработчик `zwp_input_method_manager_v2::Request::
GetInputMethod` -- то есть сам факт того, что `saai-shell` пытается
СТАТЬ input method'ом, до чтения/записи единого символа -- безусловно
делает `seat.get_keyboard().unwrap()`. На seat'е без `wl_keyboard`
(любая `panther-hardware`-сборка) это `panic!()`, убивающий весь
`saai-displayd` в момент, когда `saai-shell` попытается включить
экранную клавиатуру.

Чтобы обойти это, пробовалось дать seat'у рабочую клавиатуру -- но не
такую, что использует полноценный `xkeyboard-config` (большая задача,
которую ADR-012 сознательно отложил). Физический спайк на R620
(throwaway `xkb-spike`, кросс-компилирован тем же zig-тулчейном, что уже
используется для `saai-displayd`/`saai-shell`) проверил семь вариантов
компиляции keymap на настоящем Pixel 7, каждый в отдельном
forked-процессе:

- (A) `rules=evdev model= layout=us` -- ADR-012's исходная падающая
  комбинация -- **CRASHED (signal 5)**, подтверждает, что баг всё ещё
  воспроизводится;
- (B) `rules=base model= layout=us` -- **CRASHED**;
- (C) полностью пустой RMLVO (`rules=model=layout=variant=""`) --
  **CRASHED**;
- (D) `rules=evdev model=pc105 layout=us` -- **CRASHED**;
- (E) `xkb::Keymap::new_from_string()` с написанным вручную,
  полностью самодостаточным минимальным keymap-текстом (никакого
  обращения к rule-файлам вообще) -- **CRASHED**;
- (F) то же самое (E), но с флагом `CONTEXT_NO_DEFAULT_INCLUDES`
  (явно пропускает попытку добавить отсутствующий
  `/usr/share/X11/xkb` как include path -- и это действительно
  сработало: характерная строка
  `xkbcommon: ERROR: failed to add default include path` пропала) --
  **тем не менее CRASHED**;
- (G) `new_from_names` с пустым RMLVO + `CONTEXT_NO_DEFAULT_INCLUDES`
  -- **CRASHED**.

Все семь вариантов упали с одним и тем же `SIGTRAP`, включая (F) и (E)
-- случаи, которые НЕ обращаются ни к одному внешнему xkb-data файлу
вообще. Это **опровергает** узкий диагноз ADR-012 ("неполный/повреждённый
xkb-data, конкретно alias-файл для evdev+qwerty"): реальная причина
глубже -- судя по всему, в самой сборке `libxkbcommon.a` (ADR-008's
cross-sysroot) или в её несовместимости с этим окружением на уровне,
который проявляется при ЛЮБОЙ попытке скомпилировать keymap, не только
через RMLVO-поиск по файлам.

## Решение

1. **`zwp_text_input_manager_v3` -- реализован.** Через smithay's
   `TextInputManagerState` (`services/saai-displayd/src/main.rs`) --
   чтение исходника (`wayland/text_input/mod.rs`) подтвердило, что
   `GetTextInput` трогает только per-seat `TextInputHandle`/
   `InputMethodHandle` user-data, никогда `Seat::get_keyboard()`. Фокус
   ведётся из `activate_toplevel()` тем же способом, что уже сделан для
   `wl_data_device_manager` в ADR-021 -- независимо от клавиатуры,
   `TextInputHandle::leave()`/`set_focus()`/`enter()`.
2. **`zwp_input_method_manager_v2` -- НЕ реализован.** Осознанно
   отложенный, задокументированный пробел, не тихо пропущенный: у
   текстовых полей GTK4/Qt сейчас нет системной экранной клавиатуры,
   отвечающей на `enable()`. Сами поля не ломаются и не роняют
   compositor -- подтверждено и хостовым тестом, и физически на
   устройстве (см. Evidence).
3. **ADR-012 не отменяется, но его диагноз сужен пост-фактум**: причина
   глубже "неполного xkb-data" -- вероятно баг/несовместимость самой
   собранной библиотеки `libxkbcommon.a`. Починка "по-настоящему"
   (в любом смысле, не только xkb-data) остаётся отдельной, не
   оценённой по объёму задачей -- не блокирует этот Change, но и не
   делается здесь.
4. Три пути для будущего input-method-v2 (ни один не выбран этим ADR,
   каждый требует отдельного решения при появлении бюджета):
   - расследовать и исправить настоящую причину xkbcommon-краша
     (сначала нужно ЛОКАЛИЗОВАТЬ баг -- в этой версии библиотеки,
     сборке или окружении -- то, что не было сделано этим спайком);
   - написать input-method-v2 вручную, в обход smithay's модуля, без
     единого обращения к `KeyboardHandle`;
   - форкнуть/пропатчить smithay, чтобы `GetInputMethod` не требовал
     клавиатуру безусловно.

## Последствия

Положительные:

- GTK4/Qt-приложения (Change 7+) могут пользоваться text-input-v3 уже
  сейчас без риска уронить compositor;
- находка про глубину xkbcommon-бага физически подтверждена, не
  предположение -- если/когда OSK понадобится по-настоящему, следующая
  сессия начинает не с нуля, а с точного списка того, что уже
  проверено и не сработало;
- host-тест (`text_input_global.rs`) и throwaway on-device пробник
  оба используют один и тот же сценарий (bind -> get_text_input ->
  enable -> commit -> roundtrip) -- падение compositor'а на реальном
  железе было бы поймано, не просто предположено безопасным по коду.

Цена решения:

- текстовые поля в будущих GTK4/Qt demo-приложениях (S08) физически не
  смогут получить текст от системной клавиатуры, пока не выбран один
  из трёх путей выше -- известное, а не молчаливое ограничение;
- `saai-shell`'s собственный будущий OSK (для СВОЕГО UI, не для сторонних
  приложений) не затронут этим решением вообще -- ADR-012 уже
  установил, что это реализуется touch-хиттестами напрямую, без
  Wayland-протокола.

## Ссылки

- `docs/adr/ADR-012-skip-keyboard-on-panther.md`
- `docs/adr/ADR-021-toolkit-compat-spike.md`
- `docs/os/sprints/S08-toolkit-compatibility.md`
- `services/saai-displayd/src/main.rs` (`TextInputManagerState`,
  `activate_toplevel()`)
- `services/saai-displayd/tests/text_input_global.rs`
- Throwaway-спайки не закоммичены: `/tmp/xkb-spike/`,
  `/tmp/text-input-probe/` на R620.
