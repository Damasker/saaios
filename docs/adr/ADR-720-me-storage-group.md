# ADR-720: group Sistema storage under Устройство

## Статус

Принято, 2026-10-15. Host. Live `/data/saaios/system/saai-shell`
is `90a5c1a6…` (ADR-719). dest-no-lock stays absent. Unlock
stays the proven path (ADR-564). Experimental. PIN stays
null. Leave Сейчас. Not Visual v1 sign-off. Not VUI-09
Acceptance. Wrap Acceptance stays unchecked. Do not type PSK.

## Нумерация

После ADR-719 следующий свободный номер — **720**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

«Это устройство» joined the live model and `df` usage on one
row. ADR-126 already names «Хранилище» as its own Device
readout. dest-no-lock stays off. Do not start the shell from
COM13. Do not open modem devices. Do not type PSK. Section
«Устройство» stays. Device «Экран» stays. Section «Связь»
stays.

## Решение

1. «Это устройство» keeps the live model only.
2. Storage becomes a «Хранилище» readout under Устройство.
   The label is the existing S15 / ADR-126 name. No new
   names.
3. Host test: the summary does not contain `ГБ`; «Хранилище»
   sits between «Это устройство» and «Узел». Do not flash
   this ADR. dest-no-lock stays off.

## Откат

Join model and storage back into «Это устройство». Live
`90a5c1a6…` still dumps them on one row.

## Ссылки

- ADR-126, ADR-059, ADR-300, ADR-719, ADR-564
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-06 / VUI-09
