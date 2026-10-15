# ADR-728: defer locality JSON off the camp `/proc` turn

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `7fe7ab4b…` (ADR-727). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather.

## Нумерация

После ADR-727 следующий свободный номер — **728**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-727 fills kernel / uptime / boot / adapters on the chrome
turn. The camp `/proc` turn still read widget JSON before the
owner log could start. dest-no-lock stays off. Do not start
the shell from COM13. Do not open modem devices. Do not type
PSK. Section «Связь» stays. Device «Экран» stays.

## Решение

1. The camp turn does not call `apply_me_locality_probes`.
   `/proc` + CP leave «Местность» empty for one more frame.
2. The next event-loop turn reads widget JSON onto held facts
   and stores it on `me_chrome_hold`. The owner log still waits
   for `me_camp_log_ready`. Recapture still skips the read when
   the string is held.
3. Host test: camp turn has no `apply_me_locality_probes`;
   locality turn stores the hold; idle blocks on
   `locality_pending`. Do not flash this ADR. dest-no-lock
   stays off.

## Откат

Call `apply_me_locality_probes` from the camp turn. Live
`7fe7ab4b…` still reads the cache during `/proc`.

## Ссылки

- ADR-724, ADR-726, ADR-727, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
