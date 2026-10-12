# ADR-519: NOW current work order and Failed class copy

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-518 следующий свободный номер — **519**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-517/518 ordered «Далее». «Продолжается» and `spaces.focus` still
used list order, so a Normal Running Task could sit above a High one.
Failed copy printed unknown `error_kind` strings; taskd already maps
those to `FailureClass::Unknown`.

## Решение

1. `in_progress_work` sorts High → Normal → Low, then UUID. Space
   focus keeps the first of that list.
2. `failed_status_text` names only the WORK-06 classes. Any other
   `error_kind` is «Ошибка», same as `unknown`.

## Откат

Restore filter-only `in_progress_work` and `Some(other) => other`.

## Ссылки

- ADR-296, ADR-507, ADR-517
- `docs/os/sprints/WORK-ROADMAP.md` WORK-05 / WORK-06 / WORK-08
