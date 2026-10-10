# ADR-434: VUI-09 — compose `ContextHeader` paint reads `layout_v2()`

## Статус

Принято, 2026-10-10. Consent / pair headings already paint from the
compiled slot (ADR-433). Intent, PIN setup, and Wi-Fi password still
used `paint_context_header(content)`. Those headings now sit in the
same generated `ContextHeader` as the Field, with the Field margin.
Keyboard keys stay a formula (ADR-232). PIN stays null. Do not type
intent. Leave PIN setup with Отмена. Leave Сейчас. Not Visual v1
sign-off.

## Нумерация

После ADR-433 следующий свободный номер — **434**. Не S33.

## Контекст

Compose Field already docks through `overlay_field_rect_with`
(ADR-221). The header leaf was a full-width Fill, so paint kept the
content-rect formula. That was a second tree for the same overlay
family.

## Decision

1. **Slot.** `v2_compose_node` insets `ContextHeader` with `width/22`,
   same as the Field. Status invents no action.
2. **Paint.** Intent, PIN setup, and Wi-Fi password look up that
   slot. Inset matches NOW (150/2400).
3. **Public example.** `intent-public.sui` is header plus Status
   `Field`. No `Keyboard` — that name stays privileged. Still
   Experimental.
4. **Not this slice.** List / apps headers stay
   `paint_context_header(content)`. Keyboard keys, gallery page, and
   lock idle/wake stay formulas.

## Consequences

- Compose heading and Field share `layout_v2`. Hits are unchanged.
  Rollback: restore the unpadded header leaf and
  `paint_context_header(content)`. Still not Visual v1 sign-off.

## Verification

Host: public `ContextHeader` is `(49, 0, …)` on 1080 and matches the
Field x; Field stays Status. Panther: do not flash; leave Сейчас.
