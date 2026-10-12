# ADR-509: NOW section list uses ContextFrame

## Статус

Принято, 2026-10-12. Host. No shell flash. Lock SIGTRAP stays open.

## Нумерация

После ADR-508 следующий свободный номер — **509**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-506 labeled Background from ContextFrame. Live NOW paint/hits
used that context. `now_sections()` still called `project_with_health`
with an empty frame, so the section list could show other-space rows
that paint omitted.

## Решение

1. `now_attention_section_with` / `now_workflow_sections_with` take
   the same `AttentionContext` as paint.
2. Live `now_sections()` builds that context from `selected_space_id`
   and `context_frame`.
3. Tests without a context stay Global (ATTN-03 fixtures).

## Откат

Pass `None` for context; restore Global-only sections.

## Ссылки

- ADR-506, ADR-508
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-05
