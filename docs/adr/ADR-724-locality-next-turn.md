# ADR-724: defer locality JSON off the first Sistema chrome fill

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `cfe15b55…` (ADR-723). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.
Do not invent weather.

## Нумерация

После ADR-723 следующий свободный номер — **724**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-723 reuses «Местность» on recapture. The first chrome fill
still read widget cache JSON before Me could paint. dest-no-lock
stays off. Do not start the shell from COM13. Do not open
modem devices. Do not type PSK. Section «Связь» stays. Device
«Экран» stays.

## Решение

1. `apply_me_chrome_probes` does not call `locality_me_status`.
   First chrome paint leaves «Местность» empty.
2. The camp turn runs `apply_me_locality_probes` onto held
   chrome facts and stores the string on `me_chrome_hold`.
   Recapture still skips the read when the string is held.
3. Host test: chrome probes have no `locality_me_status`; camp
   turn calls locality probes. Do not flash this ADR.
   dest-no-lock stays off.

## Откат

Call `locality_me_status` from `apply_me_chrome_probes`. Live
`cfe15b55…` still reads the cache during the first chrome fill.

## Ссылки

- ADR-704, ADR-722, ADR-723, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
