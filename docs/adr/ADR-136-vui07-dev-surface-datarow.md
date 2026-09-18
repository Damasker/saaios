# ADR-136: VUI-07 developer surface — DataRow

## Статус

Принято, 2026-09-18. Hidden HIA-20 «Диагностика» lists live facts
through static `DataRow`, flattened to `ActionCardView`. Flashed
panther `bbc11a30…`: title below the status layer, `11 показателей`,
live space/build/model/kernel/uptime rows, `ContextFrame` `пусто`,
trailing «Назад» below the fold. Object View, consent, remote
pairing, PIN keypad chrome, and Space detail are not this slice.

## Нумерация

После ADR-135 следующий свободный номер — **136**. Не S33.

## Контекст

VUI-07 next remaining surface after intent input. Wi-Fi / Bluetooth /
trusted-client lists already paint through `draw_action_row_list`
(title `header.y + 140`, status `+200`, below the 120px PIXEL_7
status layer). `Frame::DevSurface` is the last `draw_row_list`
consumer: title at `+40` and status at `+130` sit under the clock,
and each fact is one concatenated `String` (`Пространство: name
(id)`). Title and status_line are the same word `Диагностика`.
HIA-20 already owns the 7-tap gesture and the live facts; this
slice does not invent new ones. Do not wrap a new composite around
a Static `DataRow`. Do not restyle Object View, consent, remote
pairing, or PIN keypad chrome in the same slice.

Keep behaviour: silent taps until threshold; rows stay read-only;
trailing «Назад» closes; no values are changed for the screenshot.

## Decision

1. **`dev_surface_rows() -> Vec<DataRow>`**. Each live fact is a
   Static `DataRow` (label + value). Empty `ContextFrame` stays
   named `пусто`. No installed apps stays named `нет установленных
   приложений`. Nothing is invented.
2. **`diagnostic_card_from_row`** flattens to `ActionCardView` the
   same way trusted-client cards do. Action is empty — rows are not
   buttons. Trailing «Назад» stays a control card.
3. **`draw_action_row_list`**. Title `Диагностика` at `+140`.
   Status names the real row count (`N показателей`), not a second
   copy of the title. `draw_row_list` has no remaining caller.

## Consequences

- Diagnostic facts sit below the clock, label and value apart.
- HIA-20 gesture and live sources are unchanged.
- Rollback: restore concatenated `String` rows and `draw_row_list`.

## Verification

Host: Static `DataRow`; label and value stay apart; card action is
empty; status names the row count. Panther `bbc11a30…` pid 30950:
«Диагностика» below the clock, `11 показателей`, `Пространство`
`Дом (home)`, `Сборка` `f5814718df45`. No setting was cycled. PIN
was not saved.
