# ADR-513: live NOW object fills AttentionContext.object_id

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock SIGTRAP remains an open defect (ADR-512), not this slice.

## Нумерация

После ADR-512 следующий свободный номер — **513**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-506 already carries focused `object_id`. Live helpers set it
`None`, so NOW's ObjectSummary entity was only CurrentContext
(space match). Suggestion already uses `entities.first()` — the same
entity `now_object_summary()` paints.

## Решение

1. `attention_context_from_space` takes an optional focused id.
2. `live_attention_context` / `project_live_attention` / NOW paint
   and `now_sections()` use `selected_entities.first().id`.
3. Space-only tests still pass `None` and stay CurrentContext.

## Откат

Pass `None` for focused; restore space-only CurrentContext.

## Ссылки

- ADR-506, ADR-508, ADR-509, ADR-512
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-05
