# ADR-521: unknown workflow status is «Нет статуса»

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-520 следующий свободный номер — **521**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-519 hid unknown `error_kind`. `task_status_text` still printed
an unknown `status` token. WorkflowStatus has a closed set; a spoofed
value is not a new named state.

## Решение

1. Known statuses keep their Russian copy.
2. Any other `status`, including empty-token leftovers, uses the same
   «Нет статуса» as a missing status.
3. No new label. No Retry / Replan chrome.

## Откат

Restore `Some(other) => other.to_string()`.

## Ссылки

- ADR-296, ADR-519
- `docs/os/sprints/WORK-ROADMAP.md` WORK-08
