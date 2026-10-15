# ADR-726: defer chrome sysfs off the first Sistema paint

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `e6af707f…` (ADR-725). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather.

## Нумерация

После ADR-725 следующий свободный номер — **726**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-725 fills «Местность» on the camp turn. The first Sistema
paint still read kernel / uptime / boot / adapters before Me
could commit. dest-no-lock stays off. Do not start the shell
from COM13. Do not open modem devices. Do not type PSK.
Section «Связь» stays. Device «Экран» stays.

## Решение

1. `me_facts_chrome` does not call `apply_me_chrome_probes`.
   First paint is settings only. Kernel / uptime / boot /
   adapters stay empty for one frame.
2. The next event-loop turn runs `apply_me_chrome_probes` onto
   held facts and stores `me_chrome_hold`. Camp `/proc` still
   waits for `me_full_ready`. Recapture still skips sysfs when
   kernel is held.
3. Host test: first chrome has no `apply_me_chrome_probes`;
   chrome turn stores the hold; idle blocks on
   `chrome_pending`. Do not flash this ADR. dest-no-lock
   stays off.

## Откат

Call `apply_me_chrome_probes` from `me_facts_chrome`. Live
`e6af707f…` still reads sysfs during the first paint.

## Ссылки

- ADR-672, ADR-716, ADR-724, ADR-725, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
