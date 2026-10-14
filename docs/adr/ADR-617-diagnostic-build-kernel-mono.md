# ADR-617: diagnostic build and kernel paint MonoBody

## Статус

Принято, 2026-10-14. Host. No shell flash. dest-no-lock stays
off. Unlock stays proven. Experimental. Not Visual v1
sign-off. Not VUI-09 Acceptance. Not VUI-05 Acceptance. Wrap
Acceptance stays unchecked.

## Нумерация

После ADR-616 следующий свободный номер — **617**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-615/616 painted live observation technical IDs as MonoBody.
DevSurface still painted «Сборка» and «Ядро» through
`diagnostic_row` (Body). Those values are identifiers. Space
name, ContextFrame, and model stay Body. Copy stays off. Do
not 7-tap. Leave Сейчас. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. `diagnostic_technical_row` uses `with_technical_value`.
2. Live DevSurface uses it for «Сборка» and «Ядро» only.

## Откат

Restore `diagnostic_row` for those two facts.

## Ссылки

- ADR-580, ADR-616
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-05
