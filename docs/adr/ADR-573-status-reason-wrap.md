# ADR-573: StatusIndicator reason wraps below the label

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-572 следующий свободный номер — **573**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-02 acceptance: long labels wrap; they do not clip. ADR-572 did
this for Disclosure. Section 6.4 still required the reason to wrap
below the label rather than shrinking it. Compact still hides the
reason. Gallery paint is not this slice. Font compare stays a
device task. ADR-213 thin tuning is not this slice. Do not check
the wrap Acceptance box: paint is unwired.

## Решение

1. Label wraps at two lines through `wrap_words`.
2. Visible reason wraps at two lines below the label. Compact
   `reason_lines` is empty.
3. Accessibility keeps the full label. Do not shrink `TextRole`.

## Откат

Remove StatusIndicator wrap helpers.

## Ссылки

- ADR-572, ADR-107
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
