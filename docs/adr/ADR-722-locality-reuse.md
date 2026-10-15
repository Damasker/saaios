# ADR-722: reuse Sistema locality status; skip recapture JSON

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `7a127ccc…` (ADR-721). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-721 следующий свободный номер — **722**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-721 grouped «Хранилище» on panther. Recapture still called
`locality_me_status` from `me_system_sections`, which reads the
widget cache JSON. dest-no-lock stays off. Do not start the
shell from COM13. Do not open modem devices. Do not type PSK.
Do not invent weather. Section «Связь» stays. Device «Экран»
stays.

## Решение

1. Chrome probes fill `facts.locality` once. When chrome values
   are held, they skip the read unless locality was cleared.
2. `me_system_sections` paints that held string. A «Местность»
   tap clears the hold so the next paint re-reads.
3. Host test: held locality survives apply. Do not flash this
   ADR. dest-no-lock stays off.

## Откат

Call `locality_me_status` from `me_system_sections` again. Live
`7a127ccc…` still reads the cache on recapture.

## Ссылки

- ADR-718, ADR-721, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-09
