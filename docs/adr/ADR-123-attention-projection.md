# ADR-123: Attention Projection — derived, not a second truth

## Статус

Принято, 2026-09-18. ATTN-00 (этот ADR + roadmap) — docs.
ATTN-01 — host crate: Task + Notification → AttentionProjection.
NOW/Inbox/Orb ещё не переключены. Pixel не меняется.

## Нумерация

После ADR-122 следующий свободный номер — **123**. Не S33.
Visual Language остаётся hardware-changing треком.

## Контекст

HIA уже задаёт Calm UI и NOW Attention. Сейчас `inbox_rows()` объединяет
WaitingConfirmation Tasks и undismissed Notifications, а Orb Attention
смотрит только на notifications. World Model / WSV2 добавят ещё источники.
Если каждый экран пишет свои `if`, появятся три определения «что срочно».

## Решение

**Attention — derived projection** над существующей правдой.

Не создавать `saaios.attention`, `attention.db`, `saai-attentiond`.

Inputs v1:

```text
saaios.task (WaitingConfirmation)
saaios.notification (undismissed)
```

Позже, отдельными slices: ContextFrame relevance, один Health source,
OAM suggested actions.

Notification (ADR-062/079) остаётся durable message. Attention не заменяет её.

Сортировка v1 воспроизводит `inbox_rows`: Tasks, затем Notifications,
порядок входа. Позже: priority → actionability → relevance → time → key.

Third-party `notifications.post` не задаёт Critical / surfaces / system kind.

Attention никогда не исполняет Action. Offline Orb остаётся выше Attention.

## Roadmap

`docs/os/sprints/ATTN-ROADMAP.md`

## Что не делается здесь

Нет AI ranking, magic score, interrupt banners, universal dismiss,
Health→Attention, switch of shell queries.

## Ссылки

- ADR-062, ADR-079, ADR-089, ADR-112, ADR-115, ADR-116, ADR-122
- `docs/os/architecture/human-interface-architecture-v2.md`
