# ADR-201: VUI-09 — leftover text sizes stay below Caption

## Статус

Принято, 2026-09-20. HEAD shell stays `3850427a…`. Badge, gallery
kicker/swatch, and app-tile labels stay named leftovers. Each is
smaller than Caption (36). Mapping them onto Caption would overflow
the marks: a 40px tab badge, the gallery swatch remainder, the
kicker gap, and centered app-tile names. `compile()` stays v1. No
new daemon. Open Приложения, do not launch an app, leave Сейчас.

## Нумерация

После ADR-200 следующий свободный номер — **201**. Не S33.

## Контекст

ADR-197/198 mapped near leftovers onto `Label`/`Caption`. Four
marks remained named (ADR-188): `TAB_BADGE_PX` 24, `GALLERY_KICKER_PX`
24, `GALLERY_SWATCH_PX` 18, `APP_TILE_LABEL_PX` 26. Visual v2 next
action was to map them or keep them on an explicit list. Caption is
the smallest remaining `TextRole` at Pixel 7 scale 3. Inventing a
smaller role is not this slice. 7-tap gallery stays session-blocked.

## Decision

1. **Do not map onto `TextRole`.** Host test
   `leftover_text_sizes_stay_below_caption` locks the four values
   below Caption and names `GALLERY_SWATCH_PX` in the production
   leftover list.
2. **Do not 7-tap.** Gallery kicker/swatch stay host-locked by size.
   Tab badge stays unpainted unless a real unread count exists.
3. **Device cell is the apps grid.** Footer Приложения is a public
   destination (ADR-199). Open it, do not tap a tile, leave Сейчас.
4. **Do not reflash.** Paint is unchanged.

## Consequences

- Leftover text stays an explicit named list, not a silent RGB-style
  allowlist. Rollback: drop the size test. Next: Experimental→Stable
  after Visual v1, or operator-approved lock/display/cold-boot. Still
  not Visual v1 sign-off.

## Verification

Host: leftover PX < Caption 36; `GALLERY_SWATCH_PX` named in
production. Panther: Приложения grid on HEAD, no app launch; leave
Сейчас.
