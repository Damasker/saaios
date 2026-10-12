# ADR-531: data-session suffix uses Observation labels

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-530 следующий свободный номер — **531**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`data_session_suffix` already speaks Russian for count, traffic, and
link. The data-call hop still printed `cid` / `active` / `pdp`.
Those keys already have Sistema labels: «CID», «Флаг канала»,
«Тип канала».

## Решение

1. Reuse `observation_row_label` for the three data-call fields.
2. No new cellular taxonomy. RAT acronyms in `radio_suffix` stay.

## Откат

Restore `cid` / `active` / `pdp`.

## Ссылки

- ADR-247, ADR-526
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
