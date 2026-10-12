# ADR-514: CurrentObject attention carries ObjectRef

## Статус

Принято, 2026-10-12. Host crate. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-513 следующий свободный номер — **514**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-506 labels CurrentObject. Task and Notification items still left
`object: None`. SuggestedAction already carries `ObjectRef`. Follow
on NOW uses the source id; the projection should still name the
focused object when relevance is CurrentObject.

## Решение

1. `apply_context` sets `item.object` from `context.object_id` only
   when relevance is CurrentObject.
2. CurrentContext / Related / Background / Global stay without a
   fabricated ref.
3. Health stays Global and has no object.

## Откат

Stop writing `item.object` in `apply_context`.

## Ссылки

- ADR-506, ADR-513
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-05
