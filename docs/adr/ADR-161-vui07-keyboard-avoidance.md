# ADR-161: VUI-07 keyboard avoidance — Field above docked QWERTY

## Статус

Принято, 2026-09-19. Intent and Wi-Fi password park a real `Field`
immediately above the ADR-150 docked QWERTY. Header is
`ContextHeader`, not a Surface fill of the whole panel. Flashed
panther `3d289b66…`: NOW → Новое намерение, Field on the keys,
left with Отмена. Do not type a PSK. Do not send an intent. PIN
dialer already sits above its keys (ADR-149) and is not this slice.

## Нумерация

После ADR-160 следующий свободный номер — **161**. Не S33.

## Контекст

ADR-150 docked the keyboard and let the header `Fill`.
`draw_intent_input` still painted that Fill as Surface and drew the
preview at `header.y+140`. The field was above the keys, but the
compose chrome was a full-bleed bar, not avoidance. PIN setup already
uses `ContextHeader` + `draw_gallery_field` in the first stacked row.

## Decision

1. **`intent_field_rect`** is one stacked-row-tall card whose bottom
   meets the keyboard top. Same helper for Wi-Fi password.
2. **`ContextHeader`** names «Намерение» / «Пароль». Canvas behind it,
   not Surface fill of the Fill slot.
3. **`draw_intent_input`** matches `draw_pin_setup`: header, Field,
   then shared key paint. `draw_wifi_password` reuses that path.

## Consequences

- Focus order and interrupted workflows stay later.
- Rollback: Surface fill + `y+140` preview.

## Verification

Host: field does not intersect keys; bottom equals keyboard top.
Panther: NOW → Новое намерение, screenshot Field above QWERTY, Отмена.
