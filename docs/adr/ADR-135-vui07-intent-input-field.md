# ADR-135: VUI-07 intent input — Field

## Статус

Принято, 2026-09-18. «Новое намерение» previews the draft through
`Field` (`FieldKind::Text`). Flashed panther `6239bebd…`: title
below the status layer, empty placeholder `Наберите текст…`,
keyboard unchanged, intent not sent. Consent, remote pairing,
Object View, and PIN keypad chrome are not this slice.

## Нумерация

После ADR-134 следующий свободный номер — **135**. Не S33.

## Контекст

VUI-07 next remaining surface after lock idle. ADR-132 moved the
Wi-Fi password off `draw_intent_input`; the intent composer itself
still paints title at `header.y + 40` and preview at `+130` (hidden
or clipped under the 120px PIXEL_7 status layer) and a hand-rolled
empty placeholder. `Field` already owns Text vs placeholder and
accessibility (section 6.7 / ADR-103). Store-offline `Нет связи`
was also under the status layer. Do not send an intent just to take
the screenshot. Do not restyle consent, remote pairing, Object View,
or PIN keypad chrome in the same slice.

Keep compose behavior: Cancel closes; Send still uses
`intent_submit` (persist / close-empty / KeepDraft). Keyboard
geometry stays the shared intent tree (`WifiPasswordState`'s own
doc comment).

## Decision

1. **`intent_input_field(buffer, store_connected) -> Field`**. Text.
   Label is `Новое намерение`. Placeholder `Наберите текст…` is
   distinct from an empty value. Offline sets `help` to `Нет связи`
   (not `error` — it is not a validation failure). Preview paint
   uses placeholder when empty and connected, help when empty and
   offline, `accessible_value()` when there is a draft.
2. **`draw_intent_input` takes that Field.** Title and preview sit
   at the same `+140` / `+200` inset as `draw_wifi_password`. Keys
   unchanged.
3. **No persist on screenshot.** Do not tap Send.

## Consequences

- Draft preview lives in `Field`, same header inset as the other
  keyboard surfaces.
- Offline remains named, now visible below the clock when the draft
  is empty.
- Rollback: restore `draw_intent_input(title, buffer, status)` with
  the hand-rolled placeholder.

## Verification

Host: field is Text; empty is distinct from the placeholder; offline
help is `Нет связи` not an error. Panther `6239bebd…`: `Новое
намерение` below the clock, placeholder `Наберите текст…`, keyboard
typeable. Intent was not sent for the screenshot.
