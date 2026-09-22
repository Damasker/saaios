# ADR-229: VUI-09 — overlay Field/decision paint reads `layout_v2()`

## Статус

Принято, 2026-09-20. Apps grid paint already reads generated
`layout_v2()` (ADR-228). Overlay Field rects already come from
`overlay_field_rect_with` (ADR-221). Decision accept/decline and
object-view action Buttons now paint from the same
`overlay_buttons_v2_source` tree hit-test uses. Keyboard keys stay
the existing formula. PIN stays null. Do not tap Разрешить, Сопряжь,
or type intent. Leave PIN setup with Отмена. Leave Сейчас. Not
Visual v1 sign-off.

## Нумерация

После ADR-228 следующий свободный номер — **229**. Не S33.

## Контекст

Hits for overlay Field and decision Buttons already generate
documents. Paint still took accept/decline from hand-built
`consent_view` / `task_confirm_view` / `object_view` child indexes,
a second tree.

## Decision

1. **Decision row.** Consent, remote-pair, and object-view action
   Buttons look up loc (`consent:accept` / `consent:decline`,
   `task_confirm:accept` / `task_confirm:decline`,
   `object-view-action:{i}`) on `layout_v2`.
2. **Field.** Compose Field paint stays `overlay_field_rect_with`.
   Keyboard keys stay the QWERTY/PIN formula.
3. **Empty object view.** Zero actions keep the header-only
   `object_view` leaf. No invented Buttons.
4. **No invented Buttons.** Consent capability cards stay Status
   stacked rows.

## Consequences

- Overlay Field/decision paint and hit-test share the generated
  tree. OrbHost and diagnostic paint stay later slices. Rollback:
  restore `consent_view` children in Frame builders. Still not
  Visual v1 sign-off.

## Verification

Host: consent accept/decline rects match the previous
`consent_view` row; Field height is non-zero. Panther: flash;
Сейчас; do not open overlays; leave Сейчас.
