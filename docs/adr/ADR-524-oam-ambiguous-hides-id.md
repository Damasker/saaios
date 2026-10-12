# ADR-524: Ambiguous OAM does not print the action id

## Статус

Принято, 2026-10-12. Host. No shell flash. dest-no-lock stays.
Lock remains a defect track (ADR-512).

## Нумерация

После ADR-523 следующий свободный номер — **524**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

Quick actions already say «неоднозначно». The Object View permission
line still appended the dotted `action_id`. That is a schema token,
not a board label.

## Решение

1. Ambiguous permission copy is «неоднозначно».
2. Quick action title is the first candidate's human title, not
   `action_id`. Reason stays «неоднозначно».
3. NOW still omits Ambiguous from the suggestion.

## Откат

Restore `Неоднозначно: {action_id}`.

## Ссылки

- ADR-119, ADR-477, ADR-522
