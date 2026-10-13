# ADR-571: Metric labels wrap

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-570 следующий свободный номер — **571**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-02 acceptance: long labels wrap; they do not clip. ADR-570 did
this for DataRow. Section 6.9 Metric still needed the label (and
optional verified stamp) to wrap. The numeric value stays one line
with its unit. Disclosure / StatusIndicator stay open. Font compare
stays a device task. ADR-213 thin tuning is not this slice.

## Решение

1. Metric label wraps at two lines through `wrap_words`.
2. `verified_at` wraps at two lines. Known value plus unit stay
   one line.
3. Accessibility keeps the full label. Gallery paint is not
   wired. Do not shrink `TextRole`.

## Откат

Remove Metric wrap helpers.

## Ссылки

- ADR-570, ADR-107
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
