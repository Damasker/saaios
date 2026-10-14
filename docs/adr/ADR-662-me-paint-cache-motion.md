# ADR-662: Me paint cache across motion ticks

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
is `4a03a41b…` (ADR-661) until the next flash. dest-no-lock
stays absent. Hung 4652 killed after Sistema still spun `R`
on the row-cache binary. Unlock stays the proven path
(ADR-564). Experimental. PIN stays null. Leave Сейчас. Not
Visual v1 sign-off. Not VUI-09 Acceptance. Wrap Acceptance
stays unchecked.

## Нумерация

После ADR-661 следующий свободный номер — **662**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-660/661 stopped `me_facts()` on every Selection tick. The
user still hung: `switched to Me` committed once, then pid
4652 stayed `R` with FramePace `reason=motion` `idle_ok=0`.
`tick_motion` plus compositor `frame()` still called
`compile_live_v2` + `layout_v2_scrolled` for every Sistema
SettingRow on each ≥8ms pulse. One compile longer than the
frame budget never lets the clock go idle.

## Решение

1. Cache the compiled `SuiV2Screen` and last scrolled
   `LayoutNode` in `me_paint_cache`.
2. Same width/height/offset reuses the layout. Offset-only
   change relayouts the cached screen. Row invalidation drops
   both caches.
3. Host test: a second `me_paint_view_with` at the same
   offset must not compile. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Revert `me_paint_cache` / `me_paint_view`. Live binary stays
`4a03a41b…` until the next flash.

## Ссылки

- ADR-660, ADR-661, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
