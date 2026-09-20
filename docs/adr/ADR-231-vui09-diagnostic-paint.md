# ADR-231: VUI-09 — diagnostic paint reads `layout_v2_scrolled()`

## Статус

Принято, 2026-09-20. Privileged surface `diagnostic` is vocabulary.
Live DevSurface cards and Назад paint from the same generated
`compile_v2()` / `layout_v2_scrolled()` tree hit-test already uses
(ADR-224). Data rows stay read-only. Scroll clips above the first
stacked slot, matching `dev_surface_scroll_content`. Keyboard keys,
gallery page, and lock idle/wake stay formulas. PIN stays null. Do
not 7-tap gallery this slice. Not Visual v1 sign-off.

## Нумерация

После ADR-230 следующий свободный номер — **231**. Не S33.

## Контекст

ADR-230 closed OrbHost paint. Remaining named chrome paint was the
hidden diagnostic list: `DataRow` plus trailing `row back`. Hits
already read that document. Paint still used `scrolled_row_rect`.
Diagnostic has no tabs, so Me clip does not apply; the compiler now
scrolls this screen against the first stacked slot.

## Decision

1. **Same tree.** `Frame::DevSurface` builds `layout_live_v2_scrolled`
   from `diagnostic_v2_source` and paints named `diagnostic.{i}` plus
   `back`.
2. **Clip.** Rows that leave the first stacked slot are omitted from
   the tree, not invented cards. Назад stays docked.
3. **Hits unchanged.** `dev_surface_back_tapped` still reads unscrolled
   `layout_v2`. Data rows stay Status.

## Consequences

- Live diagnostic paint and Назад hits share one Node tree. Gallery
  page flips stay `next_gallery_page`. Keyboard keys stay inside the
  IME object. Lock idle/wake stay whole-surface taps. Rollback:
  restore `scrolled_row_rect` paint. Still not Visual v1 sign-off.

## Verification

Host: three-row rest matches `scrolled_row_rect`; nine-row scroll
hides `diagnostic.0` and docks `back`. `compile_v2_public()` still
rejects `diagnostic`. Panther: flash; Сейчас; do not 7-tap gallery.
