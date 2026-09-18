# ADR-143: VUI-07 PIN setup — ContextHeader, not Surface bar

## Статус

Принято, 2026-09-19. «Новый PIN-код» sits under a real `ContextHeader`
instead of a Surface header fill. Host-green; panther hash follows
flash. Do not type digits. Do not tap **Готово**. Leave with
**Отмена**. Lock-surface unlock stays `draw_lock_pin_entry` dots.
Remote pairing and Space detail are not this slice.

## Нумерация

После ADR-142 следующий свободный номер — **143**. Не S33.

## Контекст

VUI-07 remaining setup chrome after consent. ADR-133 already previews
digits through a Password `Field`, but `draw_pin_setup` still fills a
Surface strip (`INTENT_HEADER_HEIGHT`) and paints the Field as raw
text at `header.y+140`. Other modals now use `ContextHeader`. Keypad
hit-test (`pin_keypad_rect`) stays. Do not save a PIN. Do not restyle
lock unlock. Do not restyle remote pairing in the same slice.

## Decision

1. **`Frame::PinSetup` carries `ContextHeader`**. Section title is
   `PIN`. Space name is the live selected space. No invented lifecycle.
2. **`draw_pin_setup` paints that header** with the same status-layer
   inset as consent. The Field preview sits in the first stacked row
   below it (`stacked_row_rect(0)`), still masked. No Surface header
   bar.
3. **Keys and controls stay** `pin_keypad_rect`. Labels stay
   digits / ⌫ / Отмена / Готово.

## Consequences

- PIN setup is a real section, not a diagnostic overlay.
- Empty placeholder remains `Введите новый PIN (минимум 4 цифры)`.
- Rollback: restore the Surface header fill and `header.y+140` text.

## Verification

Host: heading is `{space} · PIN`; no Surface bar at y=210. Panther:
Система → PIN-код opens the empty keypad below the clock. Only
**Отмена** is tapped.
