# ADR-616: live observation bearer and endpoint paint MonoBody

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Not VUI-05 Acceptance. Wrap
Acceptance stays unchecked.

## Нумерация

После ADR-615 следующий свободный номер — **616**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-615 painted IPv4 / IPv6 / DNS / CID as MonoBody.
`cellular.bearer` (`rmnet1`) and `cellular.endpoint` are the next
technical IDs still going through `with_value`. Labels stay
«Канал» and «Дескриптор». Copy stays off. Object-view fact-key
dump stays. Memory keys stay keys. Leave Сейчас. Font compare
stays a device task. ADR-213 thin tuning is not this slice.

## Решение

1. `LiveObservationFact::is_technical` includes `cellular.bearer`
   and `cellular.endpoint`.
2. No `with_copy`. Do not rename the rows.

## Откат

Remove the two keys from `is_technical`.

## Ссылки

- ADR-580, ADR-615
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
