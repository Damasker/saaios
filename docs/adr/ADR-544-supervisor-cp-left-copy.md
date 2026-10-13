# ADR-544: supervisor cp-left is «модем ушёл»

## Статус

Принято, 2026-10-13. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-543 следующий свободный номер — **544**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

`supervisor_phrase` printed `CP ушёл` when the modem left. `CP` is
the helper name. The same row already calls that node «модем»
(`cellular.cp_state`, `open_phrase("cp")`).

## Решение

1. Supervisor `cp-left` is «модем ушёл».
2. Wire token stays `cp-left`. Do not change modemd.
3. Other supervisor words stay. Unknown tokens stay omitted.
4. Do not send RadioState.

## Откат

Restore `CP ушёл`.

## Ссылки

- ADR-543
- `docs/os/sprints/WORLD-ROADMAP.md` WORLD-02
