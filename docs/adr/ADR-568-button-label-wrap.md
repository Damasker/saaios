# ADR-568: Button long labels wrap to two lines

## Статус

Принято, 2026-10-13. Host. dest-no-lock stays off. Unlock stays
proven. Experimental. Not Visual v1 sign-off. Not VUI-09
Acceptance.

## Нумерация

После ADR-567 следующий свободный номер — **568**. Не S33. Не 426
(занят на `wip/hdi-lab`).

## Контекст

VUI-02 acceptance: long labels wrap; they do not clip. ADR-107 did
this for `SemanticText` in the gallery renderer. Section 6.6 still
required Button: one line preferred; long labels wrap to two and
grow visual height; type size never shrinks. Field / DataRow /
Metric / Disclosure / StatusIndicator stay open. Font compare stays
a device task. ADR-213 thin tuning is not this slice.

## Решение

1. `wrap_words` is greedy, host-testable, and never splits a word.
   `fits` is the caller's measure.
2. Button wraps at two lines. One-line visual height stays
   `CONTROL_VISUAL_HEIGHT`; two lines use `TWO_LINE_ROW_HEIGHT`.
   Hit height is at least `MIN_TOUCH_TARGET`.
3. Accessibility keeps the full label. Gallery paint is not wired.
   Do not shrink `TextRole::Label`.

## Откат

Remove `wrap_words` and the Button height helpers.

## Ссылки

- ADR-107, ADR-567
- `docs/os/sprints/VISUAL-ROADMAP.md` VUI-02
