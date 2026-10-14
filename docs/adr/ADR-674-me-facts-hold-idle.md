# ADR-674: hold first Sistema facts and skip idle mid-fill

## Статус

Принято, 2026-10-14. Host. Live `/data/saaios/system/saai-shell`
is `828b6ff9…` (ADR-673). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked.

## Нумерация

После ADR-673 следующий свободный номер — **674**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-673 committed chrome then `surface=me`. The observation
turn called `me_facts()` again. The idle timer also fired
after the first long capture and requested a session lock
(`Cannot destroy session lock while locked`). pid 11339
stayed `R`. dest-no-lock stays off. Do not start the shell
from COM13.

## Решение

1. First capture clones `MeFacts` into `me_facts_held`. The
   observation turn reuses that snapshot and does not shell
   out again.
2. `check_idle_timeout` returns while `me_facts_pending` or
   `me_obs_pending`. Capture refreshes `last_activity` so
   the list can settle before idle.
3. Host tests: held facts still have «Наблюдения»; idle is
   blocked while either pending flag is set. Do not flash
   this ADR.

## Откат

Revert `me_facts_held` / `me_fill_blocks_idle`. Live
`828b6ff9…` still runs `me_facts()` twice and can lock
mid-fill.

## Ссылки

- ADR-672, ADR-673, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
