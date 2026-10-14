# ADR-682: one camp-log read for Sistema fields

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `ce4c374b…` (ADR-681). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-681 следующий свободный номер — **682**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-681 walks `/proc` once. `me_facts()` still called
`read_camp_owner_log()` once per cellular field — about forty
opens of the same owner log — then `cellular_row_with_session`
opened it five more times for the session suffix. dest-no-lock
stays off. Do not start the shell from COM13. Do not open
modem devices. Do not log the raw camp file.

## Решение

1. Field readers take the already-filtered owner-fact text.
   `me_facts` reads `CAMP_OWNER_LOG` once when camp is current
   and applies every parser to that string.
2. `cellular_row_with_session` reads the same file once for
   data-call, signal, link, phy, and barring.
3. Host test: DNS / DNS6 / registration / radio / SIM from one
   fixture string. Do not flash this ADR. dest-no-lock stays
   off.

## Откат

Revert the `log: &str` readers. Live `ce4c374b…` still opens
the camp log once per field on the live-probe turn.

## Ссылки

- ADR-667, ADR-680, ADR-681, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
