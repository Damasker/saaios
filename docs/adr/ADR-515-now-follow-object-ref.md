# ADR-515: NOW attention tap follows ObjectRef

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-514 следующий свободный номер — **515**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-514 puts `ObjectRef` on CurrentObject items. NOW tap still opened
the Task/Notification source id. ATTN-07: tap opens the object, does
not execute.

## Решение

1. `now_attention_follow_id` prefers `item.object` when it is an
   Entity. Space refs do not become a view id.
2. Without a ref, follow stays the source Task / Notification /
   SuggestedAction id. Health still has none.

## Откат

Restore source-only match in `now_attention_follow_id`.

## Ссылки

- ADR-508, ADR-514
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-05 / ATTN-07
