# ADR-508: ATTN-07 one OAM suggested action

## Статус

Принято, 2026-10-12. Host crate + NOW projection hook. Inbox and Orb
unchanged. Attention never executes. No shell flash. Lock SIGTRAP
stays open.

## Нумерация

После ADR-507 следующий свободный номер — **508**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

ADR-123 deferred OAM suggested actions. NOW already has «Далее»
from pending Action / ready Task (ADR-479). ATTN-07 is one
capability+policy OAM verb on the current object, not a second
planner and not invented board labels.

## Решение

1. `one_suggested_action` keeps a candidate only when exactly one is
   permitted. Zero or two invent nothing. Empty id/title invent
   nothing.
2. `project_with_suggestion` puts that item on NOW only
   (`ActionAvailable`). Inbox and Orb stay Task/Notification/Health.
3. Shell maps OAM resolve + preflight into candidates. Tap follows
   the object; it does not run the verb.
4. Live `ToolRegistry::new()` has no provider tools, so today's
   `display.inspect` stays hidden until a real tool is registered.
   That is Unavailable, not a fake button.

## Откат

Drop `SuggestedAction`; restore `project_with_context` only.

## Ссылки

- ADR-119, ADR-123, ADR-479
- `docs/os/sprints/ATTN-ROADMAP.md` ATTN-07
