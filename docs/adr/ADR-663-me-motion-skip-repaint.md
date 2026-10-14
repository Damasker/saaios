# ADR-663: skip Me full paint on motion ticks

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
is `95abe151…` (ADR-662 binary, not yet a proven flash).
dest-no-lock stays absent. Hung 5299 killed. Unlock stays the
proven path (ADR-564). Experimental. PIN stays null. Leave
Сейчас. Not Visual v1 sign-off. Not VUI-09 Acceptance. Wrap
Acceptance stays unchecked.

## Нумерация

После ADR-662 следующий свободный номер — **663**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-662 cached compile + layout. Flashing that binary
(`95abe151…`) still hung: `switched to Me` committed once,
then pid 5299 stayed `R` with FramePace `reason=motion`
`idle_ok=0` and no further blit. `tick_motion` plus compositor
`frame()` still painted every Sistema SettingRow on each
≥8ms pulse. Rasterizing that list is longer than the frame
budget, so the clock never goes idle.

## Решение

1. After the first Me paint fills `me_paint_cache`, motion
   ticks do not call `draw()` unless `me_scroll_dirty`.
2. Tab switch still draws once via `sync_pressed_tab`.
   Scroll still draws via `draw_pending_scroll`.
3. Host test: `me_motion_should_repaint` is false when the
   layout is cached and scroll is clean. Do not flash this
   ADR. dest-no-lock stays off.

## Откат

Revert the `tick_motion` Me skip. Live `95abe151…` still
hangs on Sistema until the next flash.

## Ссылки

- ADR-660, ADR-661, ADR-662, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
