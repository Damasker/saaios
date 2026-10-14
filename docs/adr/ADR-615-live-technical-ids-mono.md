# ADR-615: live observation technical IDs paint MonoBody

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Not VUI-05 Acceptance. Wrap
Acceptance stays unchecked.

## Нумерация

После ADR-614 следующий свободный номер — **615**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-580 added `with_technical_value` / MonoBody. Sistema
«Наблюдения» still painted every fact through `SettingRow::readout`
(Body). IPv4 / IPv6 / DNS / DNS6 / CID are technical IDs
(VUI-05). Ordinary rows stay Body. Copy is not this slice.
Object-view fact-key dump stays. Memory keys stay keys. Leave
Сейчас. Font compare stays a device task. ADR-213 thin tuning
is not this slice.

## Решение

1. `LiveObservationFact::as_row` uses `with_technical_value` for
   `cellular.ipv4`, `ipv6`, `dns`, `dns6`, and `dcall_cid`.
2. Other observation rows stay `with_value`. No `with_copy`.

## Откат

Restore `SettingRow::readout` for every observation row.

## Ссылки

- ADR-580, ADR-614
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
