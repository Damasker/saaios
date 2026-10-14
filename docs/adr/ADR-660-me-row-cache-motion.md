# ADR-660: Me row cache across motion ticks

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
still `eb3197c9…` (ADR-656) until the next flash. dest-no-lock
stays absent. Hung 3916 killed; respawn 4278 lock
`surface=lock` `backend=shm` `idle_ok=1`. Unlock stays the
proven path (ADR-564). Experimental. PIN stays null. Leave
Сейчас. Not Visual v1 sign-off. Not VUI-09 Acceptance. Wrap
Acceptance stays unchecked.

## Нумерация

После ADR-659 следующий свободный номер — **660**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

The user switched to Система on the ADR-656 binary. pid 3916
went `R` with FramePace `reason=motion` `idle_ok=0`. `me_all_rows`
rebuilt `me_facts()` (`wpa_cli` / `df` / camp logs) on every
Selection / `tick_motion` draw unless a drag was open. After
scroll it also dropped `me_row_cache`. Tab Selection redraws
every ≥8ms for ~180ms, so one Sistema tap piled I/O until the
shell stopped committing.

## Решение

1. `me_all_rows` always reads `me_row_cache` via `cached_me_rows`.
2. Me paint calls `capture_me_rows` first so the first draw fills
   the cache and later motion ticks clone it.
3. Keep the cache after a coalesced scroll draw. Invalidate stays
   on Me actions / weather refresh / drag start.
4. Host test: a second `cached_me_rows` call must not rebuild
   facts. Do not flash this ADR. dest-no-lock stays off.

## Откат

Revert `me_all_rows` / Me paint / scroll-cache keep. No binary
change on panther.

## Ссылки

- ADR-656, ADR-658, ADR-659, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
