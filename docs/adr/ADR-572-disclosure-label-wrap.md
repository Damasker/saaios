# ADR-572: Disclosure long labels wrap

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-571 следующий свободный номер — **572**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-02 acceptance: long labels wrap; they do not clip. ADR-571 did
this for Metric. Section 6.10 Disclosure still needed the label to
wrap. Hit stays at least 48; the chevron stays 16–20.
StatusIndicator stays open. Font compare stays a device task.
ADR-213 thin tuning is not this slice.

## Решение

1. Disclosure label wraps at two lines through `wrap_words`.
2. One line hits 48; two lines hit 64. Chevron mapping is
   unchanged.
3. Accessibility keeps the full label. Gallery paint is not
   wired. Do not shrink `TextRole`.

## Откат

Remove Disclosure wrap helpers.

## Ссылки

- ADR-571, ADR-107
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
