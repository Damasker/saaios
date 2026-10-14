# ADR-664: fill Sistema facts on the next event-loop turn

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
is `43b8b7b3…` (ADR-663 binary). dest-no-lock stays absent.
Hung 5712 killed after `switched to Me` still left the process
`R` with no further blit. Unlock stays the proven path
(ADR-564). Experimental. PIN stays null. Leave Сейчас. Not
Visual v1 sign-off. Not VUI-09 Acceptance. Wrap Acceptance
stays unchecked.

## Нумерация

После ADR-663 следующий свободный номер — **664**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-663 skipped motion repaint after the first Me paint. The
first paint itself still ran `me_facts()` (`wpa_cli` / `df` /
camp logs) plus compile and rasterize on the same touch-up
that switched the tab. pid 5712 stayed `R` for tens of
seconds after `switched to Me`. The skip never ran because
that first `draw()` did not return.

## Решение

1. The first Sistema `draw()` paints header + tabs with an
   empty row list and sets `me_facts_pending`.
2. `me_all_rows` returns empty when the cache is missing. It
   does not call `me_facts()`.
3. The next event-loop turn captures facts, drops the empty
   paint cache, and draws the real list once.
4. Host test: missing cache yields no rows. Do not flash this
   ADR. dest-no-lock stays off.

## Откат

Revert `me_facts_pending` / empty first Me paint. Live
`43b8b7b3…` still hangs on the first Sistema draw.

## Ссылки

- ADR-660, ADR-662, ADR-663, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
