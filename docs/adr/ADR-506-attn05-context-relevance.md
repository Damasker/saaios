# ADR-506: ATTN-05 ContextFrame relevance

## Статус

Принято, 2026-10-12. Host crate + live NOW paint/hits. Inbox order
unchanged. No shell flash. Lock SIGTRAP stays open.

## Нумерация

После ADR-505 следующий свободный номер — **506**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-123 left relevance as Global. ContextFrame already chooses the
active space (ADR-087). ATTN-05 asks for deterministic relevance
without AI ranking.

## Решение

1. `AttentionContext` carries frame `space_ids`, `primary_space_id`,
   and optional focused `object_id`.
2. `project_with_context` labels CurrentObject / CurrentContext /
   RelatedContext / Background. Empty context stays Global.
3. Background stays in the projection and Inbox. NOW and Orb drop it
   (`surfaces.now/orb = false`). Health stays Global.
4. Live Сейчас paint/hits use `selected_space_id` + `context_frame`.
   `inbox_source_ids` stays the ATTN-03 Task-then-Notification order.
5. No magic score. Third-party `priority` still cannot become Critical.

## Откат

Call `project_with_health` only; drop `AttentionContext`.

## Ссылки

- ADR-087, ADR-123
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-05
