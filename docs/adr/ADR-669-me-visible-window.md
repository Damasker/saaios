# ADR-669: paint only the visible Sistema window

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
is `1bb214c6…` (ADR-668). dest-no-lock stays absent. Hung
8960 killed (`R` on `surface=me`). Unlock stays the proven
path (ADR-564). Experimental. PIN stays null. Leave Сейчас.
Not Visual v1 sign-off. Not VUI-09 Acceptance. Wrap
Acceptance stays unchecked.

## Нумерация

После ADR-668 следующий свободный номер — **669**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The user still saw Sistema hang. Camp snapshot and deferred
facts were not enough: the first fill and every Me touch-down
compiled and painted the whole SettingRow list. Touch-down
also wiped `me_row_cache` and ran `me_facts()` on the input
path. pid 8960 stayed `R` after `idle_ok=1`.

## Решение

1. Do not invalidate or recapture on Me touch-down.
2. `me_window_range` keeps only rows that intersect the
   content pane. Paint and hits compile that window.
3. Host test: 40 rows at rest fit in ≤12 compiled rows. Do
   not flash this ADR. dest-no-lock stays off.

## Откат

Revert the window + touch-down keep. Live `1bb214c6…` still
hangs on the first full Me paint.

## Ссылки

- ADR-666, ADR-668, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
