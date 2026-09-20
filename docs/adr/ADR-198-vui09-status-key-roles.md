# ADR-198: VUI-09 — map status, keys, and remaining near leftovers onto TextRole

## Статус

Принято, 2026-09-20. HEAD shell is `3850427a…`. Status time/battery use `Label`. Key labels, Orb
menu, object related line, and gallery heading use `Caption`.
Decision overlay buttons use `Label`. Not Title. Badge, gallery
kicker/swatch, and app-tile labels stay named — Caption would
overflow those marks. `compile()` stays v1. No new daemon. Do not
7-tap. Do not send an intent. Leave intent with Отмена. Leave Сейчас.

## Нумерация

После ADR-197 следующий свободный номер — **198**. Не S33.

## Контекст

ADR-197 mapped ActionCard and tab labels. ADR-188 leftovers that sit
within a few physical pixels of Label (42) or Caption (36) were
still named constants: status time 44, battery 40, keys 32, Orb
menu 32, related 28, decision 40, gallery heading 32. Badge 24,
kicker 24, swatch 18, and app-tile 26 are smaller than Caption and
live inside tight marks.

## Decision

1. **Status time and battery:** `role_px(TextRole::Label)`. Wi-Fi
   stays Caption.
2. **Keys:** `role_px(TextRole::Caption)` in `paint_keyboard_keys`.
3. **Decision buttons:** `Label`. Orb menu and related: `Caption`.
   Gallery heading: `Caption` (semibold weight kept).
4. **Keep named:** `TAB_BADGE_PX`, `GALLERY_KICKER_PX`,
   `GALLERY_SWATCH_PX`, `APP_TILE_LABEL_PX`.
5. **Flash** this paint. Do not 7-tap. Open intent only to prove
   keys, then Отмена.

## Consequences

- Live chrome type scale is Label/Caption except four tight marks.
  Rollback: restore the seven constants. Next: those four leftovers,
  or Experimental→Stable after Visual v1. Still not Visual v1
  sign-off.

## Verification

Host: leftover const names gone for the mapped paths. Panther:
Сейчас status Label-sized; intent keys Caption-sized; Отмена; leave
Сейчас.
