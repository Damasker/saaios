# ADR-569: Field labels and support wrap

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-568 следующий свободный номер — **569**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-02 acceptance: long labels wrap; they do not clip. ADR-568 did
this for Button. Section 6.7 Field still needed the persistent
label and help/error to wrap. The editor stays one line: PIN,
password, and intent are single-line. DataRow / Metric /
Disclosure / StatusIndicator stay open. Font compare stays a
device task. ADR-213 thin tuning is not this slice.

## Решение

1. Field label wraps at two lines through `wrap_words`.
2. Error wins over help. Support wraps at two lines.
3. Editor value does not wrap. Hit height stays
   `MIN_TOUCH_TARGET`. Accessibility keeps the full label.
4. Gallery paint is not wired. Do not shrink `TextRole`.

## Откат

Remove Field wrap helpers and `with_help`.

## Ссылки

- ADR-568, ADR-107
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
